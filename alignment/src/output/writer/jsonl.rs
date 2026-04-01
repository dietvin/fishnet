use std::{fs::File, io::{BufWriter, Write}, path::PathBuf};

use crate::{
    error::output::WriterError,
    output::{schema::OutputSchema, writer::Writer}
};

/// Writes JSONL byte batches to a file.
/// 
/// Wraps a [`BufWriter`] to coalesce syscalls. Intended to receive
/// flushed buffers from a [`JsonlBuffer`].
pub struct JsonlWriter {
    writer: BufWriter<File>
}

impl JsonlWriter {
    /// Opens a file at `path` for writing.
    /// 
    /// # Arguments
    /// * `path` - Destination file path.
    /// * `force_overwrite` - If `false`, returns [`WriterError::FileExists`] when
    ///   the file already exists rather than silently overwriting it.
    pub fn new(
        path: &PathBuf,
        force_overwrite: bool
    ) -> Result<Self, WriterError> {
        if path.exists() && !force_overwrite {
            return Err(WriterError::FileExists(path.clone()));
        }

        let file = File::create(path)?;
        Ok(Self { writer: BufWriter::new(file) })
    }
}

impl<S: OutputSchema> Writer<S> for JsonlWriter {
    type Input = Vec<u8>;

    /// Writes a batch of JSONL bytes to the file.
    fn write(&mut self, batch: Self::Input) -> Result<(), WriterError> {
        self.writer.write_all(&batch)?;
        Ok(())
    }

    /// Flushes the underlying [`BufWriter`], ensuring all bytes are written to disk.
    /// Should be called once after all records have been written.
    fn finalize(&mut self) -> Result<(), WriterError> {
        self.writer.flush()?;
        Ok(())
    }
}