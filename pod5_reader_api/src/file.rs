use std::{
    collections::HashMap, 
    path::PathBuf
};
use arrow2::io::ipc::read::FileMetadata;
use uuid::Uuid;
use crate::{
    core::feather_reader::FeatherReader, 
    file::{file_async::FeatherReaderPool, signal_table_index::SignalTableIndex}, 
    core::footer::Pod5Footer, 
    read::Pod5Read, 
    core::tables::run_info::RunInfo
};

mod file_sync;
mod file_async;
mod iterator;
mod signal_table_index;

pub(crate) const EXPECTED_SIGNATURE: [u8; 8] = [139, 80, 79, 68, 13, 10, 26, 10];

/// Helper struct representing a row index within a specific chunk of the signal table.
pub(crate) struct ChunkRowIndex {
    pub(crate) chunk: usize,
    pub(crate) row: usize
}

/// Representation of a single pod5 file.
/// 
/// Provides access to reads and run information contained in the file.
/// Handles file parsing in multiple steps:
/// 1. Signature verification
/// 2. Footer parsing
/// 3. Run info extraction
/// 4. Reads table parsing
/// 5. Lazy signal data loading
#[derive(Debug)]
pub struct Pod5File {
    path: PathBuf,
    read_ids: Vec<Uuid>,
    reads: HashMap<Uuid, Pod5Read>,
    run_info : RunInfo,
    signal_table_reader: FeatherReader,
    signal_table_metadata: FileMetadata,
    signal_table_index: SignalTableIndex,
    signal_table_batch_size: u64,
    footer: Pod5Footer
}


/// Thread-safe representation of a single Pod5 file.
/// 
/// `Pod5FileAsync` provides concurrent access to all data within a Pod5 file,
/// including reads, signal data, and metadata. It's designed for scenarios where
/// multiple threads need to process different reads from the same file simultaneously.
/// 
/// ## Architecture
/// 
/// The file is structured with several key components:
/// - **Metadata Cache**: Run info and read metadata are parsed once and cached
/// - **Reader Pool**: Multiple FeatherReaders enable concurrent signal table access
/// - **Lazy Loading**: Signal data is loaded on-demand when reads are accessed
/// - **Thread Safety**: All operations are safe for concurrent use
/// 
/// ## Initialization Process
/// 
/// File initialization involves several steps:
/// 1. **Signature Verification**: Validates Pod5 file format
/// 2. **Footer Parsing**: Extracts embedded table locations
/// 3. **Run Info Loading**: Parses sequencing run metadata
/// 4. **Reads Table Parsing**: Builds read index and metadata cache
/// 5. **Reader Pool Setup**: Initializes concurrent signal table readers
/// 
/// ## Performance Characteristics
/// 
/// - **Concurrent Reads**: Multiple threads can access different reads simultaneously
/// - **Memory Efficient**: Metadata cached once, signal data loaded on-demand
/// - **I/O Optimized**: Reader pool minimizes file handle overhead
/// - **Scalable**: Performance scales with number of worker threads
/// 
/// ## Usage Examples
/// 
/// ```rust,ignore
/// use std::path::PathBuf;
/// use std::thread;
/// 
/// // Initialize async file with 4 workers
/// let file = Pod5FileAsync::new(&PathBuf::from("data.pod5"), 4)?;
/// 
/// // Concurrent access from multiple threads
/// let handles: Vec<_> = read_ids.into_iter().map(|read_id| {
///     let file_ref = &file;
///     thread::spawn(move || {
///         let read = file_ref.get(&read_id)?;
///         process_read(read)
///     })
/// }).collect();
/// ```
#[derive(Debug)]
pub struct Pod5FileAsync {
    /// Path to the Pod5 file
    path: PathBuf,
    /// Ordered list of all read IDs in the file
    read_ids: Vec<Uuid>,
    /// Fast lookup map from read ID to read metadata (without signal data)
    reads: HashMap<Uuid, Pod5Read>,
    /// Sequencing run information and metadata
    run_info : RunInfo,
    /// Thread-safe pool of readers for signal table access
    signal_table_reader_pool: FeatherReaderPool,
    /// Parsed file footer containing embedded table information
    footer: Pod5Footer
}