use arrow2::{
    array::{
        Array, 
        BooleanArray, 
        DictionaryArray, 
        FixedSizeBinaryArray, 
        ListArray, 
        PrimitiveArray, 
        UInt64Array, 
        Utf8Array
    }, 
    chunk::Chunk, 
    types::NativeType
};
use uuid::Uuid;

use crate::read::Pod5Read;

/// Intermediate helper struct that holds strongly typed references to the Arrow arrays from a chunk of the reads table.
/// It enables safe and row-wise access to read metadata for constructing Pod5Read instances.
#[derive(Debug)]
pub(crate) struct ReadsTable {
    read_id_array: FixedSizeBinaryArray,
    signal_index_array: ListArray<i32>,
    read_number_array: PrimitiveArray<u32>,
    start_array: PrimitiveArray<u64>,
    median_before_array: PrimitiveArray<f32>,
    num_minknow_events_array: PrimitiveArray<u64>,
    tracked_scaling_scale_array: PrimitiveArray<f32>,
    tracked_scaling_shift_array: PrimitiveArray<f32>,
    predicted_scaling_scale_array: PrimitiveArray<f32>,
    predicted_scaling_shift_array: PrimitiveArray<f32>,
    num_reads_since_mux_change_array: PrimitiveArray<u32>,
    time_since_mux_change_array: PrimitiveArray<f32>,
    num_samples_array: PrimitiveArray<u64>,
    channel_array: PrimitiveArray<u16>,
    well_array: PrimitiveArray<u8>,
    pore_type_array: DictionaryArray<i16>,
    calibration_offset_array: PrimitiveArray<f32>,
    calibration_scale_array: PrimitiveArray<f32>,
    end_reason_array: DictionaryArray<i16>,
    end_reason_forced_array: BooleanArray,
    run_info_array: DictionaryArray<i16>,
    current_index: usize,
    length: usize
}

impl ReadsTable {
    /// Attempts to downcast an Arrow array at the given index into a `FixedSizeBinaryArray`.
    ///
    /// # Arguments
    /// * `arrays` - A slice of boxed Arrow arrays.
    /// * `index` - The index of the target array within the slice.
    ///
    /// # Returns
    /// * `Ok(FixedSizeBinaryArray)` if the cast succeeds.
    /// * `Err(ReadsTableError::ArrayCastError)` if the array type is incompatible.
    fn cast_fixed_sized_binary_array(arrays: &[Box<dyn Array>], index: usize) -> Result<FixedSizeBinaryArray, ReadsTableError> {
        arrays[index]
            .as_any()
            .downcast_ref::<FixedSizeBinaryArray>()
            .ok_or_else(|| ReadsTableError::ArrayCastError { 
                    index, 
                    reason: format!("Failed to cast to FixedSizeBinaryArray") 
                }
            )
            .cloned()
    }

    /// Attempts to downcast an Arrow array at the given index into a `ListArray<i32>`.
    ///
    /// # Arguments
    /// * `arrays` - A slice of boxed Arrow arrays.
    /// * `index` - The index of the target array within the slice.
    ///
    /// # Returns
    /// * `Ok(ListArray<i32>)` if the cast is successful.
    /// * `Err(ReadsTableError::ArrayCastError)` if the cast fails.
    fn cast_list_array(arrays: &[Box<dyn Array>], index: usize) -> Result<ListArray<i32>, ReadsTableError> {
        arrays[index]
            .as_any()
            .downcast_ref::<ListArray<i32>>()
            .ok_or_else(|| ReadsTableError::ArrayCastError { 
                    index, 
                    reason: format!("Failed to cast to ListArray<i32>") 
                }
            )
            .cloned()
    }

    /// Attempts to downcast an Arrow array at the given index into a `DictionaryArray<i16>`.
    ///
    /// Used for dictionary-encoded string columns such as `pore_type`, `end_reason`, and `run_info`.
    ///
    /// # Arguments
    /// * `arrays` - A slice of boxed Arrow arrays.
    /// * `index` - The index of the target array.
    ///
    /// # Returns
    /// * `Ok(DictionaryArray<i16>)` on success.
    /// * `Err(ReadsTableError::ArrayCastError)` on failure.
    fn cast_dictionary_array(arrays: &[Box<dyn Array>], index: usize) -> Result<DictionaryArray<i16>, ReadsTableError> {
        arrays[index]
            .as_any()
            .downcast_ref::<DictionaryArray<i16>>()
            .ok_or_else(|| ReadsTableError::ArrayCastError { 
                    index, 
                    reason: format!("Failed to cast to DictionaryArray<i16>") 
                }
            )
            .cloned()
    }

    /// Attempts to downcast an Arrow array at the given index into a `BooleanArray`.
    ///
    /// # Arguments
    /// * `arrays` - A slice of boxed Arrow arrays.
    /// * `index` - The index of the array to downcast.
    ///
    /// # Returns
    /// * `Ok(BooleanArray)` on successful cast.
    /// * `Err(ReadsTableError::ArrayCastError)` if the type doesn't match.
    fn cast_boolean_array(arrays: &[Box<dyn Array>], index: usize) -> Result<BooleanArray, ReadsTableError> {
        arrays[index]
            .as_any()
            .downcast_ref::<BooleanArray>()
            .ok_or_else(|| ReadsTableError::ArrayCastError { 
                    index, 
                    reason: format!("Failed to cast to BooleanArray") 
                }
            )
            .cloned()
    }

