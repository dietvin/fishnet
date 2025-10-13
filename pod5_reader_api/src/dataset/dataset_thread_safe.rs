//! # Pod5 Thread-safe Dataset Module
//! 
//! This module provides a thread-safe implementation for parallel random access to multiple Pod5 files.
//! The design addresses the challenge of efficiently managing file readers across hundreds of Pod5 files
//! without creating excessive numbers of reader objects.
//! 
//! ## Architecture Overview
//! 
//! The solution implements a **reader pool at the dataset level** rather than per-file, which:
//! - Maintains a controlled number of file readers regardless of dataset size
//! - Provides efficient buffering for readers across different files
//! - Optimizes for sequential access patterns within the same file
//! - Handles concurrent access through thread-safe reader management
//! 
//! ## Performance Characteristics
//! 
//! - **Best performance**: Sequential reads from the same file (minimal reader switching overhead)
//! - **Good performance**: Sequential reads across files in order (gradual reader pool updates)
//! - **Acceptable performance**: Random access across different files (higher switching overhead)
//! 
//! ## Key Components
//! 
//! - `Pod5DatasetThreadSafe`: Main dataset interface for thread-safe operations
//! - `FeatherReaderPoolShared`: Thread-safe reader pool managing file access
//! - `Pod5FileThreadSafeShared`: Lightweight file representation optimized for dataset use
//! - `SignalReaderConfig`: Configuration for efficient signal table access
//! 
//! ## Usage Note
//! 
//! This implementation prioritizes random access performance over iteration. For efficient
//! file-by-file iteration, consider using the standard `Pod5Dataset` instead.

pub(super) mod file_shared_thread_safe;
pub(super) mod reader_pool;
mod signal_reader_config;
mod buffered_feather_reader;

use std::{
    collections::HashMap, 
    ffi::OsString, 
    path::PathBuf
};
use uuid::Uuid;

use crate::{
    dataset::{
        dataset_thread_safe::{
            file_shared_thread_safe::Pod5FileThreadSafeShared, 
            reader_pool::FeatherReaderPoolShared
        }, 
        Pod5DatasetThreadSafe
    }, 
    error::{
        dataset::Pod5DatasetError, 
        file::Pod5FileError
    }, 
    file::{
        Pod5FileThreadSafe, 
        Pod5File
    }, 
    read::Pod5Read
};

impl Pod5DatasetThreadSafe {
    /// Creates a new thread-safe dataset from a collection of Pod5 file paths.
    /// 
    /// This constructor performs several initialization steps:
    /// 1. **File Validation**: Opens and validates each Pod5 file
    /// 2. **Metadata Extraction**: Parses read tables and builds lookup indices
    /// 3. **Index Building**: Creates global read ID to file mappings
    /// 4. **Pool Initialization**: Sets up the shared reader pool
    /// 
    /// The initialization process can take some time for large datasets as it
    /// needs to parse metadata from all files.
    /// 
    /// # Arguments
    /// 
    /// * `paths` - Vector of filesystem paths to Pod5 files
    /// * `n_workers` - Number of concurrent workers (affects reader pool size)
    /// 
    /// # Returns
    /// 
    /// A new dataset ready for concurrent operations, or an error if any file
    /// cannot be processed or if the configuration is invalid.
    /// 
    /// # Errors
    /// 
    /// * `EmptyPathList` if no paths are provided
    /// * File system errors for inaccessible or missing files
    /// * Pod5 format errors for corrupted or invalid files
    /// * Pool initialization errors for invalid worker/buffer configurations
    /// 
    /// # Performance Notes
    /// 
    /// - Initialization time scales with the number and size of files
    /// - Memory usage includes cached metadata for all files
    /// - Reader pool size defaults to 2 × n_workers for optimal buffering
    pub fn new(
        paths: &Vec<PathBuf>,
        n_workers: usize,
    ) -> Result<Self, Pod5DatasetError> {
        if paths.is_empty() {
            return Err(Pod5DatasetError::EmptyPathList);
        }

        let mut files = Vec::with_capacity(paths.len());
        let mut file_index = HashMap::with_capacity(paths.len());
        let mut reads_index = HashMap::new();
        let mut read_ids = vec![];

        // Process each file and build global indices
        for (file_id, path_buf) in paths.iter().enumerate() {
            let path = path_buf.as_os_str().to_os_string();
            file_index.insert(path, file_id);
            
            let file = Pod5FileThreadSafeShared::new(file_id, path_buf)?;
            
            // Merge read IDs from this file into global collection
            let file_read_ids = file.read_ids().clone();
            read_ids.append(&mut file_read_ids.clone());

            // Map each read ID to its containing file
            for read_id in file_read_ids {
                reads_index.insert(read_id, file_id);
            }
            
            files.push(file);

        }

        let n_files = files.len();
        let n_reads = reads_index.len();

        // Initialize reader pool with default buffer size (2x workers)
        let reader_pool = FeatherReaderPoolShared::new(
            &files, 
            n_workers * 2
        )?;

        Ok(Self { 
            files, 
            file_index,
            n_files,
            read_ids,
            reads_index,
            n_reads,
            reader_pool
        })
    }

