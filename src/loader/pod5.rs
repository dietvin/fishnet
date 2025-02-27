use std::{fs::File, collections::HashMap};
use itertools::multizip;
use pod5::{polars_arrow::array::Int16Array, reader};
use super::{super::error::loader_errors::pod5_errors::{Pod5FileError, Pod5IndexError}, helpers};

// ##################################################################################################
// #                                            Structs                                             #
// ##################################################################################################

/// Represents a single read from a Pod5 file containing signal data and metadata
#[derive(Debug, Clone)]
pub struct Pod5Read {
    read_id: String,
    signal: Vec<i16>,
    num_samples: usize,
    calibration_offset: Option<f32>,
    calibration_scale: Option<f32>
}

/// A container for Pod5 file data that provides access to reads using a HashMap
/// 
/// This struct loads and parses Pod5 files, exposing the reads and their associated
/// signal data through a HashMap interface.
#[derive(Debug, Clone)]
pub struct Pod5File {
    path: String,
    reads: HashMap<String, Pod5Read>
}

/// A collection of Pod5File paths that loads files when explicitly requested
/// 
/// This struct manages multiple Pod5 file paths and loads Pod5File
/// objects only when they are explicitly requested through the load_file method.
#[derive(Debug)]
pub struct Pod5Index {
    // Stores the Pod5File object with the path to the file
    file_paths: Vec<String>,
}


// ##################################################################################################
// #                                        Implementations                                         #
// ##################################################################################################

impl Pod5Read {
    /// Creates a new Pod5Read instance with basic read information
    /// 
    /// # Arguments
    /// * `read_id` - Unique identifier for the read
    /// * `signal` - Vector of signal intensity values
    /// * `num_samples` - Total number of samples in the read
    fn init(read_id: String, signal: Vec<i16>, num_samples: usize) -> Self {
        Pod5Read{
            read_id: read_id.to_string(),
            signal,
            num_samples,
            calibration_offset: None,
            calibration_scale: None
        }
    }

    /// Updates the read with calibration data
    /// 
    /// # Arguments
    /// * `offset` - The calibration offset value for signal normalization
    /// * `scale` - The calibration scale factor for signal normalization
    fn add_read_df_data(&mut self, offset: f32, scale: f32) {
        self.calibration_offset = Some(offset);
        self.calibration_scale = Some(scale);
    }

    /// Returns the unique identifier for this read
    pub fn read_id(&self) -> &str {
        &self.read_id
    }

    /// Returns the signal intensity values for this read
    pub fn signal(&self) -> &Vec<i16> {
        &self.signal
    }

    /// Returns the number of samples in this read
    pub fn num_samples(&self) -> &usize {
        &self.num_samples
    }

    /// Returns the calibration offset if available
    pub fn calibration_offset(&self) -> Option<&f32> {
        self.calibration_offset.as_ref()
    }

    /// Returns the calibration scale factor if available
    pub fn calibration_scale(&self) -> Option<&f32> {
        self.calibration_scale.as_ref()
    }
}


