use std::collections::HashMap;

use uuid::Uuid;

use crate::{error::file::SignalTableIndexError, read::Pod5Read};

/// Helper struct for efficiently tracking signal table row processing during iteration.
/// 
/// The `SignalTableIndex` maintains a mapping between signal table rows and the reads they belong to,
/// enabling efficient reconstruction of complete signal data from potentially out-of-order chunks.
/// 
/// This struct tracks:
/// - Which read each signal table row corresponds to
/// - The position of each signal chunk within the complete signal for that read
/// - How many signal chunks remain unprocessed for each read
#[derive(Debug, Clone)]
pub(crate) struct SignalTableIndex {
    /// Maps signal table row indices to (read_id, chunk_position) pairs.
    /// 
    /// For row `i` in the signal table:
    /// - `row_to_id[i].0` is the UUID of the read that owns the signal chunk in row `i`
    /// - `row_to_id[i].1` is the position where this chunk belongs in the complete signal
    /// - `None` indicates the row is unassignable (should not occur in valid data)    
    row_to_id: Vec<Option<(Uuid, usize)>>,

    /// Tracks the number of unprocessed signal chunks for each read.
    /// 
    /// When a read's count reaches 0, all of its signal chunks have been collected
    /// and the complete signal can be reconstructed.
    remaining_chunks: HashMap<Uuid, usize>,

    /// Total number of rows in the signal table.
    /// Used for bounds checking in `process_row()`.
    n_rows: usize,

    /// Number of reads that have had all their signal chunks collected.
    /// 
    /// This should equal the number of unique reads (length of `remaining_chunks`)
    /// when processing is complete, serving as a validation check.
    n_reads_finished: usize
}

impl SignalTableIndex {
    /// Creates a new signal table index from read metadata.
    /// 
    /// This constructor analyzes the signal indices stored in each read to build
    /// the mapping between signal table rows and reads. It pre-populates the
    /// tracking structures needed for efficient row-by-row processing of the
    /// signal table.
    /// 
    /// # Arguments
    /// 
    /// * `reads_map` - HashMap containing all reads in the Pod5 file, keyed by UUID
    /// * `n_signal_table_rows` - Total number of rows in the signal table
    /// 
    /// # Returns
    /// 
    /// A new `SignalTableIndex` ready for processing signal table rows.
    pub fn new(reads_map: &HashMap<Uuid, Pod5Read>, n_signal_table_rows: usize) -> Result<Self, SignalTableIndexError> {
        let mut row_to_id: Vec<Option<(Uuid, usize)>> = vec![None; n_signal_table_rows];
        let mut remaining_chunks: HashMap<Uuid, usize> = HashMap::new();

        for (read_id, read) in reads_map {
            let signal_row_indices = read.signal_indices();

            remaining_chunks.insert(*read_id, signal_row_indices.len());
            
            for (i, row) in signal_row_indices.iter().enumerate() {
                let row = *row as usize;
                if row < n_signal_table_rows {
                    row_to_id[row] = Some((read_id.clone(), i));
                } else {
                    return Err(SignalTableIndexError::InvalidIndex(row, n_signal_table_rows));
                }
            }
        }

        Ok(SignalTableIndex { 
            row_to_id, 
            remaining_chunks, 
            n_rows: n_signal_table_rows,
            n_reads_finished: 0
        })
    }

    /// Processes a signal table row and returns information about the associated read.
    /// 
    /// This method is called for each row in the signal table during iteration. It:
    /// 1. Looks up which read the row belongs to
    /// 2. Determines where the signal chunk fits in the complete signal
    /// 3. Updates the remaining chunk count for that read
    /// 
    /// # Arguments
    /// 
    /// * `row_idx` - Index of the signal table row to process (0-based)
    /// 
    /// # Returns
    /// 
    /// A tuple containing:
    /// - `Uuid` - ID of the read that owns this signal chunk
    /// - `usize` - Position where this chunk belongs in the complete signal (0-based)
    /// - `usize` - Number of chunks remaining for this read after processing this row
    ///   (0 means all chunks have been collected)
    /// 
    /// # Errors
    /// 
    /// Returns `SignalTableIndexError` if:
    /// - `row_idx` is out of bounds
    /// - The row has no associated read (corrupted data)
    /// - All chunks for the read were already processed (duplicate processing)
    /// - Internal consistency error (read not found in tracking map)
    pub fn process_row(&mut self, row_idx: usize) -> Result<(Uuid, usize, usize), SignalTableIndexError> {
        if row_idx >= self.n_rows {
            Err(SignalTableIndexError::InvalidIndex(row_idx, self.n_rows))
        } else {
            if let Some((read_id, chunk_position)) = &self.row_to_id[row_idx] {
                if self.remaining_chunks[&read_id] == 0 {
                    return Err(SignalTableIndexError::ReadAlreadyFinished(*read_id));
                }
        
                if let Some(val) = self.remaining_chunks.get_mut(&read_id) {
                    *val -= 1;
                    let n_remaining = *val;
                    self.n_reads_finished += 1;

                    Ok((*read_id, *chunk_position, n_remaining))

                } else {
                    return Err(SignalTableIndexError::ReadIdNotFound(*read_id));
                }
            } else {
                Err(SignalTableIndexError::RowNotAssignable(row_idx))
            }

        }
    }

    /// Validates that all reads have been fully processed.
    /// 
    /// This method should be called after processing all signal table rows to ensure
    /// data integrity. It verifies that every read had all of its signal chunks
    /// collected during processing.
    /// 
    /// # Returns
    /// 
    /// `Ok(())` if all reads were completed successfully.
    /// 
    /// # Errors
    /// 
    /// Returns `SignalTableIndexError::NotAllReadsFinished` if some reads are incomplete,
    /// indicating either:
    /// - Missing signal table rows
    /// - Corrupted signal indices in read metadata  
    /// - Incomplete processing of the signal table
    pub fn properly_finished(&self) -> Result<(), SignalTableIndexError> {
        if self.remaining_chunks.len() > self.n_reads_finished {
            return Err(SignalTableIndexError::NotAllReadsFinished(
                self.remaining_chunks.len() - self.n_reads_finished
            ));
        }
        Ok(())
    }
}