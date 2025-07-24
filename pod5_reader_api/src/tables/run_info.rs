use std::collections::HashMap;

use arrow2::{array::{Array, MapArray, PrimitiveArray, StructArray, Utf8Array}, chunk::Chunk};
use chrono::{DateTime, Utc};

/// Holds the information found in the run info table in the pod5 file in
/// an easily accessible Rust-native format for easy access. 
/// 
/// The FeatherV2 dataset containing the run info table should consist
/// of one chunk containing one row and 20 columns. As the schema specifies
/// all values as optional, most of the data is stored in Options in case
/// the data is missing. Only the `acquisition_id` is required since it is
/// used to match reads to the corresponding run info.
/// 
/// To access the underlying values `require_<field-name>` functions are
/// implemented that return the Options wrapped in Oks if available and 
/// errors otherwise.
#[derive(Debug, Clone, PartialEq)]
pub struct RunInfo {
    pub acquisition_id: String,
    pub acquisition_start_time: Option<DateTime<Utc>>,
    pub adc_max: Option<i16>,
    pub adc_min: Option<i16>,
    pub context_tags: Option<HashMap<String, Option<String>>>,
    pub experiment_name: Option<String>,
    pub flow_cell_id: Option<String>,
    pub flow_cell_product_code: Option<String>,
    pub protocol_name: Option<String>,
    pub protocol_run_id: Option<String>,
    pub protocol_start_time: Option<DateTime<Utc>>,
    pub sample_id: Option<String>,
    pub sample_rate: Option<u16>,
    pub sequencing_kit: Option<String>,
    pub sequencer_position: Option<String>,
    pub sequencer_position_type: Option<String>,
    pub software: Option<String>,
    pub system_name: Option<String>,
    pub system_type: Option<String>,
    pub tracking_id: Option<HashMap<String, Option<String>>>,
}

impl RunInfo {
    /// Creates an empty `RunInfo` instance with only the `acquisition_id` set.
    /// 
    /// All other fields are initialized to `None`. This is mainly used as an
    /// internal helper when constructing a `RunInfo` instance from an Arrow chunk.
    /// 
    /// # Arguments
    /// 
    /// * `id` - The acquisition ID, which uniquely identifies the sequencing run.
    ///
    /// # Returns
    /// 
    /// An instance of `RunInfo` with all fields set to `None` except for `acquisition_id`.
    fn empty(id: String) -> Self {
        Self {
            acquisition_id: id,
            acquisition_start_time: None,
            adc_max: None,
            adc_min: None,
            context_tags: None,
            experiment_name: None,
            flow_cell_id: None,
            flow_cell_product_code: None,
            protocol_name: None,
            protocol_run_id: None,
            protocol_start_time: None,
            sample_id: None,
            sample_rate: None,
            sequencing_kit: None,
            sequencer_position: None,
            sequencer_position_type: None,
            software: None,
            system_name: None,
            system_type: None,
            tracking_id: None,
        }
    }

