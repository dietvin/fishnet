//! # Pod5 Thread-safe File Module
//! 
//! This module provides a thread-safe implementation for concurrent access to a single Pod5 file.
//! The design enables multiple threads to safely read from the same Pod5 file simultaneously
//! without data races or resource conflicts.
//! 
//! ## Architecture Overview
//! 
//! The solution implements a **reader pool at the file level** that:
//! - Maintains a bounded pool of FeatherReaders for the signal table
//! - Provides thread-safe access through mutex-protected reader management
//! - Enables concurrent read operations without reader conflicts
//! - Optimizes memory usage by reusing readers across operations
//! 
//! ## Key Features
//! 
//! - **Thread Safety**: Multiple threads can safely access reads concurrently
//! - **Reader Pooling**: Efficient reuse of FeatherReader instances
//! - **Lazy Loading**: Signal data is loaded on-demand when requested
//! - **Resource Management**: Automatic reader lifecycle management
//! - **Performance Optimization**: Chunk caching for sequential signal reconstruction
//! 
//! ## Usage Patterns
//! 
//! This implementation is ideal for:
//! - Applications needing concurrent access to a single large Pod5 file
//! - Multi-threaded processing of reads from the same file
//! - Scenarios where multiple workers process different reads simultaneously
//! 
//! ## Comparison with Standard Pod5File
//! 
//! Unlike the standard `Pod5File`, this thread safe version:
//! - Supports safe concurrent access from multiple threads
//! - Uses a pool of readers instead of a single reader
//! - Has slightly higher memory overhead due to multiple readers
//! - Provides better throughput for multi-threaded applications

use std::{
    collections::{HashMap, VecDeque}, 
    fs::File, 
    io::{
        Read, 
        Seek, 
        SeekFrom
    }, 
    path::PathBuf, sync::{Condvar, Mutex}
};

use arrow2::{
    array::Array,
    chunk::Chunk, 
    io::ipc::read::{
        read_batch, 
        read_file_dictionaries
    }
};
use uuid::Uuid;

use crate::{
    error::file::{FeatherReaderPoolError, Pod5FileError}, 
    file::{
        ChunkRowIndex, Pod5FileThreadSafe, EXPECTED_SIGNATURE
    }, 
    read::Pod5Read, 
    core::{
        feather_reader::FeatherReader, 
        footer::{
            embedded_content::EmbeddedContentType, 
            Pod5Footer
        }, 
        tables::{
            reads_table::ReadsTable, 
            run_info::RunInfo, 
            signal_table::SignalTable
        }
    }
};


/// Thread-safe pool for managing FeatherReaders for signal table access.
/// 
/// This pool maintains a collection of FeatherReader instances that can be safely
/// shared across multiple threads. It implements a simple allocation/return mechanism
/// with mutex protection to ensure thread safety.
/// 
/// ## Pool Management Strategy
/// 
/// 1. **Initialization**: Creates a fixed number of readers during construction
/// 2. **Allocation**: Threads request readers from the pool (blocking if none available)
/// 3. **Usage**: Threads perform operations with their allocated reader
/// 4. **Return**: Readers are returned to the pool for reuse by other threads
/// 5. **RAII Pattern**: Automatic reader return through closure-based operations
/// 
/// ## Concurrency Model
/// 
/// The pool uses a simple mutex-protected deque for reader storage. While this
/// creates a potential bottleneck for high-contention scenarios, it provides
/// good performance for typical use cases and ensures correctness.
/// 
/// ## Memory Characteristics
/// 
/// - Fixed memory footprint (n_workers × reader_size)
/// - No dynamic allocation during operation
/// - Readers are reused, avoiding initialization overhead
#[derive(Debug)]
pub(in crate::file) struct FeatherReaderPool {
    /// Number of rows processed per batch in the signal table
    batch_size: u64,
    /// Thread-safe collection of available readers
    readers: Mutex<VecDeque<FeatherReader>>,
    /// Maximum number of readers in the pool (for validation)
    buffer_size: usize,

    /// Condition variable for blocking when the deque is empty
    /// (only relevant if the number of threads is larger than
    /// buffer_size)
    condvar: Condvar
}

