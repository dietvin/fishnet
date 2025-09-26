
use std::time::Duration;

use console::style;
use helper::{io::OutputFormat, logger::setup_logger};
use indicatif::{ProgressBar, ProgressStyle};
use log::LevelFilter;
use pod5_reader_api::dataset::Pod5Dataset;

use crate::{
    core::{
        alignment_loader::RowIterator, 
        filter::Filter, 
        reformater::reformat
    }, 
    error::ReformatError, 
    execute::config::{
        ConfigReformat, 
        OutputShape, 
        SignalSource
    }
};

pub(super) fn run_reformat_single_threaded(config: ConfigReformat) -> Result<(), ReformatError> {
    let progress_bar_init = ProgressBar::new_spinner();
    progress_bar_init.set_style(
        ProgressStyle::default_bar()
            .tick_strings(&["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"])
            .template("{spinner} [{elapsed_precise}] {msg}")                    
            .unwrap()            
    );
    progress_bar_init.enable_steady_tick(Duration::from_millis(100));

    if *config.log_level() != LevelFilter::Off {
        progress_bar_init.set_message("Initializing logging...");
        setup_logger(
            config.log_path(), 
            *config.log_level(), 
            vec![], 
            false
        )?;
    }

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

    progress_bar_init.set_message("Initializing the read filtering...");
    log::info!("Initializing the region of interest filtering");
    let filter = Filter::from_filter_source(config.filter_source())?;
    if *config.output_shape() == OutputShape::Exploded {
        if let Err(e) = filter.require_all_lengths_equal() {
            log::error!("Failed to initialize region of interest filtering");
            return Err(ReformatError::FilterError(e));
        }
    }

    progress_bar_init.set_message("Initializing the alignment file iterator...");
    log::info!("Initializing the alignment file iterator");
    let alignment_iter = match RowIterator::new(
        config.alignment_input(),
        config.input_chunk_size(),
        config.columns_of_interest(),
        pod5_dataset,
        config.is_drna()
    ) {
        Ok(alignment_iter) => alignment_iter,
        Err(e) => {
            log::error!("Failed to initialize the alignment file iterator: {}", e);
            return Err(ReformatError::RowIteratorError(e));
        }
    };


    let _output_writer = match config.output_format() {
        OutputFormat::Parquet => {}
        OutputFormat::Tsv => {}
        _ => unreachable!("CLI restricts output formats to Parquet and TSV")
    };

    progress_bar_init.finish_with_message(format!("{}", style("Finished initialization. Starting reformating...").green()));

    let mut n_successful_reads = 0;
    let mut n_filtered_reads = 0;
    let mut n_failed_reads = 0;
    let mut progress_bar: ProgressBar = ProgressBar::new_spinner();
    progress_bar.set_style(
        ProgressStyle::default_bar()
            .tick_strings(&["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"])
            .template("{spinner} [{elapsed_precise}] Processed {pos} reads | {msg}")
            .unwrap()
    );

    for row_res in alignment_iter {
        let row = match row_res {
            Ok(row) => row,
            Err(e) => {
                log::error!("Failed to get row: {}", e);
                continue;
            }
        };
        
        log::debug!("Processing read {}", row.read_id());

        if let Some(filter_hits) = filter.hits(&row)? {
            log::debug!("Read {} matches region(s): {:?}", row.read_id(), filter_hits);

            for chunk_info in filter_hits {
                match reformat(
                    row.read_id(),
                    row.ref_region(),
                    row.sequence(),
                    row.alignment(),
                    row.signal(),
                    &chunk_info,
                    config.reformat_strategy()
                ) {
                    Ok(output_row) => {
                        // Add output row to output handler
                        log::debug!("Successfully reformated read {}", row.read_id());
                        update_progress_success(&mut progress_bar, &mut n_successful_reads, &n_filtered_reads, &n_failed_reads);
                    }
                    Err(e) => {
                        log::error!("Failed to reformat read {}: {}", row.read_id(), e);
                        update_progress_fail(&mut progress_bar, &n_successful_reads, &n_filtered_reads, &mut n_failed_reads);
                    }
                }            
            }
        } else {
            log::debug!("Read {} does not match a region of interest", row.read_id());
            update_progress_filter(&mut progress_bar, &n_successful_reads, &mut n_filtered_reads, &n_failed_reads);
        }
    }

    log::info!(
        "Finished processing. Successful for {} reads, filtered out {} reads, failed for {} reads", 
        n_successful_reads, n_filtered_reads, n_failed_reads
    );
    progress_bar.finish_with_message(format!(
        "{} | {} | {}",
        style(format!("{} Success", n_successful_reads)).green(),
        style(format!("{} Filtered", n_filtered_reads)).yellow(),
        style(format!("{} Fail", n_failed_reads)).red(),
    ));

    Ok(())
}

fn update_progress_success(progress_bar: &mut ProgressBar, n_successful_reads: &mut usize, n_filtered_reads: &usize, n_failed_reads: &usize) {
    *n_successful_reads += 1;
    progress_bar.set_message(format!("{} Success | {} Filtered | {} Fail", n_successful_reads, n_filtered_reads, n_failed_reads));
    progress_bar.inc(1);
}

fn update_progress_filter(progress_bar: &mut ProgressBar, n_successful_reads: & usize, n_filtered_reads: &mut usize, n_failed_reads: &usize) {
    *n_filtered_reads += 1;
    progress_bar.set_message(format!("{} Success | {} Filtered | {} Fail", n_successful_reads, n_filtered_reads, n_failed_reads));
    progress_bar.inc(1);
}

fn update_progress_fail(progress_bar: &mut ProgressBar, n_successful_reads: &usize, n_filtered_reads: & usize, n_failed_reads: &mut usize) {
    *n_failed_reads += 1;
    progress_bar.set_message(format!("{} Success | {} Filtered | {} Fail", n_successful_reads, n_filtered_reads, n_failed_reads));
    progress_bar.inc(1);
}