    /// Parses a `RunInfo` struct from an Arrow2 `Chunk` as extracted from the
    /// `run_info` table of a POD5 file.
    /// 
    /// The chunk is expected to contain exactly one row and 20 columns, where
    /// each column corresponds to a specific run metadata field. All values are
    /// treated as optional, except `acquisition_id`, which must be present and valid.
    /// 
    /// # Arguments
    /// 
    /// * `chunk` - The Arrow2 `Chunk` containing a single row with run info data.
    /// 
    /// # Returns
    /// 
    /// * `Ok(RunInfo)` if parsing is successful.
    /// * `Err(RunInfoError)` if the chunk is malformed or contains invalid types.
    /// 
    /// # Errors
    /// 
    /// This function will return an error if:
    /// - The chunk contains more than one row.
    /// - The `acquisition_id` field is missing or invalid.
    /// - Any field is not of the expected Arrow type.
    /// - The `MapArray` fields (e.g., `context_tags`, `tracking_id`) are malformed.
    pub fn from_arrow_chunk(chunk: Chunk<Box<dyn Array>>) -> Result<Self, RunInfoError> {
        let num_rows = chunk.len();
        if num_rows != 1 {
            println!("Warning: Expected 1 row in RunInfo chunk, found {}", num_rows);
            return Err(RunInfoError::InvalidRowCount(num_rows));
        }

        // We expect 20 arrays (one for each field)
        const EXPECTED_ARRAY_COUNT: usize = 20;
        let arrays = chunk.arrays();
        if arrays.len() < EXPECTED_ARRAY_COUNT {
            println!(
                "Warning: Expected at least {} arrays in RunInfo chunk, found {}", 
                EXPECTED_ARRAY_COUNT, 
                arrays.len()
            );
            return Err(RunInfoError::InvalidArrayCount {
                expected: EXPECTED_ARRAY_COUNT,
                found: arrays.len(),
            });
        }

        let arrays = chunk.arrays();

        // Treat acquisition id separately since this is the only field that cannot be None
        let acquisition_id = if let Some(array) = arrays.get(0) {
            if let Some(id_array) = array.as_any().downcast_ref::<Utf8Array<i32>>() {
                if id_array.is_valid(0) {
                    id_array.value(0).to_string()
                } else {
                    println!("Warning: Invalid value for 'acquisition_id'");
                    return Err(RunInfoError::InvalidAcquisitionId("Invalid value for 'acquisition_id'"));
                }
            } else {
                println!("Warning: Field 'acquisition_id' is not a UTF8 array");
                return Err(RunInfoError::InvalidAcquisitionId("Field 'acquisition_id' is not a UTF8 array"));
            }
        } else {
            println!("Warning: Could not access array at index 0");
            return Err(RunInfoError::InvalidAcquisitionId("Could not access array at index 0"));
        };

        let mut run_info = RunInfo::empty(acquisition_id);
        
        // Helper macro to downcast fields containing Utf8Array (String)
        macro_rules! extract_string {
            ($col_idx:expr, $field:ident, $field_name:expr) => {
                if let Some(array) = arrays.get($col_idx) {
                    if let Some(uft8_array) = array.as_any().downcast_ref::<Utf8Array<i32>>() {
                        if uft8_array.is_valid(0) {
                            run_info.$field = Some(uft8_array.value(0).to_string());
                        }
                    } else {
                        println!("Warning: Field '{}' is not a UTF8 array", $field_name);
                        return Err(RunInfoError::InvalidType($field_name));
                    }
                }
            };
        }
        
        // Helper macro to downcast fields containing primitive values (i16/u16/...)
        macro_rules! extract_primitive {
            ($col_idx:expr, $field:ident, $field_name:expr, $ty:ty) => {
                if let Some(array) = arrays.get($col_idx) {
                    if let Some(prim_array) = array.as_any().downcast_ref::<PrimitiveArray<$ty>>() {
                        if prim_array.is_valid(0) {
                            run_info.$field = Some(prim_array.value(0) as $ty)
                        }
                    } else {
                        println!("Warning: Field '{}' is not a primitive array of expected type", $field_name);
                        return Err(RunInfoError::InvalidType($field_name));
                    }
                }                
            };
        }

        // acquisition_start_time: PrimitiveArray<i64> -> DateTime
        if let Some(array) = arrays.get(1) {
            if let Some(ts_array) = array.as_any().downcast_ref::<PrimitiveArray<i64>>() {
                if ts_array.is_valid(0) {
                    let millis = ts_array.value(0);
                    run_info.acquisition_start_time = DateTime::from_timestamp_millis(millis);
                }
            } else {
                println!("Warning: Field 'acquisition_start_time' is not a timestamp array");
                return Err(RunInfoError::InvalidType("acquisition_start_time"));
            }
        }

        extract_primitive!(2, adc_max, "adc_max", i16);
        extract_primitive!(3, adc_min, "adc_min", i16);
    
        // context_tags: MapArray -> HashMap<String, String>
        if let Some(array) = arrays.get(4) {
            if let Some(map_array) = array.as_any().downcast_ref::<MapArray>() {
                run_info.context_tags = Self::extract_string_map(map_array)?;
            } else {
                println!("Warning: Field 'context_tags' is not a map array");
                return Err(RunInfoError::InvalidType("context_tags"));
            }
        }

        extract_string!(5, experiment_name, "experiment_name");
        extract_string!(6, flow_cell_id, "flow_cell_id");
        extract_string!(7, flow_cell_product_code, "flow_cell_product_code");
        extract_string!(8, protocol_name, "protocol_name");
        extract_string!(9, protocol_run_id, "protocol_run_id");

        // protocol_start_time: PrimitiveArray<i64> -> DateTime
        if let Some(array) = arrays.get(10) {
            if let Some(ts_array) = array.as_any().downcast_ref::<PrimitiveArray<i64>>() {
                if ts_array.is_valid(0) {
                    let millis = ts_array.value(0);
                    run_info.protocol_start_time = DateTime::from_timestamp_millis(millis);
                }
            } else {
                println!("Warning: Field 'protocol_start_time' is not a timestamp array");
                return Err(RunInfoError::InvalidType("protocol_start_time"));
            }
        }

        extract_string!(11, sample_id, "sample_id");
        extract_primitive!(12, sample_rate, "sample_rate", u16);
        extract_string!(13, sequencing_kit, "sequencing_kit");
        extract_string!(14, sequencer_position, "sequencer_position");
        extract_string!(15, sequencer_position_type, "sequencer_position_type");
        extract_string!(16, software, "software");
        extract_string!(17, system_name, "system_name");
        extract_string!(18, system_type, "system_type");

        // tracking_id: MapArray -> HashMap<String, String>
        if let Some(array) = arrays.get(19) {
            if let Some(map_array) = array.as_any().downcast_ref::<MapArray>() {
                run_info.tracking_id = Self::extract_string_map(map_array)?;
            } else {
                println!("Warning: Field 'tracking_id' is not a map array");
                return Err(RunInfoError::InvalidType("tracking_id"));
            }
        }

        Ok(run_info)
    }