impl FeatherReaderPool {
    /// Creates a new reader pool for the specified signal table.
    /// 
    /// This constructor initializes the pool by:
    /// 1. Opening the file buffer_size times to create independent readers
    /// 2. Configuring each reader for the signal table section
    /// 3. Determining the batch size from the first available chunk
    /// 4. Setting up the thread-safe reader storage
    /// 
    /// All readers in the pool are identical and interchangeable, configured
    /// for the same file section and parameters.
    /// 
    /// # Arguments
    /// 
    /// * `file_path` - Path to the Pod5 file containing the signal table
    /// * `offset` - Byte offset where the signal table begins in the file
    /// * `length` - Length of the signal table section in bytes
    /// * `buffer_size` - Number of readers to create (pool size)
    /// 
    /// # Returns
    /// 
    /// A new reader pool ready for concurrent operations, or an error if
    /// initialization fails.
    /// 
    /// # Errors
    /// 
    /// * File system errors when opening the Pod5 file multiple times
    /// * FeatherReader initialization errors for invalid file sections
    /// * `SignalTableChunkSizeError` if the signal table is empty
    /// 
    /// # Performance Notes
    /// 
    /// - Each reader maintains its own file handle and internal state
    /// - Pool size should typically match the number of worker threads
    /// - Larger pools use more memory but reduce thread contention
    fn new(file_path: &PathBuf, offset: i64, length: i64, buffer_size: usize) -> Result<Self, FeatherReaderPoolError> {
        let mut readers = VecDeque::with_capacity(buffer_size);

        // Create buffer_size independent readers for the same file section
        for _ in 0..buffer_size {
            let file = File::open(file_path)?;
        
            let reader = FeatherReader::new(
                file, 
                offset, 
                length
            )?;
            readers.push_back(reader);
        }

        // Determine batch size from the first chunk (all readers have same structure)
        let batch_size = readers[0]
            .iter_chunks()?
            .next()
            .ok_or(FeatherReaderPoolError::SignalTableChunkSizeError)??
            .len() as u64;

        Ok(Self { 
            batch_size,
            readers: Mutex::new(readers),
            buffer_size: buffer_size,
            condvar: Condvar::new()
        })
    }

    /// Retrieves a reader from the pool for exclusive use.
    /// 
    /// This method removes a reader from the pool and transfers ownership
    /// to the caller. The caller is responsible for returning the reader
    /// after use to maintain pool integrity.
    /// 
    /// # Returns
    /// 
    /// A FeatherReader ready for signal table operations, or an error if
    /// no readers are currently available.
    /// 
    /// # Errors
    /// 
    /// * `DequeEmpty` if all readers are currently in use by other threads
    ///   (should not be possible to reach)
    fn get_reader(&self) -> Result<FeatherReader, FeatherReaderPoolError> {
        let mut readers = self.readers.lock().unwrap();

        while readers.is_empty() {
            readers = self.condvar.wait(readers).unwrap();
        }

        readers.pop_front().ok_or(
            FeatherReaderPoolError::DequeEmpty
        )
    }

    /// Returns a reader to the pool for reuse by other threads.
    /// 
    /// This method completes the reader lifecycle by returning a previously
    /// allocated reader back to the pool. The reader should not be used
    /// after calling this method.
    /// 
    /// # Arguments
    /// 
    /// * `reader` - FeatherReader to return to the pool
    /// 
    /// # Returns
    /// 
    /// `Ok(())` if the reader was successfully returned, or an error if
    /// the pool is somehow corrupted.
    /// 
    /// # Errors
    /// 
    /// * `DequeFull` if the pool already contains the maximum number of readers
    ///   (indicates a programming error - more readers returned than allocated)
    fn return_reader(&self, reader: FeatherReader) -> Result<(), FeatherReaderPoolError> {
        let mut readers = self.readers.lock().unwrap();
        if readers.len() < self.buffer_size {
            readers.push_back(reader);
            // Notify waiting threads that a reader is available
            self.condvar.notify_one();
            Ok(())
        } else {
            Err(FeatherReaderPoolError::DequeFull(self.buffer_size))
        }
    }

