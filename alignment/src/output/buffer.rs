use crate::{error::output::BufferError, output::{record::OutputRecord, schema::OutputSchema}};

pub mod parquet;
pub mod jsonl;

/// A generic buffering abstraction for batching `OutputRecord`s into
/// a format-specific intermediate representation.
///
/// The buffer accumulates records until a flushing condition is met
/// (e.g., memory threshold), at which point it produces a batch suitable
/// for a downstream `Writer`.
///
/// # Type Parameters
///
/// * `S` - Compile-time output schema controlling which fields are present.
///
/// # Semantics
///
/// - `push` appends a single record to the buffer.
/// - `should_flush` indicates whether the buffer reached its flush condition.
/// - `flush` materializes the buffered data into a batch and resets the buffer.
pub trait Buffer<S: OutputSchema>: Clone + Send {
    type FlushOutput: Send;

    fn push(&mut self, record: OutputRecord) -> Result<(), BufferError>;
    fn should_flush(&self) -> bool;
    fn flush(&mut self) -> Result<Self::FlushOutput, BufferError>;
}