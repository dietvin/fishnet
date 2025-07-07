/*! 
 * This module provides functionality for writing alignment data to Arrow/Parquet files
 * with efficient batching and compression. It implements the `AlignmentWriter` trait
 * to output alignments in a columnar format optimized for analytical workloads.
 * 
 * ## Features
 * 
 * - **Batched Writing**: Buffers alignment records in memory before writing to disk
 * for improved I/O performance
 * - **Parquet Format**: Outputs data in Apache Parquet format with SNAPPY compression
 * for efficient storage and fast query performance
 * - **Columnar Schema**: Stores read IDs, query-to-signal alignments, and reference-to-signal
 * alignments in separate columns for optimal analytics
 * - **Optional Data Handling**: Properly handles cases where alignment data may be missing
 * (e.g., unmapped reads)
 * - **Error Handling**: Comprehensive error handling for file operations and data serialization
 * 
 * ## Data Schema
 * 
 * The output Parquet file contains three columns:
 * - `read_id` (String): Unique identifier for each sequencing read
 * - `query_to_signal` (List<UInt64>): Optional array of query-to-signal alignment positions
 * - `ref_to_signal` (List<UInt64>): Optional array of reference-to-signal alignment positions
 * 
 * ## Performance Considerations
 * 
 * - Larger batch sizes reduce I/O overhead but increase memory usage
 * - SNAPPY compression provides good balance between compression ratio and speed
 * - Arrow format enables efficient columnar analytics on the output data
 */

use std::{fs::File, path::PathBuf, sync::Arc};
use arrow::{array::{ArrayRef, ListBuilder, RecordBatch, StringBuilder, UInt64Builder}, datatypes::{DataType, Field, Schema}};
use parquet::{arrow::ArrowWriter, file::properties::WriterProperties};
use crate::error::output_errors::OutputError;

use super::AlignmentWriter;

/// Writer that buffers alignment data and writes it to an Arrow file in batches
///
/// This struct buffers read IDs and alignment data (query-to-signal and reference-to-signal)
/// until a specified batch size is reached, then writes the data to an Arrow file in
/// the Parquet format with SNAPPY compression.
pub struct OutputWriterArrow {
    writer: Option<ArrowWriter<File>>,
    schema: Arc<Schema>,
    batch_size: usize,

    // Buffers for collecting data
    buf_read_ids: Vec<String>,
    buf_query_alignments: Vec<Option<Vec<usize>>>,
    buf_ref_alignments: Vec<Option<Vec<usize>>>
}

impl AlignmentWriter for OutputWriterArrow {
    /// Creates a new Arrow file writer for alignment data
    ///
    /// # Arguments
    ///
    /// * `path` - Path to the output Arrow file
    /// * `force_overwrite` - If true, overwrites existing file; if false, returns error when file exists
    /// * `batch_size` - Number of records to buffer before writing to disk
    ///
    /// # Returns
    ///
    /// A new `OutputWriterArrow` instance or an error if initialization fails
    ///
    /// # Errors
    ///
    /// Returns `OutputError::FileExists` if the file exists and `force_overwrite` is false
    /// Returns I/O errors if file creation fails
    /// Returns Arrow errors if writer initialization fails
    fn new(path: &PathBuf, force_overwrite: bool, batch_size: usize) -> Result<Self, OutputError> {
        if path.exists() && !force_overwrite {
            return Err(OutputError::FileExists(path.clone()));
        }

        let schema = Arc::new(Schema::new(vec![
            Field::new("read_id", DataType::Utf8, false),
            Field::new(
                "query_to_signal", 
                DataType::List(Arc::new(
                    Field::new("item", DataType::UInt64, true)
                )), 
                true),
            Field::new(
                "ref_to_signal",
                DataType::List(Arc::new(
                    Field::new("item", DataType::UInt64, true)
                )), 
                true
            )
        ]));

        let file = File::create(path)?;

        let props = WriterProperties::builder()
            .set_compression(parquet::basic::Compression::SNAPPY)
            .build();
        
        let writer = ArrowWriter::try_new(file, schema.clone(), Some(props))?;

        Ok(OutputWriterArrow { 
            writer: Some(writer),
            schema,
            batch_size, 
            buf_read_ids: Vec::with_capacity(batch_size), 
            buf_query_alignments: Vec::with_capacity(batch_size), 
            buf_ref_alignments: Vec::with_capacity(batch_size)
        })
    }

