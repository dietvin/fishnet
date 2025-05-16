/*! 
# Multi-threaded Resquiggling

This module implements a multi-threaded framework for processing alignments
between POD5 nanopore signal data and BAM alignment records.

## Architecture

The system uses a producer-consumer architecture with four types of threads:

1. **Producer Thread**: Loads reads from POD5 files and corresponding BAM entries,
   then distributes work to worker threads through a bounded channel.

2. **Worker Threads**: Perform the actual signal-to-sequence alignment and refinement 
   for each read, then send results back through a separate channel.

3. **Main Thread**: Collects results from worker threads and writes them to the output file.

4. **Progress Thread**: Displays and updates a progress bar based on completed alignments.

## Communication Channels

The system uses three bounded crossbeam channels for thread communication:

- **Data Channel**: Transfers read data from the producer to worker threads
  (Pod5Read and BamRead objects for each read ID)
  
- **Result Channel**: Transfers alignment results from worker threads to the main thread
  (read ID and alignment vectors)
  
- **Progress Channel**: Sends success/failure signals to update the progress display

## Workflow

1. The main thread initializes resources (BAM file, POD5 index, kmer table)
2. Worker threads are spawned and wait for input data
3. The producer thread iterates through POD5 reads, finds matching BAM entries, 
   and sends them to workers
4. Workers perform alignments in parallel:
   - Create AlignedRead object
   - Align query to signal
   - Align reference to signal (if requested)
   - Run signal mapping refinement
   - Send results back to main thread
5. The main thread writes results to the output file (Parquet or JSON)
6. Progress is continuously updated through the progress bar

## Error Handling

- All threads implement comprehensive error handling
- Failures in individual reads don't stop the entire process
- The progress bar shows both successful and failed alignments
- Critical errors that prevent continuing will exit the program with error code 1
*/

use std::{sync::Arc, thread};

use console::style;
use crossbeam::channel::{bounded, SendError};
use indicatif::{ProgressBar, ProgressStyle};
use log::LevelFilter;
use crate::{
    cli::{
        output::{
            output_arrow::OutputWriterArrow, 
            output_json::OutputWriterJsonl, 
            AlignmentWriter
        }, 
        parse::args_to_input::{
            Config, 
            OutputFormat, 
            WhichToAlign
        }
    }, 
    core::{
        alignment::aligned_read::AlignedRead, 
        loader::{
            bam::{BamFileLazy, BamRead}, 
            pod5::{Pod5Index, Pod5Read}
        }, 
        refinement::{
            kmer_table::KmerTable, signal_map_refiner::SigMapRefiner
        }
    }, error::FishnetError, logger::setup_logger
};

