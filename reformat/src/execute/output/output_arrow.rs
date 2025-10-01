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
        output::{arrow_buffer::ArrowBuffer, output_data::OutputData, ReformatWriter}
    }
};

pub(crate) struct OutputWriterArrow {
    writer: Option<FileWriter<File>>,
    schema: Schema,
    options: WriteOptions,
    encodings: Vec<Vec<Encoding>>,
    batch_size: usize,
    output_shape: OutputShape,
    reformat_strategy: ReformatStrategy,
    uniform_roi_length: Option<usize>,

    buffer: ArrowBuffer,
    current_buffer_size: usize
}

impl OutputWriterArrow {
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

    fn finalize(&mut self) -> Result<(), OutputError> {
        self.flush()?;

        if let Some(mut writer) = self.writer.take() {
            writer.end(None)?;
        }

        Ok(())
    }
}
