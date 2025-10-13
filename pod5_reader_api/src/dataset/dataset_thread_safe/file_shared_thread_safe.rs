use std::{
    collections::HashMap, 
    fs::File, 
    io::{
        Read, 
        Seek, 
        SeekFrom
    }, 
    path::PathBuf
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
    dataset::dataset_thread_safe::signal_reader_config::SignalReaderConfig, 
    error::file::Pod5FileError, 
    file::{
        Pod5FileThreadSafe, 
        Pod5File, 
        ChunkRowIndex, 
        EXPECTED_SIGNATURE
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
            signal_table::SignalTable
        }
    }
};


/// Optimized Pod5 file representation for dataset-level operations.
/// 
/// This is a streamlined version of `Pod5FileThreadSafe` designed specifically for use within
/// `Pod5DatasetThreadSafe`. Unlike the full file implementation, this version:
/// - Doesn't maintain its own reader pool (delegates to dataset-level pool)
/// - Focuses on metadata caching and read access
/// - Optimizes memory usage by avoiding redundant reader storage
/// - Provides conversion methods to full file objects when needed
/// 
/// ## Design Trade-offs
/// 
/// * **Memory Efficiency**: Minimal per-file overhead in large datasets
/// * **Access Speed**: Fast metadata lookups with cached read information
/// * **Flexibility**: Can convert to full file objects for comprehensive operations
pub(in crate::dataset) struct Pod5FileThreadSafeShared {
    /// Path to the Pod5 file on the filesystem
    path: PathBuf,
    /// Ordered list of all read IDs in this file
    read_ids: Vec<Uuid>,
    /// Fast lookup map from read ID to read metadata
    reads: HashMap<Uuid, Pod5Read>,
    /// Configuration for signal table reader initialization
    signal_reader_config: SignalReaderConfig
}

impl Pod5FileThreadSafeShared {
    /// Initializes a new shared Pod5 file from a filesystem path.
    /// 
    /// This constructor performs the minimal parsing necessary for dataset operations:
    /// 1. Validates file signature and format
    /// 2. Parses and caches the reads table for fast metadata access
    /// 3. Extracts signal table configuration for reader pool use
    /// 4. Does not load signal data (loaded on-demand)
    /// 
    /// # Arguments
    /// 
    /// * `file_id` - Unique identifier for this file within the dataset
    /// * `path` - Filesystem path to the Pod5 file
    /// 
    /// # Returns
    /// 
    /// A new shared file instance ready for dataset integration, or an error
    /// if the file cannot be read or is not a valid Pod5 file.
    /// 
    /// # Errors
    /// 
    /// * File system errors (file not found, permissions, etc.)
    /// * Pod5 format errors (invalid signature, corrupted metadata)
    /// * Arrow format errors in embedded tables
    pub(super) fn new(file_id: usize, path: &PathBuf) -> Result<Self, Pod5FileError> {
        let mut file = File::open(path)?;

        // Validate Pod5 file format
        Self::check_signature(&mut file, SeekFrom::Start(0))?;
        Self::check_signature(&mut file, SeekFrom::End(-8))?;

        let footer = Pod5Footer::new(&mut file)?;

        // Parse reads table and extract metadata for fast access
        let (read_ids, reads) = Self::parse_reads_table(&file, &footer)?;
        
        // Configure signal table access for the reader pool
        let signal_reader_config = Self::parse_signal_table(
            &file, 
            &footer, 
            file_id,
            path
        )?;

        Ok(Self { 
            path: path.to_path_buf(),
            read_ids,
            reads,
            signal_reader_config
        })
    }

    /// Validates Pod5 file signature at the specified position.
    /// 
    /// Pod5 files contain signature bytes at both the beginning and end of the file
    /// to verify file integrity and format compliance.
    /// 
    /// # Arguments
    /// 
    /// * `file` - File handle positioned for reading
    /// * `start` - Seek position for signature check (start or end of file)
    /// 
    /// # Returns
    /// 
    /// `Ok(())` if signature is valid, error otherwise.
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

