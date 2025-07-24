use std::collections::HashMap;

use uuid::Uuid;

use crate::{feather_reader::{ChunkIterator, FeatherReaderError}, file::Pod5File, read::{Pod5Read, Pod5ReadError}, tables::signal_table::{SignalTable, SignalTableError, SignalTableRow}};

/// Iterator for efficiently traversing reads in a pod5 file.
/// 
/// Handles signal data reconstruction by processing signal table chunks.
pub struct ReadIterator<'a> {
    read_index: HashMap<Uuid, Pod5Read>,
    chunk_iterator: ChunkIterator<'a>,
    current_signal_table: Option<SignalTable>,
    current_signal_table_len: usize,
    current_row_idx: usize,
    current_read_id: Option<Uuid>,
    current_signal: Vec<i16>,
    finished: bool,
}


impl<'a> ReadIterator<'a> {
    /// Creates a new ReadIterator for the given pod5 file.
    /// 
    /// # Arguments
    /// * `pod5_file` - Reference to the Pod5File to iterate over
    /// 
    /// # Returns
    /// Result containing the initialized iterator or an error
    pub fn new(pod5_file: &'a mut Pod5File) -> Result<Self, ReadIteratorError> {
        let read_index = pod5_file.reads().clone();
        let chunk_iterator = pod5_file.signal_table_reader_mut().iter_chunks()?;

        Ok(ReadIterator {
            read_index,
            chunk_iterator,
            current_signal_table: None,
            current_signal_table_len: 0,
            current_row_idx: 0,
            current_read_id: None,
            current_signal: Vec::new(),
            finished: false
        })
    }

    /// Loads the next signal table chunk.
    /// 
    /// # Returns
    /// Result indicating whether a new chunk was loaded
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

    /// Gets the current row from the signal table.
    /// 
    /// # Returns
    /// Result containing the SignalTableRow or an error
    fn get_current_row(&self) -> Result<SignalTableRow, ReadIteratorError> {
        self.current_signal_table
            .as_ref()
            .ok_or(ReadIteratorError::SignalTableNone)?
            .get(self.current_row_idx)
            .map_err(ReadIteratorError::SignalTableError)
    }

    /// Completes the current read and starts a new one.
    /// 
    /// # Arguments
    /// * `new_row` - First row of the new read
    /// 
    /// # Returns
    /// Result containing the completed read or an error
    fn complete_current_read_and_start_new(
        &mut self,
        new_row: SignalTableRow
    ) -> Result<Pod5Read, ReadIteratorError> {
        // Complete the current read
        let completed_read = self.finalize_current_read()?;
        
        // Start the new read
        self.start_new_read(new_row);
        
        Ok(completed_read)
    }

    /// Finalizes the current read with its complete signal data.
    /// 
    /// # Returns
    /// Result containing the completed Pod5Read or an error
    fn finalize_current_read(&mut self) -> Result<Pod5Read, ReadIteratorError> {
        let current_read_id = self.current_read_id
            .ok_or_else(|| ReadIteratorError::ExpectedSignalLenNotFound)?;

        let read = self.read_index
            .get(&current_read_id)
            .ok_or(ReadIteratorError::ReadNotFoundInIndex(current_read_id))?;

        let mut read = read.clone();
        let expected_len = read
            .require_num_samples()
            .map_err(|_| ReadIteratorError::ExpectedSignalLenNotFound)?
            as usize;

        if self.current_signal.len() != expected_len {
            return Err(ReadIteratorError::DiscordantSignalLength(
                self.current_signal.len(), 
                expected_len
            ));
        }

        read.set_signal(self.current_signal.clone());
        Ok(read)
    }


    /// Starts a new read with the given row data.
    /// 
    /// # Arguments
    /// * `row` - First signal row of the new read
    fn start_new_read(&mut self, row: SignalTableRow) {
        self.current_read_id = Some(row.read_id);
        self.current_signal = row.signal;
    }


    /// Processes the current row, handling read transitions.
    /// 
    /// # Returns
    /// Option containing either a completed read or None if still processing current read
    fn process_current_row(&mut self) -> Option<Result<Pod5Read, ReadIteratorError>> {
        let row = match self.get_current_row() {
            Ok(row) => row,
            Err(e) => return Some(Err(e))
        };

        match self.current_read_id {
            // First row ever - start the first read
            None => {
                self.start_new_read(row);
                self.current_row_idx += 1;
                None
            }
            // Continuing the same read - append signal data
            Some(current_id) if row.read_id == current_id => {
                self.current_signal.extend_from_slice(&row.signal);
                self.current_row_idx += 1;
                None
            }
            // Different read - complete current read and start new one
            Some(_) => {
                let result = self.complete_current_read_and_start_new(row);
                self.current_row_idx += 1;
                Some(result)
            }
        }
    }
}


impl<'a> Iterator for ReadIterator<'a> {
    type Item = Result<Pod5Read, ReadIteratorError>;

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
                        if self.current_read_id.is_some() {
                            let result = self.finalize_current_read();
                            self.current_read_id = None; // Ensure it's not returned again
                            return Some(result);
                        }
                        return None;
                    }
                    Err(e) => return Some(Err(e))
                }
            }
        }
    }
}

/// Enum representing all possible errors that can occur during read iteration.
#[derive(Debug, thiserror::Error)]
pub enum ReadIteratorError {
    #[error("Could not get chunk iterator: {0}")]
    ChunkIteratorError(#[from] FeatherReaderError),
    #[error("Arrow2 error: {0}")]
    Arrow2Error(#[from] arrow2::error::Error),
    #[error("No chunks found in iterator")]
    EmptyIterator,
    #[error("Signal table error: {0}")]
    SignalTableError(#[from] SignalTableError),
    #[error("Read '{0}' was not found in the read index")]
    ReadNotFoundInIndex(Uuid),
    #[error("Pod5 read error: {0}")]
    Pod5ReadError(#[from] Pod5ReadError),
    #[error("Discordant signal lengths: {0} vs {1} (expected)")]
    DiscordantSignalLength(usize, usize),
    #[error("Expected signal length not set in read")]
    ExpectedSignalLenNotFound,
    #[error("Signal table is none")]
    SignalTableNone,
}

