use std::collections::HashMap;

use uuid::Uuid;

use crate::{
    error::file::ReadIteratorError, 
    file::{
        Pod5File, 
        signal_table_index::SignalTableIndex
    }, 
    read::Pod5Read, 
    core::{
        feather_reader::ChunkIterator, 
        tables::signal_table::{
            SignalTable, 
            SignalTableRow
        }
    }
};

/// Iterator for efficiently reconstructing complete reads from Pod5 signal table data.
/// 
/// The `ReadIterator` handles the task of reading signal data from a Pod5 file's
/// signal table and reconstructing complete reads with their full signal data. It manages
/// out-of-order signal chunks by using a `SignalTableIndex` to track which chunks belong
/// to which reads and when reads are complete.
/// 
/// # Key Features
/// 
/// - **Handles out-of-order chunks**: Signal chunks for a single read may be scattered
///   across the signal table, and this iterator correctly reassembles them
/// - **Memory efficient**: Only keeps incomplete read data in memory, immediately yielding
///   completed reads
/// - **Error resilient**: Comprehensive error handling for data integrity issues
/// - **Streaming**: Processes the signal table in chunks without loading everything into memory
/// 
/// # Architecture
/// 
/// The iterator works by:
/// 1. Loading signal table data in chunks using `ChunkIterator`
/// 2. For each row, using `SignalTableIndex` to determine which read it belongs to
/// 3. Accumulating signal chunks for each read until complete
/// 4. Yielding completed reads with reconstructed signal data
/// 
/// # Memory Usage
/// 
/// The iterator maintains:
/// - Current signal table chunk (one chunk at a time)
/// - Partial signal data for incomplete reads
/// - Metadata structures for tracking progress
/// 
/// Total memory usage scales with the number of concurrent incomplete reads, not
/// the total file size.
pub struct ReadIterator<'a> {
    /// Index of all reads in the Pod5 file, keyed by UUID.
    /// Used to lookup read metadata when completing reads.
    read_index: HashMap<Uuid, Pod5Read>,

    /// Helper structure that maps signal table rows to reads and tracks completion.
    /// Handles the complex logic of determining which signal chunks belong to which reads.
    signal_table_index: SignalTableIndex,

    /// Iterator over signal table chunks from the Pod5 file.
    /// Provides streaming access to signal table data without loading everything into memory.
    chunk_iterator: ChunkIterator<'a>,

    /// Currently loaded signal table chunk, if any.
    /// Only one chunk is kept in memory at a time for efficiency.
    current_signal_table: Option<SignalTable>,

    /// Number of rows in the current signal table chunk.
    current_signal_table_len: usize,

    /// Current row index within the current signal table chunk.
    current_row_idx: usize,

    /// Storage for signal chunks of reads that are not yet complete.
    /// 
    /// Maps read UUID to a vector where index corresponds to chunk position
    /// in the final signal. `None` entries represent chunks not yet collected.
    /// Once all chunks are collected, the complete signal is reconstructed
    /// and the read is yielded.
    incomplete_reads: HashMap<Uuid, Vec<Option<Vec<i16>>>>,

    /// Global row index across all processed signal table chunks.
    /// Used to coordinate with `SignalTableIndex` which expects absolute row indices.
    global_row_idx: usize,

    /// Whether the iterator has finished processing all chunks.
    finished: bool,
}


impl<'a> ReadIterator<'a> {
    /// Creates a new iterator for the specified Pod5 file.
    /// 
    /// This constructor initializes all the tracking structures needed for efficient
    /// signal reconstruction. It obtains the signal table index from the Pod5 file
    /// and sets up the chunk iterator for streaming access to signal data.
    /// 
    /// # Arguments
    /// 
    /// * `pod5_file` - Mutable reference to the Pod5 file to iterate over
    /// 
    /// # Returns
    /// 
    /// A new `ReadIterator` ready to process signal data, or an error if initialization fails.
    /// 
    /// # Errors
    /// 
    /// Returns `ReadIteratorError` if:
    /// - The signal table chunk iterator cannot be created
    /// - Required metadata is missing from the Pod5 file
    pub fn new(pod5_file: &'a mut Pod5File) -> Result<Self, ReadIteratorError> {
        let read_index = pod5_file.reads().clone();
        let signal_table_index = pod5_file.signal_table_index().clone();
        let chunk_iterator = pod5_file.signal_table_reader_mut().iter_chunks()?;

        Ok(ReadIterator {
            read_index,
            signal_table_index,
            chunk_iterator,
            current_signal_table: None,
            current_signal_table_len: 0,
            current_row_idx: 0,

            incomplete_reads: HashMap::new(),
            global_row_idx: 0,
            
            finished: false
        })
    }

