use crossbeam::channel::Receiver;

use crate::{execute::pipeline::helpers::catch_error, output::{buffer::Buffer, schema::OutputSchema, writer::Writer}};


/// Consumes serialised output batches and writes them to the configured
/// output file.
///
/// This function runs on a dedicated writer thread and is the final stage in
/// the pipeline. It receives pre-serialised data batches (produced by
/// [`worker_pipeline`]) via `results_receiver` and delegates each batch to the
/// underlying [`Writer`] implementation (e.g. Parquet or JSON Lines).
///
/// Once `results_receiver` is closed (all worker threads have finished and
/// dropped their `results_sender` clones), the writer is finalised via
/// [`Writer::finalize`] to flush any internal buffers and close the output
/// file cleanly.
///
/// # Type Parameters
/// * `OS` - Output schema; describes the structure of the data being written.
/// * `OB` - Output buffer type; its `FlushOutput` associated type is the
///           batch format expected by `OW`.
/// * `OW` - Concrete [`Writer`] implementation (e.g. [`ParquetWriter`] or
///           [`JsonlWriter`]).
///
/// # Arguments
/// * `results_receiver` - Channel from which serialised output batches are
///                        received.
/// * `output_writer`    - The writer instance that persists batches to disk.
///
/// # Errors / Exits
/// Both write failures and finalisation failures are treated as fatal. The
/// error is logged at `ERROR` level and the process exits with code `1` via
/// [`catch_error`].
pub(super) fn writer_pipeline<OS, OB, OW>(
    results_receiver: Receiver<OW::Input>,
    mut output_writer: OW
)
where
    OS: OutputSchema,
    OB: Buffer<OS>,
    OW: Writer<OS, Input = OB::FlushOutput>
{
    for (batch_num, output_batch) in results_receiver.iter().enumerate() {
        match output_writer.write(output_batch) {
            Ok(_) => {
                log::debug!("Successfully wrote batch {batch_num}");
            }
            Err(e) => catch_error(e, &format!(
                "Failed to write batch {}", batch_num
            ))
        }
    }

    if let Err(e) = output_writer.finalize() {
        catch_error(e, "Failed to finalize the writer");
    }
}