    /// Helper function to extract a `HashMap<String, Option<String>>` from a `MapArray`.
    /// 
    /// Used to parse the `context_tags` and `tracking_id` fields in the `run_info`
    /// chunk. Returns `None` if the map field is invalid or absent.
    /// 
    /// # Arguments
    /// 
    /// * `map_array` - A reference to the Arrow2 `MapArray` representing key-value pairs.
    /// 
    /// # Returns
    /// 
    /// * `Ok(Some(map))` if the map contains entries.
    /// * `Ok(None)` if the map is empty or null.
    /// * `Err(RunInfoError)` if the structure or types are not as expected.
    /// 
    /// # Errors
    /// 
    /// This function will return an error if:
    /// - The map array has invalid offsets.
    /// - The underlying struct array is malformed.
    /// - The keys or values are not valid UTF-8 arrays.
    fn extract_string_map(map_array: &MapArray) -> Result<Option<HashMap<String, Option<String>>>, RunInfoError> {
        // In case the map is empty
        if !map_array.is_valid(0) {
            return Ok(None);
        }

        let (start, end) = map_array.offsets().as_slice().windows(2)
            .nth(0)
            .map(|w| (w[0] as usize, w[1] as usize))
            .ok_or_else(|| {
                println!("Warning: Invalid map offsets");
                RunInfoError::InvalidMapStructure("Invalid map offsets")
            })?;

        let struct_array = map_array.field().as_any().downcast_ref::<StructArray>()
            .ok_or_else(|| {
                println!("Warning: Map field is not a struct array");
                RunInfoError::InvalidMapStructure("Map field is not a struct array")
            })?;
        
        if struct_array.values().len() < 2 {
            println!("Warning: Map struct array has insufficient fields");
            return Err(RunInfoError::InvalidMapStructure("Map struct array has insufficient fields"));
        }

        let key_array = struct_array.values()[0].as_any().downcast_ref::<Utf8Array<i32>>()
            .ok_or_else(|| {
                println!("Warning: Map key array is not UTF8");
                RunInfoError::InvalidMapStructure("Map key array is not UTF8")
            })?;
        let value_array = struct_array.values()[1].as_any().downcast_ref::<Utf8Array<i32>>()
            .ok_or_else(|| {
                println!("Warning: Map value array is not UTF8");
                RunInfoError::InvalidMapStructure("Map value array is not UTF8")
            })?;

        let mut map = HashMap::new();
        
        for i in start..end {
            let key = key_array.value(i).to_string();
            let value = if value_array.is_valid(i) {
                Some(value_array.value(i).to_string())
            } else {
                None
            };
            map.insert(key, value);
        }

        Ok(Some(map))
    }

