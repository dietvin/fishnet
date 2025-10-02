use std::{fs::File, io::{BufWriter, Write}, path::PathBuf};

use crate::{
    core::reformater::reformated::ReformatedData, 
    error::execute::{
        OutputError, 
        TsvOutputError
    }, 
    execute::{
        config::{
            OutputShape, 
            ReformatStrategy
        }, 
        output::{
            output_data::OutputData, 
            ReformatWriter
        }
    }
};

/// Buffer size for TSV file I/O operations (8 MB).
/// 
/// This size balances memory usage with I/O performance, reducing the number
/// of system calls needed for writing large TSV files.
const TSV_BUFFER_SIZE: usize = 8 * 1024 * 1024;

/// Writer for outputting processed reads to tab-separated output format (TSV).
///
/// This writer handles the serialization of reformatted sequencing data into TSV format,
/// supporting multiple output shapes (Melted and Exploded) and reformat strategies
/// (ReadWiseStats and Interpolation). The writer uses buffered I/O for optimal performance
/// when writing large datasets.
/// 
/// Note that TSV output does not support `nested` output shape.
pub(crate) struct OutputWriterTsv {
    /// Buffered writer for efficient TSV output to file
    writer: BufWriter<File>,
    /// The output shape determining how data is organized in rows and columns
    output_shape: OutputShape,
    /// The reformatting strategy (statistics or interpolation) being applied to the data
    reformat_strategy: ReformatStrategy
}

impl OutputWriterTsv {
    /// Writes the header row to the TSV file based on the output configuration.
    ///
    /// The header structure varies depending on the combination of output shape and
    /// reformat strategy. All headers begin with three common columns: read_id,
    /// start_index_on_read, and region_of_interest.
    ///
    /// # Arguments
    ///
    /// * `writer` - Mutable reference to the buffered file writer
    /// * `output_shape` - The shape determining column organization (Melted or Exploded)
    /// * `reformat_strategy` - The data processing strategy (ReadWiseStats or Interpolation)
    /// * `uniform_roi_length` - Required length for Exploded output; must be Some when
    ///   output_shape is Exploded, as all regions must have the same length
    ///
    /// # Returns
    ///
    /// * `Ok(())` if header was written successfully
    /// * `Err(TsvOutputError)` if:
    ///   - Output shape is Nested (not supported for TSV)
    ///   - I/O error occurs during writing
    ///
    /// # Header Formats
    ///
    /// ## ReadWiseStats + Melted
    /// ```text
    /// read_id  start_index_on_read  region_of_interest  base_index  base  [stat_names...]
    /// ```
    ///
    /// ## ReadWiseStats + Exploded
    /// ```text
    /// read_id  start_index_on_read  region_of_interest  base_0  base_1  ...  stat_0_0  stat_0_1  ...
    /// ```
    ///
    /// ## Interpolation + Melted
    /// ```text
    /// read_id  start_index_on_read  region_of_interest  base_index  base  signal_0  signal_1  ...  dwell
    /// ```
    ///
    /// ## Interpolation + Exploded
    /// ```text
    /// read_id  start_index_on_read  region_of_interest  base_0  base_1  ...  signal_base0_0  ...  dwell_0  dwell_1  ...
    /// ```
    fn write_header(
        writer: &mut BufWriter<File>,
        output_shape: &OutputShape,
        reformat_strategy: &ReformatStrategy,
        uniform_roi_length: Option<usize>
    ) -> Result<(), TsvOutputError> {
        write!(writer, "read_id\tstart_index_on_read\tregion_of_interest")?;

        match reformat_strategy {
            ReformatStrategy::ReadWiseStats { stats } => {
                match output_shape {
                    OutputShape::Melted => {
                        write!(writer, "\tbase_index\tbase")?;
                        for stat in stats {
                            write!(writer, "\t{}", stat.to_str())?;
                        }
                    }
                    OutputShape::Exploded => {
                        if let Some(roi_length) = uniform_roi_length {
                            for base_idx in 0..roi_length {
                                write!(writer, "\tbase_{}", base_idx)?;
                            }

                            for stat in stats {
                                for base_idx in 0..roi_length {
                                    write!(writer, "\t{}_{}", stat.to_str(), base_idx)?;
                                }
                            }
                        } else {
                            unreachable!("It's checked before that all regions of interest have the same length when output shape is Exploded")
                        }
                    }
                    OutputShape::Nested => return Err(TsvOutputError::NestedOutputNotAvailable)
                }
            }

            ReformatStrategy::Interpolation { target_len } => {
                match output_shape {
                    OutputShape::Melted => {
                        write!(writer, "\tbase_index\tbase")?;
                        for signal_idx in 0..*target_len {
                            write!(writer, "\tsignal_{}", signal_idx)?;
                        }
                        write!(writer, "\tdwell")?;
                    }
                    OutputShape::Exploded => {
                        if let Some(roi_length) = uniform_roi_length {
                            for base_idx in 0..roi_length {
                                write!(writer, "\tbase_{}", base_idx)?;
                            }

                            for base_idx in 0..roi_length {
                                for signal_idx in 0..*target_len {
                                    write!(writer, "\tsignal_base{}_{}", base_idx, signal_idx)?;
                                }
                            }
                        } else {
                            unreachable!("It's checked before that all regions of interest have the same length when output shape is Exploded")
                        }
                    }
                    OutputShape::Nested => return Err(TsvOutputError::NestedOutputNotAvailable)
                }
            }
        }

        writeln!(writer)?;
        Ok(())
    }