impl Pod5File {
    /// Creates a new Pod5File by parsing the specified Pod5 file
    /// 
    /// # Overview
    /// This method opens and processes a Pod5 file in two main steps:
    /// 1. First, it extracts basic read information (read_id, signal data, and sample count)
    ///    from the signal dataframes.
    /// 2. Then, it enriches these reads with calibration data (offset and scale values) 
    ///    from the read dataframes.
    /// 
    /// The resulting Pod5File provides access to all reads through a HashMap where
    /// read_ids are keys and Pod5Read objects are values.
    /// 
    /// # Arguments
    /// * `path` - Path to the Pod5 file to load
    /// 
    /// # Returns
    /// * `Result<Self, Pod5FileError>` - A Pod5File instance on success, or an error if file 
    ///   reading or parsing fails
    /// 
    /// # Errors
    /// * `Pod5FileError::IoError` - If the file cannot be opened
    /// * `Pod5FileError::ReadDataError` - If required data columns are missing or malformed
    /// * `Pod5FileError::KeyError` - If a read_id in the read dataframe doesn't match any in the signal dataframe    
    pub fn new(path: &str) -> Result<Self, Pod5FileError> {
        let file = File::open(path)?;
        let mut pod5_reader = reader::Reader::from_reader(file)?;

        let mut read_collection = HashMap::new();

        // Extract the needed information from the signal_df dataframes
        // (i.e. columns read_id, signal and samples)
        for signal_df in pod5_reader.signal_dfs()?.flatten() {
            let df = signal_df
                .decompress_signal("signal_decompressed")?
                .into_inner();
            // Create an iterator for each column
            let read_id_col_iter = df
                .column("read_id")?
                .binary()?
                .iter();
            let signal_col_iter = df
                .column("signal_decompressed")?
                .list()?
                .iter();
            let num_samples_col_iter = df
                .column("samples")?
                .u32()?
                .iter();

            // Collectively iterate through each row of the columns
            // https://stackoverflow.com/questions/72440403/iterate-over-rows-polars-rust
            for (read_id, signal, num_samples) in multizip((
                read_id_col_iter, signal_col_iter, num_samples_col_iter
            )) {
                let read_id = helpers::read_id_from_binary(read_id)?;
                
                // Convert signal data to rust-native Vec<i16>
                let signal = signal
                    .ok_or(Pod5FileError::ColumnDataMissingError { 
                        column: "signal".to_string(), 
                        read_id: read_id.clone()
                    })?
                    .as_any()
                    .downcast_ref::<Int16Array>()
                    .ok_or(Pod5FileError::DowncastError(read_id.clone()))?
                    .values()
                    .to_vec();
                
                let num_samples = num_samples.ok_or(Pod5FileError::ColumnDataMissingError{
                    column: "num_samples".to_string(), 
                    read_id: read_id.clone()
                })? as usize;

                let read = Pod5Read::init(
                    read_id.clone(), 
                    signal, 
                    num_samples
                );

                read_collection.insert(read_id, read);
            }
        }

        // Extract the needed information from the signal_df dataframes and 
        // add it to the existing Pod5Read objects.
        // (i.e. columns calibration_offset & calibration_scale)
        for read_df in pod5_reader.read_dfs()?.flatten() {
            let df = read_df.into_inner();

            // Create an iterator for each column
            let read_id_col_iter = df
                .column("read_id")?
                .binary()?
                .iter();
            let offset_col_iter = df
                .column("calibration_offset")?
                .f32()?
                .iter();
            let scale_col_iter = df
                .column("calibration_scale")?
                .f32()?
                .iter();

            // Collectively iterate through each row of the columns
            for (read_id, offset, scale) in multizip((
                read_id_col_iter, offset_col_iter, scale_col_iter
            )) {
                let read_id = helpers::read_id_from_binary(read_id)?;

                let offset = offset.ok_or(Pod5FileError::ColumnDataMissingError { 
                    column: "calibration_offset".to_string(), 
                    read_id: read_id.clone()
                })?;

                let scale = scale.ok_or(Pod5FileError::ColumnDataMissingError { 
                    column: "calibration_scale".to_string(), 
                    read_id: read_id.clone()
                })?;

                // Add the extracted information to the read at hand
                read_collection
                    .get_mut(&read_id)
                    .ok_or(Pod5FileError::ReadNotFound(read_id))?
                    .add_read_df_data(offset, scale);
            }
        }

        Ok(Pod5File {
            path: path.to_string(),
            reads: read_collection
        })
    }

    /// Retrieves the Pod5Read behind the given read id
    /// if available as a reference
    pub fn get(&self, read_id: &str) -> Option<&Pod5Read> {
        self.reads.get(read_id)
    }

    /// Retrieves the Pod5Read behind the given read id
    /// if available as a mutable reference
    pub fn get_mut(&mut self, read_id: &str) -> Option<&mut Pod5Read> {
        self.reads.get_mut(read_id)
    }

    /// Returns the number of reads stored in the Pod5File
    pub fn num_reads(&self) -> usize {
        self.reads.len()
    }
}


/// Enables iterating over read_ids and Pod5Read references
/// 
/// Supports `for (read_id, read) in &pod5_file { ... }` syntax.
impl<'a> IntoIterator for &'a Pod5File {
    type Item = (&'a String, &'a Pod5Read);
    type IntoIter = std::collections::hash_map::Iter<'a, String, Pod5Read>;

    fn into_iter(self) -> Self::IntoIter {
        self.reads.iter()
    }
}


/// Enables iterating over read_ids and mutable Pod5Read references
/// 
/// Supports `for (read_id, read) in &mut pod5_file { ... }` syntax.
impl<'a> IntoIterator for &'a mut Pod5File {
    type Item = (&'a String, &'a mut Pod5Read);
    type IntoIter = std::collections::hash_map::IterMut<'a, String, Pod5Read>;

