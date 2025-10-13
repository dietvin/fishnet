mod dataset;
mod dataset_thread_safe;

use std::{
    collections::HashMap, 
    ffi::OsString, 
};


use uuid::Uuid;

use crate::{
    dataset::dataset_thread_safe::{
        file_shared_thread_safe::Pod5FileThreadSafeShared, 
        reader_pool::FeatherReaderPoolShared
    }, 
    file::Pod5File
};


/// A collection of POD5 files that can be accessed as a single dataset.
/// 
/// Provides both indexed and path-based access to individual POD5 files,
/// along with iteration capabilities.
#[derive(Debug)]
pub struct Pod5Dataset {
    files: Vec<Pod5File>,
    file_index: HashMap<OsString, usize>,
    n_files: usize,
    reads_index: HashMap<Uuid, usize>,
    n_reads: usize
}

/// Thread-safe dataset for efficient random access across multiple Pod5 files.
/// 
/// `Pod5DatasetThreadSafe` provides high-performance, concurrent access to reads distributed
/// across multiple Pod5 files. It's designed for applications that need to randomly access
/// reads by ID without the overhead of managing individual file readers.
/// 
/// ## Key Features
/// 
/// - **Thread-Safe**: Supports concurrent read access from multiple threads
/// - **Memory Efficient**: Uses a shared reader pool instead of per-file readers
/// - **Random Access**: O(1) lookup of reads by UUID across all files
/// - **Optimized Buffering**: Intelligent reader caching for common access patterns
/// 
/// ## Performance Characteristics
/// 
/// The dataset is optimized for applications where:
/// - Reads are accessed randomly by ID rather than sequentially by file
/// - The same file tends to be accessed repeatedly (reader pool optimization)
/// - Memory usage needs to be controlled even with hundreds of files
/// 
/// ## Usage Example
/// 
/// ```rust,ignore
/// use std::path::PathBuf;
/// 
/// // Initialize dataset with multiple Pod5 files
/// let paths = vec![
///     PathBuf::from("file1.pod5"),
///     PathBuf::from("file2.pod5"),
/// ];
/// let dataset = Pod5DatasetThreadSafe::new(&paths, 4)?;
/// 
/// // Random access to reads by ID
/// let read = dataset.get_read(&read_id)?;
/// let signal_data = read.signal().unwrap();
/// ```
/// 
/// ## Memory Management
/// 
/// The dataset maintains a bounded pool of file readers (default: 2 × n_workers)
/// to balance memory usage with performance. Readers are allocated on-demand and
/// cached using an LRU eviction policy.
pub struct Pod5DatasetThreadSafe {
    /// Lightweight representations of all Pod5 files in the dataset
    files: Vec<Pod5FileThreadSafeShared>,
    /// Map from filename to file index for path-based lookups
    file_index: HashMap<OsString, usize>,
    /// Total number of files in the dataset
    n_files: usize,

    /// Combined read IDs from all files, maintaining file order
    read_ids: Vec<Uuid>,
    /// Map from read ID to file index for O(1) read location
    reads_index: HashMap<Uuid, usize>,
    /// Total number of reads across all files
    n_reads: usize,

    /// Shared reader pool for efficient file access
    reader_pool: FeatherReaderPoolShared
}