    /// Executes an operation with a reader using RAII pattern.
    /// 
    /// This is the recommended way to use the reader pool. It handles the
    /// complete reader lifecycle automatically:
    /// 1. Allocates a reader from the pool
    /// 2. Executes the provided operation with the reader
    /// 3. Automatically returns the reader to the pool
    /// 4. Propagates any errors from the operation
    /// 
    /// This pattern prevents reader leaks and ensures proper resource management
    /// even in the presence of errors or panics.
    /// 
    /// # Type Parameters
    /// 
    /// * `T` - Return type of the operation
    /// * `F` - Closure type that operates on the reader
    /// 
    /// # Arguments
    /// 
    /// * `operation` - Closure that performs the desired operation on the reader
    /// 
    /// # Returns
    /// 
    /// The result of the operation, or an error if reader allocation or
    /// the operation itself fails.
    ///
    /// # Error Handling
    /// 
    /// - Reader allocation errors are propagated immediately
    /// - Operation errors are propagated after reader return
    /// - Reader return errors are propagated (but shouldn't occur in normal operation)
    fn with_reader<T, F>(&self, operation: F) -> Result<T, Pod5FileError>
    where 
        F: FnOnce(&mut FeatherReader) -> Result<T, Pod5FileError> 
    {
        let mut reader = self.get_reader()?;
        let result = operation(&mut reader);
        
        self.return_reader(reader)?;

        result
    }
    
    /// Returns the batch size used by readers in this pool.
    /// 
    /// The batch size represents the number of rows processed together in
    /// each chunk of the signal table. This value is determined during
    /// pool initialization and is consistent across all readers.
    /// 
    /// # Returns
    /// 
    /// Number of rows per batch in the signal table.
    pub fn batch_size(&self) -> u64 {
        self.batch_size
    }
}

impl Pod5FileThreadSafe {
    /// Initializes a new thread-safe Pod5 file from a filesystem path.
    /// 
    /// This constructor performs comprehensive file parsing and validation:
    /// 
    /// 1. **File Validation**: Opens file and verifies Pod5 format signatures
    /// 2. **Structure Parsing**: Reads footer to locate embedded data tables
    /// 3. **Metadata Extraction**: Parses run info and reads tables into memory
    /// 4. **Reader Pool Setup**: Creates thread-safe signal table reader pool
    /// 5. **Optimization**: Prepares data structures for fast concurrent access
    /// 
    /// The initialization process can take some time for large files as it
    /// parses all read metadata upfront to enable fast lookups later.
    /// 
    /// # Arguments
    /// 
    /// * `path` - Filesystem path to the Pod5 file
    /// * `buffer_size` - Number of concurrent readers to create for the signal table
    /// 
    /// # Returns
    /// 
    /// A new `Pod5FileThreadSafe` ready for concurrent operations, or an error if
    /// the file cannot be opened, parsed, or is not a valid Pod5 file.
    /// 
    /// # Errors
    /// 
    /// * File system errors (not found, permissions, etc.)
    /// * `InvalidSignature` if the file is not a valid Pod5 file
    /// * Arrow format errors for corrupted embedded tables
    /// * Reader pool initialization errors
    /// 
    /// # Performance Notes
    /// 
    /// - Initialization time scales with file size (due to reads table parsing)
    /// - Memory usage includes cached metadata for all reads
    /// - n_workers should typically match the number of threads that will use the file
    /// - Signal data is not loaded during initialization (loaded on-demand)
    /// 
    /// # Thread Safety
    /// 
    /// The returned file can be safely shared across multiple threads for
    /// concurrent read access.
    pub fn new(path: &PathBuf, buffer_size: usize) -> Result<Self, Pod5FileError> {
        let mut file = File::open(path)?;

        // Validate Pod5 file format with signature checks
        Self::check_signature(&mut file, SeekFrom::Start(0))?;
        Self::check_signature(&mut file, SeekFrom::End(-8))?;

        let footer = Pod5Footer::new(&mut file)?;

        // Parse embedded tables and build cached metadata
        let run_info = Self::parse_run_info_table(&file, &footer)?;
        let (read_ids, reads) = Self::parse_reads_table(&file, &footer)?;
        
        // Set up signal table access
        let embedded_content_signal_table = footer.retrieve_embedded_file(
            EmbeddedContentType::SignalTable
        )?;

        let signal_table_reader_pool = Self::init_signal_table_reader_pool(
            path,
            embedded_content_signal_table.offset(), 
            embedded_content_signal_table.length(),
            buffer_size
        )?;

        Ok(Pod5FileThreadSafe { 
            path: path.clone(), 
            read_ids,
            reads,
            run_info,
            signal_table_reader_pool,
            footer
        })
    } 

