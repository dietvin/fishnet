use std::{
    collections::{
        HashMap, 
        VecDeque
    },
    sync::{Condvar, Mutex}
};
use crate::{
    dataset::dataset_async::{
        buffered_feather_reader::BufferedFeatherReader, 
        file_shared_async::Pod5FileAsyncShared, 
        signal_reader_config::SignalReaderConfig
    }, 
    error::dataset::{
        Pod5DatasetError, 
        PoolSharedError
    }, 
    core::feather_reader::FeatherReader
};

/// Thread-safe pool for managing FeatherReaders across multiple Pod5 files.
/// 
/// This pool implements a sophisticated reader management strategy that balances
/// memory usage with performance. It maintains a bounded buffer of readers and
/// handles reader lifecycle including initialization, reuse, and cleanup.
/// 
/// ## Reader Management Strategy
/// 
/// 1. **Initialization**: Pool starts with readers configured for the first file
/// 2. **Allocation**: When a read is requested, compatible readers are reused if available
/// 3. **On-demand Creation**: New readers are created when no compatible reader exists
/// 4. **Return and Buffering**: Returned readers are added to the buffer (LRU eviction)
/// 5. **Concurrency Control**: Limits simultaneous readers to prevent resource exhaustion
/// 
/// ## Performance Optimization
/// 
/// The pool optimizes for the common case where reads access the same file sequentially,
/// while gracefully handling cross-file access patterns with acceptable overhead.
pub(in crate::dataset) struct FeatherReaderPoolShared {
    /// Queue of available readers, organized as least-recently-used buffer    
    reader_buffer: Mutex<VecDeque<BufferedFeatherReader>>,
    /// Cached configurations for all dataset files
    file_configs: HashMap<usize, SignalReaderConfig>,
    /// Maximum number of readers kept in the buffer
    buffer_size: usize,
    /// Condition variable for blocking when the deque is empty
    condvar: Condvar
}

impl FeatherReaderPoolShared {
    /// Creates a new reader pool for the specified dataset files.
    /// 
    /// The pool is initialized with readers for the first file in the dataset,
    /// optimizing for a sequential access pattern. Configuration metadata
    /// for all files is cached to enable fast reader initialization on demand.
    /// 
    /// # Arguments
    /// 
    /// * `files` - Vector of Pod5 files that will be accessed through this pool
    /// * `n_workers` - Number of concurrent workers (determines max simultaneous readers)
    /// * `buffer_size` - Maximum number of readers to keep in the buffer
    /// 
    /// # Returns
    /// 
    /// A new reader pool ready for concurrent access, or an error if initialization fails.
    /// 
    /// # Errors
    /// 
    /// * `BufferSizeError` if buffer_size < n_workers (insufficient buffer capacity)
    /// * File access errors when creating initial readers for the first file
    /// 
    /// # Panics
    /// 
    /// Panics if the files vector is empty, though this is validated during dataset initialization.
    pub(super) fn new(
        files: &Vec<Pod5FileAsyncShared>,
        buffer_size: usize
    ) -> Result<Self, PoolSharedError> {
        // Cache configurations for all files to enable fast reader initialization
        let mut file_configs = HashMap::with_capacity(files.len());
        for (file_id, file) in files.iter().enumerate() {
            let config = file.signal_table_feather_config().clone();
            file_configs.insert(file_id, config);
        }

        // Initialize buffer with readers for the first file (optimization for sequential access)
        let mut reader_buffer = VecDeque::with_capacity(buffer_size);
        let first_file_config = files[0].signal_table_feather_config();
        for _ in 0..buffer_size {
            reader_buffer.push_back(
                BufferedFeatherReader::from_config(first_file_config)?
            );
        }

        Ok(Self {
            reader_buffer: Mutex::new(reader_buffer),
            file_configs, 
            buffer_size,
            condvar: Condvar::new()
        }) 
    }

    /// Retrieves or creates a reader for the specified file.
    /// 
    /// This method implements the core reader allocation logic:
    /// 1. Searches the buffer for a compatible existing reader
    /// 2. If found, removes and returns the reader
    /// 3. If not found, creates a new reader from cached configuration
    /// 4. If no reader is available, waits until one is returned
    /// 
    /// # Arguments
    /// 
    /// * `file_id` - Identifier of the file for which a reader is needed
    /// 
    /// # Returns
    /// 
    /// A `BufferedFeatherReader` ready for use, or an error if allocation fails.
    /// 
    /// # Errors
    /// 
    /// * `FileNotInPool` if the file_id doesn't exist in the dataset
    /// * Reader creation errors for file system or format issues
    fn get_reader(&self, file_id: &usize) -> Result<BufferedFeatherReader, PoolSharedError> {
        let config = self.file_configs.get(file_id)
            .ok_or(PoolSharedError::FileNotInPool(file_id.clone()))?;

        let mut reader_buffer = self.reader_buffer.lock().unwrap();

        loop {
            if let Some(pos) = reader_buffer.iter()
                .position(|r| r.is_compatible(*file_id)) 
            {
                // Using unwrap since it can't be None (it was checked in the line directly above!)
                let reader = reader_buffer.remove(pos).unwrap();
                return Ok(reader);                
            }

            if reader_buffer.len() < self.buffer_size {
                let reader = BufferedFeatherReader::from_config(config)?;
                return Ok(reader);
            }   

            // Wait until a new reader is available to check
            reader_buffer = self.condvar.wait(reader_buffer).unwrap();
        }
    }

    /// Returns a reader to the pool for potential reuse.
    /// 
    /// Implements a least-recently-used (LRU) eviction policy: returned readers
    /// are added to the back of the buffer, and if the buffer is full, the
    /// oldest reader (front of queue) is dropped to make space.
    /// 
    /// # Arguments
    /// 
    /// * `reader` - The reader to return to the pool
    fn return_reader(&self, reader: BufferedFeatherReader) {
        let mut reader_buffer = self.reader_buffer.lock().unwrap();

        // LRU eviction: remove oldest reader if buffer is full
        if reader_buffer.len() >= self.buffer_size {
            reader_buffer.pop_front();
        }

        reader_buffer.push_back(reader);
        // Notify that a new reader was added in case another thread is waiting
        self.condvar.notify_one();
    }

    /// Executes an operation with a reader for the specified file.
    /// 
    /// This is the primary interface for pool operations, implementing the
    /// Resource Acquisition Is Initialization (RAII) pattern. It handles:
    /// - Reader acquisition from the pool
    /// - Operation execution with the reader
    /// - Automatic reader return to the pool
    /// - Error propagation and cleanup
    /// 
    /// # Type Parameters
    /// 
    /// * `T` - Return type of the operation
    /// * `F` - Closure type that operates on the reader
    /// 
    /// # Arguments
    /// 
    /// * `file_id` - Identifier of the file to access
    /// * `operation` - Closure that performs the desired operation on the reader
    /// 
    /// # Returns
    /// 
    /// The result of the operation, or an error if reader acquisition fails.
    pub(super) fn with_reader<T, F>(&self, file_id: &usize, operation: F) -> Result<T, Pod5DatasetError> 
    where
        F: FnOnce(&mut FeatherReader) -> Result<T, Pod5DatasetError>
    {
        let mut reader = self.get_reader(file_id)?;
        let result = operation(&mut reader.reader);
        self.return_reader(reader);
        result
    }
}
