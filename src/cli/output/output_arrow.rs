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
use arrow::{array::{ArrayRef, Int16Builder, ListBuilder, RecordBatch, StringBuilder, UInt64Builder}, datatypes::{DataType, Field, Schema}};
use parquet::{arrow::ArrowWriter, file::properties::WriterProperties};
use crate::{cli::output::{OutputData, OutputSchema}, error::output_errors::OutputError};

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
    output_schema: OutputSchema,

    // Basic buffers
    buf_read_ids: Vec<String>,
    buf_query_alignments: Vec<Option<Vec<usize>>>,
    buf_ref_alignments: Vec<Option<Vec<usize>>>,

    // Optional buffers
    buf_query_seq: Vec<Option<String>>,
    buf_ref_seq: Vec<Option<String>>,
    buf_signal: Vec<Option<Vec<i16>>>
}

impl OutputWriterArrow {
    /// Creates the Arrow schema based on the ouput schema
    fn create_schema(output_schema: &OutputSchema) -> Arc<Schema> {
        let mut fields = vec![
            Field::new("read_id", DataType::Utf8, false),
            Field::new(
                "query_to_signal", 
                DataType::List(Arc::new(
                    Field::new("item", DataType::UInt64, true)
                )), 
                true
            ),
            Field::new(
                "ref_to_signal",
                DataType::List(Arc::new(
                    Field::new("item", DataType::UInt64, true)
                )), 
                true
            )
        ];

        match output_schema {
            OutputSchema::Basic => {}, // nothing needs to be added
            OutputSchema::WithSequences => {
                fields.push(Field::new("query_sequence", DataType::Utf8, true));
                fields.push(Field::new("ref_sequence", DataType::Utf8, true));
            },
            OutputSchema::WithSequencesAndSignal => {
                fields.push(Field::new("query_sequence", DataType::Utf8, true));
                fields.push(Field::new("ref_sequence", DataType::Utf8, true));
                fields.push(Field::new(
                    "signal", 
                    DataType::List(Arc::new(
                        Field::new("item", DataType::Int16, true)
                    )),
                    true
                ));
            }
        }

        Arc::new(Schema::new(fields))
    }
}

impl AlignmentWriter for OutputWriterArrow {
    /// Creates a new Arrow file writer for alignment data
    ///
    /// # Arguments
    ///
    /// * `path` - Path to the output Arrow file
    /// * `force_overwrite` - If true, overwrites existing file; if false, returns error when file exists
    /// * `batch_size` - Number of records to buffer before writing to disk
    /// * `output_schema` - The schema determining which columns get written to the output file
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
    fn new(
        path: &PathBuf, 
        force_overwrite: bool, 
        batch_size: usize, 
        output_schema: OutputSchema
    ) -> Result<Self, OutputError> {
        if path.exists() && !force_overwrite {
            return Err(OutputError::FileExists(path.clone()));
        }

        let schema = OutputWriterArrow::create_schema(&output_schema);
        let file = File::create(path)?;

        let props = WriterProperties::builder()
            .set_compression(parquet::basic::Compression::SNAPPY)
            .build();
        
        let writer = ArrowWriter::try_new(file, schema.clone(), Some(props))?;

        Ok(OutputWriterArrow { 
            writer: Some(writer), 
            schema, 
            batch_size, 
            output_schema, 
            buf_read_ids: Vec::with_capacity(batch_size), 
            buf_query_alignments: Vec::with_capacity(batch_size), 
            buf_ref_alignments: Vec::with_capacity(batch_size), 
            buf_query_seq: Vec::with_capacity(batch_size), 
            buf_ref_seq: Vec::with_capacity(batch_size), 
            buf_signal: Vec::with_capacity(batch_size) 
        })
    }

