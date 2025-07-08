/*!
 * Pod5 file parsing and signal data management module.
 * 
 * This module provides functionality for reading and processing Pod5 files. 
 * It offers three main components:
 * 
 * - `Pod5Read`: Represents individual reads with signal data, calibration parameters, and
 *   support for signal trimming based on alignment tags (sp, ts, ns).
 * 
 * - `Pod5File`: Container for loading and accessing all reads from a single Pod5 file,
 *   providing HashMap-based access by read ID.
 * 
 * - `Pod5Index`: Collection manager for multiple Pod5 files that supports lazy loading
 *   and provides iterators for processing files and reads across multiple files.
 * 
 * The module handles the complexity of Pod5 file structure, including signal chunks,
 * calibration data extraction, and signal trimming for alignment purposes. It supports
 * both forward and reverse signal processing for direct RNA sequencing workflows.
 * 
 * # Example Usage
 * ```ignore
 * // Load a single Pod5 file
 * let file = Pod5File::new(&path_to_file)?;
 * 
 * // Access a specific read
 * if let Some(read) = file.get("read_id") {
 *     println!("Signal length: {}", read.num_samples());
 * }
 * 
 * // Create an index for multiple files
 * let index = Pod5Index::from_dir(&directory_path, true)?;
 * 
 * // Process all reads across all files
 * for result in index.reads() {
 *     let (file_path, read_id, read) = result?;
 *     // Process read...
 * }
 * ```
 */

use std::{collections::HashMap, fs::File, path::PathBuf};
use itertools::multizip;
use pod5::{polars_arrow::array::Int16Array, reader};
use crate::{error::loader_errors::pod5_errors::{Pod5FileError, Pod5IndexError, Pod5ReadError}, core::loader::helpers};


/// Represents a single read from a Pod5 file containing signal data and metadata
#[derive(Debug, Clone)]
pub struct Pod5Read {
    read_id: String,
    calibration_offset: f32,
    calibration_scale: f32,
    signal: Vec<i16>,
    num_samples: usize,
    signal_trimmed: Option<Vec<i16>>,
    num_samples_trimmed: Option<usize>,
    signal_offset: Option<usize>
}


impl Pod5Read {
    /// Creates a new Pod5Read instance with basic read information. Initializes the
    /// signal as an empty Vector and the num_samples count as 0.
    /// 
    /// The signal chunks are added iteratively while parsing the overarching pod5
    /// file.
    /// 
    /// Member variables that correspond to the trimmed signal (signal_trimmed, 
    /// num_samples_trimmed, signal_offset) are intitialized with None. The values 
    /// are fileld in the `update_signal` function.
    /// 
    /// # Arguments
    /// * `read_id` - Unique identifier for the read
    /// * `signal` - Vector of signal intensity values
    /// * `num_samples` - Total number of samples in the read
    fn init(read_id: String, offset: f32, scale: f32) -> Self {
        Pod5Read{
            read_id: read_id.to_string(),
            calibration_offset: offset,
            calibration_scale: scale,
            signal: Vec::new(),
            num_samples: 0,
            signal_trimmed: None,
            num_samples_trimmed: None,
            signal_offset: None
        }
    }

