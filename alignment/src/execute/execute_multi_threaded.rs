/*! 
 * This module implements a multi-threaded framework for processing alignments
 * between POD5 nanopore signal data and BAM alignment records.
 * 
 * ## Architecture
 * 
 * The system uses a producer-consumer architecture with four types of threads:
 * 
 * 1. **Producer Thread**: Loads reads from POD5 files and corresponding BAM entries,
 *    then distributes work to worker threads through a bounded channel.
 * 
 * 2. **Worker Threads**: Perform the actual signal-to-sequence alignment and refinement 
 *    for each read, then send results back through a separate channel.
 * 
 * 3. **Main Thread**: Collects results from worker threads and writes them to the output file.
 * 
 * 4. **Progress Thread**: Displays and updates a progress bar based on completed alignments.
 * 
 * ## Communication Channels
 * 
 * The system uses three bounded crossbeam channels for thread communication:
 * 
 * - **Data Channel**: Transfers read data from the producer to worker threads
 *   (Pod5Read and BamRead objects for each read ID)
 *   
 * - **Result Channel**: Transfers alignment results from worker threads to the main thread
 *   (read ID and output data)
 *   
 * - **Progress Channel**: Sends success/failure signals to update the progress display
 * 
 * ## Workflow
 * 
 * 1. The main thread initializes resources (BAM file, POD5 index, kmer table)
 * 2. Worker threads are spawned and wait for input data
 * 3. The producer thread iterates through POD5 reads, finds matching BAM entries, 
 *    and sends them to workers
 * 4. Workers perform alignments in parallel:
 *    - Create AlignedRead object
 *    - Align query to signal
 *    - Align reference to signal (if requested)
 *    - Run signal mapping refinement
 *    - Send results back to main thread
 * 5. The main thread writes results to the output file (Parquet or JSON)
 * 6. Progress is continuously updated through the progress bar
 * 
 * ## Error Handling
 * 
 * - All threads implement comprehensive error handling
 * - Failures in individual reads don't stop the entire process
 * - The progress bar shows both successful and failed alignments
 * - Critical errors that prevent continuing will exit the program with error code 1
 */

use std::{sync::Arc, thread, time::Duration};

use console::style;
use crossbeam::channel::{bounded, SendError};
use indicatif::{ProgressBar, ProgressStyle};
use log::LevelFilter;
use pod5_reader_api::{
    dataset::Pod5Dataset,
    read::Pod5Read
};
use crate::{
    core::{
        alignment::aligned_read::AlignedRead, 
        loader::bam::{BamFileLazy, BamRead}, 
        refinement::{
            kmer_table::KmerTable, signal_map_refiner::SigMapRefiner
        }
    }, error::FishnetError, execute::{config::{ConfigAlign, OutputFormat, WhichToAlign}, output::{
        output_arrow::OutputWriterArrow, output_data::OutputData, output_json::OutputWriterJsonl, AlignmentWriter
    }}, logger::setup_logger
};