    /// Retrieves the value associated with the specified key from the `context_tags` field.
    /// 
    /// This method returns a reference to the value associated with the key in the
    /// `context_tags` map, if both the map and the key are present.
    /// 
    /// # Arguments
    /// 
    /// * `key` - The key to look up in the `context_tags` hashmap.
    /// 
    /// # Returns
    /// 
    /// * `Some(Some(value))` if the key exists and has a value.
    /// * `Some(None)` if the key exists but the value is null.
    /// * `None` if the `context_tags` field is `None` or the key does not exist.
    pub fn get_context_tag(&self, key: &str) -> Option<&Option<String>> {
        self.context_tags
            .as_ref()?
            .get(key)
    }

    /// Retrieves the value associated with the specified key from the `tracking_id` field.
    /// 
    /// This method returns a reference to the value associated with the key in the
    /// `tracking_id` map, if both the map and the key are present.
    /// 
    /// # Arguments
    /// 
    /// * `key` - The key to look up in the `tracking_id` hashmap.
    /// 
    /// # Returns
    /// 
    /// * `Some(Some(value))` if the key exists and has a value.
    /// * `Some(None)` if the key exists but the value is null.
    /// * `None` if the `tracking_id` field is `None` or the key does not exist.
    pub fn get_tracking_value(&self, key: &str) -> Option<&Option<String>> {
        self.tracking_id
            .as_ref()?
            .get(key)
    }

    /// Returns the `acquisition_id` associated with this `RunInfo`.
    /// 
    /// The `acquisition_id` uniquely identifies the acquisition session and is
    /// the only required field in the `RunInfo` struct.
    /// 
    /// # Returns
    /// 
    /// A string slice referencing the internal acquisition ID.
    pub fn acquisition_id(&self) -> &str {
        self.acquisition_id.as_str()
    }

    /// Returns a reference to the `acquisition_start_time` if present.
    /// 
    /// This method is part of the `require_<field>` convention, which ensures
    /// that optional fields are safely unwrapped with proper error handling.
    /// 
    /// # Returns
    /// 
    /// * `Ok(&DateTime<Utc>)` if the acquisition start time is available.
    /// * `Err(RunInfoError::MissingField)` if the field is `None`.
    pub fn require_acquisition_start_time(&self) -> Result<&DateTime<Utc>, RunInfoError> {
        self.acquisition_start_time.as_ref().ok_or(RunInfoError::MissingField("acquisition_start_time"))
    }

    /// Returns a reference to the `adc_max` if present.
    /// 
    /// This method is part of the `require_<field>` convention, which ensures
    /// that optional fields are safely unwrapped with proper error handling.
    /// 
    /// # Returns
    /// 
    /// * `Ok(&i16)` if the acquisition start time is available.
    /// * `Err(RunInfoError::MissingField)` if the field is `None`.
    pub fn require_adc_max(&self) -> Result<&i16, RunInfoError> {
        self.adc_max.as_ref().ok_or(RunInfoError::MissingField("adc_max"))

    }

    /// Returns a reference to the `adc_min` if present.
    /// 
    /// This method is part of the `require_<field>` convention, which ensures
    /// that optional fields are safely unwrapped with proper error handling.
    /// 
    /// # Returns
    /// 
    /// * `Ok(&i16)` if the acquisition start time is available.
    /// * `Err(RunInfoError::MissingField)` if the field is `None`.
    pub fn require_adc_min(&self) -> Result<&i16, RunInfoError> {
        self.adc_min.as_ref().ok_or(RunInfoError::MissingField("adc_min"))

    }

    /// Returns a reference to the `context_tags` if present.
    /// 
    /// This method is part of the `require_<field>` convention, which ensures
    /// that optional fields are safely unwrapped with proper error handling.
    /// 
    /// # Returns
    /// 
    /// * `Ok(&HashMap<String, Option<String>>)` if the acquisition start time is available.
    /// * `Err(RunInfoError::MissingField)` if the field is `None`.
    pub fn require_context_tags(&self) -> Result<&HashMap<String, Option<String>>, RunInfoError> {
        self.context_tags.as_ref().ok_or(RunInfoError::MissingField("context_tags"))

    }

    /// Returns a reference to the `experiment_name` if present.
    /// 
    /// This method is part of the `require_<field>` convention, which ensures
    /// that optional fields are safely unwrapped with proper error handling.
    /// 
    /// # Returns
    /// 
    /// * `Ok(&String)` if the acquisition start time is available.
    /// * `Err(RunInfoError::MissingField)` if the field is `None`.
    pub fn require_experiment_name(&self) -> Result<&String, RunInfoError> {
        self.experiment_name.as_ref().ok_or(RunInfoError::MissingField("experiment_name"))

    }