    /// Parses the reads table and extracts read metadata.
    /// 
    /// This method processes the embedded reads table to create:
    /// 1. An ordered vector of read IDs for iteration
    /// 2. A hash map for O(1) read metadata lookup
    /// 
    /// Note: Unlike the full file implementation, this doesn't parse the signal table
    /// index since dataset-level access uses a different signal loading strategy.
    /// 
    /// # Arguments
    /// 
    /// * `file` - File handle for the Pod5 file
    /// * `footer` - Parsed footer containing embedded table locations
    /// 
    /// # Returns
    /// 
    /// A tuple of (read_ids_vector, reads_hashmap) for efficient access patterns,
    /// or an error if table parsing fails.
    /// 
    /// # Errors
    /// 
    /// * Arrow format errors when reading the embedded reads table
    /// * Pod5 format errors for malformed read entries
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

    /// Extracts signal table configuration for reader pool initialization.
    /// 
    /// This method analyzes the signal table structure to determine the optimal
    /// configuration for FeatherReader instances. It reads just enough of the table
    /// to determine batch sizing, which is critical for efficient chunk access.
    /// 
    /// # Arguments
    /// 
    /// * `file` - File handle for the Pod5 file
    /// * `footer` - Parsed footer with embedded table information
    /// * `file_id` - Dataset-level identifier for this file
    /// * `path` - Filesystem path for reader initialization
    /// 
    /// # Returns
    /// 
    /// A `SignalReaderConfig` ready for use by the reader pool, or an error
    /// if signal table analysis fails.
    /// 
    /// # Errors
    /// 
    /// * `SignalTableChunkSizeError` if the signal table is empty or malformed
    /// * Arrow format errors when reading signal table metadata
    fn parse_signal_table(
        file: &File,
        footer: &Pod5Footer,
        file_id: usize,
        path: &PathBuf
    ) -> Result<SignalReaderConfig, Pod5FileError> {
        let embedded_content_signal_table = footer.retrieve_embedded_file(
            EmbeddedContentType::SignalTable
        )?;

        let mut reader = FeatherReader::new(
            file.try_clone()?, 
            embedded_content_signal_table.offset(),
            embedded_content_signal_table.length()
        )?;

        // Determine batch size from first chunk
        let batch_size = reader
            .iter_chunks()?
            .next()
            .ok_or(Pod5FileError::SignalTableChunkSizeError)??
            .len() as u64;

        Ok(SignalReaderConfig::new(
            file_id,
            path,
            embedded_content_signal_table.offset(),
            embedded_content_signal_table.length(),
            batch_size
        ))
    }

    /// Returns a reference to all read IDs in this file.
    /// 
    /// The read IDs are ordered as they appear in the original reads table,
    /// enabling both iteration and lookup operations.
    pub(super) fn read_ids(&self) -> &Vec<Uuid> {
        &self.read_ids
    }

    /// Returns the signal table reader configuration for this file.
    /// 
    /// Used by the reader pool to initialize FeatherReaders with the correct
    /// file offset, length, and batch size parameters.
    pub(super) fn signal_table_feather_config(&self) -> &SignalReaderConfig {
        &self.signal_reader_config
    }

    /// Retrieves a complete Pod5Read with signal data loaded.
    /// 
    /// This method implements the core read access logic for the dataset:
    /// 1. Looks up cached read metadata
    /// 2. If signal data is already loaded, returns immediately
    /// 3. Otherwise, uses the provided reader to load signal chunks
    /// 4. Reconstructs the complete signal from distributed chunks
    /// 5. Validates signal integrity and length
    /// 
    /// # Arguments
    /// 
    /// * `read_id` - UUID identifying the read to retrieve
    /// * `reader` - FeatherReader configured for this file's signal table
    /// 
    /// # Returns
    /// 
    /// A complete `Pod5Read` with signal data loaded, or an error if the read
    /// cannot be found or signal reconstruction fails.
    /// 
    /// # Errors
    /// 
    /// * `ReadNotFound` if the read_id doesn't exist in this file
    /// * Signal reconstruction errors for corrupted or inconsistent data
    /// * Arrow format errors when reading signal table chunks
    pub(super) fn get(&self, read_id: &Uuid, reader: &mut FeatherReader) -> Result<Pod5Read, Pod5FileError> {
        let mut read = self.reads.get(read_id)
            .ok_or(Pod5FileError::ReadNotFound(read_id.clone()))?
            .clone();

        // Return immediately if signal data is already loaded
        if read.signal().is_some() {
            return Ok(read);
        }

        // Load and reconstruct signal data from chunks
        let signal = Self::extract_signal(
            &mut read, 
            reader, 
            read_id, 
            self.signal_reader_config.batch_size
        )?;

        read.set_signal(signal);
        Ok(read)
    }