    /// Loads the next signal table chunk from the Pod5 file.
    /// 
    /// This method advances the chunk iterator and loads the next batch of signal
    /// table rows. It's called automatically when the current chunk is exhausted.
    /// Only one chunk is kept in memory at a time for efficiency.
    /// 
    /// # Returns
    /// 
    /// - `Ok(true)` if a new chunk was successfully loaded
    /// - `Ok(false)` if no more chunks are available (end of file)
    /// - `Err(ReadIteratorError)` if an error occurred while loading
    /// 
    /// # Side Effects
    /// 
    /// - Updates `current_signal_table` with the new chunk
    /// - Resets `current_row_idx` to 0
    /// - Updates `current_signal_table_len`
    /// - Sets `finished` to true if no more chunks are available
    fn load_next_chunk(&mut self) -> Result<bool, ReadIteratorError> {
        match self.chunk_iterator.next() {
            Some(chunk_res) => {
                let chunk = chunk_res?;
                let signal_table = SignalTable::from_chunk(chunk)?;
                self.current_signal_table_len = signal_table.len();
                self.current_signal_table = Some(signal_table);
                self.current_row_idx = 0;
                Ok(true)
            }
            None => {
                self.finished = true;
                Ok(false)
            }
        }
    }

    /// Retrieves the current signal table row being processed.
    /// 
    /// This is a helper method that extracts the signal table row at the current
    /// position within the current chunk. The row contains the signal data and
    /// read ID for processing.
    /// 
    /// # Returns
    /// 
    /// The `SignalTableRow` at the current position, or an error if:
    /// - No signal table chunk is currently loaded
    /// - The current row index is out of bounds
    /// 
    /// # Errors
    /// 
    /// - `ReadIteratorError::SignalTableNone` if no chunk is loaded
    /// - `ReadIteratorError::SignalTableError` for other signal table issues
    fn get_current_row(&self) -> Result<SignalTableRow, ReadIteratorError> {
        self.current_signal_table
            .as_ref()
            .ok_or(ReadIteratorError::SignalTableNone)?
            .get(self.current_row_idx)
            .map_err(ReadIteratorError::SignalTableError)
    }

    /// Creates a completed Pod5Read with reconstructed signal data.
    /// 
    /// This method takes a read ID and complete signal vector and creates a finalized
    /// `Pod5Read` object. It validates that the signal length matches the expected
    /// number of samples and sets the signal data on a clone of the original read.
    /// 
    /// # Arguments
    /// 
    /// * `read_id` - UUID of the read to finalize
    /// * `signal` - Complete reconstructed signal data
    /// 
    /// # Returns
    /// 
    /// A complete `Pod5Read` with signal data attached, ready for use.
    /// 
    /// # Errors
    /// 
    /// Returns `ReadIteratorError` if:
    /// - The read ID is not found in the read index
    /// - The signal length doesn't match the expected number of samples
    /// - The read metadata is missing required fields
    /// 
    /// # Validation
    /// 
    /// The method performs crucial validation to ensure data integrity:
    /// - Verifies signal length matches metadata expectations
    /// - Confirms read exists in the original read index
    fn finalize_current_read(&self, read_id: &Uuid, signal: Vec<i16>) -> Result<Pod5Read, ReadIteratorError> {

        let read = self.read_index
            .get(read_id)
            .ok_or(ReadIteratorError::ReadNotFoundInIndex(*read_id))?;

        let mut read = read.clone();

        let expected_len = read
            .require_num_samples()
            .map_err(|_| ReadIteratorError::ExpectedSignalLenNotFound)?
            as usize;

        if signal.len() != expected_len {
            return Err(ReadIteratorError::DiscordantSignalLength(
                signal.len(), 
                expected_len
            ));
        }

        read.set_signal(signal);
        Ok(read)
    }

