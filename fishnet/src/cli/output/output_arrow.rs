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
use std::{
    fs::File, 
    path::PathBuf, 
    vec
};
use arrow2::{
    datatypes::{DataType, Field, Schema},
    io::parquet::write::{
        CompressionOptions, 
        Encoding, 
        FileWriter, 
        Version, 
        WriteOptions
    } 
};
use crate::{
    cli::{
        output::{
            output_data::OutputData, 
            OutputConfig
        },
        parse::args_to_input::WhichToAlign
    }, 
    error::output_errors::OutputError
};
use super::{AlignmentWriter, arrow_buffer::ArrowBuffer};


/// Writer that buffers alignment data and writes it to an Arrow file in batches
///
/// This struct buffers read IDs and alignment data (query-to-signal and reference-to-signal)
/// until a specified batch size is reached, then writes the data to an Arrow file in
/// the Parquet format with SNAPPY compression.
pub struct OutputWriterArrow {
    writer: Option<FileWriter<File>>,
    schema: Schema,
    options: WriteOptions,
    encodings: Vec<Vec<Encoding>>,
    batch_size: usize,
    output_config: OutputConfig,
    // Buffers
    buffer: ArrowBuffer,
    current_buffer_size: usize
}

impl OutputWriterArrow {
    /// Creates the Arrow schema based on the output schema
    fn create_schema(output_schema: &OutputConfig) -> Schema {
        // Read id column is always present
        let mut fields = vec![Field::new("read_id", DataType::Utf8, false)];

        let alignment_type = output_schema.alignment_type();
        let include_sequences = output_schema.include_sequences();
        let include_signal = output_schema.include_signal();

        match alignment_type {
            WhichToAlign::Query => {
                fields.push(
                    Field::new(
                        "query_to_signal", 
                        DataType::List(Box::new(
                            Field::new("item", DataType::UInt64, true)
                        )), 
                        true
                    )
                );
            }
            WhichToAlign::Reference => {
                fields.append(&mut vec![
                    Field::new(
                        "ref_to_signal",
                        DataType::List(Box::new(
                            Field::new("item", DataType::UInt64, true)
                        )), 
                        true
                    ),
                    Field::new("ref_name", DataType::Utf8, false),
                    Field::new("ref_start", DataType::UInt64, false),
                ]);
            }
            WhichToAlign::Both => {
                fields.append(&mut vec![
                    Field::new(
                        "query_to_signal", 
                        DataType::List(Box::new(
                            Field::new("item", DataType::UInt64, true)
                        )), 
                        true
                    ),
                    Field::new(
                        "ref_to_signal",
                        DataType::List(Box::new(
                            Field::new("item", DataType::UInt64, true)
                        )), 
                        true
                    ),
                    Field::new("ref_name", DataType::Utf8, false),
                    Field::new("ref_start", DataType::UInt64, false),
                ]);
            }
        }

        if include_sequences {
            match alignment_type {
                WhichToAlign::Query => {
                    fields.push(Field::new("query_sequence", DataType::Utf8, true));
                }
                WhichToAlign::Reference => {
                    fields.push(Field::new("ref_sequence", DataType::Utf8, true));
                }
                WhichToAlign::Both => {
                    fields.push(Field::new("query_sequence", DataType::Utf8, true));
                    fields.push(Field::new("ref_sequence", DataType::Utf8, true));
                }
            }

            if include_signal {
                fields.push(Field::new(
                    "signal", 
                    DataType::List(Box::new(
                        Field::new("item", DataType::Int16, true)
                    )),
                    true
                ));
            }
        }

        Schema::from(fields)
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
        output_config: OutputConfig
    ) -> Result<Self, OutputError> {
        if path.exists() && !force_overwrite {
            return Err(OutputError::FileExists(path.clone()));
        }

        let schema = OutputWriterArrow::create_schema(&output_config);
        let file = File::create(path)?;

        let options = WriteOptions {
            write_statistics: true,
            compression: CompressionOptions::Snappy,
            version: Version::V2,
            data_pagesize_limit: None,
        };

        let encodings = schema
            .fields
            .iter()
            .map(|_| vec![Encoding::Plain])
            .collect::<Vec<Vec<Encoding>>>();

        let writer = FileWriter::try_new(file, schema.clone(), options)?;

        let buffer = ArrowBuffer::new(&output_config);

        Ok(OutputWriterArrow { 
            writer: Some(writer), 
            schema, 
            options,
            encodings,
            batch_size, 
            output_config, 
            buffer,
            current_buffer_size: 0
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
        if !data.matches(&self.output_config) {
            return Err(OutputError::InvalidOutputSchema(
                format!(
                    "OutputData type {:?} does not match writer OutputSchema {:?}",
                    std::mem::discriminant(&data),
                    self.output_config
                )
            ));
        }

        self.buffer.push_data(&data)?;
        self.current_buffer_size += 1;

        if self.current_buffer_size >= self.batch_size {
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

        if self.current_buffer_size == 0 {
            return Ok(());
        }

        let row_groups = self.buffer.buffer_to_rowgroupiter(
            &self.schema, 
            &self.encodings, 
            &self.options
        )?;

        for group in row_groups {
            writer.write(group?)?;
        }

        // Clear buffer by re-initializing it
        self.buffer = ArrowBuffer::new(&self.output_config);
        self.current_buffer_size = 0;

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

        if let Some(mut writer) = self.writer.take() {
            writer.end(None)?;
        }

        Ok(())
    }
}