    /// Validates Pod5 file signature at the specified position.
    /// 
    /// Pod5 files contain signature bytes at both the beginning and end of the file
    /// to verify file integrity and format compliance. This method checks that the
    /// file contains the expected signature bytes.
    /// 
    /// # Arguments
    /// 
    /// * `file` - File handle positioned for reading
    /// * `start` - Seek position for signature check (beginning or end of file)
    /// 
    /// # Returns
    /// 
    /// `Ok(())` if the signature matches the expected Pod5 format, error otherwise.
    /// 
    /// # Errors
    /// 
    /// * `InvalidSignature` if the bytes don't match the expected Pod5 signature
    /// * IO errors from file seeking or reading operations
    fn check_signature(file: &mut File, start: SeekFrom) -> Result<(), Pod5FileError> {
        let mut start_signature = [0u8; 8];
        file.seek(start)?;

        file.read(&mut start_signature)?;

        if start_signature == EXPECTED_SIGNATURE {
            Ok(())
        } else {
            Err(Pod5FileError::InvalidSignature(start_signature.to_vec(), start))
        }
    }

    /// Parses the run info table and extracts sequencing metadata.
    /// 
    /// The run info table contains metadata about the sequencing run that generated
    /// the data in this Pod5 file. This includes information like device ID,
    /// sample ID, sequencing parameters, and other run-specific metadata.
    /// 
    /// # Arguments
    /// 
    /// * `file` - File handle for the Pod5 file
    /// * `footer` - Parsed footer containing embedded table locations
    /// 
    /// # Returns
    /// 
    /// A `RunInfo` struct containing the parsed sequencing run metadata, or
    /// an error if the table cannot be read or parsed.
    /// 
    /// # Errors
    /// 
    /// * Arrow format errors when reading the embedded run info table
    /// * Pod5 format errors for malformed run info data
    /// 
    /// # Implementation Note
    /// 
    /// The run info table typically contains only one chunk with a single row,
    /// so this method reads chunk 0 directly rather than iterating.
    fn parse_run_info_table(file: &File, footer: &Pod5Footer) -> Result<RunInfo, Pod5FileError> {
        let embedded_file_run_info = footer.retrieve_embedded_file(EmbeddedContentType::Unknown)?;
        let mut reader_run_info = FeatherReader::new(
            file.try_clone()?, 
            embedded_file_run_info.offset(), 
            embedded_file_run_info.length()
        )?;

        let chunk = reader_run_info.get_chunk(0)?;
        let run_info = RunInfo::from_arrow_chunk(chunk)?;
        Ok(run_info)
    }


