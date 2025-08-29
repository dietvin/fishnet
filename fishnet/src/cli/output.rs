/*!
 * This module handles writing the alignments to file.
 */

use std::path::PathBuf;

use crate::{cli::{output::output_data::OutputData, parse::args_to_input::WhichToAlign}, error::output_errors::OutputError};

pub mod output_arrow;
pub mod output_json;
pub mod arrow_buffer;
pub mod output_data;

#[derive(Debug, Clone, PartialEq)]
pub struct OutputConfig {
    pub alignment_type: WhichToAlign,
    pub include_sequences: bool,
    pub include_signal: bool 
}

impl OutputConfig {
    pub fn new(
        alignment_type: WhichToAlign,
        include_sequences: bool,
        include_signal: bool
    ) -> Self {
        OutputConfig { 
            alignment_type,
            include_sequences,
            include_signal
        }
    }

    pub fn alignment_type(&self) -> &WhichToAlign {
        &self.alignment_type
    }

    pub fn include_sequences(&self) -> bool {
        self.include_sequences
    }

    pub fn include_signal(&self) -> bool {
        self.include_signal
    }

    pub fn which_to_include(&self) -> (&bool, &bool) {
        (&self.include_sequences, &self.include_signal)
    }
}


/// Trait for alignment output writers
///
/// This trait defines the common interface for writing read alignments 
/// to various output formats, such as Arrow, BAM/SAM, or JSON.
pub trait AlignmentWriter: Send {
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
        schema: OutputConfig
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