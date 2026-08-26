use std::mem::replace;

use serde::Serialize;

use crate::{
    error::output::BufferError,
    output::{buffer::Buffer, record::OutputRecord, schema::OutputSchema}
};

/// A buffer that serializes [`OutputRecord`]s to JSONL format in memory.
/// 
/// Records are serialized and accumulated in an internal byte buffer.
/// Once the buffer exceeds `flush_threshold_bytes`, it should be flushed
/// and handed off to a [`Writer`].
#[derive(Clone)]
pub struct JsonlBuffer {
    buffer: Vec<u8>,
    flush_threshold_bytes: usize
}

impl JsonlBuffer {
    /// Creates a new `JsonlBuffer`.
    /// 
    /// # Arguments
    /// * `flush_threshold_bytes` - Buffer size in bytes at which flushing is recommended.
    ///   A sensible default is `12_000_000` (12 MB). The internal buffer is allocated
    ///   with 2 MB of headroom beyond this threshold to avoid reallocations mid-batch.
    pub fn new(
        flush_threshold_bytes: usize
    ) -> Self {
        let buffer = Vec::with_capacity(
            flush_threshold_bytes + 2_000_000
        );

        Self { 
            buffer,
            flush_threshold_bytes
        }
    }
}

/// Helper struct for serializing [`OutputRecord`]s to JSON.
/// Fields set to `None` are omitted from the output entirely.
#[derive(Serialize)]
struct OutputRecordSer {
    read_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    query_to_sig: Option<Vec<usize>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    query_shift: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    query_scale: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    ref_to_sig: Option<Vec<usize>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    ref_shift: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    ref_scale: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    ref_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    ref_start: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    query_seq: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    ref_seq: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    signal: Option<Vec<f32>>
}

impl<S: OutputSchema> Buffer<S> for JsonlBuffer {
    type FlushOutput = Vec<u8>;

    /// Serializes a record to JSON and appends it to the internal buffer
    /// followed by a newline, in accordance with the JSONL format.
    /// Fields not present in the schema `S` are omitted from the output.
    fn push(&mut self, record: OutputRecord) -> Result<(), BufferError> {
        let record_ser = OutputRecordSer {
            read_id:        record.read_id,
            query_to_sig:   if S::HAS_QUERY_TO_SIGNAL { record.query_to_sig } else { None },
            query_shift:    if S::HAS_QUERY_TO_SIGNAL { record.query_shift } else { None },
            query_scale:    if S::HAS_QUERY_TO_SIGNAL { record.query_scale } else { None },
            ref_to_sig:     if S::HAS_REF_TO_SIGNAL { record.ref_to_sig } else { None },
            ref_shift:      if S::HAS_REF_TO_SIGNAL { record.ref_shift } else { None },
            ref_scale:      if S::HAS_REF_TO_SIGNAL { record.ref_scale } else { None },
            ref_name:       if S::HAS_REF_META { record.ref_name } else { None },
            ref_start:      if S::HAS_REF_META { record.ref_start } else { None },
            query_seq:      if S::HAS_QUERY_SEQ { record.query_seq } else { None },
            ref_seq:        if S::HAS_REF_SEQ { record.ref_seq } else { None },
            signal:         if S::HAS_SIGNAL { record.signal } else { None },
        };

        serde_json::to_writer(&mut self.buffer, &record_ser)?;
        self.buffer.push(b'\n');

        Ok(())
    }

    /// Returns `true` if the buffer has reached or exceeded `flush_threshold_bytes`.
    fn should_flush(&self) -> bool {
        self.buffer.len() >= self.flush_threshold_bytes
    }

    /// Returns the internal buffer and replaces it with a fresh allocation.
    /// The returned bytes are valid JSONL and can be passed directly to a [`Writer`].
    fn flush(&mut self) -> Result<Self::FlushOutput, BufferError> {
        let buffer = replace(
            &mut self.buffer,
            Vec::with_capacity(self.flush_threshold_bytes + 2_000_000)
        );

        Ok(buffer)
    }
}