    /// Constructs a `ReadsTable` from a `Chunk<Box<dyn Array>>` originating from a FeatherV2 dataset.
    ///
    /// This function validates the number and type of columns and converts each column
    /// into a strongly typed Arrow array for efficient row-wise access.
    ///
    /// # Arguments
    /// * `chunk` - The Arrow chunk containing raw columnar data.
    ///
    /// # Returns
    /// * `Ok(ReadsTable)` if the chunk has the correct structure and types.
    /// * `Err(ReadsTableError)` if any casting or validation fails.
    pub fn from_chunk(chunk: Chunk<Box<dyn Array>>) -> Result<Self, ReadsTableError> {
        if chunk.arrays().len() != 21 {
            return Err(ReadsTableError::InvalidChunkStructure {
                expected: 21,
                actual: chunk.arrays().len(),
            });
        }    

        let arrays = chunk.arrays();

        // Helper macro that handles casting from Box<dyn Array> into a PrimitiveArrays
        macro_rules! downcast_primitive {
            ($idx:expr, $ty:ty, $name:expr) => {
                arrays[$idx]
                    .as_any()
                    .downcast_ref::<PrimitiveArray<$ty>>()
                    .ok_or_else(|| ReadsTableError::ArrayCastError { 
                            index: $idx, 
                            reason: format!("Failed to cast to PrimitiveArray<{}>", $name)
                        }
                    )
                    .cloned()
            };
        }

        let read_id_array = Self::cast_fixed_sized_binary_array(arrays, 0)?;
        let signal_index_array = Self::cast_list_array(arrays, 1)?;
        let read_number_array = downcast_primitive!(2, u32, "u32")?;
        let start_array = downcast_primitive!(3, u64, "u64")?;
        let median_before_array = downcast_primitive!(4, f32, "f32")?;
        let num_minknow_events_array = downcast_primitive!(5, u64, "u64")?;
        let tracked_scaling_scale_array = downcast_primitive!(6, f32, "f32")?;
        let tracked_scaling_shift_array = downcast_primitive!(7, f32, "f32")?;
        let predicted_scaling_scale_array = downcast_primitive!(8, f32, "f32")?;
        let predicted_scaling_shift_array = downcast_primitive!(9, f32, "f32")?;
        let num_reads_since_mux_change_array = downcast_primitive!(10, u32, "u32")?;
        let time_since_mux_change_array = downcast_primitive!(11,f32, "PrimitiveArray<f32>")?;
        let num_samples_array = downcast_primitive!(12, u64, "u64")?;
        let channel_array = downcast_primitive!(13, u16, "u16")?;
        let well_array = downcast_primitive!(14, u8, "u8")?;
        let pore_type_array = Self::cast_dictionary_array(arrays, 15)?;
        let calibration_offset_array = downcast_primitive!(16, f32, "f32")?;
        let calibration_scale_array = downcast_primitive!(17, f32, "f32")?;
        let end_reason_array = Self::cast_dictionary_array(arrays, 18)?;
        let end_reason_forced_array = Self::cast_boolean_array(arrays, 19)?;
        let run_info_array = Self::cast_dictionary_array(arrays, 20)?;

        let length = read_id_array.len();

        Ok(ReadsTable {
            read_id_array,
            signal_index_array,
            read_number_array,
            start_array,
            median_before_array,
            num_minknow_events_array,
            tracked_scaling_scale_array,
            tracked_scaling_shift_array,
            predicted_scaling_scale_array,
            predicted_scaling_shift_array,
            num_reads_since_mux_change_array,
            time_since_mux_change_array,
            num_samples_array,
            channel_array,
            well_array,
            pore_type_array,
            calibration_offset_array,
            calibration_scale_array,
            end_reason_array,
            end_reason_forced_array,
            run_info_array,
            current_index: 0,
            length,
        })
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


#[derive(Debug, thiserror::Error)]
pub(crate) enum ReadsTableError {
    #[error("UUID parsing error: {0}")]
    UuidError(#[from] uuid::Error),
    #[error("Invalid UUID bytes: expected 16 bytes, got {0}")]
    InvalidUuidLength(usize),
    #[error("Signal index out of bounds")]
    SignalIndexOutOfBounds,
    #[error("Invalid chunk structure: expected {expected} arrays, got {actual}")]
    InvalidChunkStructure { expected: usize, actual: usize },
    #[error("Array type mismatch at index {index}: expected {expected:?}, got {actual:?}")]
    ArrayTypeMismatch {
        index: usize,
        expected: String,
        actual: String,
    },
    #[error("Failed to cast array at index {index}: {reason}")]
    ArrayCastError { index: usize, reason: String },
    #[error("Could not downcast the signal indices array")]
    SignalIndexArrayCastError,
    
}