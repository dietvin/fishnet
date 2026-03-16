use std::collections::HashMap;
use arrow2::{
    array::{
        Array, 
        BooleanArray, 
        DictionaryArray, 
        FixedSizeBinaryArray, 
        ListArray, 
        PrimitiveArray, 
        UInt64Array, 
        Utf8Array, new_null_array
    }, 
    chunk::Chunk, datatypes::{
        DataType, 
        Schema
    }, 
    types::NativeType
};
use uuid::Uuid;

use crate::{
    error::tables::ReadsTableError,
    read::Pod5Read
};


/// Describes the versioning status of a column in a reads table.
/// 
/// Used during schema evaluation to determine whether a missing
/// column should be treated as a hard error or a known version
/// compatibility gap. 
#[derive(Debug, Clone, PartialEq)]
enum ColumnStatus {
    /// Column was defined in the base pod5 spec and must be present
    /// in all files
    Required,
    /// Column was added in a later version. Files written by older
    /// versions do not contain this column. When absent, it is 
    /// substituted with a null array
    AddedIn(&'static str),
    /// Column was deprecated in the given pod5 spec version and will
    /// eventually be removed. Files written in newer versions my omit
    /// this column. When abscent it is substituted with a null array.
    DeprecatedIn(&'static str)
}

/// Describes a single column in the pod5 reads table schema, including its name
/// and versioning status.
struct ColumnSpec {
    /// The column name as it appears in the Arrow schema fields.
    name: &'static str,
    /// The corresponding versioning status of this column.
    status: ColumnStatus
}

/// Description of the pod5 reads table column as specified in pod5 v.0.3.33.
/// 
/// This is used during parsing to validate that all required columns are present
/// and log messages when optional columns are absent.
const COLUMN_SPECS: &[ColumnSpec] = &[
    ColumnSpec { name: "read_id",                       status: ColumnStatus::Required },
    ColumnSpec { name: "signal",                        status: ColumnStatus::Required },
    ColumnSpec { name: "channel",                       status: ColumnStatus::Required },
    ColumnSpec { name: "well",                          status: ColumnStatus::Required },
    ColumnSpec { name: "pore_type",                     status: ColumnStatus::Required },
    ColumnSpec { name: "calibration_offset",            status: ColumnStatus::Required },
    ColumnSpec { name: "calibration_scale",             status: ColumnStatus::Required },
    ColumnSpec { name: "read_number",                   status: ColumnStatus::Required },
    ColumnSpec { name: "start",                         status: ColumnStatus::Required },
    ColumnSpec { name: "median_before",                 status: ColumnStatus::Required },
    ColumnSpec { name: "tracked_scaling_scale",         status: ColumnStatus::DeprecatedIn("0.4.0") },
    ColumnSpec { name: "tracked_scaling_shift",         status: ColumnStatus::DeprecatedIn("0.4.0") },
    ColumnSpec { name: "predicted_scaling_scale",       status: ColumnStatus::DeprecatedIn("0.4.0") },
    ColumnSpec { name: "predicted_scaling_shift",       status: ColumnStatus::DeprecatedIn("0.4.0") },
    ColumnSpec { name: "num_reads_since_mux_change",    status: ColumnStatus::DeprecatedIn("0.4.0") },
    ColumnSpec { name: "time_since_mux_change",         status: ColumnStatus::DeprecatedIn("0.4.0") },
    ColumnSpec { name: "num_minknow_events",            status: ColumnStatus::Required },
    ColumnSpec { name: "end_reason",                    status: ColumnStatus::Required },
    ColumnSpec { name: "end_reason_forced",             status: ColumnStatus::Required },
    ColumnSpec { name: "run_info",                      status: ColumnStatus::Required },
    ColumnSpec { name: "num_samples",                   status: ColumnStatus::Required },
    ColumnSpec { name: "open_pore_level",               status: ColumnStatus::AddedIn("0.3.33") },
];


/// Holds strongly-typed Arrow arrays for each column of a pod5 reads table chunk.
/// 
/// This struct acts as an intermediate helper struct that handles the initial
/// parsing of the Arrow data from a raw [`Chunk`] and its [`Schema`]. It provides
/// safe row-wise (read-wise) access via [`ReadsTable::get`] and [`Iterator`].
/// 
/// Optional columns that may be absent due to pod5 spec version differences are
/// represented as null arrays, so [`ReadsTable::get`] always returns a consistent
/// [`Pod5Read`] struct regardless of which version wrote a given file.
/// 
/// Information is taken from the [pod5 source](https://github.com/nanoporetech/pod5-file-format/blob/0.3.33/docs/tables/reads.toml).
#[derive(Debug)]
pub(crate) struct ReadsTable {
    read_id_array: FixedSizeBinaryArray,
    signal_index_array: ListArray<i32>,
    channel_array: PrimitiveArray<u16>,
    well_array: PrimitiveArray<u8>,
    pore_type_array: DictionaryArray<i16>,
    calibration_offset_array: PrimitiveArray<f32>,
    calibration_scale_array: PrimitiveArray<f32>,
    read_number_array: PrimitiveArray<u32>,
    start_array: PrimitiveArray<u64>,
    median_before_array: PrimitiveArray<f32>,
    tracked_scaling_scale_array: PrimitiveArray<f32>,
    tracked_scaling_shift_array: PrimitiveArray<f32>,
    predicted_scaling_scale_array: PrimitiveArray<f32>,
    predicted_scaling_shift_array: PrimitiveArray<f32>,
    num_reads_since_mux_change_array: PrimitiveArray<u32>,
    time_since_mux_change_array: PrimitiveArray<f32>,
    num_minknow_events_array: PrimitiveArray<u64>,
    end_reason_array: DictionaryArray<i16>,
    end_reason_forced_array: BooleanArray,
    run_info_array: DictionaryArray<i16>,
    num_samples_array: PrimitiveArray<u64>,
    open_pore_level_array: PrimitiveArray<f32>,

    /// Number of rows in the table
    length: usize,
    /// Current row index (used during iteration process)
    current_index: usize,
}

impl ReadsTable {

    /// Constructs a `ReadsTable` from an Arrow [`Chunk`] and its associated [`Schema`].
    /// 
    /// The schema fields are indexed by name and used to locate each expected column
    /// within the chunks array slice, allowing for possible varying column orders.
    /// 
    /// Columns are validated against [`COLUMN_SPECS`] before any casting is attempted:
    /// * [`ColumnStatus::Required`] columns cause an error if missing in the schema.
    /// * [`ColumnStatus::AddedIn`] or [`ColumnStatus::DeprecatedIn`] columns substituted
    /// with a null array of the appropriate type and a debug message is logged to explain
    /// the version mismatch.
    /// 
    /// # Arguments
    /// * `chunk` - A chunk of Arrow data from the reads table
    /// * `schema` - The Arrow schema associated with the chunk, used to resolve
    ///              column names to array indices
    /// 
    /// # Errors
    /// * [`ReadsTableError::MissingRequiredColumn`] if a required column is absent
    /// * [`ReadsTableError::ArrayCastError`] if a column cannot be downcast to its 
    /// expected Arrow array type.
    pub(crate) fn from_chunk_and_schema(chunk: Chunk<Box<dyn Array>>, schema: &Schema) -> Result<Self, ReadsTableError> {
        // Build a name -> array index lookup from the schema fields
        let index_map = schema.fields.iter()
            .enumerate()
            .map(|(i, f)| (f.name.as_str(), i))
            .collect::<HashMap<&str, usize>>();

        let arrays = chunk.arrays();
        let length = arrays.first().map(|a| a.len()).unwrap_or(0);

        // Validate that all required columns are present
        for spec in COLUMN_SPECS {
            if !index_map.contains_key(spec.name) {
                match spec.status {
                    ColumnStatus::Required => {
                        return Err(ReadsTableError::MissingRequiredColumn(spec.name.to_string()));
                    }
                    ColumnStatus::AddedIn(version) => {
                        log::debug!(
                            "Column '{}' is absent (added in version {}). File is likely older",
                            spec.name, version
                        );
                    }
                    ColumnStatus::DeprecatedIn(version) => {
                        log::debug!(
                            "Column '{}' is absent (was deprecated in version {})",
                            spec.name, version
                        )
                    }
                }
            }
        }

        // Cast types that occur in only a single column

        let read_id_idx = index_map["read_id"];
        let read_id_array = arrays[read_id_idx]
            .as_any()
            .downcast_ref::<FixedSizeBinaryArray>()
            .ok_or_else(|| ReadsTableError::ArrayCastError { 
                index: read_id_idx, 
                reason: format!("Failed to cast 'read_id' column to FixedSizeBinaryArray") 
            })
            .cloned()?;

        let signal_idx = index_map["signal"];
        let signal_index_array = arrays[signal_idx]
            .as_any()
            .downcast_ref::<ListArray<i32>>()
            .ok_or_else(|| ReadsTableError::ArrayCastError { 
                    index: signal_idx, 
                    reason: format!("Failed to cast 'signal' column to ListArray<i32>") 
            })
            .cloned()?;

        let end_reason_forced_idx = index_map["end_reason_forced"];
        let end_reason_forced_array = arrays[end_reason_forced_idx]
            .as_any()
            .downcast_ref::<BooleanArray>()
            .ok_or_else(|| ReadsTableError::ArrayCastError { 
                index: end_reason_forced_idx, 
                reason: format!("Failed to cast 'end_reason_forced' column to BooleanArray") 
            })
            .cloned()?;

        // Cast dictionary columns

        let pore_type_array = Self::cast_dict_array(arrays, index_map["pore_type"], "pore_type")?;
        let end_reason_array = Self::cast_dict_array(arrays, index_map["end_reason"], "end_reason")?;
        let run_info_array = Self::cast_dict_array(arrays, index_map["run_info"], "run_info")?;

        // Cast primitive columns

        // Attempts to downcast a &dyn Array to PrimitiveArray<T>
        macro_rules! cast_primitive {
            ($arr:expr, $ty:ty, $name:expr) => {
                $arr.as_any()
                    .downcast_ref::<PrimitiveArray<$ty>>()
                    .ok_or_else(|| ReadsTableError::ArrayCastError { 
                        index: index_map.get($name).copied().unwrap_or(usize::MAX), 
                        reason: format!(
                            "Failed to cast '{}' to PrimitiveArray<{}>",
                            $name, stringify!($ty)
                        )
                    })
                    .cloned()
            };
        }

        // Look up the required array by its index and attempts downcasting
        macro_rules! require_primitive {
            ($name:expr, $ty:ty) => {
                cast_primitive!(&arrays[index_map[$name]], $ty, $name)
            };
        }

        // Determines whether an optional array is present, substituting it
        // with a null array if missing. Then attempts to downcast.
        macro_rules! optional_primitive {
            ($name:expr, $data_type:expr, $type:ty) => {{
                let arr = match index_map.get($name) {
                    Some(&i) => arrays[i].clone(),
                    None => new_null_array($data_type, length)        
                };
                cast_primitive!(arr, $type, $name)
            }};
        }

        let channel_array = require_primitive!("channel", u16)?;
        let well_array = require_primitive!("well", u8)?;
        let calibration_offset_array = require_primitive!("calibration_offset", f32)?;
        let calibration_scale_array = require_primitive!("calibration_scale", f32)?;
        let read_number_array = require_primitive!("read_number", u32)?;
        let start_array = require_primitive!("start", u64)?;
        let median_before_array = require_primitive!("median_before", f32)?;
        let num_minknow_events_array = require_primitive!("num_minknow_events", u64)?;
        let num_samples_array = require_primitive!("num_samples", u64)?;

        let tracked_scaling_scale_array =  optional_primitive!("tracked_scaling_scale", DataType::Float32, f32)?;
        let tracked_scaling_shift_array =  optional_primitive!("tracked_scaling_shift", DataType::Float32, f32)?;
        let predicted_scaling_scale_array =  optional_primitive!("predicted_scaling_scale", DataType::Float32, f32)?;
        let predicted_scaling_shift_array =  optional_primitive!("predicted_scaling_shift", DataType::Float32, f32)?;
        let num_reads_since_mux_change_array =  optional_primitive!("num_reads_since_mux_change", DataType::UInt32, u32)?;
        let time_since_mux_change_array =  optional_primitive!("time_since_mux_change", DataType::Float32, f32)?;
        let open_pore_level_array =  optional_primitive!("open_pore_level", DataType::Float32, f32)?;

        Ok(ReadsTable { 
            read_id_array,
            signal_index_array,
            channel_array,
            well_array,
            pore_type_array,
            calibration_offset_array,
            calibration_scale_array,
            read_number_array,
            start_array,
            median_before_array,
            tracked_scaling_scale_array,
            tracked_scaling_shift_array,
            predicted_scaling_scale_array,
            predicted_scaling_shift_array,
            num_reads_since_mux_change_array,
            time_since_mux_change_array,
            num_minknow_events_array,
            end_reason_array,
            end_reason_forced_array,
            run_info_array,
            num_samples_array,
            open_pore_level_array,
            length,
            current_index: 0
        })
    }

    /// Downcasts an Arrow array at the given index to a [`DictionaryArray<i16>`].
    ///
    /// Used for dictionary-encoded string columns (`pore_type`, `end_reason`, `run_info`),
    /// where values are stored as integer keys into a shared string dictionary.
    ///
    /// # Arguments
    /// * `arrays` - The full slice of arrays from the chunk.
    /// * `index`  - The position of the target column within the slice.
    /// * `name`   - The column name, included in the error message on failure.
    ///
    /// # Errors
    /// Returns [`ReadsTableError::ArrayCastError`] if the array at `index` is not a
    /// `DictionaryArray<i16>`.
    fn cast_dict_array(arrays: &[Box<dyn Array>], index: usize, name: &str) -> Result<DictionaryArray<i16>, ReadsTableError> {
        arrays[index]
            .as_any()
            .downcast_ref::<DictionaryArray<i16>>()
            .ok_or_else(|| ReadsTableError::ArrayCastError { 
                    index, 
                    reason: format!("Failed to cast '{}' to DictionaryArray<i16>", name)
                }
            )
            .cloned()
    }

    /// Retrieves a dictionary-encoded string value from a `DictionaryArray<i16>` at the given index.
    ///
    /// # Arguments
    /// * `dict_array` - The dictionary array to extract the value from.
    /// * `index` - Row index in the array.
    ///
    /// # Returns
    /// * `Some(String)` if the value exists.
    /// * `None` if the value is null or decoding fails.
    fn get_dict_value(dict_array: &DictionaryArray<i16>, index: usize) -> Option<String> {
        if dict_array.is_null(index) {
            None
        } else {
            let key = dict_array.keys().value(index);
            if let Some(utf8_array) = dict_array
                .values()
                .as_any()
                .downcast_ref::<Utf8Array<i32>>() 
            {
                Some(utf8_array
                    .value(key as usize)
                    .to_string()
                )  
            } else {
                None
            }

        }
    }

    /// Retrieves a value from a `PrimitiveArray<T>` at the given index, or returns `None` if null.
    ///
    /// # Type Parameters
    /// * `T` - The native type of the primitive array.
    ///
    /// # Arguments
    /// * `array` - The array to access.
    /// * `index` - The row index.
    ///
    /// # Returns
    /// * `Some(value)` if the value exists.
    /// * `None` if the value is null.
    fn get_primitive<T: NativeType>(array: &PrimitiveArray<T>, index: usize) -> Option<T> {
        if array.is_null(index) {
            None
        } else {
            Some(array.value(index))
        }
    }

    /// Extracts a row from the reads table and returns it as a `Pod5Read`.
    ///
    /// Converts Arrow arrays into typed fields and constructs the read metadata
    /// using helper structs such as `Pore`, `TrackedScaling`, and `Calibration`.
    ///
    /// # Arguments
    /// * `index` - Index of the row to extract.
    ///
    /// # Returns
    /// * `Ok(Pod5Read)` if the index is valid and conversion succeeds.
    /// * `Err(ReadsTableError)` if the index is out of bounds or casting fails.
    pub fn get(&self, index: usize) -> Result<Pod5Read, ReadsTableError> {
        if index >= self.length {
            return Err(ReadsTableError::SignalIndexOutOfBounds);
        }

        // Extract UUID from FixedSizeBinaryArray
        let uuid_bytes = self.read_id_array.value(index);
        if uuid_bytes.len() != 16 {
            return Err(ReadsTableError::InvalidUuidLength(uuid_bytes.len()));
        }
        let mut uuid_array = [0u8; 16];
        uuid_array.copy_from_slice(uuid_bytes);
        let read_id = Uuid::from_bytes(uuid_array);

        // Extract signal indices from ListArray
        let signal_indices = self.signal_index_array
            .value(index)
            .as_any()
            .downcast_ref::<UInt64Array>()
            .map(|arr| arr.values().to_vec())
            .ok_or_else(|| ReadsTableError::SignalIndexArrayCastError)?;

        Ok(Pod5Read::new(
            read_id,
            signal_indices,
            Self::get_primitive(&self.read_number_array, index),
            Self::get_primitive(&self.start_array, index),
            Self::get_primitive(&self.median_before_array, index),
            Self::get_primitive(&self.num_minknow_events_array, index),
            TrackedScaling {
                scale: Self::get_primitive(&self.tracked_scaling_scale_array, index),
                shift: Self::get_primitive(&self.tracked_scaling_shift_array, index),
            },
            PredictedScaling {
                scale: Self::get_primitive(&self.predicted_scaling_scale_array, index),
                shift: Self::get_primitive(&self.predicted_scaling_shift_array, index),
            },
            Self::get_primitive(&self.num_reads_since_mux_change_array, index),
            Self::get_primitive(&self.time_since_mux_change_array, index),
            Self::get_primitive(&self.num_samples_array, index),
            Pore {
                pore_type: Self::get_dict_value(&self.pore_type_array, index),
                channel: Self::get_primitive(&self.channel_array, index),
                well: Self::get_primitive(&self.well_array, index),
            },
            Calibration {
                scale: Self::get_primitive(&self.calibration_scale_array, index),
                offset: Self::get_primitive(&self.calibration_offset_array, index),
            },
            EndReason {
                name: Self::get_dict_value(&self.end_reason_array, index),
                forced: if self.end_reason_forced_array.is_null(index) {
                    None
                } else {
                    Some(self.end_reason_forced_array.value(index))
                },
            },
            Self::get_dict_value(&self.run_info_array, index),
            Self::get_primitive(&self.open_pore_level_array, index),
            None
        ))
    }
}

// Iterates row-wise over a ReadsTable, producing a Pod5Read for each row
impl Iterator for ReadsTable {
    type Item = Result<Pod5Read, ReadsTableError>;

    /// Advances the iterator and returns the next `Pod5Read` in the table.
    ///
    /// # Returns
    /// * `Some(Ok(Pod5Read))` if a row exists and is parsed successfully.
    /// * `Some(Err(...))` if an error occurs during row extraction.
    /// * `None` if all rows have been consumed.
    fn next(&mut self) -> Option<Self::Item> {
        if self.current_index >= self.length {
            return None;
        }

        let result = self.get(self.current_index);
        self.current_index += 1;
        Some(result)
    }
}

/// Describes the pore context of a read, including the pore type, 
/// the physical channel on the flow cell, and the well number.
#[derive(Debug, Clone)]
pub struct Pore {
    pub pore_type: Option<String>, 
    pub channel: Option<u16>,
    pub well: Option<u8>,
}

/// Holds scaling parameters that were tracked during data 
/// acquisition to normalize raw signal values.
#[derive(Debug, Clone)]
pub struct TrackedScaling {
    pub scale: Option<f32>,
    pub shift: Option<f32>
}

/// Contains predicted scaling parameters that estimate 
/// signal normalization settings.
#[derive(Debug, Clone)]
pub struct PredictedScaling {
    pub scale: Option<f32>,
    pub shift: Option<f32>
}

/// Stores calibration values used to adjust the signal 
/// scale and offset during read processing.
#[derive(Debug, Clone)]
pub struct Calibration {
    pub scale: Option<f32>,
    pub offset: Option<f32>
}

/// Describes why a read ended, including the categorical
/// reason and whether it was forced.
#[derive(Debug, Clone)]
pub struct EndReason {
    pub name: Option<String>,
    pub forced: Option<bool>
}