
use console::style;
use helper::{io::OutputFormat, logger::setup_logger};
use indicatif::{ProgressBar, ProgressStyle};
use log::LevelFilter;
use pod5_reader_api::dataset::Pod5Dataset;

use crate::{core::loader::alignment::RowIterator, error::ReformatError, execute::config::{ConfigReformat, SignalSource}};

pub(super) fn run_reformat_single_threaded(config: ConfigReformat) -> Result<(), ReformatError> {
    let progress_bar_init = ProgressBar::new_spinner();
    progress_bar_init.set_style(
        ProgressStyle::default_bar()
            .tick_strings(&["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"])
            .template("{spinner} [{elapsed_precise}] {msg}")                    
            .unwrap()            
    );


    if *config.log_level() != LevelFilter::Off {
        progress_bar_init.set_message("Initializing logging...");
        if let Err(e) = setup_logger(
            config.log_path(), 
            *config.log_level(), 
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

    let pod5_dataset = match config.signal_source() {
        SignalSource::SignalFromAlignment => None,
        SignalSource::SignalFromFiles { paths } => {
            progress_bar_init.set_message("Indexing the POD5 data...");
            Some(
                Pod5Dataset::new(paths)?
            )
        }
    };

    progress_bar_init.set_message("Initializing the alignment file iterator...");
    let alignment_iter = RowIterator::new(
        config.alignment_input(),
        config.input_chunk_size(),
        config.columns_of_interest(),
        pod5_dataset
    )?;


    let _output_writer = match config.output_format() {
        OutputFormat::Parquet => {}
        OutputFormat::Tsv => {}
        _ => unreachable!("CLI restricts output formats to Parquet and TSV")
    };

    let progress_bar: ProgressBar = ProgressBar::new_spinner();
    progress_bar.set_style(
        ProgressStyle::default_bar()
            .tick_strings(&["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"])
            .template("{spinner} [{elapsed_precise}] Processed {pos} chunks")
            .unwrap()
    );

    for row_res in alignment_iter {
        let row = row_res?;
        let _ = row.read_id();
        progress_bar.inc(1);
    }

    progress_bar.finish_with_message("Finished processing");

    Ok(())
}