    /// Updates the signal and num_samples variables. 
    /// Important note: 
    /// # Arguments
    /// * `offset` - The calibration offset value for signal normalization
    /// * `scale` - The calibration scale factor for signal normalization
    /// 
    /// # Note
    /// Signals can be split into multiple chunks where each chunk is stored in a row of the signal 
    /// dataframe in the pod5 file. The length of these subsets taken together results in the number 
    /// of samples stored in the Reads dataframe. 
    /// 
    /// The method handles these chunks by simply appending the latest chunk (in order of the rows in the df)
    /// to the signal already stored in the Pod5Read. This is assuming that the original order is retained in
    /// the pod5 file. When all chunks are added the length is equal to the num_samples value. 
    fn add_signal_df_data(&mut self, signal_chunk: &mut Vec<i16>, signal_chunk_len: usize) {
        self.signal.append(signal_chunk);
        self.num_samples += signal_chunk_len;
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

    /// Returns the trimmed signal intensity values for this read
    pub fn signal_trimmed(&self) -> Result<&Vec<i16>, Pod5ReadError> {
        self.signal_trimmed
            .as_ref()
            .ok_or(Pod5ReadError::TrimmedSignalNotFound)
    }

    /// Returns the number of samples in the trimmed signal
    pub fn num_samples_trimmed(&self) -> Result<&usize, Pod5ReadError> {
        self.num_samples_trimmed
            .as_ref()
            .ok_or(Pod5ReadError::TrimmedSignalNotFound)
    }

    /// Returns the offset from which the alignment starts 
    /// Corresponds to either *sp* + *ts* or signal_len - *ns*
    /// for reversed signal.
    /// 
    /// The offset is used to adjust the final alignment so it 
    /// can be used directly with the (untrimmed) signal found 
    /// in a pod5 read. 
    pub fn trimmed_signal_offset(&self) -> Result<&usize, Pod5ReadError> {
        self.signal_offset
            .as_ref()
            .ok_or(Pod5ReadError::TrimmedSignalNotFound)
    }

    /// Returns the calibration offset for this read
    pub fn calibration_offset(&self) -> &f32 {
        &self.calibration_offset
    }

    /// Returns the calibration scale factor for this read
    pub fn calibration_scale(&self) -> &f32 {
        &self.calibration_scale
    }

    /// Trim the signal based on the *sp*, *ts* and *ns* tags
    /// found in the corresponding bam read. Once called the 
    /// original signal is overwritten to minimize memory usage.
    /// 
    /// This function is called when initializing an AlignedRead.
    /// At this point the AlignedRead takes ownership of the read.
    /// 
    /// # Arguments
    /// * `reverse_signal` - bool indicating if the signal must be reversed
    /// (in case of direct RNA sequencing reads)
    /// * `parent_signal_offset` - value behind the *sp* tag if available
    /// * `trimmed_signal_len` - value behind the *ts* tag if available
    /// * `subread_signal_len` - value behind the *ns* tag if available
    /// 
    /// # Errors
    /// * `Pod5ReadError::TrimError` - If the trimming fails
    /// 
    /// # Note: 
    /// The *ts* and *ns* values are relative to the signal starting at the offset
    /// given by *sp*. Accordingly the *sp* value must be added to account for it.
    /// ```text
    /// --------------------------
    /// |   |                    |
    /// s   sp                   size
    ///     ----------------------
    ///     |    |          |    |
    ///     s_o  ts         ns
    ///          -----------
    ///         trimmed signal
    /// ```
    pub fn update_signal(
        &mut self,
        reverse_signal: bool,
        parent_signal_offset: Option<usize>,
        trimmed_signal_len: Option<usize>,
        subread_signal_len: Option<usize>
    ) -> Result<(), Pod5ReadError> {
        match self.signal_trimmed {
            None => {
                let parent_signal_offset = match parent_signal_offset {
                    Some(v) => v,
                    None => 0            
                };
                let trimmed_signal_len = match trimmed_signal_len {
                    Some(v) => v,
                    None => 0
                };
        
                let start = parent_signal_offset + trimmed_signal_len;
        
                let end = match subread_signal_len {
                    Some(v) => parent_signal_offset + v,
                    None => self.num_samples            
                };
        
                if end > self.num_samples {
                    return Err(Pod5ReadError::TrimError(
                        format!(
                            "'subread_signal_len' ({}) out of bounds with signal length {}",
                            end, self.num_samples
                        )
                    ));
                } else if start >= end {
                    return Err(Pod5ReadError::TrimError(
                        format!(
                            "Start index ({}) must be smaller than end index ({})",
                            start, end
                        )
                    ));
                }
        
                let mut signal = self.signal.clone();
                if reverse_signal {
                    signal = signal[start..end].to_vec();
                    signal = helpers::reverse_signal(&signal);
                } else {
                    signal = signal[start..end].to_vec();
                }
        
                log::debug!(
                    "update_signal info: trimmed signal contains data from signal[{}..{}]; sig. len before = {}, after = {}",
                    start, end, self.num_samples, signal.len()
                );

                self.num_samples_trimmed = Some(signal.len());
                self.signal_trimmed = Some(signal);

                // This offset will be added to the alignment(s) in the end so the alignment can be used 
                // with the signal untrimmed signal stored in the pod5 file without the tag information
                self.signal_offset = if reverse_signal {
                    Some(self.num_samples - end)
                } else {
                    Some(start)
                };

                Ok(())
            }
            // If the trimming was performed before, no update is needed
            Some(_) => Ok(())
        }
    }
}



/// A container for Pod5 file data that provides access to reads using a HashMap
/// 
/// This struct loads and parses Pod5 files, exposing the reads and their associated
/// signal data through a HashMap interface.
#[derive(Debug, Clone)]
pub struct Pod5File {
    path: PathBuf,
    reads: HashMap<String, Pod5Read>
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
    pub fn new(path: &PathBuf) -> Result<Self, Pod5FileError> {
        log::info!("Initializing Pod5File from path '{}'", path.display());

        let file = File::open(path)?;
        let mut pod5_reader = reader::Reader::from_reader(file)?;

        let mut read_collection = HashMap::new();

        // Extract the needed information from the signal_df dataframes and 
        // add it to the existing Pod5Read objects.
        // (i.e. columns calibration_offset & calibration_scale)
        for read_df in pod5_reader.read_dfs()?.flatten() {
            let df = read_df.into_inner();

            // Create an iterator for each column
            let read_id_col_iter = df
                .column("minknow.uuid")?
                .str()?
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
                let read_id = read_id.ok_or(Pod5FileError::MissingReadId)?.to_string();

                let offset = offset.ok_or(Pod5FileError::ColumnDataMissingError { 
                    column: "calibration_offset".to_string(), 
                    read_id: read_id.clone()
                })?;

                let scale = scale.ok_or(Pod5FileError::ColumnDataMissingError { 
                    column: "calibration_scale".to_string(), 
                    read_id: read_id.clone()
                })?;

                let read = Pod5Read::init(
                    read_id.clone(), 
                    offset, 
                    scale
                );
                read_collection.insert(read_id, read);
            }
        }


