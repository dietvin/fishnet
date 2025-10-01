use std::{fs::File, path::PathBuf};

use arrow2::{
    datatypes::{
        DataType, 
        Field, 
        Schema
    }, 
    io::parquet::write::{
        CompressionOptions, 
        Encoding, 
        FileWriter, 
        Version, 
        WriteOptions
    }
};

use crate::{
    error::execute::OutputError, 
    execute::{
        config::{
            OutputShape, 
            ReformatStrategy
        }, 
        output::{
            arrow_buffer::ArrowBuffer, 
            output_data::OutputData, 
            ReformatWriter
        }
    }
};

/// Writer for outputting processed reads to Parquet files.
///
/// This struct manages the entire lifecycle of writing data to Parquet:
/// - Schema creation based on reformat strategy and output shape
/// - Buffering incoming data records
/// - Periodic flushing of buffered data to disk
/// - File finalization with proper metadata
///
/// The writer uses a buffering strategy where data is accumulated in memory until
/// `batch_size` records are collected, then written as a row group to the Parquet file.
/// This improves write performance and compression efficiency.
pub(crate) struct OutputWriterArrow {
    /// The underlying Parquet file writer, wrapped in Option to allow taking ownership during finalization
    writer: Option<FileWriter<File>>,
    /// Arrow schema defining the structure and data types of the output columns
    schema: Schema,
    /// Write options including compression algorithm, statistics, and Parquet version
    options: WriteOptions,
    /// Encoding specifications for each column in the Parquet file
    encodings: Vec<Vec<Encoding>>,
    /// Number of records to buffer before flushing to disk
    batch_size: usize,
    /// The output format shape (melted, exploded, or nested)
    output_shape: OutputShape,
    /// The data processing strategy (statistics or interpolation)
    reformat_strategy: ReformatStrategy,
    /// Required for exploded format: the uniform length of all regions of interest
    uniform_roi_length: Option<usize>,
    /// In-memory buffer for accumulating data before writing
    buffer: ArrowBuffer,
    /// Current number of records in the buffer
    current_buffer_size: usize
}