    /// Reconstructs a complete signal from collected chunks.
    /// 
    /// This method takes the collected signal chunks for a read and concatenates
    /// them in the correct order to form the complete signal. All chunks must
    /// be present (no `None` values) for successful reconstruction.
    /// 
    /// # Arguments
    /// 
    /// * `read_id` - UUID of the read (used for error reporting)
    /// * `signal_chunks` - Vector of signal chunks in order (should all be `Some`)
    /// 
    /// # Returns
    /// 
    /// A complete signal vector formed by concatenating all chunks in order.
    /// 
    /// # Errors
    /// 
    /// Returns `ReadIteratorError::ConstructingIncompleteSignal` if any chunk
    /// is missing (`None`), indicating incomplete data collection.
    /// 
    /// # Performance
    /// 
    /// The method pre-allocates space when possible and uses efficient slice
    /// operations to minimize memory allocations during concatenation.
    fn construct_complete_signal(read_id: Uuid, signal_chunks: Vec<Option<Vec<i16>>>) -> Result<Vec<i16>, ReadIteratorError> {
        let mut complete_signal = Vec::new();

        for (i, chunk_opt) in signal_chunks.iter().enumerate() {
            match chunk_opt {
                Some(chunk) => complete_signal.extend_from_slice(chunk),
                None => return Err(ReadIteratorError::ConstructingIncompleteSignal(read_id, i))
            }
        }

        Ok(complete_signal)
    }

    /// Processes the current signal table row and updates read tracking.
    /// 
    /// This is the core processing method that:
    /// 1. Retrieves the current signal table row
    /// 2. Consults the signal table index to determine read ownership
    /// 3. Stores the signal chunk in the appropriate position
    /// 4. Checks if the read is now complete and finalizes if so
    /// 
    /// The method handles both new reads (first chunk encountered) and
    /// continuation of existing reads (additional chunks).
    /// 
    /// # Returns
    /// 
    /// - `Some(Ok(Pod5Read))` if a read was completed with this row
    /// - `Some(Err(...))` if an error occurred during processing  
    /// - `None` if the row was processed but no read was completed
    /// 
    /// # State Updates
    /// 
    /// - Increments global and local row indices
    /// - Updates or creates entries in `incomplete_reads`
    /// - May remove completed reads from `incomplete_reads`
    /// 
    /// # Error Handling
    /// 
    /// Comprehensive error checking for:
    /// - Invalid row data or indices
    /// - Mismatched read IDs between row and index
    /// - Out-of-bounds chunk positions
    /// - Missing tracking entries (should not occur)
    fn process_current_row(&mut self) -> Option<Result<Pod5Read, ReadIteratorError>> {
        let row = match self.get_current_row() {
            Ok(row) => row,
            Err(e) => return Some(Err(e))
        };

        // Get the information about the current row from the signal table index
        let (index_read_id, index_chunk_pos, n_remaining) = match self.signal_table_index.process_row(self.global_row_idx) {
            Ok((r_id, chunk_pos, is_finished)) => (r_id, chunk_pos, is_finished),
            Err(e) => return Some(Err(ReadIteratorError::SignalTableIndexError(e)))
        };

        self.global_row_idx += 1;
        self.current_row_idx += 1;

        // Validate the read id between row and index
        if row.read_id != index_read_id {
            return Some(Err(ReadIteratorError::ReadIdMismatch(row.read_id, index_read_id)));
        }

        if !self.incomplete_reads.contains_key(&index_read_id) {
            // Create new entry and add the signal chunk at the given index

            // This is the first time a row for the current read id was accessed.
            // This in turn means that the number of remaining chunks must have
            // been decreased only once. As such the number of total chunks is 
            // n_remaining + 1
            let n_chunks_total = n_remaining + 1;
            self.incomplete_reads.insert(index_read_id, vec![None; n_chunks_total]);
        }

        // Add the signal chunk at the given index
        if let Some(val) = self.incomplete_reads.get_mut(&index_read_id) {
            if index_chunk_pos < val.len() {
                val[index_chunk_pos] = Some(row.signal)
            } else {
                return Some(Err(ReadIteratorError::InvalidSignalChunkIndex(index_chunk_pos, val.len())));
            }
        } else {
            // Unreachable since even if the entry did not exist before, it got created directly above
            return Some(Err(ReadIteratorError::IncompleteReadsEntryNotFound(index_read_id)));
        }
        
        if n_remaining == 0 {
            // All signal chunks are collected, read is ready to be finished
            if let Some(signal_chunks) = self.incomplete_reads.remove(&index_read_id) {
                let complete_signal = match Self::construct_complete_signal(index_read_id, signal_chunks) {
                    Ok(sig) => sig,
                    Err(e) => return Some(Err(e))
                };
                let completed_read_res = self.finalize_current_read(&index_read_id, complete_signal);
                Some(completed_read_res)
            } else {
                // Unreachable since even if the entry did not exist before, it got created directly above
                return Some(Err(ReadIteratorError::IncompleteReadsEntryNotFound(index_read_id)));
            }
        } else {
            // There are signal chunks to come in later iterations, continue
            None
        }
    }
}