pub fn run_alignment_multi_threaded(input: Config) -> Result<(), FishnetError> {    
    let progress_bar_init = ProgressBar::new_spinner();
    progress_bar_init.set_style(
        ProgressStyle::default_bar()
            .tick_strings(&["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"])
            .template("{spinner} [{elapsed_precise}] {msg}")                    
            .unwrap()            
    );

    if *input.log_level() != LevelFilter::Off {
        progress_bar_init.set_message("Initializing logging...");
        if let Err(e) = setup_logger(
            input.log_path(), 
            *input.log_level(), 
            vec![], 
            false
        ) {
            eprintln!("Failed to initialize logger: {e}");
            std::process::exit(1);
        }
    }

    progress_bar_init.set_message("Loading the BAM file...");
    let bam_path: &std::path::PathBuf = input.bam_input();
    let mut bam_file = match BamFileLazy::new(bam_path) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("Failed to read Bam file: {e}");
            log::error!("Failed to read Bam file: {e}");
            std::process::exit(1);
        }
    };

    progress_bar_init.set_message("Indexing the POD5 data...");
    let pod5_paths = input.pod5_input();
    let pod5_index = match Pod5Index::from_files(pod5_paths) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("Failed to read pod5 files: {e}");
            log::error!("Failed to read pod5 files: {e}");
            std::process::exit(1);
        }
    };

    let refine_settings = Arc::new(input.refine_settings().clone());

    progress_bar_init.set_message("Initializing the kmer table...");
    let kmer_table_path = input.kmer_table_input();
    let mut kmer_table = match KmerTable::new(kmer_table_path) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("Failed to load kmer table: {e}");
            log::error!("Failed to load kmer table: {e}");
            std::process::exit(1);
        }
    };

    if *refine_settings.normalize_levels() {
        if let Err(e) = kmer_table.fix_gauge() {
            eprintln!("Failed to normalize kmer table levels: {e}");
            log::error!("Failed to normalize kmer table levels: {e}");
            std::process::exit(1);
        }
    }

    let kmer_table = Arc::new(kmer_table);

    progress_bar_init.set_message("Initializing the output writer...");
    let output_dir = input.output_dir();
    let bam_stem = bam_path.file_stem().unwrap_or_else(|| {
        eprintln!("BAM file has no valid file stem");
        log::error!("BAM file has no valid file stem");
        std::process::exit(1);
    });
    let extension = match input.output_format() {
        OutputFormat::Parquet => "parquet",
        OutputFormat::Json => "jsonl"
    };
    let output_path = output_dir.join(format!("{}.{}", bam_stem.to_string_lossy(), extension));

    let output_writer_res = match input.output_format() {
        OutputFormat::Parquet => OutputWriterArrow::new(&output_path, input.force_overwrite(), input.output_batch_size())
            .map(|w| Box::new(w) as Box<dyn AlignmentWriter>),
        OutputFormat::Json => OutputWriterJsonl::new(&output_path, input.force_overwrite(), input.output_batch_size())
            .map(|w| Box::new(w) as Box<dyn AlignmentWriter>)
    };
    
    let mut output_writer = match output_writer_res {
        Ok(v) => v,
        Err(e) => {
            eprintln!("Failed to initialize the output writer: {e}");
            log::error!("Failed to initialize the output writer: {e}");
            std::process::exit(1);
        }
    };

    progress_bar_init.finish_with_message(format!("{}", style("Finished initialization. Starting alignment...").green()));

    let is_drna = input.is_drna();
    let alignment_type = input.alignment_type().clone();

    // Handles data transfer from the producer thread to the worker threads
    let (data_sender, data_receiver) = bounded::<(String, Pod5Read, BamRead)>(input.queue_size());
    // Handles data transfer from the worker threads to the main thread for output writing
    let (result_sender, result_receiver) = bounded::<(String, Option<Vec<usize>>, Option<Vec<usize>>)>(input.queue_size());
    // Handles update signals for the progress bar
    let (progress_sender, progress_receiver) = bounded::<bool>(input.queue_size());


    // Initalize the progress bar thread

    // let total_reads = match pod5_index.num_reads() {
    //     Ok(v) => v,
    //     Err(e) => {
    //         eprintln!("Failed to count number of reads: {e}");
    //         log::error!("Failed to count number of reads: {e}");
    //         std::process::exit(1);
    //     }
    // };

    let progress_handler = match thread::Builder::new()
        .name("progress".to_string())
        .spawn(move || {
            let mut n_successful_reads = 0;
            let mut n_failed_reads = 0;
            let progress_bar = ProgressBar::new_spinner();
            progress_bar.set_style(
                ProgressStyle::default_spinner()
                    .tick_strings(&["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"])
                    .template("{spinner} [{elapsed_precise}] Processed {pos} reads | {msg}")                    
                    .unwrap()            
            );

            for is_success in progress_receiver {
                if is_success {
                    n_successful_reads += 1;
                } else {
                    n_failed_reads += 1;
                }
                progress_bar.set_message(format!("{} ✓ | {} ✗", n_successful_reads, n_failed_reads));
                progress_bar.inc(1);
            }
            progress_bar.set_style(
                ProgressStyle::default_spinner()
                    .template("{spinner} [{elapsed_precise}] {msg}")
                    .unwrap()            
            );
            progress_bar.finish_with_message(format!(
                "{} | {} | {}",
                style(format!("Finished. Processed {} reads", progress_bar.position())).green(),
                style(format!("{} ✓ ", n_successful_reads)).green(),
                style(format!("{} ✗ ", n_failed_reads)).red()
            ));
        }) {
            Ok(v) => v,
            Err(e) => {
                log::error!("Failed to spawn progress thread: {e}");
                eprintln!("Failed to spawn progress thread: {e}");
                std::process::exit(1);
            }
        };


    // Initialize the worker threads

    let num_workers = input.n_threads();
    let mut worker_handles = Vec::with_capacity(num_workers);
    for thread_id in 0..num_workers {
        let data_rx = data_receiver.clone();
        let result_tx = result_sender.clone();
        let progress_tx = progress_sender.clone();

        let kmer_table = Arc::clone(&kmer_table);
        let refine_settings = Arc::clone(&refine_settings);
        let is_drna = is_drna.clone();
        let alignment_type = alignment_type.clone();

        let handle = match thread::Builder::new()
            .name(format!("worker{thread_id}"))
            .spawn(move || {
                for (read_id, mut pod5_read, mut bam_read) in data_rx {
                    let mut aligned_read = match AlignedRead::new(
                        &mut pod5_read, 
                        &mut bam_read,
                        is_drna
                    ) {
                        Ok(v) => v,
                        Err(e) => {
                            log::error!("Failed to set up aligned read for {read_id}: {e}");
                            if let Err(e) = progress_tx.send(false) {
                                handle_channels_error(e);
                            }
                            continue;
                        }
                    };

                    if let Err(e) = aligned_read.align_query_to_signal() {
                        log::error!("Query to sequence alignment failed for {read_id}: {e}");
                        if let Err(e) = progress_tx.send(false) {
                            handle_channels_error(e);
                        }
                        continue;
                    };

                    if alignment_type == WhichToAlign::Both || alignment_type == WhichToAlign::Reference {
                        if aligned_read.is_mapped() {
                            if let Err(e) = aligned_read.align_reference_to_signal() {
                                log::error!("Reference to sequence alignment failed for {read_id}: {e}");
                                if let Err(e) = progress_tx.send(false) {
                                    handle_channels_error(e);
                                }
                                continue;
                            }
                        } else {
                            log::error!("Reference to sequence alignment not possible for {read_id}: Read is unmapped.");
                            if let Err(e) = progress_tx.send(false) {
                                handle_channels_error(e);
                            }
                            continue;
                        }
                    }

                    let mut sig_map_refiner = match SigMapRefiner::new(
                        &kmer_table, 
                        &mut aligned_read, 
                        &refine_settings
                    ) {
                        Ok(v) => v,
                        Err(e) => {
                            log::error!("Failed to initialize signal mapping refiner for {read_id}: {e}");
                            if let Err(e) = progress_tx.send(false) {
                                handle_channels_error(e);
                            }
                            continue;
                        }
                    };
                    
                    if let Err(e) = sig_map_refiner.start() {
                        log::error!("Mapping refinement failed for {read_id}: {e}");
                        if let Err(e) = progress_tx.send(false) {
                            handle_channels_error(e);
                        }
                        continue;
                    }

                    let query_to_signal = sig_map_refiner.refined_query_to_sig().cloned();
                    let ref_to_signal = sig_map_refiner.refined_ref_to_sig().cloned();
                    
                    if let Err(e) = result_tx.send((read_id, query_to_signal, ref_to_signal)) {
                        handle_channels_error(e);
                    }
                }
            }) {
                Ok(v) => v,
                Err(e) => {
                    log::error!("Failed to spawn worker thread {thread_id}: {e}");
                    eprintln!("Failed to spawn worker thread {thread_id}: {e}");
                    std::process::exit(1);
                }
            };

        worker_handles.push(handle);    
    }

    drop(result_sender);


    // Initialize the producer thread

    let progress_tx = progress_sender.clone();
    let producer_handle = match thread::Builder::new()
        .name("producer".to_string())
        .spawn(move || {
            for read in pod5_index.reads() {
                let (file_path, read_id, pod5_read) = match read {
                    Ok(v) => v,
                    Err(e) => {
                        log::error!("Failed to load pod5 read: {e}");
                        if let Err(e) = progress_tx.send(false) {
                            handle_channels_error(e);
                        }
                        continue;
                    }
                };

                let bam_read = match bam_file.get(&read_id) {
                    Ok(v) => v,
                    Err(e) => {
                        log::error!("Failed to load bam read {read_id}: {e}");
                        if let Err(e) = progress_tx.send(false) {
                            handle_channels_error(e);
                        }
                        continue;
                    }
                };
                log::info!("Starting alignment for read {read_id} from file {}", file_path.display());
                
                if let Err(e) = data_sender.send((read_id, pod5_read, bam_read)) {
                    handle_channels_error(e);
                }
            }

            drop(data_sender);
        }) {
            Ok(v) => v,
            Err(e) => {
                log::error!("Failed to spawn producer thread: {e}");
                eprintln!("Failed to spawn producer thread: {e}");
                std::process::exit(1);
            }
        };


    // Write the results in the main thread

    for (read_id, query_to_signal, ref_to_signal) in result_receiver {
        match output_writer.write_record(
            &read_id, 
            query_to_signal.as_ref(), 
            ref_to_signal.as_ref()
        ) {
            Ok(_) => {
                log::info!("Successfully processed read {read_id}");
                if let Err(e) = progress_sender.send(true) {
                    handle_channels_error(e);
                }
            }
            Err(e) => {
                log::error!("Failed to write alignment(s) to file for {read_id}: {e}");
                if let Err(e) = progress_sender.send(false) {
                    handle_channels_error(e);
                }
                continue;
            }
        }
    }

    if let Err(e) = output_writer.finalize() {
        eprintln!("Failed to write the remaining buffer to file: {e}");
        std::process::exit(1);
    }


    // Join all threads

    if let Err(e) = producer_handle.join() {
        log::error!("Failed to join threads: {:?}", e);
        eprintln!("Failed to join threads: {:?}", e);
        std::process::exit(1);
    }
    for handle in worker_handles {
        if let Err(e) = handle.join() {
            log::error!("Failed to join threads: {:?}", e);
            eprintln!("Failed to join threads: {:?}", e);
            std::process::exit(1);
        }
    }

    // Drop the progress sender at the end to ensure that all updates are tracked
    drop(progress_sender);

    if let Err(e) = progress_handler.join() {
        log::error!("Failed to join threads: {:?}", e);
        eprintln!("Failed to join threads: {:?}", e);
        std::process::exit(1);
    }

    Ok(())
}

fn handle_channels_error<T>(e: SendError<T>) {
    log::error!("Failed to send data: {e}");
    eprintln!("Failed to send data: {e}");
    std::process::exit(1);
}