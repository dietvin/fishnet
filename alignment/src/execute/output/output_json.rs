/*!
 * This module provides functionality for writing alignment data to JSONL (JSON Lines)
 * files with efficient batching and buffering. It implements the `AlignmentWriter` trait
 * to output alignments in a text-based format that is both human-readable and machine-
 * parseable.
 *
 * ## Features
 *
 * - **Batched Writing**: Buffers JSON records in memory before writing to disk
 *   for improved I/O performance and reduced system call overhead
 * - **JSONL Format**: Outputs data in JSON Lines format where each line contains
 *   a complete JSON object representing one alignment record
 * - **Buffered I/O**: Uses `BufWriter` for efficient file writing operations
 * - **Structured Data**: Each record contains read ID and optional alignment arrays
 *   in a well-defined JSON schema
 * - **Optional Data Handling**: Properly serializes missing alignment data as JSON null
 * - **Error Handling**: Comprehensive error handling for file operations and JSON serialization
 *
 * ## Output Format
 *
 * Each line in the output file contains a JSON object with the following structure:
 * ```json
 * {
 *   "read_id": "read_001",
 *   "query_to_signal": [1, 2, 3, 4, 5],
 *   "ref_to_signal": [10, 20, 30, 40, 50]
 * }
 * ```
 *
 * For unmapped reads, alignment arrays may be `null`:
 * ```json
 * {
 *   "read_id": "read_002",
 *   "query_to_signal": [1, 2, 3],
 *   "ref_to_signal": null
 * }
 * ```
 *
 * ## Usage Pattern
 *
 * ```ignore
 * // Create writer with batch size of 1000
 * let mut writer = OutputWriterJsonl::new(&output_path, true, 1000)?;
 *
 * // Write alignment records
 * writer.write_record("read_001", Some(&query_alignment), Some(&ref_alignment))?;
 * writer.write_record("read_002", Some(&query_alignment), None)?; // Unmapped read
 *
 * // Finalize to flush remaining data and close file
 * writer.finalize()?;
 * ```
 *
 * ## Performance Considerations
 *
 * - Larger batch sizes reduce I/O overhead but increase memory usage
 * - JSONL format enables streaming processing of large datasets
 * - Each line is independently parseable, making it suitable for distributed processing
 * - Human-readable format facilitates debugging and manual inspection
 */

use std::{fs::File, io::{BufWriter, Write}, path::PathBuf};
use crate::{error::output_errors::OutputError, execute::{config::output_config::OutputConfig, output::output_data::OutputData}};

use super::AlignmentWriter;

pub struct OutputWriterJsonl {
    writer: Option<BufWriter<File>>,
    batch_size: usize,
    output_config: OutputConfig,
    buffer: Vec<serde_json::Value>
}

impl AlignmentWriter for OutputWriterJsonl {
    fn new(
        path: &PathBuf, 
        force_overwrite: bool, 
        batch_size: usize,
        output_config: OutputConfig
    ) -> Result<Self, crate::error::output_errors::OutputError> {
        if path.exists() && !force_overwrite {
            return Err(OutputError::FileExists(path.clone()));
        }
        
        let file = std::fs::File::create(path)?;
        let writer = std::io::BufWriter::new(file);
        
        Ok(OutputWriterJsonl {
            writer: Some(writer),
            batch_size,
            output_config,
            buffer: Vec::with_capacity(batch_size),
        })
    }

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

        let record = match data {
            OutputData::QueryBasic { read_id, query_to_signal } => {
                serde_json::json!({
                    "read_id": read_id,
                    "query_to_signal": query_to_signal,
                })
            }
            OutputData::RefBasic { read_id, ref_to_signal, ref_name, ref_start } => {
                serde_json::json!({
                    "read_id": read_id,
                    "ref_to_signal": ref_to_signal,
                    "ref_name": ref_name,
                    "ref_start": ref_start,
                })
            }
            OutputData::BothBasic { read_id, query_to_signal, ref_to_signal, ref_name, ref_start } => {
                serde_json::json!({
                    "read_id": read_id,
                    "query_to_signal": query_to_signal,
                    "ref_to_signal": ref_to_signal,
                    "ref_name": ref_name,
                    "ref_start": ref_start,
                })
            }
            OutputData::QueryWithSeq { read_id, query_to_signal, query_sequence } => {
                serde_json::json!({
                    "read_id": read_id,
                    "query_to_signal": query_to_signal,
                    "query_sequence": query_sequence,
                })
            }
            OutputData::RefWithSeq { read_id, ref_to_signal, ref_sequence, ref_name, ref_start } => {
                serde_json::json!({
                    "read_id": read_id,
                    "ref_to_signal": ref_to_signal,
                    "ref_sequence": ref_sequence,
                    "ref_name": ref_name,
                    "ref_start": ref_start,
                })
            }
            OutputData::BothWithSeq { read_id, query_to_signal, query_sequence, ref_to_signal, ref_sequence, ref_name, ref_start } => {
                serde_json::json!({
                    "read_id": read_id,
                    "query_to_signal": query_to_signal,
                    "query_sequence": query_sequence,
                    "ref_to_signal": ref_to_signal,
                    "ref_sequence": ref_sequence,
                    "ref_name": ref_name,
                    "ref_start": ref_start,
                })
            }
            OutputData::QueryWithSeqAndSig { read_id, query_to_signal, query_sequence, signal } => {
                serde_json::json!({
                    "read_id": read_id,
                    "query_to_signal": query_to_signal,
                    "query_sequence": query_sequence,
                    "signal": signal,
                })
            }
            OutputData::RefWithSeqAndSig { read_id, ref_to_signal, ref_sequence, ref_name, ref_start, signal } => {
                serde_json::json!({
                    "read_id": read_id,
                    "ref_to_signal": ref_to_signal,
                    "ref_sequence": ref_sequence,
                    "ref_name": ref_name,
                    "ref_start": ref_start,
                    "signal": signal,
                })
            }
            OutputData::BothWithSeqAndSig { read_id, query_to_signal, query_sequence, ref_to_signal, ref_sequence, ref_name, ref_start, signal } => {
                serde_json::json!({
                    "read_id": read_id,
                    "query_to_signal": query_to_signal,
                    "query_sequence": query_sequence,
                    "ref_to_signal": ref_to_signal,
                    "ref_sequence": ref_sequence,
                    "ref_name": ref_name,
                    "ref_start": ref_start,
                    "signal": signal,
                })
            }
        };
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