impl<'a> Iterator for ReadIterator<'a> {
    type Item = Result<Pod5Read, ReadIteratorError>;

    /// Advances the iterator and returns the next completed read.
    /// 
    /// This method implements the core iteration logic:
    /// 1. Loads signal table chunks as needed
    /// 2. Processes rows within each chunk
    /// 3. Returns completed reads as they become available
    /// 4. Handles end-of-file cleanup and validation
    /// 
    /// The iterator processes signal table data in a streaming fashion,
    /// only keeping one chunk in memory at a time while accumulating
    /// signal data for incomplete reads.
    /// 
    /// # Returns
    /// 
    /// - `Some(Ok(Pod5Read))` - A completed read with full signal data
    /// - `Some(Err(ReadIteratorError))` - An error occurred during processing
    /// - `None` - No more reads available (end of iterator)
    /// 
    /// # End-of-File Handling
    /// 
    /// When all chunks have been processed, the iterator:
    /// 1. Validates that all reads were properly completed
    /// 2. Checks that no incomplete reads remain
    /// 3. Returns appropriate errors for data integrity issues
    /// 
    /// # Error Recovery
    /// 
    /// Most errors are fatal and will terminate iteration. However,
    /// some validation errors provide detailed information about
    /// what went wrong for debugging purposes.
    fn next(&mut self) -> Option<Self::Item> {
        if self.finished {
            return None;
        }

        loop {
            // Load first chunk if needed
            if self.current_signal_table.is_none() {
                match self.load_next_chunk() {
                    Ok(true) => {} // Successfully loaded chunk
                    Ok(false) => return None, // No more chunks
                    Err(e) => return Some(Err(e))
                }
            }

            // Process rows in current chunk
            if self.current_row_idx < self.current_signal_table_len {
                if let Some(result) = self.process_current_row() {
                    return Some(result);
                }
            } else {
                // End of current chunk - try to load next chunk
                match self.load_next_chunk() {
                    Ok(true) => continue, // Successfully loaded next chunk
                    Ok(false) => {
                        // No more chunks - finalize the last read if there is one
                        if let Err(e) = self.signal_table_index.properly_finished() {
                            return Some(Err(ReadIteratorError::SignalTableIndexError(e)));
                        }
                        if !self.incomplete_reads.is_empty() {
                            return Some(Err(ReadIteratorError::IncompleteReadsAfterFinish(
                                self.incomplete_reads.len()
                            )));
                        }
                        return None;
                    }
                    Err(e) => return Some(Err(e))
                }
            }
        }
    }
}