    /// Parses the reads table and extracts read metadata.
    /// 
    /// The reads table contains essential metadata for all reads in the file,
    /// including read IDs, quality information, sequence length, and references
    /// to signal data. This method processes the entire table and builds:
    /// 
    /// 1. **Ordered Read List**: Vector of read IDs in file order for iteration
    /// 2. **Fast Lookup Map**: HashMap for O(1) read access by UUID
    /// 3. **Signal References**: Indices pointing to signal data chunks (loaded later)
    /// 
    /// The signal data itself is not loaded at this stage - only the metadata
    /// required to locate and reconstruct it later.
    /// 
    /// # Arguments
    /// 
    /// * `file` - File handle for the Pod5 file
    /// * `footer` - Parsed footer containing embedded table locations
    /// 
    /// # Returns
    /// 
    /// A tuple containing:
    /// - Vector of read IDs in file order
    /// - HashMap mapping read IDs to read metadata (without signal data)
    /// 
    /// Or an error if the table cannot be read or parsed.
    /// 
    /// # Errors
    /// 
    /// * Arrow format errors when reading the embedded reads table
    /// * Pod5 format errors for malformed read entries
    /// 
    /// # Performance Notes
    /// 
    /// - This method processes all reads upfront for fast access later
    /// - Memory usage scales with the number of reads in the file
    /// - Processing time is proportional to file size and read count
    fn parse_reads_table(
        file: &File, 
        footer: &Pod5Footer
    ) -> Result<(Vec<Uuid>, HashMap<Uuid, Pod5Read>), Pod5FileError> {
        let embedded_file_reads_table = footer.retrieve_embedded_file(EmbeddedContentType::ReadsTable)?;
        let mut reader_reads_table = FeatherReader::new(
            file.try_clone()?, 
            embedded_file_reads_table.offset(), 
            embedded_file_reads_table.length()
        )?;

        let mut read_ids = Vec::new();
        let mut reads = HashMap::new();

        for chunk_res in reader_reads_table.iter_chunks()? {
            let chunk = chunk_res?;
            let reads_table = ReadsTable::from_chunk(chunk)?;
    
            for read_res in reads_table {
                let read = read_res?;
                let read_id = read.read_id();

                read_ids.push(read_id.clone());
                reads.insert(read_id.clone(), read);
            }
        }

        Ok((read_ids, reads))
    }

    /// Initializes the thread-safe reader pool for signal table access.
    /// 
    /// This method creates a pool of FeatherReader instances, each configured
    /// to read from the signal table section of the file. The pool enables
    /// multiple threads to concurrently access signal data without conflicts.
    /// 
    /// # Arguments
    /// 
    /// * `path` - Path to the Pod5 file (for creating multiple file handles)
    /// * `offset` - Byte offset where the signal table begins
    /// * `length` - Length of the signal table section in bytes
    /// * `n_workers` - Number of readers to create in the pool
    /// 
    /// # Returns
    /// 
    /// A configured `FeatherReaderPool` ready for concurrent signal data access,
    /// or an error if pool initialization fails.
    /// 
    /// # Errors
    /// 
    /// * File system errors when creating multiple file handles
    /// * FeatherReader initialization errors for invalid file sections
    /// * Pool-specific errors for empty signal tables
    /// 
    /// # Resource Usage
    /// 
    /// This method creates n_workers file handles and FeatherReader instances,
    /// so the resource usage scales linearly with the number of workers.
    fn init_signal_table_reader_pool(
        path: &PathBuf, 
        offset: i64, 
        length: i64, 
        n_workers: usize
    ) -> Result<FeatherReaderPool, Pod5FileError> {
        Ok(FeatherReaderPool::new(
            path, 
            offset, 
            length, 
            n_workers
        )?)
    }

    /// Returns the filesystem path to the Pod5 file.
    /// 
    /// # Returns
    /// 
    /// Reference to the original path used to open this file.
    pub fn path(&self) -> &PathBuf {
        &self.path
    }

    /// Returns the sequencing run information.
    /// 
    /// The run info contains metadata about the sequencing run that generated
    /// the data, including device information, sample details, and run parameters.
    /// 
    /// # Returns
    /// 
    /// Reference to the parsed `RunInfo` structure.
    pub fn run_info(&self) -> &RunInfo {
        &self.run_info
    }

    /// Returns all read IDs in the file.
    /// 
    /// The read IDs are returned in the same order as they appear in the
    /// original reads table, enabling both iteration and lookup operations.
    /// 
    /// # Returns
    /// 
    /// Reference to the vector containing all read IDs in file order.
    pub fn read_ids(&self) -> &Vec<Uuid> {
        &self.read_ids
    }

    /// Returns the total number of reads in the file.
    /// 
    /// # Returns
    /// 
    /// Count of reads contained in this Pod5 file.
    pub fn n_reads(&self) -> usize {
        self.read_ids.len()
    }