        // Extract the needed information from the signal_df dataframes
        // (i.e. columns read_id, signal and samples)
        for signal_df in pod5_reader.signal_dfs()?.flatten() {
            let df = signal_df
                .into_inner();
            // Create an iterator for each column
            let read_id_col_iter = df
                .column("minknow.uuid")?
                .str()?
                .iter();
            let signal_col_iter = df
                .column("minknow.vbz")?
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
                let read_id = read_id.ok_or(Pod5FileError::MissingReadId)?.to_string();
                
                // Convert signal data to rust-native Vec<i16>
                let mut signal = signal
                    .ok_or(Pod5FileError::ColumnDataMissingError { 
                        column: "minknow.vbz".to_string(), 
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

                // Add the extracted information to the read at hand
                read_collection
                    .get_mut(&read_id)
                    .ok_or(Pod5FileError::ReadNotFound(read_id))?
                    .add_signal_df_data(&mut signal, num_samples);
            }
        }

        log::debug!(
            "Pod5File::new info: from '{}'; num reads = {}", 
            path.display(), read_collection.len()
        );

        Ok(Pod5File {
            path: path.clone(),
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

    /// Returns the path of the underlying pod5 file
    pub fn path(&self) -> &PathBuf {
        &self.path
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



/// A collection of Pod5File paths that loads files when explicitly requested
/// 
/// This struct manages multiple Pod5 file paths and loads Pod5File
/// objects only when they are explicitly requested through the load_file method.
#[derive(Debug)]
pub struct Pod5Index {
    // Stores the Pod5File object with the path to the file
    file_paths: Vec<PathBuf>,
}


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
    pub fn from_dir(path: &PathBuf, recursive: bool) -> Result<Self, Pod5IndexError> {
        let file_paths = helpers::find_files_in_dir(path, "pod5", recursive)?;

        log::info!("Initialized Pod5Index from '{}' containing {} files", path.display(), file_paths.len());
        
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
    pub fn from_files(paths: &Vec<PathBuf>) -> Result<Self, Pod5IndexError> {
        let file_paths = helpers::get_files(paths, "pod5")?;

        log::info!("Initialized Pod5Index from {} files", file_paths.len());

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
    pub fn load_file(&self, file_path: &PathBuf) -> Result<Pod5File, Pod5IndexError> {
        log::info!("Loading file '{}'", file_path.display());
        
        // Check if the file path is in the current collection
        if !self.file_paths.contains(&file_path) {
            return Err(Pod5IndexError::FileNotFound(file_path.clone()));
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
    pub fn file_paths(&self) -> &[PathBuf] {
        &self.file_paths
    }

    /// Returns the number of files in the collection
    /// 
    /// # Returns
    /// * `usize` - Number of files
    pub fn num_files(&self) -> usize {
        self.file_paths.len()
    }

    pub fn num_reads(&self) -> Result<usize, Pod5IndexError> {
        let mut n_reads = 0;
        for file in self.files() {
            let file = file?;
            n_reads += file.num_reads();
        }
        Ok(n_reads)
    }


    /// Returns an iterator that yields all files.
    ///
    /// This method provides a convenient way to process all files stored in the Pod5Index.
    ///
    /// # Returns
    /// * `Pod5FileIterator` - An iterator that yields a Pod5File
    ///
    /// # Example
    /// ```ignore
    /// let index = Pod5Index::from_dir("data/", true)?;
    /// for result in index.files() {
    ///     match result {
    ///         Ok(file) => {
    ///             println!("Path: {}, Num reads: {}", 
    ///                     file.path(), file.num_reads());
    ///         },
    ///         Err(err) => eprintln!("Error: {}", err),
    ///     }
    /// }
    /// ```
    #[cfg_attr(doctest, ignore)]
    pub fn files(&self) -> Pod5FileIterator {
        Pod5FileIterator {
            index: self,
            current_ixd: 0
        }
    }

    /// Returns an iterator that yields all reads from all Pod5 files in the Pod5Index
    ///
    /// This method provides a convenient way to process all reads across multiple
    /// Pod5 files without manually loading each file.
    ///
    /// # Returns
    /// * `Pod5ReadsIterator` - An iterator that yields (file_path, read_id, Pod5Read) triples
    ///
    /// # Example
    /// ```ignore
    /// let index = Pod5Index::from_dir("data/", true)?;
    /// for result in index.reads() {
    ///     match result {
    ///         Ok((file_path, read_id, read)) => {
    ///             println!("File: {}, Read ID: {}, Samples: {}", 
    ///                     file_path, read_id, read.num_samples());
    ///         },
    ///         Err(err) => eprintln!("Error: {}", err),
    ///     }
    /// }
    /// ```
    pub fn reads(&self) -> Pod5ReadIterator {
        Pod5ReadIterator {
            file_paths: self.file_paths.clone(),
            current_file_idx: 0,
            current_reads: Vec::new(),
            current_read_idx: 0
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

/// An iterator that yields reads from all Pod5 files in the index
pub struct Pod5ReadIterator {
    file_paths: Vec<PathBuf>,
    current_file_idx: usize,
    current_reads: Vec<(String, Pod5Read)>,
    current_read_idx: usize
}

impl Iterator for Pod5ReadIterator {
    type Item = Result<(PathBuf, String, Pod5Read), Pod5IndexError>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.current_read_idx < self.current_reads.len() {
            // Check if we have reads from the current file to yield
            let (read_id, read) = self.current_reads[self.current_read_idx].clone();
            let file_path = self.file_paths[self.current_file_idx - 1].clone();
            self.current_read_idx += 1;
            return Some(Ok((file_path, read_id, read)));
        } else if self.current_file_idx >= self.file_paths.len() { 
            // No more reads in the current file, try to load the next file
            return None;
        }

        let file_path = &self.file_paths[self.current_file_idx];
        self.current_file_idx += 1;

        // Load the next file
        match Pod5File::new(file_path) {
            Ok(file) => {
                // Extract all reads from the file
                self.current_reads.clear();
                self.current_read_idx = 0;

                // Convert the HashMap into a Vec of (read_id, Pod5Read) pairs
                for (read_id, read) in file.reads {
                    self.current_reads.push((read_id, read));
                }
                
                // If the file has reads, return the first one
                if !self.current_reads.is_empty() {
                    let (read_id, read) = self.current_reads[0].clone();
                    self.current_read_idx = 1;
                    return Some(Ok((file_path.clone(), read_id, read)));
                } else {
                    // Try the next file if this one has no reads
                    return self.next();
                }
            },
            Err(err) => Some(Err(Pod5IndexError::FileLoadingError(err)))
        }
    }
}