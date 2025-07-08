/*!
 * This module handles writing the alignments to file.
 */

use std::path::PathBuf;

use crate::error::output_errors::OutputError;

pub mod output_arrow;
pub mod output_json;


/// Represents different schemas to write to file
#[derive(Debug, Clone)]
pub enum OutputData {
    /// Minimal output format containing only the alignments
    Basic {
        read_id: String,
        query_to_signal: Option<Vec<usize>>,
        ref_to_signal: Option<Vec<usize>>,
    },
    /// Extended output format containing the sequences in 
    /// addition to the alignments
    WithSequences {
        read_id: String,
        query_to_signal: Option<Vec<usize>>,
        ref_to_signal: Option<Vec<usize>>,
        query_sequence: Option<String>,
        ref_sequence: Option<String>,
    },
    /// Extended output format containing the alignements,
    /// the sequences and the signal
    WithSequencesAndSignal {
        read_id: String,
        query_to_signal: Option<Vec<usize>>,
        ref_to_signal: Option<Vec<usize>>,
        query_sequence: Option<String>,
        ref_sequence: Option<String>,
        signal: Option<Vec<i16>>
    }
}

/// Constructor functions for each option
impl OutputData {
    pub fn basic(
        read_id: String,
        query_to_signal: Option<Vec<usize>>,
        ref_to_signal: Option<Vec<usize>>,
    ) -> Self {
        OutputData::Basic { 
            read_id, 
            query_to_signal,
            ref_to_signal
        }
    }

    pub fn with_seq(
        read_id: String,
        query_to_signal: Option<Vec<usize>>,
        ref_to_signal: Option<Vec<usize>>,
        query_sequence: Option<String>,
        ref_sequence: Option<String>,
    ) -> Self {
        OutputData::WithSequences { 
            read_id,
            query_to_signal,
            ref_to_signal,
            query_sequence,
            ref_sequence 
        }
    }

    pub fn with_seq_and_signal(
        read_id: String,
        query_to_signal: Option<Vec<usize>>,
        ref_to_signal: Option<Vec<usize>>,
        query_sequence: Option<String>,
        ref_sequence: Option<String>,
        signal: Option<Vec<i16>>   
    ) -> Self {
        OutputData::WithSequencesAndSignal { 
            read_id,
            query_to_signal,
            ref_to_signal,
            query_sequence,
            ref_sequence,
            signal
        }
    }

    pub fn read_id(&self) -> &str {
        match self {
            OutputData::Basic { read_id, .. } => read_id,
            OutputData::WithSequences { read_id, .. } => read_id,
            OutputData::WithSequencesAndSignal { read_id, .. } => read_id
        }
    }

    /// Checks if a given output schema corresponds to the output data at hand.
    pub fn matches(&self, output_schema: &OutputSchema) -> bool {
        match (self, output_schema) {
            (OutputData::Basic { .. }, OutputSchema::Basic) => true,
            (OutputData::WithSequences { .. }, OutputSchema::WithSequences) => true,
            (OutputData::WithSequencesAndSignal { .. }, OutputSchema::WithSequencesAndSignal) => true,
            _ => false
        }
    }
}



#[derive(Debug, Clone, PartialEq)]
pub enum OutputSchema {
    Basic,
    WithSequences,
    WithSequencesAndSignal
}


/// Trait for alignment output writers
///
/// This trait defines the common interface for writing read alignments 
/// to various output formats, such as Arrow, BAM/SAM, or JSON.
pub trait AlignmentWriter {
    /// Creates a new alignment writer
    ///
    /// # Arguments
    ///
    /// * `path` - Path to the output file
    /// * `force_overwrite` - If true, overwrites existing file; if false, returns error when file exists
    /// * `batch_size` - Number of records to buffer before writing to disk
    /// * `schema` - The schema defining what data fields to include
    /// 
    /// # Returns
    ///
    /// A new writer instance or an error if initialization fails
    fn new(
        path: &PathBuf, 
        force_overwrite: bool, 
        batch_size: usize, 
        schema: OutputSchema
    ) -> Result<Self, OutputError> 
    where 
        Self: Sized;

    /// Writes a single read's alignment data
    ///
    /// # Arguments
    ///
    /// * `data` - OutputData containing the data for one row (read)
    ///
    /// # Returns
    ///
    /// `Ok(())` if the record was added successfully, or an error otherwise
    fn write_record(
        &mut self,
        data: OutputData
    ) -> Result<(), OutputError>;

    /// Writes all buffered data to disk
    ///
    /// # Returns
    ///
    /// `Ok(())` if the flush was successful, or an error otherwise
    fn flush(&mut self) -> Result<(), OutputError>;

    /// Finalizes the writer, flushing any remaining data and closing the file
    ///
    /// Consumes the writer, preventing further use after finalization.
    ///
    /// # Returns
    ///
    /// `Ok(())` if finalization was successful, or an error otherwise
    fn finalize(&mut self) -> Result<(), OutputError>;
}