    /// Returns all read IDs across the entire dataset.
    /// 
    /// The read IDs are ordered by file (all reads from file 0, then file 1, etc.)
    /// and within each file by their original order in the reads table.
    /// 
    /// # Returns
    /// 
    /// A reference to the vector containing all read IDs in the dataset.
    pub fn read_ids(&self) -> &Vec<Uuid> {
        &self.read_ids
    }

    /// Retrieves a complete Pod5Read by its UUID.
    /// 
    /// This is the primary data access method for the dataset. It:
    /// 1. **Locates the File**: Uses the read index to find which file contains the read
    /// 2. **Acquires a Reader**: Gets an appropriate reader from the pool
    /// 3. **Loads the Data**: Retrieves read metadata and reconstructs signal data
    /// 4. **Returns the Reader**: Automatically returns the reader to the pool
    /// 
    /// The operation is thread-safe and can be called concurrently from multiple threads.
    /// 
    /// # Arguments
    /// 
    /// * `read_id` - UUID of the read to retrieve
    /// 
    /// # Returns
    /// 
    /// A complete `Pod5Read` with all metadata and signal data loaded, or an error
    /// if the read cannot be found or loaded.
    /// 
    /// # Errors
    /// 
    /// * `ReadNotFound` if the read_id doesn't exist in any file
    /// * Reader pool errors if too many concurrent operations are in progress
    /// * Signal reconstruction errors for corrupted data
    /// 
    /// # Performance Notes
    /// 
    /// - Best performance when accessing reads from the same file sequentially
    /// - Reader pool minimizes overhead for cross-file access patterns
    /// - Signal data is reconstructed on each call (not cached)
    pub fn get_read(&self, read_id: &Uuid) -> Result<Pod5Read, Pod5DatasetError> {
        let file_id = self.reads_index.get(read_id).ok_or(
            Pod5FileError::ReadNotFound(read_id.clone())
        )?.clone();

        self.reader_pool.with_reader(&file_id, |signal_table_reader| {
            Ok(self.files[file_id].get(read_id, signal_table_reader)?)
        })
    }

    /// Retrieves a standard Pod5File by its dataset index.
    /// 
    /// This method converts the internal shared file representation to a full
    /// Pod5File instance. The returned file can be used for operations not
    /// supported by the dataset interface, such as iteration or detailed metadata access.
    /// 
    /// # Arguments
    /// 
    /// * `file_idx` - Zero-based index of the file (matches order in constructor paths)
    /// 
    /// # Returns
    /// 
    /// A new `Pod5File` instance ready for comprehensive operations, or an error
    /// if the index is invalid or file conversion fails.
    /// 
    /// # Performance Warning
    /// 
    /// This method re-initializes the file from scratch, which can be expensive
    /// for large files. The new file instance has its own reader and doesn't
    /// benefit from the dataset's reader pool.
    /// 
    /// # Errors
    /// 
    /// * `FileIndexError` if the file_idx is out of bounds
    /// * File initialization errors for inaccessible or corrupted files
    pub fn get_file_by_index(&self, file_idx: usize) -> Result<Pod5File, Pod5DatasetError> {
        let file = self.files
            .get(file_idx)
            .ok_or(Pod5DatasetError::FileIndexError(file_idx, self.n_files))?
            .to_pod5_file()?;

        Ok(file)
    }

