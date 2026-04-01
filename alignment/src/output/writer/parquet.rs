use std::{fs::File, iter::once, path::PathBuf};

use arrow2::{
    array::Array,
    chunk::Chunk,
    datatypes::{DataType, Field, Schema},
    io::parquet::write::{
        CompressionOptions,
        Encoding,
        FileWriter,
        RowGroupIterator,
        Version,
        WriteOptions
    }
};

use crate::{
    error::output::WriterError,
    output::{
        schema::OutputSchema,
        writer::Writer
    }
};


/// Parquet file writer backed by `arrow2`.
///
/// Consumes Arrow `Chunk`s and writes them as Parquet row groups
/// using a fixed schema derived from `OutputSchema`.
///
/// # Notes
///
/// - Encoding is currently fixed to `Plain`.
/// - Compression uses Snappy by default.
/// - Schema must match the structure of incoming batches.
pub struct ParquetWriter {
    writer: FileWriter<File>,
    schema: Schema,
    options: WriteOptions,
    encodings: Vec<Vec<Encoding>>
}

impl ParquetWriter {
    pub fn new<S: OutputSchema>(
        path: &PathBuf,
        force_overwrite: bool,
    ) -> Result<Self, WriterError> {
        if path.exists() && !force_overwrite {
            return Err(WriterError::FileExists(path.clone()));
        }

        let file = File::create(path)?;
        let schema = Self::create_schema::<S>();
        let options = WriteOptions {
            write_statistics: true,
            compression: CompressionOptions::Snappy,
            version: Version::V2,
            data_pagesize_limit: None
        };
        let writer = FileWriter::try_new(file, schema.clone(), options)?;
        
        let encodings = schema.fields
            .iter()
            .map(|_| vec![Encoding::Plain])
            .collect::<Vec<Vec<Encoding>>>();
    
        Ok(Self { 
            writer, 
            schema,
            options,
            encodings
        })
    }

    fn create_schema<S: OutputSchema>() -> Schema {
        let mut fields = vec![
            Field::new("read_id", DataType::Utf8, false)
        ];

        if S::HAS_QUERY_TO_SIGNAL {
            fields.push(Field::new(
                "query_to_signal",
                DataType::List(Box::new(
                    Field::new("item", DataType::UInt64, true)
                )),
                true
            ));
        }

        if S::HAS_REF_TO_SIGNAL {
            fields.push(Field::new(
                "ref_to_signal",
                DataType::List(Box::new(
                    Field::new("item", DataType::UInt64, true)
                )),
                true
            ));
        }

        if S::HAS_REF_META {
            fields.push(Field::new(
                "ref_name",
                DataType::Utf8,
            false
            ));
            fields.push(Field::new(
                "ref_start",
                DataType::UInt64,
            false
            ));
        }

        if S::HAS_QUERY_SEQ {
            fields.push(Field::new(
                "query_sequence",
                DataType::Utf8,
                true
            ));
        }

        if S::HAS_REF_SEQ {
            fields.push(Field::new(
                "ref_sequence",
                DataType::Utf8,
                true
            ));
        }

        if S::HAS_SIGNAL {
            fields.push(Field::new(
                "signal", 
                DataType::List(Box::new(
                    Field::new("item", DataType::Int16, true)
                )),
                true
            ));
        }

        Schema::from(fields)
    }
}

impl<S: OutputSchema> Writer<S> for ParquetWriter {
    type Input = Chunk<Box<dyn Array>>;

    fn write(&mut self, batch: Self::Input) -> Result<(), WriterError> {
        let row_group_iter = RowGroupIterator::try_new(
            once(Ok(batch)),
            &self.schema,
            self.options.clone(),
            self.encodings.clone()
        )?;         

        for group_res in row_group_iter {
            let group = group_res?;
            self.writer.write(group)?;
        }

        Ok(())
    }

    fn finalize(&mut self) -> Result<(), WriterError> {
        self.writer.end(None)?;
        Ok(())
    }
}