    /// Parses and writes a single output data record to the TSV file.
    ///
    /// This method handles the conversion of structured output data into TSV format,
    /// respecting the configured output shape and reformat strategy. It performs
    /// validation to ensure the data matches the expected strategy.
    ///
    /// # Arguments
    /// * `data` - The output data containing read information and reformatted values
    ///
    /// # Returns
    /// * `Ok(())` if the record was written successfully
    /// * `Err(TsvOutputError)` if:
    ///   - Data type doesn't match the configured reformat strategy
    ///   - Output shape is Nested (not supported)
    ///   - Required statistics or signal data is missing
    ///   - Array indexing fails due to inconsistent data lengths
    ///   - I/O error occurs during writing
    ///     
    /// # Precision
    ///
    /// Floating-point values (statistics, signals, dwells) are formatted with 8 decimal
    /// places for consistency and sufficient precision for downstream analysis.
    fn parse_output_data(&mut self, data: OutputData) -> Result<(), TsvOutputError> {
        let (
            read_id,
            start_index_on_alignment,
            matched_region_name,
            reformated_data 
        ) = data.into_inner();

        match reformated_data {
            ReformatedData::Stats(data_stats) => {
                // Validate that the correct reformat strategy is used
                if let ReformatStrategy::ReadWiseStats { stats } = &self.reformat_strategy {
                    let (sequence, statistics) = data_stats.into_inner()?;

                    match self.output_shape {
                        OutputShape::Melted => {
                            // Write one row per base position
                            for (base_idx, base) in sequence.iter().enumerate() {
                                write!(
                                    self.writer,
                                    "{}\t{}\t{}\t{}\t{}",
                                    read_id,
                                    start_index_on_alignment,
                                    matched_region_name,
                                    base_idx,
                                    base
                                )?;

                                // Write each statistic value for this base position
                                for stat in stats {
                                    write!(
                                        self.writer,
                                        "\t{:.8}",
                                        &statistics
                                            .get(&stat)
                                            .ok_or(TsvOutputError::KeyError(stat.clone()))?
                                            .get(base_idx)
                                            .ok_or(TsvOutputError::IndexError(base_idx, sequence.len()))?
                                    )?;
                                }
                                writeln!(self.writer)?;
                            }
                        }
                        OutputShape::Exploded => {
                            // Write common fields
                            write!(
                                self.writer,
                                "{}\t{}\t{}",
                                read_id,
                                start_index_on_alignment,
                                matched_region_name
                            )?;

                            // Write all bases in consecutive columns
                            for base in &sequence {
                                write!(
                                    self.writer,
                                    "\t{}", 
                                    *base as char
                                )?;
                            }
                            
                            // Write all statistic values in consecutive columns
                            // organized by statistic type, then by base position
                            for stat in stats {
                                for base_idx in 0..sequence.len() {
                                    write!(
                                        self.writer,
                                        "\t{:.8}",
                                        statistics
                                            .get(&stat)
                                            .ok_or(TsvOutputError::KeyError(stat.clone()))?
                                            .get(base_idx)
                                            .ok_or(TsvOutputError::IndexError(base_idx, sequence.len()))?
                                    )?;
                                }
                            }
                            writeln!(self.writer)?;
                        }
                        OutputShape::Nested => return Err(TsvOutputError::NestedOutputNotAvailable)
                    }
                } else {
                    return Err(TsvOutputError::ReformatStratMismatch(
                        "read-wise stats", 
                        "interpolation"
                    ));
                }
            }
            ReformatedData::Interp(data_interp) => {
                // Validate that the correct reformat strategy is used
                if let ReformatStrategy::Interpolation { .. } = self.reformat_strategy {
                    let (sequence, signal, dwells) = data_interp.into_inner()?;

                    match self.output_shape {
                        OutputShape::Melted => {
                            // Write one row per base position
                            for (base_idx, base) in sequence.iter().enumerate() {
                                // Write common fields and base information
                                write!(
                                    self.writer,
                                    "{}\t{}\t{}\t{}\t{}",
                                    read_id,
                                    start_index_on_alignment,
                                    matched_region_name,
                                    base_idx,
                                    *base as char
                                )?;

                                // Write all interpolated signal values for this base
                                let base_signal = signal
                                    .get(base_idx)
                                    .ok_or(TsvOutputError::IndexError(base_idx, sequence.len()))?;

                                for interp_measurement in base_signal {
                                    write!(
                                        self.writer,
                                        "\t{:.8}",
                                        interp_measurement
                                    )?;
                                }

                                // Write dwell time for this base
                                write!(
                                    self.writer,
                                    "\t{:.8}",
                                    dwells
                                        .get(base_idx)
                                        .ok_or(TsvOutputError::IndexError(base_idx, sequence.len()))?
                                )?;
                                writeln!(self.writer)?;
                            }
                        }
                        OutputShape::Exploded => {
                            // Write common fields
                            write!(
                                self.writer,
                                "{}\t{}\t{}",
                                read_id,
                                start_index_on_alignment,
                                matched_region_name
                            )?;

                            // Write all bases in consecutive columns
                            for base in &sequence {
                                write!(
                                    self.writer,
                                    "\t{}", 
                                    *base as char
                                )?;
                            }
                            
                            // Write all interpolated signal values in consecutive columns
                            // organized by base position, then by signal measurement
                            for base_idx in 0..sequence.len() {
                                let base_signal = signal
                                    .get(base_idx)
                                    .ok_or(TsvOutputError::IndexError(base_idx, sequence.len()))?;
                                for interp_measurement in base_signal {
                                    write!(
                                        self.writer,
                                        "\t{:.8}",
                                        interp_measurement
                                    )?;
                                }
                            }

                            // Write all dwell times in consecutive columns
                            for base_idx in 0..sequence.len() {
                                write!(
                                    self.writer,
                                    "\t{:.8}", 
                                    dwells
                                        .get(base_idx)
                                        .ok_or(TsvOutputError::IndexError(base_idx, sequence.len()))?
                                )?;
                            }

                            writeln!(self.writer)?;
                        }
                        OutputShape::Nested => return Err(TsvOutputError::NestedOutputNotAvailable)
                    }
                } else {
                    return Err(TsvOutputError::ReformatStratMismatch(
                        "interpolation",
                        "read-wise stats" 
                    ));
                }
            }
        }
        Ok(())
    }
}

