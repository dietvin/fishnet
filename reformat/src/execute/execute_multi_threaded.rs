
use std::{sync::Arc, thread, time::Duration};

use console::style;
use crossbeam::channel::{bounded, SendError};
use helper::{io::OutputFormat, logger::setup_logger};
use indicatif::{ProgressBar, ProgressStyle};
use log::LevelFilter;
use pod5_reader_api::dataset::Pod5Dataset;
use uuid::Uuid;

use crate::{
    core::{
        alignment_loader::{Row, RowIterator}, 
        filter::Filter, 
        reformater::reformat
    }, 
    error::ReformatError, 
    execute::{
        config::{
            ConfigReformat, 
            OutputShape, 
            SignalSource
        }, 
        output::{
            output_arrow::OutputWriterArrow, output_data::OutputData, output_tsv::OutputWriterTsv, ReformatWriter
        }
    }
};

enum ProgressType {
    Success,
    Filter,
    Fail
}

pub(super) fn run_reformat_multi_threaded(config: ConfigReformat) -> Result<(), ReformatError> {
    // Initialize the initalization progress bar
    let progress_bar_init = ProgressBar::new_spinner();
    progress_bar_init.set_style(
        ProgressStyle::default_bar()
            .tick_strings(&["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"])
            .template("{spinner} [{elapsed_precise}] {msg}")                    
            .unwrap()            
    );
    progress_bar_init.enable_steady_tick(Duration::from_millis(100));

    // Initialize logging
    if *config.log_level() != LevelFilter::Off {
        progress_bar_init.set_message("Initializing logging...");
        setup_logger(
            config.log_path(), 
            *config.log_level(), 
            vec![], 
            false
        )?;
    }

    // Load the pod5 file(s) if needed
    let pod5_dataset = match config.signal_source() {
        SignalSource::SignalFromAlignment => None,
        SignalSource::SignalFromFiles { paths } => {
            progress_bar_init.set_message("Indexing the POD5 data...");
            log::info!("Loading pod5 dataset from paths: {:?}", paths);
            Some(
                match Pod5Dataset::new(paths) {
                    Ok(d) => d,
                    Err(e) => {
                        log::error!("Failed to initialize Pod5Dataset: {}", e);
                        return Err(ReformatError::Pod5DatasetError(e));
                    }
                }
            )
        }
    };

    // Load the regions of interest for filtering
    progress_bar_init.set_message("Initializing the read filtering...");
    log::info!("Initializing the region of interest filtering");
    let filter = Arc::new(
        Filter::from_filter_source(config.filter_source())?
    );

    // Check if all regions of interest have the same length in case of exploded output shape
    let output_exploded_all_lengths_equal = filter.equal_lengths();
    if *config.output_shape() == OutputShape::Exploded && output_exploded_all_lengths_equal.is_none() {
        log::error!("Failed to initialize region of interest filtering");
        return Err(ReformatError::ExplodedWithUnequalFilterLength);
    }

    // TODO: Change so the signal can get retrieved in parallel...
    // Initialize the alignment reader
    progress_bar_init.set_message("Initializing the alignment file iterator...");
    log::info!("Initializing the alignment file iterator");
    let alignment_iter = match RowIterator::new(
        config.alignment_input(),
        config.input_chunk_size(),
        config.columns_of_interest(),
        pod5_dataset,
        config.is_drna(),
        config.norm_signal()
    ) {
        Ok(alignment_iter) => alignment_iter,
        Err(e) => {
            log::error!("Failed to initialize the alignment file iterator: {}", e);
            return Err(ReformatError::RowIteratorError(e));
        }
    };

    // Initialize the output writer
    let mut output_writer = match config.output_format() {
        OutputFormat::Parquet => {
            let writer = match OutputWriterArrow::new(
                config.output_file(), 
                config.force_overwrite(), 
                config.output_batch_size(), 
                config.reformat_strategy(), 
                config.output_shape(), 
                output_exploded_all_lengths_equal
            ) {
                Ok(writer) => writer,
                Err(e) => {
                    log::error!("Failed to initialize the arrow output writer: {}", e);
                    return Err(ReformatError::OutputError(e));
                }
            };

            Box::new(writer) as Box<dyn ReformatWriter>
        }
        OutputFormat::Tsv => {
            let writer = match OutputWriterTsv::new(
                config.output_file(), 
                config.force_overwrite(), 
                config.output_batch_size(), 
                config.reformat_strategy(), 
                config.output_shape(), 
                output_exploded_all_lengths_equal
            ) {
                Ok(writer) => writer,
                Err(e) => {
                    log::error!("Failed to initialize the TSV output writer: {}", e);
                    return Err(ReformatError::OutputError(e));
                }
            };

            Box::new(writer) as Box<dyn ReformatWriter>
        }
        _ => unreachable!("CLI restricts output formats to Parquet and TSV")
    };

    progress_bar_init.finish_with_message(format!("{}", style("Finished initialization. Starting reformating...").green())); 
    
    // Initialize the progress bar thread 
    let (progress_sender, progress_receiver) = bounded::<ProgressType>(config.queue_size());
    let progress_handler = match thread::Builder::new()
        .name("progress".to_string())
        .spawn(move || {
            let mut n_success = 0;
            let mut n_filtered = 0;
            let mut n_failed = 0;
            let mut counter = 0;

            let progress_bar: ProgressBar = ProgressBar::new_spinner();
            progress_bar.set_style(
                ProgressStyle::default_bar()
                    .tick_strings(&["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"])
                    .template("{spinner} [{elapsed_precise}] Processed {pos} reads | {msg}")
                    .unwrap()
            );

            for progress_type in progress_receiver {
                match progress_type {
                    ProgressType::Success => n_success += 1,
                    ProgressType::Filter => n_filtered += 1,
                    ProgressType::Fail => n_failed += 1,
                }
                counter += 1;
                if counter % 100 == 0 {
                    progress_bar.set_message(
                        format!("{} Success | {} Filtered | {} Fail", 
                        n_success, 
                        n_filtered, 
                        n_failed
                    ));
                    progress_bar.inc(100);
                }
            }

            progress_bar.finish_with_message(format!(
                "{} | {} | {}",
                style(format!("{} Success", n_success)).green(),
                style(format!("{} Filtered", n_filtered)).yellow(),
                style(format!("{} Fail", n_failed)).red(),
            ));
        }) {
            Ok(handle) => handle,
            Err(e) => {
                log::error!("Failed to spawn progress thread: {e}");
                return Err(ReformatError::IoError(e));            
            }
        };

    // Initialize the writer thread 
    let (output_sender, output_receiver) = bounded::<(Uuid, OutputData)>(config.queue_size());
    let writer_handle = match thread::Builder::new()
        .name("writer".to_string())
        .spawn(move || {
            for (read_id, output_data) in output_receiver {
                match output_writer.write_record(output_data) {
                    Ok(_) => {
                        log::debug!("Successfully reformated read {}", &read_id);
                    }
                    Err(e) => {
                        log::error!("Failed to write processed read {} to file: {}", &read_id, e);
                        return Err(ReformatError::OutputError(e)); // TODO: Check if this makes sense
                    }
                }
            }

            if let Err(e) = output_writer.finalize() {
                log::error!("Failed to write the remaining buffer to file: {e}");
                return Err(ReformatError::OutputError(e));
            }

            Ok(())
        }) {
            Ok(handle) => handle,
            Err(e) => {
                log::error!("Failed to spawn writer thread: {e}");
                return Err(ReformatError::IoError(e));            
            }
        };

    let (data_sender, data_receiver) = bounded::<Row>(config.queue_size());
    let num_workers = config.n_threads();
    let mut worker_handles = Vec::with_capacity(num_workers);
    for thread_id in 0..num_workers {
        let data_rx = data_receiver.clone();
        let output_tx = output_sender.clone();
        let progress_tx = progress_sender.clone();

        let filter = Arc::clone(&filter);
        let reformat_strategy = config.reformat_strategy().clone();
        let norm_dwells = config.norm_dwells().clone();

        let handle = match thread::Builder::new()
            .name(format!("worker{thread_id}"))
            .spawn(move || {
                for row in data_rx {
                    log::debug!("Processing read {}", row.read_id());

                    match filter.hits(&row) {
                        // Overlap with at least one region of interest
                        Ok(Some(filter_hits)) => {
                            log::debug!("Read {} matches region(s): {:?}", row.read_id(), filter_hits);

                            // Reformat the data for each overlapping region of interest
                            for chunk_info in filter_hits {
                                match reformat(
                                    row.read_id(),
                                    row.sequence(),
                                    row.alignment(),
                                    row.signal(),
                                    &chunk_info,
                                    &reformat_strategy,
                                    norm_dwells
                                ) {
                                    // Reformating was successful
                                    Ok(output_data) => {
                                        log::debug!("Successfully reformated read {}", row.read_id());
                                        if let Err(e) = progress_tx.send(ProgressType::Success) {
                                            handle_channels_error(e);
                                        }
                                        if let Err(e) = output_tx.send((row.read_id().clone(), output_data)) {
                                            handle_channels_error(e);
                                        }
                                    }
                                    // Reformating failed
                                    Err(e) => {
                                        log::error!("Failed to reformat read {}: {}", row.read_id(), e);
                                        if let Err(e) = progress_tx.send(ProgressType::Fail) {
                                            handle_channels_error(e);
                                        }
                                    }
                                }            
                            }
                        }
                        // No overlap with any region of interest
                        Ok(None) => {
                            log::debug!("Read {} does not match a region of interest", row.read_id());
                            if let Err(e) = progress_tx.send(ProgressType::Filter) {
                                handle_channels_error(e);
                            }
                        }
                        // Failed to determine overlap
                        Err(e) => {
                            log::error!("Failed to check for region of interest overlap for read {}: {}", row.read_id(), e);
                            if let Err(e) = progress_tx.send(ProgressType::Fail) {
                                handle_channels_error(e);
                            }
                        }
                    }
                }
            }) {
                Ok(handle) => handle,
                Err(e) => {
                    log::error!("Failed to spawn worker thread {thread_id}: {e}");
                    return Err(ReformatError::IoError(e));
                }
            };
        worker_handles.push(handle);
    }

    drop(output_sender);

    // Initialize the producer thread
    let progress_tx = progress_sender.clone();
    let producer_handle = match thread::Builder::new()
        .name("producer".to_string())
        .spawn(move || {
            for row_res in alignment_iter {
                match row_res {
                    Ok(row) => {
                        if let Err(e) = data_sender.send(row) {
                            handle_channels_error(e);
                        }
                    }
                    Err(e) => {
                        log::error!("Failed to get row from iterator: {}", e);
                        if let Err(e) = progress_tx.send(ProgressType::Fail) {
                            handle_channels_error(e);
                        }
                    }
                }
            }

            drop(data_sender);
        }) {
            Ok(v) => v,
            Err(e) => {
                log::error!("Failed to spawn producer thread: {e}");
                return Err(ReformatError::IoError(e));
            }
        };
        
    // Join all threads
    if let Err(e) = producer_handle.join() {
        log::error!("Failed to join producer thread: {:?}", e);
        return Err(ReformatError::ThreadJoinError("producer"));
    }
    for handle in worker_handles {
        if let Err(e) = handle.join() {
            log::error!("Failed to join worker threads: {:?}", e);
            return Err(ReformatError::ThreadJoinError("worker"));
        }
    }

    drop(progress_sender);
    if let Err(e) = progress_handler.join() {
        log::error!("Failed to join progress thread: {:?}", e);
        return Err(ReformatError::ThreadJoinError("progress"));
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
        return Err(ReformatError::ThreadJoinError("writer"));
    }

    progress_bar_finishing.finish_with_message(format!("{}", style("Finished.").green()));
    Ok(())
}

fn handle_channels_error<T>(e: SendError<T>) {
    log::error!("Failed to send data: {e}");
    eprintln!("Failed to send data: {e}");
    std::process::exit(1);
}