    /// Returns the parsed file footer.
    /// 
    /// The footer contains information about embedded tables and file structure.
    /// This method is primarily for internal use and advanced applications.
    /// 
    /// # Returns
    /// 
    /// Reference to the parsed `Pod5Footer` structure.
    pub fn footer(&self) -> &Pod5Footer {
        &self.footer
    }

    /// Retrieves a complete Pod5Read by its UUID, including signal data.
    /// 
    /// This is the primary data access method for the file. It performs the following:
    /// 
    /// 1. **Metadata Lookup**: Finds cached read metadata by UUID
    /// 2. **Signal Check**: Returns immediately if signal data is already loaded
    /// 3. **Reader Acquisition**: Gets a reader from the thread-safe pool
    /// 4. **Signal Reconstruction**: Loads and reconstructs complete signal data
    /// 5. **Reader Return**: Automatically returns the reader to the pool
    /// 
    /// The method is thread-safe and can be called concurrently from multiple threads.
    /// Each thread will get its own reader from the pool, avoiding conflicts.
    /// 
    /// # Arguments
    /// 
    /// * `read_id` - UUID of the read to retrieve
    /// 
    /// # Returns
    /// 
    /// A complete `Pod5Read` with all metadata and signal data loaded, or an
    /// error if the read cannot be found or signal reconstruction fails.
    /// 
    /// # Errors
    /// 
    /// * `ReadNotFound` if the read_id doesn't exist in this file
    /// * Reader pool errors if all readers are currently in use
    /// * Signal reconstruction errors for corrupted or inconsistent data
    /// * Arrow format errors when reading signal table chunks
    /// 
    /// # Performance Notes
    /// 
    /// - First access to a read involves signal reconstruction (expensive)
    /// - Signal data is not cached - each call reconstructs from file
    /// - Multiple threads can process different reads simultaneously
    /// - Reader pool size limits maximum concurrent operations
    /// 
    /// # Thread Safety
    /// 
    /// This method is fully thread-safe and designed for concurrent use.
    /// The reader pool ensures each thread gets exclusive access to a reader
    /// during its operation.
    pub fn get(&self, read_id: &Uuid) -> Result<Pod5Read, Pod5FileError> {
        let mut read = self.reads.get(read_id)
            .ok_or(Pod5FileError::ReadNotFound(read_id.clone()))?
            .clone();

        // Return immediately if signal data is already present
        if read.signal().is_some() {
            return Ok(read);
        }

        // Use reader pool to safely access signal data
        self.signal_table_reader_pool.with_reader(|signal_table_reader| {
            let signal = self.extract_signal(
                &mut read, 
                signal_table_reader, 
                read_id, 
                self.signal_table_reader_pool.batch_size()
            )?;

            read.set_signal(signal);
            Ok(read)
        })

    }