impl OutputWriterArrow {
    /// Creates an Arrow schema matching the reformat strategy and output shape.
    ///
    /// The schema structure varies based on the combination of strategy and shape:
    /// 
    /// - Common fields (all variants):
    ///     - `read_id`: Sequencing read identifier (Utf8)
    ///     - `start_index_on_read`: Starting position of ROI on read (UInt64)
    ///     - `region_of_interest`: Name of matched region/filter (Utf8)
    ///
    /// - Melted format (long):
    ///     - `base_index`: Position index within ROI (UInt64)
    ///     - `base`: Nucleotide base (Utf8)
    ///     - For stats: one column per statistic (Float64)
    ///     - For interpolation: columns for each signal position + dwell (Float64)
    ///
    /// - Exploded format (wide):
    ///     - Separate columns for each base position (e.g., `base_0`, `base_1`, ...)
    ///     - For stats: columns for each stat at each position (e.g., `mean_0`, `mean_1`, ...)
    ///     - For interpolation: columns for each signal at each base position + dwell columns
    ///
    /// - Nested format:
    ///     - `bases`: Concatenated bases as string (Utf8)
    ///     - For stats: list columns for each statistic (List<Float64>)
    ///     - For interpolation: nested list for signals (List<List<Float64>>) + list for dwells
    ///
    /// # Arguments
    /// * `reformat_strategy` - The data processing strategy determining which columns are created
    /// * `output_shape` - The output format determining how data is structured
    /// * `uniform_roi_length` - Required for exploded format to determine number of columns
    ///
    /// # Returns
    /// An Arrow schema ready for Parquet file creation
    ///
    /// # Panics
    /// Panics if `output_shape` is `Exploded` but `uniform_roi_length` is `None`
    /// This is checked for before invoking the function, so it should never happen
    fn create_schema(
        reformat_strategy: &ReformatStrategy,
        output_shape: &OutputShape,
        uniform_roi_length: Option<usize>
    ) -> Schema {
        let mut fields = vec![
            Field::new("read_id", DataType::Utf8, false),
            Field::new("start_index_on_read", DataType::UInt64, false),
            Field::new("region_of_interest", DataType::Utf8, false),
        ];

        match (reformat_strategy, output_shape) {
            (ReformatStrategy::ReadWiseStats { stats }, OutputShape::Melted) => {
                fields.push(Field::new("base_index", DataType::UInt64, false));
                fields.push(Field::new("base", DataType::Utf8, false));
                for stat in stats {
                    fields.push(Field::new(stat.to_str(), DataType::Float64, false));
                }
            }

            (ReformatStrategy::ReadWiseStats { stats }, OutputShape::Exploded) => {
                if let Some(roi_length) = uniform_roi_length {
                    // One column for each base
                    for base_idx in 0..roi_length {
                        fields.push(Field::new(
                            format!("base_{}", base_idx),
                            DataType::Utf8,
                            false
                        ));
                    }
                    // One column for each stat at each base
                    for stat in stats {
                        for base_idx in 0..roi_length {
                            fields.push(Field::new(
                                format!("{}_{}", stat.to_str(), base_idx),
                                DataType::Float64,
                                false
                            ));
                        }
                    }
                } else {
                    unreachable!("It's checked before that all regions of interest have the same length when output shape is Exploded")
                }
            }

            (ReformatStrategy::ReadWiseStats { stats }, OutputShape::Nested) => {
                fields.push(Field::new("bases", DataType::Utf8, false));
                for stat in stats {
                    fields.push(Field::new(
                        format!("{}", stat.to_str()), 
                        DataType::List(Box::new(
                            Field::new("item", DataType::Float64, false)
                        )), 
                        false
                    ));
                }
            }
            (ReformatStrategy::Interpolation { target_len }, OutputShape::Melted) => {
                fields.push(Field::new("base_index", DataType::UInt64, false));
                fields.push(Field::new("base", DataType::Utf8, false));

                for signal_idx in 0..*target_len {
                    fields.push(Field::new(
                        format!("signal_{}", signal_idx), 
                        DataType::Float64, 
                        false
                    ));
                }

                fields.push(Field::new("dwell", DataType::Float64, false));
            }

            (ReformatStrategy::Interpolation { target_len }, OutputShape::Exploded) => {
                if let Some(roi_length) = uniform_roi_length {

                    for base_idx in 0..roi_length {
                        fields.push(Field::new(
                            format!("base_{}", base_idx),
                            DataType::Utf8,
                            false
                        ));
                    }

                    for base_idx in 0..roi_length {
                        for signal_idx in 0..*target_len {
                            fields.push(Field::new(
                                format!("signal_base{}_{}", base_idx, signal_idx),
                                DataType::Float64,
                                false
                            ));
                        }
                    }

                    for base_idx in 0..roi_length {
                        fields.push(Field::new(
                            format!("dwell_{}", base_idx),
                            DataType::Float64,
                            false
                        ));
                    }

                } else {
                    unreachable!("It's checked before that all regions of interest have the same length when output shape is Exploded")
                }
            }

            (ReformatStrategy::Interpolation { .. }, OutputShape::Nested) => {
                fields.push(Field::new("bases", DataType::Utf8, false));

                // Nested list for the signal (each base x each interpolated position)
                fields.push(Field::new(
                    "signals",
                    DataType::List(Box::new(
                        Field::new(
                            "signal_for_base", 
                            DataType::List(Box::new(
                                Field::new("interpolated_measurement", DataType::Float64, false)
                            )), 
                            false
                        )
                    )), 
                    false
                ));

                fields.push(Field::new(
                    "dwells", 
                    DataType::List(Box::new(
                        Field::new("dwell", DataType::Float64, false)
                    )), 
                    false
                ));

            }
        }

        Schema::from(fields)
    }
}

