use crate::{error::output::WriterError, output::schema::OutputSchema};

pub mod parquet;
pub mod jsonl;

/// A sink for writing buffered batches to an output format.
///
/// Writers consume batches produced by a `Buffer` and handle
/// serialization and I/O.
///
/// # Type Parameters
///
/// * `S` - Compile-time schema associated with the input batches.
pub trait Writer<S: OutputSchema>: Send {
    type Input;

    fn write(&mut self, batch: Self::Input) -> Result<(), WriterError>;
    fn finalize(&mut self) -> Result<(), WriterError>;
}