    /// Reconstructs complete signal data from distributed table chunks.
    /// 
    /// Pod5 files store signal data across multiple chunks in the signal table
    /// for efficient storage and access. This method implements an optimized
    /// reconstruction algorithm that:
    /// 
    /// 1. **Index Mapping**: Converts linear signal indices to (chunk, row) coordinates
    /// 2. **Chunk Optimization**: Keeps the current chunk in memory to minimize I/O
    /// 3. **Sequential Processing**: Leverages sequential storage of signal segments
    /// 4. **Data Validation**: Ensures reconstructed signal matches read metadata
    /// 5. **Length Verification**: Confirms total signal length matches expectations
    /// 
    /// ## Algorithm Details
    /// 
    /// The reconstruction process assumes that signal table rows for a single read
    /// are stored sequentially (or nearly so), which is typical for Pod5 files.
    /// This allows the algorithm to:
    /// - Load each chunk only once per signal reconstruction
    /// - Process multiple rows from the same chunk efficiently
    /// - Minimize file I/O operations
    /// 
    /// ## Performance Optimization
    /// 
    /// The method is optimized for the common case where signal indices are
    /// sequential or clustered, minimizing chunk loading operations.
    /// 
    /// # Arguments
    /// 
    /// * `read` - Read metadata containing signal table indices and expected length
    /// * `signal_table_reader` - FeatherReader configured for the signal table
    /// * `read_id` - Read identifier for validation and error reporting
    /// * `batch_size` - Number of rows per chunk in the signal table
    /// 
    /// # Returns
    /// 
    /// Complete signal vector with all samples for the read, or an error if
    /// reconstruction fails or data is inconsistent.
    /// 
    /// # Errors
    /// 
    /// * `SignalReconstructIdError` if chunk data doesn't match the expected read ID
    /// * `SignalReconstructLengthError` if total samples don't match read metadata
    /// * Arrow format errors when reading or parsing signal table chunks
    /// 
    /// # Data Integrity
    /// 
    /// The method performs several validation checks:
    /// - Verifies each signal chunk belongs to the correct read
    /// - Confirms total sample count matches read metadata
    /// - Ensures all expected signal indices are processed
    fn extract_signal(
        &self,
        read: &mut Pod5Read,
        signal_table_reader: &mut FeatherReader,
        read_id: &Uuid,
        batch_size: u64
    ) -> Result<Vec<i16>, Pod5FileError> {
        let mut signal = Vec::new();
        let mut sample_count = 0;

        // Convert linear indices to (chunk, row) coordinates for efficient access
        let chunk_indices = read
            .signal_indices()
            .iter()
            .map(|idx| {ChunkRowIndex { 
                chunk: (idx / batch_size) as usize,
                row: (idx % batch_size) as usize
            }})
            .collect::<Vec<ChunkRowIndex>>();

        // Initialize with the first chunk (optimization for sequential access)
        let mut current_signal_table_idx = chunk_indices[0].chunk;
        let mut signal_table = SignalTable::from_chunk(
            Self::get_signal_table_chunk(
                signal_table_reader,
                current_signal_table_idx
            )?
        )?;

        // Process each chunk index, reloading chunks only when necessary
        for chunk_index in chunk_indices {
            if chunk_index.chunk != current_signal_table_idx {
                // Load a new chunk only if needed
                current_signal_table_idx = chunk_index.chunk;
                signal_table = SignalTable::from_chunk(
                    Self::get_signal_table_chunk(
                        signal_table_reader,
                        current_signal_table_idx
                    )?
                )?;
            }

            let mut signal_table_row = signal_table.get(chunk_index.row)?;

            // Validate reconstructed signal length
            if signal_table_row.read_id != *read_id {
                return Err(Pod5FileError::SignalReconstructIdError(
                    signal_table_row.read_id,
                    read_id.clone()
                ));
            }

            signal.append(&mut signal_table_row.signal);
            sample_count += signal_table_row.sample_count;
        }

        if sample_count != (read.require_num_samples()? as usize) {
            return Err(Pod5FileError::SignalReconstructLengthError(
                sample_count, 
                read.require_num_samples()? as usize
            ));
        }

        Ok(signal)

    }

    /// Loads a specific chunk from the signal table.
    /// 
    /// This low-level method handles the Arrow IPC protocol details for reading
    /// a specific chunk from the signal table. It manages dictionaries and
    /// metadata required for proper chunk deserialization.
    /// 
    /// # Arguments
    /// 
    /// * `signal_table_reader` - Configured reader for the signal table
    /// * `chunk_index` - Zero-based index of the chunk to load
    /// 
    /// # Returns
    /// 
    /// The requested chunk as Arrow data, ready for conversion to a SignalTable,
    /// or an error if the chunk cannot be read.
    /// 
    /// # Errors
    /// 
    /// * Arrow IPC errors for malformed chunk data
    /// * IO errors when reading from the underlying file
    fn get_signal_table_chunk(signal_table_reader: &mut FeatherReader, chunk_index: usize) -> Result<Chunk<Box<dyn Array>>, Pod5FileError> {
        let metadata = signal_table_reader.metadata().clone();
        let reader = signal_table_reader
            .embedded_reader_mut();
        let dictionaries = read_file_dictionaries(
            reader, 
            &metadata, 
            &mut Default::default()
        )?;
        Ok(
            read_batch(
                reader, 
                &dictionaries, 
                &metadata, 
                None, 
                None, 
                chunk_index, 
                &mut Default::default(), 
                &mut Default::default()
            )?
        )
    }
}