    /// Reconstructs complete signal data from distributed table chunks.
    /// 
    /// Pod5 files store signal data across multiple chunks for efficient storage.
    /// This method implements an optimized reconstruction algorithm:
    /// 
    /// 1. **Index Mapping**: Converts linear signal indices to (chunk, row) coordinates
    /// 2. **Chunk Caching**: Keeps the current chunk in memory to minimize I/O
    /// 3. **Sequential Access**: Leverages the fact that signal indices are typically sequential
    /// 4. **Validation**: Ensures reconstructed signal matches expected read properties
    /// 
    /// ## Performance Optimization
    /// 
    /// The algorithm is optimized for the common case where signal table rows for a
    /// single read are stored sequentially, minimizing chunk loading operations.
    /// 
    /// # Arguments
    /// 
    /// * `read` - Read metadata containing signal table indices
    /// * `signal_table_reader` - Configured reader for the signal table
    /// * `read_id` - Read identifier for validation
    /// * `batch_size` - Number of rows per chunk in the signal table
    /// 
    /// # Returns
    /// 
    /// Complete signal vector reconstructed from all chunks, or an error if
    /// reconstruction fails or data is inconsistent.
    /// 
    /// # Errors
    /// 
    /// * `SignalReconstructIdError` if chunk data doesn't match the expected read
    /// * `SignalReconstructLengthError` if total samples don't match read metadata
    /// * Arrow errors when reading or parsing signal table chunks
    fn extract_signal(
        read: &mut Pod5Read,
        signal_table_reader: &mut FeatherReader,
        read_id: &Uuid,
        batch_size: u64
    ) -> Result<Vec<i16>, Pod5FileError> {
        let mut signal = Vec::new();
        let mut sample_count = 0;

        // Convert linear indices to (chunk, row) coordinates
        let chunk_indices = read
            .signal_indices()
            .iter()
            .map(|idx| {ChunkRowIndex { 
                chunk: (idx / batch_size) as usize,
                row: (idx % batch_size) as usize
            }})
            .collect::<Vec<ChunkRowIndex>>();

        // Initialize with first chunk (optimization for sequential access)
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
                current_signal_table_idx = chunk_index.chunk;
                signal_table = SignalTable::from_chunk(
                    Self::get_signal_table_chunk(
                        signal_table_reader,
                        current_signal_table_idx
                    )?
                )?;
            }

            let mut signal_table_row = signal_table.get(chunk_index.row)?;

            // Validate chunk data consistency
            if signal_table_row.read_id != *read_id {
                return Err(Pod5FileError::SignalReconstructIdError(
                    signal_table_row.read_id,
                    read_id.clone()
                ));
            }

            signal.append(&mut signal_table_row.signal);
            sample_count += signal_table_row.sample_count;
        }

        // Validate reconstructed signal length
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

    /// Creates a standard Pod5File from this shared representation.
    /// 
    /// This conversion method enables access to full Pod5File functionality when needed,
    /// such as for operations not supported by the dataset interface. The conversion
    /// involves re-parsing the file, so it has some performance overhead.
    /// 
    /// # Returns
    /// 
    /// A new `Pod5File` instance for comprehensive file operations, or an error
    /// if the file cannot be re-opened or parsed.
    /// 
    /// # Performance Note
    /// 
    /// This method re-initializes the file from scratch, which can be expensive
    /// for large files. Use sparingly and prefer dataset-level operations when possible.
    pub(super) fn to_pod5_file(&self) -> Result<Pod5File, Pod5FileError> {
        Pod5File::new(&self.path)
    }

    /// Creates an thread-safe Pod5File from this shared representation.
    /// 
    /// Similar to `to_pod5_file()`, but creates a thread-safe Pod5FileThreadSafe instance
    /// with its own reader pool. This is useful when you need full thread-safe file
    /// functionality alongside dataset operations.
    /// 
    /// # Arguments
    /// 
    /// * `n_workers` - Number of worker threads for the thread-safe file's reader pool
    /// 
    /// # Returns
    /// 
    /// A new `Pod5FileThreadSafe` instance ready for concurrent operations, or an error
    /// if initialization fails.
    /// 
    /// # Performance Note
    /// 
    /// Like `to_pod5_file()`, this method has initialization overhead and should
    /// be used judiciously.
    pub(super) fn to_pod5_file_thread_safe(&self, n_workers: usize) -> Result<Pod5FileThreadSafe, Pod5FileError> {
        Pod5FileThreadSafe::new(&self.path, n_workers)
    }

}