impl ReformatWriter for OutputWriterTsv {
    /// Creates a new TSV output writer with the specified configuration.
    ///
    /// This method initializes the file, sets up buffered I/O, and writes the header row.
    /// The file will be created or overwritten based on the `force_overwrite` parameter.
    ///
    /// # Arguments
    ///
    /// * `path` - Path where the TSV file will be created
    /// * `force_overwrite` - If true, overwrites existing files; if false, returns error
    ///   if file exists
    /// * `_batch_size` - Unused parameter (TSV writing is not batched); included for
    ///   trait compatibility
    /// * `reformat_strategy` - The data processing strategy determining output columns
    /// * `output_shape` - The organizational structure (Melted or Exploded)
    /// * `uniform_roi_length` - Required for Exploded output; must be Some with the
    ///   consistent length of all regions of interest
    ///
    /// # Returns
    ///
    /// * `Ok(OutputWriterTsv)` - Successfully initialized writer with header written
    /// * `Err(OutputError)` if:
    ///   - File exists and `force_overwrite` is false
    ///   - File creation fails due to permissions or disk space
    ///   - Header writing fails
    ///   - Output shape is Nested
    ///
    /// # Performance
    ///
    /// Uses an 8 MB buffer (TSV_BUFFER_SIZE) to minimize system calls and improve
    /// write performance for large datasets.
    fn new(
        path: &PathBuf,
        force_overwrite: bool,
        _batch_size: usize,
        reformat_strategy: &ReformatStrategy,
        output_shape: &OutputShape,
        uniform_roi_length: Option<usize>
    ) -> Result<Self, OutputError> 
    where Self: Sized 
    {
        // Check if file exists and should not be overwritten
        if path.exists() && !force_overwrite {
            return Err(OutputError::FileExists(path.clone()));
        }

        // Create the file and set up buffered writing
        let file = File::create(path)?;
        let mut writer = BufWriter::with_capacity(TSV_BUFFER_SIZE, file);

        // Write the header row before returning the writer
        Self::write_header(&mut writer, output_shape, reformat_strategy, uniform_roi_length)?;
        
        Ok(Self { 
            writer, 
            output_shape: output_shape.clone(), 
            reformat_strategy: reformat_strategy.clone()
        })
    }

