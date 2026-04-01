use std::{sync::Arc, time::Duration};

use console::style;
use helper::logger::setup_logger;
use indicatif::{ProgressBar, ProgressStyle};
use kmer_table::kmer_table::KmerTable;
use log::LevelFilter;
use pod5_reader_api::dataset::Pod5Dataset;

use crate::{bam::file::BamFileLazy, execute::{config::Config, pipeline::{helpers::catch_error, initialize::load_kmer_table::load_kmer_table}}};

mod load_kmer_table;


/// Initialises all data sources required before the processing pipeline can
/// start: the logger, the BAM file, the POD5 dataset, and the k-mer level
/// table.
///
/// A spinner progress bar is shown for each initialisation step so the user
/// receives feedback during potentially slow I/O operations (e.g. indexing
/// many POD5 files).
///
/// # Returns
/// A tuple of:
/// * [`Pod5Dataset`]        - An indexed handle over all provided POD5 files.
/// * [`BamFileLazy`]        - A lazily-loaded BAM file.
/// * [`Arc<KmerTable>`]     - A reference-counted k-mer signal-level table,
///                            either loaded from the path given in `config` or
///                            extracted from the BAM file header.
///
/// # Errors / Exits
/// Any initialisation failure is treated as fatal: the error is logged, printed
/// to `stderr`, and the process exits with code `1` via [`catch_error`].
pub(super) fn load_data_pipeline(config: &Config) -> (Pod5Dataset, BamFileLazy, Arc<KmerTable>) {
    let progress_bar_init = ProgressBar::new_spinner();
    progress_bar_init.set_style(
        ProgressStyle::default_bar()
            .tick_strings(&["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"])
            .template("{spinner} [{elapsed_precise}] {msg}")                    
            .unwrap()            
    );
    progress_bar_init.enable_steady_tick(Duration::from_millis(100));

    // Initialize the logger

    if config.log_config.level != LevelFilter::Off {
        progress_bar_init.set_message("Initializing logging...");
        match setup_logger(
            &config.log_config.path, 
            config.log_config.level,
            vec![], 
            false
        ) {
            Ok(_) => {
                log::info!(
                    "Successfully initialized logger with level {}. Writing to {}",
                    config.log_config.level, config.log_config.path.display()
                );
            }
            Err(e) => catch_error(e, "Failed to initialize logger")
        }
    }

    // Initialize and load the BAM file

    progress_bar_init.set_message("Loading the BAM file...");
    let bam_file = match BamFileLazy::new(&config.bam_path) {
        Ok(v) => {
            log::info!(
                "Successfully initialized BAM file {}",
                config.bam_path.display()
            );
            v
        }
        Err(e) => catch_error(e, &format!(
            "Failed to read BAM file {}",
            config.bam_path.display()
        ))
    };

    // Initialize and load the POD5 file

    progress_bar_init.set_message("Indexing the POD5 data...");
    let pod5_dataset = match Pod5Dataset::new(&config.pod5_paths) {
        Ok(v) => {
            log::info!(
                "Successfully initialized POD5 data from {} files",
                config.pod5_paths.len()
            );
            v
        },
        Err(e) => catch_error(e, &format!(
            "Failed to initialize Pod5Dataset for {} POD5 files",
            config.pod5_paths.len()
        ))
    };

    // Initialize the kmer table

    let kmer_table = match &config.kmer_table_config {
        Some(kmer_table_config) => {
            match KmerTable::from_file(
                &kmer_table_config.path,
                kmer_table_config.normalize_levels
            ) {
                Ok(v) => Arc::new(v),
                Err(e) => catch_error(e, &format!(
                    "Failed to read kmer table from file '{}'",
                    &kmer_table_config.path.display()
                ))
            }
        }
        None => {
            match load_kmer_table(bam_file.header()) {
                Ok(v) => Arc::new(v),
                Err(e) => {
                    log::error!("Failed to load embedded kmer table: {e}. Retry with manually providing a kmer table via the `--kmer-table` flag");
                    eprintln!(
                        "{}: {}",
                        style("Warning").yellow().bold(),
                        style(format!("Failed to load an embedded kmer table ({e}). Please provide a file via the `--kmer-table` flag.")).yellow(),
                    );
                    std::process::exit(1);
                }
            }
        }
    };

    (pod5_dataset, bam_file, kmer_table)
}