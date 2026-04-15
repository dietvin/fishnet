use std::{sync::Arc, thread, time::Duration};

use console::style;
use crossbeam::channel::bounded;
use indicatif::{ProgressBar, ProgressStyle};
use pod5_reader_api::read::Pod5Read;

use crate::{
    bam::read::BamRead,
    core::{
        alignment::AlignmentMode, refinement::{
            RefinementMode, 
            dp::forward_step::RefinementAlgo,
            rescaling::RescaleAlgo, 
            rough_rescaling::RoughRescaleAlgo
        }
    },
    execute::{
        config::Config,
        pipeline::{
            helpers::{catch_thread_join_error, catch_thread_spawn_error},
            initialize::load_data_pipeline,
            producer::producer_pipeline,
            progress::progress_pipeline,
            worker::worker_pipeline,
            writer::writer_pipeline
        }
    },
    output::{
        buffer::Buffer,
        record::IntoOutputRecord,
        schema::OutputSchema,
        writer::Writer
    }
};

mod initialize;
mod progress;
mod producer;
mod worker;
mod writer;
mod helpers;


/// Assembles and runs the full multi-threaded signal-alignment pipeline.
///
/// This is the single concrete entry point reached after all compile-time type
/// dispatch has been resolved. It wires together the concurrent threads:
///
/// ```text
///
///       |
///       |
///       ▼
///  ┌──────────┐  (Pod5Read, BamRead, read_id)  ┌──────────┐   OB::FlushOutput    ┌──────────┐
///  │ producer │ ─────────────────────────────> │ workerxN │ ───────────────────> │  writer  │
///  └──────────┘     data_channel               └──────────┘   results_channel    └──────────┘
///       |                                            |
///       |                                            |
///       |            ┌──────────┐                    |
///        ──────────> │ progress │ <──────────────────
///       bool (fail)  └──────────┘     bool (success/fail)
/// ```
///
/// **Thread roles**
///
/// | Thread      | Function               | Count        |
/// |-------------|------------------------|--------------|
/// | `producer`  | [`producer_pipeline`]  | 1            |
/// | `worker`    | [`worker_pipeline`]    | `n_threads`  |
/// | `progress`  | [`progress_pipeline`]  | 1            |
/// | `writer`    | [`writer_pipeline`]    | 1            |
///
/// **Shutdown sequence**
///
/// 1. The producer thread finishes iterating over all POD5 reads and drops
///    `data_sender`, closing the data channel.
/// 2. Each worker thread drains its local buffer, sends remaining records, and
///    returns. When the last worker drops its `results_sender` clone, the
///    results channel closes.
/// 3. The writer thread drains the results channel, finalises the output file,
///    and returns.
/// 4. The main thread joins producer, workers, and progress (in that order),
///    then waits for the writer while showing a "Writing remaining data…"
///    spinner.
///
/// # Type Parameters
/// * `A`  - [`AlignmentMode`]: query-only, reference, or both.
/// * `S`  - [`RoughRescaleAlgo`]: coarse rescaling algorithm.
/// * `T`  - [`RescaleAlgo`]: fine rescaling algorithm.
/// * `U`  - [`RefinementAlgo`]: DP segmentation algorithm (Viterbi or
///           dwell-penalty).
/// * `R`  - [`RefinementMode`]: wires `S`, `T`, `U` together and consumes
///           `A::Output`.
/// * `OS` - [`OutputSchema`]: describes the output columns/fields.
/// * `OB` - [`Buffer<OS>`]: per-worker accumulation buffer; cloned once per
///           worker thread.
/// * `OW` - [`Writer<OS>`]: serialises flushed buffer batches to disk.
///
/// # Arguments
/// * `config`          - Fully validated pipeline configuration.
/// * `alignment_mode`  - Concrete alignment mode instance (cloned per worker).
/// * `refinement_mode` - Concrete refinement mode instance (cloned per
///                       worker).
/// * `output_buffer`   - Buffer prototype cloned for each worker thread.
/// * `output_writer`   - Output writer; moved into the dedicated writer thread.
///
/// # Panics / Exits
/// Thread spawn or join failures and channel send errors are all treated as
/// fatal via the helpers in [`super::helpers`]; the process exits with code
/// `1`.
pub(super) fn start_pipeline<A, S, T, U, R, OS, OB, OW>(
    config: Config,
    alignment_mode: A,
    refinement_mode: R,
    output_buffer: OB,
    output_writer: OW,
)
where 
    A: AlignmentMode + 'static,
    S: RoughRescaleAlgo,
    T: RescaleAlgo,
    U: RefinementAlgo,
    R: RefinementMode<S, T, U, Input = A::Output> + 'static,
    R::Output: IntoOutputRecord<OS>,
    OS: OutputSchema,
    OB: Buffer<OS> + 'static,
    OB::FlushOutput: 'static,
    OW: Writer<OS, Input = OB::FlushOutput> + 'static
{
    // Load the required data
    let (pod5_dataset, bam_file, kmer_table) = load_data_pipeline(&config);

    // Initialize the data queues
    let (data_sender, data_receiver) = bounded::<(Pod5Read, BamRead, String)>(config.queue_size);
    let (results_sender, results_receiver) = bounded::<OB::FlushOutput>(config.queue_size);
    let (progress_sender, progress_receiver) = bounded::<bool>(config.queue_size);

    // Initialize the progress thread
    let progress_handle = catch_thread_spawn_error(thread::Builder::new()
        .name("progress".to_string())
        .spawn(move || progress_pipeline(progress_receiver))
    );
    
    // Initialize the writer thread
    let writer_handle = catch_thread_spawn_error(thread::Builder::new()
        .name("writer".to_string())
        .spawn(move || writer_pipeline::<OS, OB, OW>(results_receiver, output_writer))
    );

    // Initialize the worker threads
    let mut worker_handles = Vec::with_capacity(config.n_threads);
    for thread_id in 0..config.n_threads {
        let data_rx = data_receiver.clone();
        let results_tx = results_sender.clone();
        let progress_tx = progress_sender.clone();
        let kmer_table = Arc::clone(&kmer_table);
        let alignment_mode_c = alignment_mode.clone();
        let refinement_mode_c = refinement_mode.clone();
        let outpu_buffer_c = output_buffer.clone();

        let handle = catch_thread_spawn_error(thread::Builder::new()
            .name(format!("worker{thread_id}"))
            .spawn(move || worker_pipeline(
                data_rx,
                kmer_table,
                alignment_mode_c,
                refinement_mode_c,
                outpu_buffer_c,
                results_tx,
                progress_tx
            ))
        );
        worker_handles.push(handle);   
    }
    drop(results_sender);

    // Initialize the producer thread
    let producer_handle = catch_thread_spawn_error(thread::Builder::new()
        .name("producer".to_string())
        .spawn(move || producer_pipeline(
            pod5_dataset,
            bam_file,
            data_sender,
            progress_sender
        ))
    );

    // Join producer, worker and progress threads
    catch_thread_join_error(producer_handle.join(), "producer");
    log::info!("Joined producer thread");

    let mut i = 0;
    for handle in worker_handles {
        catch_thread_join_error(handle.join(), &format!("worker{i}"));
        log::info!("Joined worker{i}");
        i += 1;
    }
    catch_thread_join_error(progress_handle.join(), "progress");
    log::info!("Joined progress thread");

    // Join the writer thread with an additional progress bar 
    // in case it takes a bit longer to write remaining data
    let progress_bar_finishing = ProgressBar::new_spinner();
    progress_bar_finishing.set_style(
        ProgressStyle::default_bar()
            .tick_strings(&["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"])
            .template("{spinner} [{elapsed_precise}] {msg}")                    
            .unwrap()            
    );
    progress_bar_finishing.enable_steady_tick(Duration::from_millis(100));
    progress_bar_finishing.set_message("Writing remaining data...");

    catch_thread_join_error(writer_handle.join(), "writer");
    log::info!("Joined writer thread");

    progress_bar_finishing.finish_with_message(format!("{}", style("Finished.").green()));

    log::info!("*********** Finished processing ***********");
}