    /// Writes a single data record to the TSV file.
    ///
    /// Records are written to the internal buffer and may not be immediately flushed
    /// to disk until the buffer is full or `flush()` is explicitly called.
    ///
    /// # Arguments
    ///
    /// * `data` - The output data to write, containing read metadata and reformatted values
    ///
    /// # Returns
    ///
    /// * `Ok(())` if the record was successfully written to the buffer
    /// * `Err(OutputError)` if writing fails (see `parse_output_data` for error conditions)
    fn write_record(
        &mut self,
        data: OutputData
    ) -> Result<(), OutputError> {
        self.parse_output_data(data)?;
        Ok(())
    }

    /// Flushes any buffered data to disk.
    ///
    /// This method ensures that all data written to the internal buffer is physically
    /// written to the file system.
    ///
    /// # Returns
    ///
    /// * `Ok(())` if flush succeeded
    /// * `Err(OutputError)` if I/O error occurs during flush
    ///
    /// # Notes
    ///
    /// The buffer is automatically flushed when it becomes full (8 MB)
    fn flush(&mut self) -> Result<(), crate::error::execute::OutputError> {
        self.writer.flush()?;
        Ok(())
    }

    /// Finalizes the TSV file by flushing remaining buffered data.
    ///
    /// This method should be called when all data has been written to ensure the file
    /// is complete and consistent. After calling this method, the writer should not
    /// be used for further writes.
    ///
    /// # Returns
    ///
    /// * `Ok(())` if finalization succeeded
    /// * `Err(OutputError)` if final flush fails
    ///
    /// # Notes
    ///
    /// The writer's buffer will be dropped after this call, automatically closing
    /// the file handle.
    fn finalize(&mut self) -> Result<(), crate::error::execute::OutputError> {
        self.flush()?;
        Ok(())
    }
}