    /// Returns a reference to the `flow_cell_id` if present.
    /// 
    /// This method is part of the `require_<field>` convention, which ensures
    /// that optional fields are safely unwrapped with proper error handling.
    /// 
    /// # Returns
    /// 
    /// * `Ok(&String)` if the acquisition start time is available.
    /// * `Err(RunInfoError::MissingField)` if the field is `None`.
    pub fn require_flow_cell_id(&self) -> Result<&String, RunInfoError> {
        self.flow_cell_id.as_ref().ok_or(RunInfoError::MissingField("flow_cell_id"))

    }

    /// Returns a reference to the `flow_cell_product_code` if present.
    /// 
    /// This method is part of the `require_<field>` convention, which ensures
    /// that optional fields are safely unwrapped with proper error handling.
    /// 
    /// # Returns
    /// 
    /// * `Ok(&String)` if the acquisition start time is available.
    /// * `Err(RunInfoError::MissingField)` if the field is `None`.
    pub fn require_flow_cell_product_code(&self) -> Result<&String, RunInfoError> {
        self.flow_cell_product_code.as_ref().ok_or(RunInfoError::MissingField("flow_cell_product_code"))

    }

    pub fn require_protocol_name(&self) -> Result<&String, RunInfoError> {
        self.protocol_name.as_ref().ok_or(RunInfoError::MissingField("protocol_name"))

    }

    /// Returns a reference to the `protocol_run_id` if present.
    /// 
    /// This method is part of the `require_<field>` convention, which ensures
    /// that optional fields are safely unwrapped with proper error handling.
    /// 
    /// # Returns
    /// 
    /// * `Ok(&String)` if the acquisition start time is available.
    /// * `Err(RunInfoError::MissingField)` if the field is `None`.
    pub fn require_protocol_run_id(&self) -> Result<&String, RunInfoError> {
        self.protocol_run_id.as_ref().ok_or(RunInfoError::MissingField("protocol_run_id"))

    }

    /// Returns a reference to the `protocol_start_time` if present.
    /// 
    /// This method is part of the `require_<field>` convention, which ensures
    /// that optional fields are safely unwrapped with proper error handling.
    /// 
    /// # Returns
    /// 
    /// * `Ok(&DateTime<Utc>)` if the acquisition start time is available.
    /// * `Err(RunInfoError::MissingField)` if the field is `None`.
    pub fn require_protocol_start_time(&self) -> Result<&DateTime<Utc>, RunInfoError> {
        self.protocol_start_time.as_ref().ok_or(RunInfoError::MissingField("protocol_start_time"))

    }

    /// Returns a reference to the `sample_id` if present.
    /// 
    /// This method is part of the `require_<field>` convention, which ensures
    /// that optional fields are safely unwrapped with proper error handling.
    /// 
    /// # Returns
    /// 
    /// * `Ok(&String)` if the acquisition start time is available.
    /// * `Err(RunInfoError::MissingField)` if the field is `None`.
    pub fn require_sample_id(&self) -> Result<&String, RunInfoError> {
        self.sample_id.as_ref().ok_or(RunInfoError::MissingField("sample_id"))

    }

    /// Returns a reference to the `sample_rate` if present.
    /// 
    /// This method is part of the `require_<field>` convention, which ensures
    /// that optional fields are safely unwrapped with proper error handling.
    /// 
    /// # Returns
    /// 
    /// * `Ok(&u16)` if the acquisition start time is available.
    /// * `Err(RunInfoError::MissingField)` if the field is `None`.
    pub fn require_sample_rate(&self) -> Result<&u16, RunInfoError> {
        self.sample_rate.as_ref().ok_or(RunInfoError::MissingField("sample_rate"))

    }

    /// Returns a reference to the `sequencing_kit` if present.
    /// 
    /// This method is part of the `require_<field>` convention, which ensures
    /// that optional fields are safely unwrapped with proper error handling.
    /// 
    /// # Returns
    /// 
    /// * `Ok(&String)` if the acquisition start time is available.
    /// * `Err(RunInfoError::MissingField)` if the field is `None`.
    pub fn require_sequencing_kit(&self) -> Result<&String, RunInfoError> {
        self.sequencing_kit.as_ref().ok_or(RunInfoError::MissingField("sequencing_kit"))

    }

