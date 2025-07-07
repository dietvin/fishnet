/*!
 * This module handles writing the alignments to file.
 */

use std::path::PathBuf;

use crate::error::output_errors::OutputError;

pub mod output_arrow;
pub mod output_json;

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
    ///
    /// # Returns
    ///
    /// A new writer instance or an error if initialization fails
    fn new(path: &PathBuf, force_overwrite: bool, batch_size: usize) -> Result<Self, OutputError> 
    where 
        Self: Sized;

    /// Writes a single read's alignment data
    ///
    /// # Arguments
    ///
    /// * `read_id` - Unique identifier for the read
    /// * `query_to_signal` - Optional query-to-signal alignment vector
    /// * `ref_to_signal` - Optional reference-to-signal alignment vector
    ///
    /// # Returns
    ///
    /// `Ok(())` if the record was added successfully, or an error otherwise
    fn write_record(
        &mut self,
        read_id: &str,
        query_to_signal: Option<&Vec<usize>>,
        ref_to_signal: Option<&Vec<usize>>
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