    /// Writes a single read's alignment data to the buffer
    ///
    /// Adds the read ID and alignment data to internal buffers. Automatically flushes
    /// the buffers to disk when the batch size is reached.
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
    ///
    /// # Errors
    ///
    /// Returns `OutputError::AlreadyFinalized` if the writer has been finalized
    /// Propagates any errors from flushing if the batch size is reached
    fn write_record(
        &mut self,
        read_id: &str,
        query_to_signal: Option<&Vec<usize>>,
        ref_to_signal: Option<&Vec<usize>>,
    ) -> Result<(), OutputError> {
        if self.writer.is_none() {
            return Err(OutputError::AlreadyFinalized);
        }

        self.buf_read_ids.push(read_id.to_string());
        self.buf_query_alignments.push(query_to_signal.cloned());
        self.buf_ref_alignments.push(ref_to_signal.cloned());

        if self.buf_read_ids.len() >= self.batch_size {
            self.flush()?
        }

        Ok(())
    }

    /// Writes all buffered data to disk
    ///
    /// Creates Arrow arrays from the buffered data and writes them as a record batch.
    /// Clears all buffers after a successful write.
    ///
    /// # Returns
    ///
    /// `Ok(())` if the flush was successful, or an error otherwise
    ///
    /// # Errors
    ///
    /// Returns `OutputError::AlreadyFinalized` if the writer has been finalized
    /// Returns Arrow errors if creating arrays or writing the batch fails
    fn flush(&mut self) -> Result<(), OutputError> {
        let writer = match &mut self.writer {
            None => return Err(OutputError::AlreadyFinalized),
            Some(w) => w
        };

        if self.buf_read_ids.is_empty() {
            return Ok(());
        }

        // Build the read id column
        let mut read_id_builder = StringBuilder::new();
        for read_id in &self.buf_read_ids {
            read_id_builder.append_value(read_id);
        }
        let read_id_array = Arc::new(read_id_builder.finish()) as ArrayRef;

        // Build query alignment column
        let mut query_builder = ListBuilder::new(UInt64Builder::new());

        for alignment_opt in &self.buf_query_alignments {
            if let Some(alignment) = alignment_opt {
                for &val in alignment {
                    query_builder.values().append_value(val as u64);
                }
                query_builder.append(true);
            } else {
                query_builder.append(false);
            }
        }
        let query_array = Arc::new(query_builder.finish()) as ArrayRef;

        // Build ref alignment array
        let mut ref_builder = ListBuilder::new(UInt64Builder::new());
        
        for alignment_opt in &self.buf_ref_alignments {
            if let Some(alignment) = alignment_opt {
                for &value in alignment {
                    ref_builder.values().append_value(value as u64);
                }
                ref_builder.append(true);
            } else {
                ref_builder.append(false);
            }
        }
        let ref_array = Arc::new(ref_builder.finish()) as ArrayRef;

        // Create batch from arrays
        let batch = RecordBatch::try_new(
            self.schema.clone(),
            vec![read_id_array, query_array, ref_array]
        )?;

        writer.write(&batch)?;

        self.buf_read_ids.clear();
        self.buf_query_alignments.clear();
        self.buf_ref_alignments.clear();

        Ok(())
    }

    /// Finalizes the writer, flushing any remaining data and closing the file
    ///
    /// Consumes the writer, preventing further use after finalization.
    ///
    /// # Returns
    ///
    /// `Ok(())` if finalization was successful, or an error otherwise
    ///
    /// # Errors
    ///
    /// Propagates any errors from flushing or closing the writer
    fn finalize(&mut self) -> Result<(), OutputError> {
        self.flush()?;

        if let Some(writer) = self.writer.take() {
            writer.close()?;
        }

        Ok(())
    }
}