    /// Retrieves a standard Pod5File by its original path.
    /// 
    /// Similar to `get_file_by_index()`, but uses the original file path for lookup.
    /// This is useful when you need to convert from path-based operations back to
    /// file-based operations.
    /// 
    /// # Arguments
    /// 
    /// * `path` - Original filesystem path used during dataset initialization
    /// 
    /// # Returns
    /// 
    /// A new `Pod5File` instance, or an error if the path is not found or
    /// file conversion fails.
    /// 
    /// # Performance Warning
    /// 
    /// Same performance considerations as `get_file_by_index()` apply.
    /// 
    /// # Errors
    /// 
    /// * `InvalidKey` if the path was not used during dataset initialization
    /// * File initialization errors for inaccessible or corrupted files
    pub fn get_file(&self, path: &OsString) -> Result<Pod5File, Pod5DatasetError> {
        let file_idx = self.file_index.get(path)
                .ok_or(Pod5DatasetError::InvalidKey(path.clone()))?
                .clone();
        self.get_file_by_index(file_idx)
    }

    /// Retrieves a thread-safe Pod5File by its dataset index.
    /// 
    /// This method creates a new `Pod5FileThreadSafe` instance with its own reader pool.
    /// Unlike the dataset's shared pool, the returned file's pool is dedicated to
    /// that specific file, which can be more efficient for intensive single-file operations.
    /// 
    /// # Arguments
    /// 
    /// * `file_idx` - Zero-based index of the file
    /// * `n_workers` - Number of workers for the file's dedicated reader pool
    /// 
    /// # Returns
    /// 
    /// A new `Pod5FileThreadSafe` instance ready for concurrent operations, or an error
    /// if initialization fails.
    /// 
    /// # Performance Warning
    /// 
    /// Creates a new file instance with initialization overhead. Consider whether
    /// dataset-level operations might be more appropriate.
    /// 
    /// # Errors
    /// 
    /// * `FileIndexError` if the file_idx is out of bounds
    /// * File initialization errors for configuration or access problems
    pub fn get_file_thread_safe_by_index(&self, file_idx: usize, n_workers: usize) -> Result<Pod5FileThreadSafe, Pod5DatasetError> {
        let file = self.files
            .get(file_idx)
            .ok_or(Pod5DatasetError::FileIndexError(file_idx, self.n_files))?
            .to_pod5_file_thread_safe(n_workers)?;

        Ok(file)
    }

    /// Retrieves an thread-safe Pod5File by its original path.
    /// 
    /// Path-based version of `get_file_thread_safe_by_index()`. Creates a new thread-safe file
    /// instance with its own dedicated reader pool.
    /// 
    /// # Arguments
    /// 
    /// * `path` - Original filesystem path used during dataset initialization
    /// * `n_workers` - Number of workers for the file's reader pool
    /// 
    /// # Returns
    /// 
    /// A new `Pod5FileThreadSafe` instance, or an error if the path is invalid or
    /// initialization fails.
    /// 
    /// # Performance Warning
    /// 
    /// Same considerations as `get_file_thread_safe_by_index()` apply.
    /// 
    /// # Errors
    /// 
    /// * `InvalidKey` if the path was not used during dataset initialization  
    /// * File initialization errors for configuration or access problems
    pub fn get_file_thread_safe(&self, path: &OsString, n_workers: usize) -> Result<Pod5FileThreadSafe, Pod5DatasetError> {
        let file_idx = self.file_index.get(path)
                .ok_or(Pod5DatasetError::InvalidKey(path.clone()))?
                .clone();
        self.get_file_thread_safe_by_index(file_idx, n_workers)
    }


    /// Returns the total number of files in the dataset.
    /// 
    /// # Returns
    /// 
    /// Count of Pod5 files included in this dataset.
    pub fn n_files(&self) -> usize {
        self.n_files
    }
    
    /// Returns the total number of reads across all files in the dataset.
    /// 
    /// # Returns
    /// 
    /// Combined count of all reads from all files in the dataset.
    pub fn n_reads(&self) -> usize {
        self.n_reads
    }
}