/// Perform the signal to sequence alignment concurrently with a given configuration 
/// set by the user through the command line interface.
pub fn run_alignment_multi_threaded(input: ConfigAlign) -> Result<(), FishnetError> {    
    // Initialize the progress bar for the initialization steps
    let progress_bar_init = ProgressBar::new_spinner();
    progress_bar_init.set_style(
        ProgressStyle::default_bar()
            .tick_strings(&["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"])
            .template("{spinner} [{elapsed_precise}] {msg}")                    
            .unwrap()            
    );
    progress_bar_init.enable_steady_tick(Duration::from_millis(100));

    // Initialize the logger
    if *input.log_level() != LevelFilter::Off {
        progress_bar_init.set_message("Initializing logging...");
        if let Err(e) = setup_logger(
            input.log_path(), 
            *input.log_level(), 
            vec![], 
            false
        ) {
            eprintln!(
                "Failed to initialize logger: {}",
                format!("{}", style(e).red())
            );
            std::process::exit(1);
        }
    }

    // Initialize and load the BAM file
    progress_bar_init.set_message("Loading the BAM file...");
    let bam_path: &std::path::PathBuf = input.bam_input();
    let mut bam_file = match BamFileLazy::new(bam_path) {
        Ok(v) => v,
        Err(e) => {
            log::error!("Failed to read Bam file: {e}");
            eprintln!(
                "Failed to read Bam file: {}",
                format!("{}", style(e).red())
            );
            std::process::exit(1);
        }
    };

    // Initialize and load the POD5 file
    progress_bar_init.set_message("Indexing the POD5 data...");
    let pod5_paths = input.pod5_input();
    let mut pod5_dataset = match Pod5Dataset::new(&pod5_paths) {
        Ok(v) => v,
        Err(e) => {
            log::error!("Failed to read pod5 files: {e}");
            eprintln!(
                "Failed to read pod5 files: {}",
                format!("{}", style(e).red())
            );
            std::process::exit(1);
        }
    };

    // Initialze the refinement settings from the input settings
    let refine_settings = Arc::new(input.refine_settings().clone());

    // Initialize the kmer table
    progress_bar_init.set_message("Initializing the kmer table...");
    let kmer_table_path = input.kmer_table_input();
    let mut kmer_table = match KmerTable::new(kmer_table_path) {
        Ok(v) => v,
        Err(e) => {
            log::error!("Failed to load kmer table: {e}");
            eprintln!(
                "Failed to load kmer table: {}",
                format!("{}", style(e).red())
            );
            std::process::exit(1);
        }
    };

    if *refine_settings.normalize_levels() {
        if let Err(e) = kmer_table.fix_gauge() {
            log::error!("Failed to normalize kmer table levels: {e}");
            eprintln!(
                "Failed to normalize kmer table levels: {}",
                format!("{}", style(e).red())
            );
            std::process::exit(1);
        }
    }

    let kmer_table = Arc::new(kmer_table);

    // Initialize the output writer
    progress_bar_init.set_message("Initializing the output writer...");
    let output_path = input.output_file();
    let output_writer_res = match input.output_format() {
        OutputFormat::Parquet => OutputWriterArrow::new(
            &output_path, 
            input.force_overwrite(), 
            input.output_batch_size(), 
            input.output_config()
            .clone()
        ).map(|w| Box::new(w) as Box<dyn AlignmentWriter>),
        OutputFormat::Json => OutputWriterJsonl::new(
            &output_path, 
            input.force_overwrite(), 
            input.output_batch_size(), 
            input.output_config()
            .clone()
        ).map(|w| Box::new(w) as Box<dyn AlignmentWriter>)
    };
    
    let mut output_writer = match output_writer_res {
        Ok(v) => v,
        Err(e) => {
            log::error!("Failed to initialize the output writer: {e}");
            eprintln!(
                "Failed to initialize the output writer: {}",
                format!("{}", style(e).red())
            );
            std::process::exit(1);
        }
    };

    progress_bar_init.finish_with_message(format!("{}", style("Finished initialization. Starting alignment...").green()));

    let is_drna = input.is_drna();
    let alignment_type = input.alignment_type().clone();
    let output_config = input.output_config().clone();

    // Handles data transfer from the producer thread to the worker threads
    let (data_sender, data_receiver) = bounded::<(String, Pod5Read, BamRead)>(input.queue_size());
    // Handles data transfer from the worker threads to the main thread for output writing
    let (result_sender, result_receiver) = bounded::<(String, OutputData)>(input.queue_size());
    // Handles update signals for the progress bar
    let (progress_sender, progress_receiver) = bounded::<bool>(input.queue_size());


    // Initalize the progress bar thread
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
            progress_bar.enable_steady_tick(Duration::from_millis(100));

            let mut counter = 0;
            for is_success in progress_receiver {
                if is_success {
                    n_successful_reads += 1;
                } else {
                    n_failed_reads += 1;
                }

                counter += 1;
                if counter % 100 == 0 {
                    progress_bar.set_message(format!("{} ✓ | {} ✗", n_successful_reads, n_failed_reads));
                    progress_bar.inc(100);
                }
            }
            progress_bar.set_style(
                ProgressStyle::default_spinner()
                    .template("{spinner} [{elapsed_precise}] {msg}")
                    .unwrap()            
            );
            progress_bar.finish_with_message(format!(
                "{} | {} | {}",
                style(format!("Finished processing {} reads", progress_bar.position())).green(),
                style(format!("{} ✓ ", n_successful_reads)).green(),
                style(format!("{} ✗ ", n_failed_reads)).red()
            ));
        }) {
            Ok(v) => v,
            Err(e) => {
                log::error!("Failed to spawn progress thread: {e}");
                eprintln!(
                    "Failed to spawn progress thread: {}",
                    format!("{}", style(e).red())
                );
                std::process::exit(1);
            }
        };

    // Initialize the writer thread
    let writer_handle = match thread::Builder::new()
        .name("writer".to_string())
        .spawn(move || {
            for (read_id, output_data) in result_receiver {
                match output_writer.write_record(
                    output_data
                ) {
                    Ok(_) => {
                        log::info!("Successfully processed read {read_id}");
                    }
                    Err(e) => {
                        log::error!("Failed to write alignment(s) to file for {read_id}: {e}");
                    }
                }
            }

            if let Err(e) = output_writer.finalize() {
                log::error!("Failed to write the remaining buffer to file: {e}");
                eprintln!(
                    "Failed to write the remaining buffer to file: {}",
                    format!("{}", style(e).red())
                );
                std::process::exit(1);
            }
        }) {
            Ok(v) => v,
            Err(e) => {
                log::error!("Failed to spawn writer thread: {e}");
                eprintln!(
                    "Failed to spawn writer thread: {}",
                    format!("{}", style(e).red())
                );
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

        let output_config = output_config.clone();

        let handle = match thread::Builder::new()
            .name(format!("worker{thread_id}"))
            .spawn(move || {
                for (read_id, mut pod5_read, mut bam_read) in data_rx {
                    // Initialize the aligned read
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

                    // Perform the query to signal alignment (always done)
                    if let Err(e) = aligned_read.align_query_to_signal() {
                        log::error!("Query to sequence alignment failed for {read_id}: {e}");
                        if let Err(e) = progress_tx.send(false) {
                            handle_channels_error(e);
                        }
                        continue;
                    };

                    // If specified by the user, perform the reference to signal alignment
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

                    // Initialize the signal mapping refiner
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
                    
                    // Start the refinement process
                    if let Err(e) = sig_map_refiner.start() {
                        log::error!("Mapping refinement failed for {read_id}: {e}");
                        if let Err(e) = progress_tx.send(false) {
                            handle_channels_error(e);
                        }
                        continue;
                    }

                    // Extract the refined query to signal alignment (only if needed)
                    let query_to_signal = match &alignment_type {
                        WhichToAlign::Reference => None,
                        WhichToAlign::Query | WhichToAlign::Both => {
                            match sig_map_refiner.refined_query_to_sig_offset_adjusted() {
                                Ok(v) => v,
                                Err(e) => {
                                    log::error!("Failed to adjust the signal offset for {read_id}: {e}");
                                    if let Err(e) = progress_tx.send(false) {
                                        handle_channels_error(e);
                                    }
                                    continue;
                                }
                            }
                        }
                    };

                    // Extract the refined reference to signal alignment (only if needed)
                    let ref_to_signal = match &alignment_type {
                        WhichToAlign::Query => None,
                        WhichToAlign::Reference | WhichToAlign::Both => {
                            match sig_map_refiner.refined_ref_to_sig_offset_adjusted() {
                                Ok(v) => v,
                                Err(e) => {
                                    log::error!("Failed to adjust the signal offset for {read_id}: {e}");
                                    if let Err(e) = progress_tx.send(false) {
                                        handle_channels_error(e);
                                    }
                                    continue;
                                }
                            }
                        }
                    };            
                    
                    let output_data = OutputData::new(
                        &output_config,
                        read_id.clone(), 
                        query_to_signal, 
                        ref_to_signal, 
                        &sig_map_refiner
                    );

                    // Send the output data to the main thread for actually writing it to file
                    if let Err(e) = result_tx.send((read_id, output_data)) {
                        handle_channels_error(e);
                    }

                    // If everything worked for the read, send a success update to the progress thread
                    if let Err(e) = progress_tx.send(true) {
                        handle_channels_error(e);
                    }
                }
            }) {
                Ok(v) => v,
                Err(e) => {
                    log::error!("Failed to spawn worker thread {thread_id}: {e}");
                    eprintln!(
                        "Failed to spawn worker thread {thread_id}: {}",
                        format!("{}", style(e).red())
                    );
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
            for pod5_file in pod5_dataset.iter_files_mut() {
                let read_iterator = match pod5_file.iter_reads() {
                    Ok(it) => it,
                    Err(e) => {
                        log::error!("Failed to load pod5 read: {e}");
                        if let Err(e) = progress_tx.send(false) {
                            handle_channels_error(e);
                        }
                        continue;
                    }
                };
                for pod5_read_res in read_iterator {
                    let pod5_read = match pod5_read_res {
                        Ok(v) => v,
                        Err(e) => {
                            log::error!("Failed to load pod5 read: {e}");
                            if let Err(e) = progress_tx.send(false) {
                                handle_channels_error(e);
                            }
                            continue;
                        }
                    };
                    let read_id = pod5_read.read_id_string();

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
                    log::info!("Starting alignment for read {read_id}");
                    
                    if let Err(e) = data_sender.send((read_id, pod5_read, bam_read)) {
                        handle_channels_error(e);
                    }
                }
            }

            drop(data_sender);
        }) {
            Ok(v) => v,
            Err(e) => {
                log::error!("Failed to spawn producer thread: {e}");
                eprintln!(
                    "Failed to spawn producer thread: {}",
                    format!("{}", style(e).red())
                );
                std::process::exit(1);
            }
        };

    // Join all threads
    if let Err(e) = producer_handle.join() {
        log::error!("Failed to join producer thread: {:?}", e);
        eprintln!("Failed to join producer thread: {:?}", e);
        std::process::exit(1);
    }
    for handle in worker_handles {
        if let Err(e) = handle.join() {
            log::error!("Failed to join worker threads: {:?}", e);
            eprintln!("Failed to join worker threads: {:?}", e);
            std::process::exit(1);
        }
    }

    // Drop the progress sender at the end to ensure that all updates are tracked
    drop(progress_sender);

    if let Err(e) = progress_handler.join() {
        log::error!("Failed to join progress thread: {:?}", e);
        eprintln!("Failed to join progress thread: {:?}", e);
        std::process::exit(1);
    }

    let progress_bar_finishing = ProgressBar::new_spinner();
    progress_bar_finishing.set_style(
        ProgressStyle::default_bar()
            .tick_strings(&["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"])
            .template("{spinner} [{elapsed_precise}] {msg}")                    
            .unwrap()            
    );
    progress_bar_finishing.enable_steady_tick(Duration::from_millis(100));
    progress_bar_finishing.set_message("Writing remaining data...");

    if let Err(e) = writer_handle.join() {
        log::error!("Failed to join writer thread: {:?}", e);
        eprintln!("Failed to join writer thread: {:?}", e);
        std::process::exit(1);
    }

    progress_bar_finishing.finish_with_message(format!("{}", style("Finished.").green()));

    Ok(())
}

fn handle_channels_error<T>(e: SendError<T>) {
    log::error!("Failed to send data: {e}");
    eprintln!("Failed to send data: {e}");
    std::process::exit(1);
}