    /// Writes a single read's alignment data to the buffer
    ///
    /// Adds the read ID and alignment data to internal buffers. Automatically flushes
    /// the buffers to disk when the batch size is reached.
    ///
    /// # Arguments
    ///
    /// * `data` - Output data containing the actual data that gets written to file
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
        data: OutputData
    ) -> Result<(), OutputError> {
        if self.writer.is_none() {
            return Err(OutputError::AlreadyFinalized);
        }

        // Check if the provided data matches the expected output schema
        if !data.matches(&self.output_schema) {
            return Err(OutputError::InvalidOutputSchema(
                format!(
                    "OutputData type {:?} does not match writer OutputSchema {:?}",
                    std::mem::discriminant(&data),
                    self.output_schema
                )
            ));
        }

        match data {
            OutputData::Basic { 
                read_id, 
                query_to_signal, 
                ref_to_signal 
            } => {
                self.buf_read_ids.push(read_id.to_string());
                self.buf_query_alignments.push(query_to_signal);
                self.buf_ref_alignments.push(ref_to_signal);
            }

            OutputData::WithSequences { 
                read_id, 
                query_to_signal, 
                ref_to_signal, 
                query_sequence, 
                ref_sequence 
            } => {
                self.buf_read_ids.push(read_id.to_string());
                self.buf_query_alignments.push(query_to_signal);
                self.buf_ref_alignments.push(ref_to_signal);

                self.buf_query_seq.push(query_sequence.map(|s| s.to_string()));
                self.buf_ref_seq.push(ref_sequence.map(|s| s.to_string()));
            }

            OutputData::WithSequencesAndSignal { 
                read_id, 
                query_to_signal, 
                ref_to_signal, 
                query_sequence, 
                ref_sequence, 
                signal 
            } => {
                self.buf_read_ids.push(read_id.to_string());
                self.buf_query_alignments.push(query_to_signal);
                self.buf_ref_alignments.push(ref_to_signal);
                
                self.buf_query_seq.push(query_sequence.map(|s| s.to_string()));
                self.buf_ref_seq.push(ref_sequence.map(|s| s.to_string()));

                self.buf_signal.push(signal);
            }
        }

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

        // Collects all columns
        let mut columns: Vec<ArrayRef> = Vec::new();

        // Build the read id column (always present)
        let mut read_id_builder = StringBuilder::new();
        for read_id in &self.buf_read_ids {
            read_id_builder.append_value(read_id);
        }
        columns.push(Arc::new(read_id_builder.finish()));

        // Build query alignment column (always present)
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
        columns.push(Arc::new(query_builder.finish()));

        // Build ref alignment array (always present)
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
        columns.push(Arc::new(ref_builder.finish()));

        match self.output_schema {
            OutputSchema::Basic => {}, // nothing needs to be added
            OutputSchema::WithSequences | OutputSchema::WithSequencesAndSignal => {
                // Build the query sequence column
                let mut query_seq_builder = StringBuilder::new();
                for seq in &self.buf_query_seq {
                    if let Some(s) = seq {
                        query_seq_builder.append_value(s);
                    } else {
                        query_seq_builder.append_null();
                    }
                }
                columns.push(Arc::new(query_seq_builder.finish()));

                // Build the reference sequence column
                let mut ref_seq_builder = StringBuilder::new();
                for seq in &self.buf_ref_seq {
                    if let Some(s) = seq {
                        ref_seq_builder.append_value(s);
                    } else {
                        ref_seq_builder.append_null();
                    }
                }
                columns.push(Arc::new(ref_seq_builder.finish()));

                if matches!(self.output_schema, OutputSchema::WithSequencesAndSignal) {
                    // Build the signal column
                    let mut signal_builder = ListBuilder::new(Int16Builder::new());
                    for signal_opt in &self.buf_signal {
                        if let Some(signal) = signal_opt {
                            for &val in signal {
                                signal_builder.values().append_value(val);
                            }
                            signal_builder.append(true);
                        } else {
                            signal_builder.append(false);
                        }
                    }
                    columns.push(Arc::new(signal_builder.finish()));
                }
            }
        }

        // Create batch from arrays
        let batch = RecordBatch::try_new(
            self.schema.clone(),
            columns
        )?;

        writer.write(&batch)?;

        self.buf_read_ids.clear();
        self.buf_query_alignments.clear();
        self.buf_ref_alignments.clear();
        self.buf_query_seq.clear();
        self.buf_ref_seq.clear();
        self.buf_signal.clear();

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