impl ReformatWriter for OutputWriterArrow {
    /// Creates a new Parquet writer and initializes the output file.
    ///
    /// This method:
    /// 1. Checks for file existence and handles overwrite logic
    /// 2. Creates the Arrow schema matching the reformat strategy and output shape
    /// 3. Opens the output file for writing
    /// 4. Configures write options (compression: Snappy, version: V2, statistics: enabled)
    /// 5. Initializes the Parquet file writer with metadata
    /// 6. Allocates the internal buffer for data accumulation
    ///
    /// # Arguments
    /// * `path` - Path where the Parquet file will be created
    /// * `force_overwrite` - If true, overwrites existing file; if false, returns error if file exists
    /// * `batch_size` - Number of records to buffer before flushing to disk (row group size)
    /// * `reformat_strategy` - The data processing strategy (stats or interpolation)
    /// * `output_shape` - The desired output format (melted, exploded, or nested)
    /// * `uniform_roi_length` - Required for exploded format: uniform length of all ROIs
    ///
    /// # Returns
    /// * `Ok(Self)` - A fully initialized writer ready to accept data
    /// * `Err(OutputError::FileExists)` - If file exists and `force_overwrite` is false
    /// * `Err(OutputError)` - If file creation or writer initialization fails
    fn new(
        path: &PathBuf,
        force_overwrite: bool,
        batch_size: usize,
        reformat_strategy: &ReformatStrategy,
        output_shape: &OutputShape,
        uniform_roi_length: Option<usize>
    ) -> Result<Self, OutputError> {
        if path.exists() && !force_overwrite {
            return Err(OutputError::FileExists(path.clone()));
        }

        let schema = Self::create_schema(reformat_strategy, output_shape, uniform_roi_length);
        let file = File::create(path)?;

        let options = WriteOptions {
            write_statistics: true, 
            compression: CompressionOptions::Snappy,
            version: Version::V2,
            data_pagesize_limit: None
        };

        let encodings = schema
            .fields
            .iter()
            .map(|_| vec![Encoding::Plain])
            .collect::<Vec<Vec<Encoding>>>();

        let writer = FileWriter::try_new(file, schema.clone(), options)?;

        let buffer = ArrowBuffer::new(
            reformat_strategy, 
            output_shape, 
            batch_size, 
            uniform_roi_length
        );

        Ok(Self { 
            writer: Some(writer), 
            schema, 
            options, 
            encodings, 
            batch_size, 
            output_shape: output_shape.clone(),
            reformat_strategy: reformat_strategy.clone(),
            uniform_roi_length,
            buffer, 
            current_buffer_size: 0
        })
    }

    /// Adds a single data record to the buffer and flushes if batch size is reached.
    ///
    /// Records are accumulated in memory until `batch_size` records have been added,
    /// at which point they are automatically flushed to disk as a row group. This
    /// batching potentially improves write performance and compression efficiency.
    ///
    /// # Arguments
    /// * `data` - Processed data for one read, including metadata and reformatted values
    ///
    /// # Returns
    /// * `Ok(())` - Record was successfully buffered or flushed
    /// * `Err(OutputError::AlreadyFinalized)` - Writer has been finalized and cannot accept new records
    /// * `Err(OutputError)` - If buffer operation or flushing fails
    fn write_record(
        &mut self,
        data: OutputData
    ) -> Result<(), OutputError> {
        if self.writer.is_none() {
            return Err(OutputError::AlreadyFinalized);
        }

        self.buffer.push_data(data)?;
        self.current_buffer_size += 1;

        if self.current_buffer_size >= self.batch_size {
            self.flush()?;
        }

        Ok(())
    }

    /// Writes all buffered data to the Parquet file and clears the buffer.
    ///
    /// This method:
    /// 1. Checks if there is data to flush (returns early if buffer is empty)
    /// 2. Converts buffered data into Arrow arrays
    /// 3. Writes the arrays as a row group to the Parquet file
    /// 4. Re-initializes the buffer for the next batch
    ///
    /// This method is called automatically by `write_record` when the batch size is reached,
    /// as well as in `finalize` once all reads are processed.
    ///
    /// # Returns
    /// * `Ok(())` - Buffer was successfully flushed or was already empty
    /// * `Err(OutputError::AlreadyFinalized)` - Writer has been finalized
    /// * `Err(OutputError)` - If Arrow conversion or Parquet writing fails
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
        self.buffer = ArrowBuffer::new(
            &self.reformat_strategy,
            &self.output_shape,
            self.batch_size,
            self.uniform_roi_length
        );
        self.current_buffer_size = 0;

        Ok(())
    }

    /// Finalizes the Parquet file by flushing remaining data and writing file metadata.
    ///
    /// This method must be called to properly close the Parquet file. It:
    /// 1. Flushes any remaining buffered data
    /// 2. Writes Parquet file footer with schema and metadata
    /// 3. Closes the underlying file handle
    ///
    /// After calling this method, the writer cannot be used to write additional records.
    /// Attempting to call `write_record` or `flush` after finalization will return an error.
    ///
    /// # Returns
    /// * `Ok(())` - File was successfully finalized and closed
    /// * `Err(OutputError)` - If flushing or file finalization fails
    fn finalize(&mut self) -> Result<(), OutputError> {
        self.flush()?;

        if let Some(mut writer) = self.writer.take() {
            writer.end(None)?;
        }

        Ok(())
    }
}
