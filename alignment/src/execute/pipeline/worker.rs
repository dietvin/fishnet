use std::sync::Arc;

use crossbeam::channel::{Receiver, Sender};
use kmer_table::kmer_table::KmerTable;
use pod5_reader_api::read::Pod5Read;

use crate::{
    bam::read::BamRead, core::{
        alignment::AlignmentMode,
        refinement::{
            RefinementMode,
            dp::forward_step::RefinementAlgo,
            rescaling::RescaleAlgo,
            rough_rescaling::RoughRescaleAlgo
        }, 
        run_alignment
    }, execute::pipeline::helpers::{catch_channel_error, catch_error}, output::{
        buffer::Buffer,
        record::IntoOutputRecord,
        schema::OutputSchema
    }
};


/// Processes reads and accumulates results into a typed output buffer.
///
/// This function forms the core compute stage of the pipeline and is intended
/// to run on one of `n_threads` parallel worker threads. Each worker:
///
/// 1. Receives `(Pod5Read, BamRead, read_id)` tuples from `data_receiver`.
/// 2. Runs the full alignment and signal-refinement workflow via
///    [`run_alignment`] using the provided `alignment_mode` and
///    `refinement_mode`.
/// 3. Converts the result into a typed output record via
///    [`IntoOutputRecord::into_output_record`].
/// 4. Pushes the record into the local `buffer`. When the buffer reports it
///    should be flushed, the flushed batch is forwarded to the writer thread
///    via `results_sender`.
/// 5. Reports per-read success or failure on `progress_sender`.
///
/// After `data_receiver` is exhausted any remaining buffered records are
/// flushed and sent before the function returns.
///
/// # Type Parameters
/// * `A`  - Alignment mode (query-only, reference, or both).
/// * `S`  - Rough-rescaling algorithm (Least squares or Theil-Sen).
/// * `T`  - Fine-rescaling algorithm (Least squares or Theil-Sen).
/// * `U`  - Refinement/DP algorithm (Viterbi or dwell-penalty).
/// * `R`  - Refinement mode that wires `S`, `T`, `U` together.
/// * `OS` - Output schema that describes the columns/fields written.
/// * `OB` - Output buffer type; accumulates records until a flush threshold.
///
/// # Arguments
/// * `data_receiver`   - Channel from which paired reads are received.
/// * `kmer_table`      - Shared reference to the k-mer signal-level table.
/// * `alignment_mode`  - Configures which sequences (query/reference/both)
///                       are aligned to the raw signal.
/// * `refinement_mode` - Configures the rescaling and DP refinement steps.
/// * `buffer`          - Per-thread output buffer; cloned once per worker.
/// * `results_sender`  - Channel on which flushed output batches are sent to
///                       the writer thread.
/// * `progress_sender` - Channel on which per-read success/failure flags are
///                       sent to the progress thread.
pub(super) fn worker_pipeline<A, S, T, U, R, OS, OB>(
    data_receiver: Receiver<(Pod5Read, BamRead, String)>,
    kmer_table: Arc<KmerTable>,
    alignment_mode: A,
    refinement_mode: R,
    mut buffer: OB,
    results_sender: Sender<OB::FlushOutput>,
    progress_sender: Sender<bool>
)
where 
    A: AlignmentMode,
    S: RoughRescaleAlgo,
    T: RescaleAlgo,
    U: RefinementAlgo,
    R: RefinementMode<S, T, U, Input = A::Output>,
    R::Output: IntoOutputRecord<OS>,
    OS: OutputSchema,
    OB: Buffer<OS>,
{
    log::info!("Worker thread started");

    for (pod5_read, bam_read, read_id) in data_receiver {

        log::debug!("{read_id}: Starting alignment for read");

        let alignment_result = match run_alignment(
            &pod5_read,
            &bam_read,
            &kmer_table,
            &alignment_mode,
            &refinement_mode
        ) {
            Ok(v) => v,
            Err(e) => {
                log::error!("{read_id}: Failed to perform initial alignment ({e})");
                catch_channel_error(progress_sender.send(false));
                continue;
            }
        };

        let output_record = match alignment_result.into_output_record(
            pod5_read,
            bam_read
        ) {
            Ok(v) => v,
            Err(e) => {
                log::error!("{read_id}: Failed to perform refinement ({e})");
                catch_channel_error(progress_sender.send(false));
                continue;
            }
        };

        match buffer.push(output_record) {
            Ok(()) => {
                catch_channel_error(progress_sender.send(true));
            }
            Err(e) => {
                log::error!("{read_id}: Failed to push into buffer ({e})");
                catch_channel_error(progress_sender.send(false));
                continue;
            }
        }
        if buffer.should_flush() {
            let flushed_data = match buffer.flush() {
                Ok(v) => v,
                Err(e) => catch_error(e, "Failed to flush buffered data")
            };
            catch_channel_error(results_sender.send(flushed_data));
        }
    }

    let flushed_data = match buffer.flush() {
        Ok(v) => v,
        Err(e) => catch_error(e, "Failed to flush remaining buffered data")
    };
    catch_channel_error(results_sender.send(flushed_data));

}