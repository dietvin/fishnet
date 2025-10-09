/*!
 * This module implements a single-threaded framework for processing alignments
 * between POD5 nanopore signal data and BAM alignment records.
 * 
 * ## Workflow
 * 
 * 1. Initialize logging and progress tracking
 * 2. Load BAM file containing sequence alignment data
 * 3. Index POD5 files containing raw nanopore signals
 * 4. Initialize kmer table for signal refinement
 * 5. Set up output writer for results
 * 6. Process each read:
 * - Load POD5 read data and corresponding BAM alignment
 * - Perform query-to-signal alignment
 * - Optionally perform reference-to-signal alignment
 * - Refine alignments using kmer table
 * - Write results to output file
 * 7. Finalize output and display summary statistics
 * 
 * ## Usage
 * 
 * The main entry point is `run_alignment_single_threaded()` which takes a `Config`
 * struct containing all necessary parameters for the alignment process.
 */

use console::style;
use helper::{io::OutputFormat, logger::setup_logger};
use indicatif::{ProgressBar, ProgressStyle};
use log::LevelFilter;
use pod5_reader_api::dataset::Pod5Dataset;
use crate::{
    core::{
        alignment::aligned_read::AlignedRead, 
        loader::bam::BamFileLazy, 
        refinement::{
            kmer_table::KmerTable, 
            signal_map_refiner::SigMapRefiner
        }
    }, error::AlignmentError, execute::{config::{ConfigAlign, WhichToAlign}, output::{output_arrow::OutputWriterArrow, output_data::OutputData, output_json::OutputWriterJsonl, AlignmentWriter}}
};


