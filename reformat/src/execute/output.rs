use std::path::PathBuf;

use crate::{error::execute::OutputError, execute::{config::{OutputShape, ReformatStrategy}, output::output_data::OutputData}};

pub(crate) mod output_data;
pub(crate) mod output_arrow;
pub(crate) mod output_tsv;
mod arrow_buffer;

 /// Trait for alignment output writers
 ///
 /// This trait defines the common interface for writing read alignments 
 /// to various output formats, such as Arrow, BAM/SAM, or JSON.
 pub trait ReformatWriter: Send {
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
        reformat_strategy: &ReformatStrategy,
        output_shape: &OutputShape,
        uniform_roi_length: Option<usize>
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