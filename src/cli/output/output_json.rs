use std::{fs::File, io::{BufWriter, Write}, path::PathBuf};
use crate::error::output_errors::OutputError;

use super::AlignmentWriter;

pub struct OutputWriterJsonl {
    writer: Option<BufWriter<File>>,
    batch_size: usize,

    buffer: Vec<serde_json::Value>
}

impl AlignmentWriter for OutputWriterJsonl {
    fn new(
        path: &PathBuf, 
        force_overwrite: bool, 
        batch_size: usize
    ) -> Result<Self, crate::error::output_errors::OutputError> {
        if path.exists() && !force_overwrite {
            return Err(OutputError::FileExists(path.clone()));
        }
        
        let file = std::fs::File::create(path)?;
        let writer = std::io::BufWriter::new(file);
        
        Ok(OutputWriterJsonl {
            writer: Some(writer),
            batch_size,
            buffer: Vec::with_capacity(batch_size),
        })
    }

    fn write_record(
        &mut self,
        read_id: &str,
        query_to_signal: Option<&Vec<usize>>,
        ref_to_signal: Option<&Vec<usize>>
    ) -> Result<(), OutputError> {
        if self.writer.is_none() {
            return Err(OutputError::AlreadyFinalized);
        }

        let record = serde_json::json!({
            "read_id": read_id,
            "query_to_signal": query_to_signal,
            "ref_to_signal": ref_to_signal,
        });

        self.buffer.push(record);

        if self.buffer.len() >= self.batch_size {
            self.flush()?;
        }

        Ok(())
    }

    fn flush(&mut self) -> Result<(), OutputError> {
        match &mut self.writer {
            None => Err(OutputError::AlreadyFinalized),
            Some(writer) => {
                if self.buffer.is_empty() {
                    return Ok(());
                }
        
                for record in &self.buffer {
                    let json_string = serde_json::to_string(record)?;
                    writeln!(writer, "{}", json_string)?;
                }
                
                writer.flush()?;
                self.buffer.clear();
                
                Ok(())
            }
        } 
    }

    fn finalize(&mut self) -> Result<(), OutputError> {
        self.flush()?;

        if let Some(mut writer) = self.writer.take() {
            writer.flush()?;
        }

        Ok(())
    }
}