pub fn run_alignment_single_threaded(input: ConfigAlign) -> Result<(), AlignmentError> {
    let progress_bar_init = ProgressBar::new_spinner();
    progress_bar_init.set_style(
        ProgressStyle::default_bar()
            .tick_strings(&["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"])
            .template("{spinner} [{elapsed_precise}] {msg}")                    
            .unwrap()            
    );

    if *input.log_level() != LevelFilter::Off {
        progress_bar_init.set_message("Initializing logging...");
        setup_logger(
            input.log_path(), 
            *input.log_level(), 
            vec![], 
            false
        )?;
    }

    progress_bar_init.set_message("Loading the BAM file...");
    let bam_path: &std::path::PathBuf = input.bam_input();
    let mut bam_file = match BamFileLazy::new(bam_path) {
        Ok(v) => v,
        Err(e) => {
            log::error!("Failed to read Bam file: {e}");
            return Err(AlignmentError::BamFileError(e));
        }
    };

    progress_bar_init.set_message("Indexing the POD5 data...");
    let pod5_paths = input.pod5_input();
    let mut pod5_dataset = match Pod5Dataset::new(pod5_paths) {
        Ok(v) => v,
        Err(e) => {
            log::error!("Failed to read pod5 files: {e}");
            return Err(AlignmentError::Pod5DatasetError(e));
        }
    };


    let refine_settings = input.refine_settings();

    progress_bar_init.set_message("Initializing the kmer table...");
    let kmer_table_path = input.kmer_table_input();
    let mut kmer_table = match KmerTable::new(kmer_table_path) {
        Ok(v) => v,
        Err(e) => {
            log::error!("Failed to load kmer table: {e}");
            return Err(AlignmentError::KmerTableError(e));
        }
    };

    if *refine_settings.normalize_levels() {
        if let Err(e) = kmer_table.fix_gauge() {
            log::error!("Failed to normalize kmer table levels: {e}");
            return Err(AlignmentError::KmerTableError(e));
        }
    }

    progress_bar_init.set_message("Initializing the output writer...");

    let output_path = input.output_file();

    let output_writer_res = match input.output_format() {
        OutputFormat::Parquet => OutputWriterArrow::new(&output_path, input.force_overwrite(), input.output_batch_size(), input.output_config().clone())
            .map(|w| Box::new(w) as Box<dyn AlignmentWriter>),
        OutputFormat::Json => OutputWriterJsonl::new(&output_path, input.force_overwrite(), input.output_batch_size(), input.output_config().clone())
            .map(|w| Box::new(w) as Box<dyn AlignmentWriter>),
        _ => unreachable!("CLI restricts output formats to Parquet and JSONL")
    };
    
    let mut output_writer = match output_writer_res {
        Ok(v) => v,
        Err(e) => {
            log::error!("Failed to initialize the output writer: {e}");
            return Err(AlignmentError::OutputError(e));
        }
    };

    progress_bar_init.finish_with_message(format!("{}", style("Finished initialization. Starting alignment...").green()));

    let mut n_successful_reads = 0;
    let mut n_failed_reads = 0;
    let mut progress_bar = ProgressBar::new_spinner();
    progress_bar.set_style(
        ProgressStyle::default_spinner()
            .tick_strings(&["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"])
            .template("{spinner} [{elapsed_precise}] Processed {pos} reads | {msg}")                    
            .unwrap()            
    );


    for pod5_file in pod5_dataset.iter_files_mut() {
        let read_iterator = match pod5_file.iter_reads() {
            Ok(it) => it,
            Err(e) => {
                update_progress_fail(&mut progress_bar, &n_successful_reads, &mut n_failed_reads);
                log::error!("Failed to load pod5 read: {e}");
                continue;
            }
        };
        for pod5_read_res in read_iterator {
            let mut pod5_read = match pod5_read_res {
                Ok(v) => v,
                Err(e) => {
                    update_progress_fail(&mut progress_bar, &n_successful_reads, &mut n_failed_reads);
                    log::error!("Failed to load pod5 read: {e}");
                    continue;
                }
            };
            let read_id = pod5_read.read_id_string();

            
            log::info!("Starting alignment for read {read_id}");

            let mut bam_read = match bam_file.get(&read_id) {
                Ok(v) => v,
                Err(e) => {
                    log::error!("Failed to load bam read {read_id}: {e}");
                    update_progress_fail(&mut progress_bar, &n_successful_reads, &mut n_failed_reads);
                    continue;
                }
            };

            let mut aligned_read = match AlignedRead::new(
                &mut pod5_read, 
                &mut bam_read, 
                input.is_drna()
            ) {
                Ok(v) => v,
                Err(e) => {
                    log::error!("Failed to set up aligned read for {read_id}: {e}");
                    update_progress_fail(&mut progress_bar, &n_successful_reads, &mut n_failed_reads);
                    continue;
                }
            };

            if let Err(e) = aligned_read.align_query_to_signal() {
                log::error!("Query to sequence alignment failed for {read_id}: {e}");
                    update_progress_fail(&mut progress_bar, &n_successful_reads, &mut n_failed_reads);
                    continue;
            };

            if *input.alignment_type() == WhichToAlign::Both || *input.alignment_type() == WhichToAlign::Reference {
                if aligned_read.is_mapped() {
                    if let Err(e) = aligned_read.align_reference_to_signal() {
                        log::error!("Reference to sequence alignment failed for {read_id}: {e}");
                        update_progress_fail(&mut progress_bar, &n_successful_reads, &mut n_failed_reads);
                    continue;
                    }
                } else {
                    log::error!("Reference to sequence alignment not possible for {read_id}: Read is unmapped.");
                    update_progress_fail(&mut progress_bar, &n_successful_reads, &mut n_failed_reads);
                    continue;
                }
            }

            let mut sig_map_refiner = match SigMapRefiner::new(
                &kmer_table, 
                &mut aligned_read, 
                refine_settings
            ) {
                Ok(v) => v,
                Err(e) => {
                    log::error!("Failed to initialize signal mapping refiner for {read_id}: {e}");
                    update_progress_fail(&mut progress_bar, &n_successful_reads, &mut n_failed_reads);
                    continue;
                }
            };

            if let Err(e) = sig_map_refiner.start() {
                log::error!("Mapping refinement failed for {read_id}: {e}");
                update_progress_fail(&mut progress_bar, &n_successful_reads, &mut n_failed_reads);
                continue;
            }

            let query_to_signal = match sig_map_refiner.refined_query_to_sig_offset_adjusted() {
                Ok(v) => v,
                Err(e) => {
                    log::error!("Failed to adjust the signal offset for {read_id}: {e}");
                    update_progress_fail(&mut progress_bar, &n_successful_reads, &mut n_failed_reads);
                    continue;
                }
            };

            let ref_to_signal = match sig_map_refiner.refined_ref_to_sig_offset_adjusted() {
                Ok(v) => v,
                Err(e) => {
                    log::error!("Failed to adjust the signal offset for {read_id}: {e}");
                    update_progress_fail(&mut progress_bar, &n_successful_reads, &mut n_failed_reads);
                    continue;
                }
            };

            let output_data = OutputData::new(
                input.output_config(), 
                read_id.clone(), 
                query_to_signal, 
                ref_to_signal, 
                &sig_map_refiner
            );

            match output_writer.write_record(
                output_data
            ) {
                Ok(_) => {
                    log::info!("Successfully processed read {read_id}");
                    update_progress_success(&mut progress_bar, &mut n_successful_reads, &n_failed_reads);
                }
                Err(e) => {
                    log::error!("Failed to write alignment(s) to file for {read_id}: {e}");
                    update_progress_fail(&mut progress_bar, &n_successful_reads, &mut n_failed_reads);
                    continue;
                }
            }
        }
    }

    if let Err(e) = output_writer.finalize() {
        log::error!("Failed to write the remaining buffer to file: {e}");
        return Err(AlignmentError::OutputError(e));
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


    Ok(())
}


fn update_progress_success(progress_bar: &mut ProgressBar, n_successful_reads: &mut usize, n_failed_reads: &usize) {
    *n_successful_reads += 1;
    progress_bar.set_message(format!("{} ✓ | {} ✗", n_successful_reads, n_failed_reads));
    progress_bar.inc(1);
}

fn update_progress_fail(progress_bar: &mut ProgressBar, n_successful_reads: &usize, n_failed_reads: &mut usize) {
    *n_failed_reads += 1;
    progress_bar.set_message(format!("{} ✓ | {} ✗", n_successful_reads, n_failed_reads));
    progress_bar.inc(1);
}