    /// Returns a reference to the `sequencer_position` if present.
    /// 
    /// This method is part of the `require_<field>` convention, which ensures
    /// that optional fields are safely unwrapped with proper error handling.
    /// 
    /// # Returns
    /// 
    /// * `Ok(&String)` if the acquisition start time is available.
    /// * `Err(RunInfoError::MissingField)` if the field is `None`.
    pub fn require_sequencer_position(&self) -> Result<&String, RunInfoError> {
        self.sequencer_position.as_ref().ok_or(RunInfoError::MissingField("sequencer_position"))

    }

    /// Returns a reference to the `sequencer_position_type` if present.
    /// 
    /// This method is part of the `require_<field>` convention, which ensures
    /// that optional fields are safely unwrapped with proper error handling.
    /// 
    /// # Returns
    /// 
    /// * `Ok(&String)` if the acquisition start time is available.
    /// * `Err(RunInfoError::MissingField)` if the field is `None`.
    pub fn require_sequencer_position_type(&self) -> Result<&String, RunInfoError> {
        self.sequencer_position_type.as_ref().ok_or(RunInfoError::MissingField("sequencer_position_type"))

    }

    /// Returns a reference to the `software` if present.
    /// 
    /// This method is part of the `require_<field>` convention, which ensures
    /// that optional fields are safely unwrapped with proper error handling.
    /// 
    /// # Returns
    /// 
    /// * `Ok(&String)` if the acquisition start time is available.
    /// * `Err(RunInfoError::MissingField)` if the field is `None`.
    pub fn require_software(&self) -> Result<&String, RunInfoError> {
        self.software.as_ref().ok_or(RunInfoError::MissingField("software"))

    }

    /// Returns a reference to the `system_name` if present.
    /// 
    /// This method is part of the `require_<field>` convention, which ensures
    /// that optional fields are safely unwrapped with proper error handling.
    /// 
    /// # Returns
    /// 
    /// * `Ok(&String)` if the acquisition start time is available.
    /// * `Err(RunInfoError::MissingField)` if the field is `None`.
    pub fn require_system_name(&self) -> Result<&String, RunInfoError> {
        self.system_name.as_ref().ok_or(RunInfoError::MissingField("system_name"))

    }

    /// Returns a reference to the `system_type` if present.
    /// 
    /// This method is part of the `require_<field>` convention, which ensures
    /// that optional fields are safely unwrapped with proper error handling.
    /// 
    /// # Returns
    /// 
    /// * `Ok(&String)` if the acquisition start time is available.
    /// * `Err(RunInfoError::MissingField)` if the field is `None`.
    pub fn require_system_type(&self) -> Result<&String, RunInfoError> {
        self.system_type.as_ref().ok_or(RunInfoError::MissingField("system_type"))

    }

    /// Returns a reference to the `tracking_id` if present.
    /// 
    /// This method is part of the `require_<field>` convention, which ensures
    /// that optional fields are safely unwrapped with proper error handling.
    /// 
    /// # Returns
    /// 
    /// * `Ok(&HashMap<String, Option<String>>)` if the acquisition start time is available.
    /// * `Err(RunInfoError::MissingField)` if the field is `None`.
    pub fn require_tracking_id(&self) -> Result<&HashMap<String, Option<String>>, RunInfoError> {
        self.tracking_id.as_ref().ok_or(RunInfoError::MissingField("tracking_id"))

    }
}


#[derive(Debug, thiserror::Error)]
pub enum RunInfoError {
    #[error("Expected 1 row in chunk, found {0}")]
    InvalidRowCount(usize),
    #[error("Expected at least {expected} arrays in chunk, found {found}")]
    InvalidArrayCount { expected: usize, found: usize },
    #[error("Invalid acquisition id: {0}")]
    InvalidAcquisitionId(&'static str),
    #[error("Field '{0}' is missing or null")]
    MissingField(&'static str),
    #[error("Invalid type for field '{0}'")]
    InvalidType(&'static str),
    #[error("Invalid map array. Could not acces first value")]
    InvalidMapArray,
    #[error("Invalid map structure: {0}")]
    InvalidMapStructure(&'static str),
}