    fn into_iter(self) -> Self::IntoIter {
        self.reads.iter_mut()
    }
}
// Note: Structure of the Reads dataframe:
//     "read_id" -> 0
//     "signal" -> 1
//     "read_number" -> 2
//     "start" -> 3
//     "median_before" -> 4
//     "num_minknow_events" -> 5
//     "tracked_scaling_scale" -> 6
//     "tracked_scaling_shift" -> 7
//     "predicted_scaling_scale" -> 8
//     "predicted_scaling_shift" -> 9
//     "num_reads_since_mux_change" -> 10
//     "time_since_mux_change" -> 11
//     "num_samples" -> 12
//     "channel" -> 13
//     "well" -> 14
//     "pore_type" -> 15
//     "calibration_offset" -> 16
//     "calibration_scale" -> 17
//     "end_reason" -> 18
//     "end_reason_forced" -> 19
//     "run_info" -> 20
//     "uuid" -> 21 (added via the parse_read_ids function)

// Note: Structure of the Signal dataframe:
//     "read_id" -> 0
//     "signal" -> 1
//     "samples" -> 2
//     "signal_decompressed" -> 3

impl Pod5Index {
    /// Initializes a Pod5Index from a directory path
    ///
    /// # Arguments
    /// * `path` - Path to the directory containing Pod5 files
    /// * `recursive` - If true, search subdirectories recursively
    /// 
    /// # Returns
    /// * `Result<Self, Pod5IndexError>` - A Pod5Index instance on success, or an error if directory
    ///   reading fails
    /// 
    /// # Errors
    /// * `Pod5IndexError::IoInvalidDir` - If the directory cannot be read or doesn't contain pod5 files
    pub fn from_dir(path: &str, recursive: bool) -> Result<Self, Pod5IndexError> {
        let file_paths = helpers::find_files_in_dir(path, "pod5", recursive)?;

        Ok(Pod5Index {
            file_paths,
        })
    }

    /// Initializes a Pod5Index from file paths
    /// 
    /// # Arguments
    /// * `paths` - Vector of paths to Pod5 files
    /// 
    /// # Returns
    /// * `Result<Self, Pod5IndexError>` - A Pod5Index instance on success, or an error if any file
    ///   path is invalid
    /// 
    /// # Errors
    /// * `Pod5IndexError::IoInvalidFileList` - If a file in the list doesn't have the .pod5 extension    
    pub fn from_files(paths: &Vec<String>) -> Result<Self, Pod5IndexError> {
        let file_paths = helpers::get_files(paths, "pod5")?;

        Ok(Pod5Index {
            file_paths,
        })
    }

    /// Loads and returns a Pod5File by its path
    /// 
    /// # Arguments
    /// * `file_path` - Path to the Pod5 file
    /// 
    /// # Returns
    /// * `Result<Pod5File, Pod5IndexError>` - The loaded Pod5File if successful
    /// 
    /// # Errors
    /// * `Pod5IndexError::LoadPod5Error` - If the Pod5 file cannot be loaded
    /// * `Pod5IndexError::FileNotFound` - If the file path is not in the collection
    pub fn load_file(&self, file_path: &str) -> Result<Pod5File, Pod5IndexError> {
        // Check if the file path is in the current collection
        if !self.file_paths.contains(&file_path.to_string()) {
            return Err(Pod5IndexError::FileNotFound(file_path.to_string()));
        }

        // Load the file
        match Pod5File::new(file_path) {
            Ok(pod5_file) => Ok(pod5_file),
            Err(err) => Err(Pod5IndexError::FileLoadingError(err))
         }
    }

    /// Lists all file paths in the collection
    /// 
    /// # Returns
    /// * `&[String]` - Slice of file paths
    pub fn file_paths(&self) -> &[String] {
        &self.file_paths
    }

    /// Returns the number of files in the collection
    /// 
    /// # Returns
    /// * `usize` - Number of files
    pub fn num_files(&self) -> usize {
        self.file_paths.len()
    }

    pub fn files(&self) -> Pod5FileIterator {
        Pod5FileIterator {
            index: self,
            current_ixd: 0
        }
    }
}

/// An iterator that loads and yields Pod5File objects on demand
pub struct Pod5FileIterator<'a> {
    index: &'a Pod5Index,
    current_ixd: usize
}

impl<'a> Iterator for Pod5FileIterator<'a> {
    type Item = Result<Pod5File, Pod5IndexError>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.current_ixd >= self.index.file_paths().len() {
            return None;
        }

        let file_path = &self.index.file_paths()[self.current_ixd];
        self.current_ixd += 1;
        Some(self.index.load_file(file_path))
    }   
}