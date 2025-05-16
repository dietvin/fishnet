use console::style;
use indicatif::{ProgressBar, ProgressStyle};
use log::LevelFilter;
use crate::{
    cli::{
        output::{
            output_arrow::OutputWriterArrow,
            output_json::OutputWriterJsonl, AlignmentWriter
        }, parse::args_to_input::{
            Config, OutputFormat, WhichToAlign
        }
    }, 
    core::{
        alignment::aligned_read::AlignedRead, 
        loader::{
            bam::BamFileLazy, 
            pod5::Pod5Index
        }, 
        refinement::{
            kmer_table::KmerTable, signal_map_refiner::SigMapRefiner
        }
    }, 
    error::FishnetError, 
    logger::setup_logger
};


pub fn run_alignment_single_threaded(input: Config) -> Result<(), FishnetError> {
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


    let refine_settings = input.refine_settings();

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

    progress_bar_init.set_message("Initializing the output writer...");
    let output_dir = input.output_dir();
    let bam_stem = bam_path.file_stem().unwrap_or_else(|| {
        eprintln!("BAM file has no valid file stem.");
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
            .map(|w| Box::new(w) as Box<dyn AlignmentWriter>),
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

    let mut n_successful_reads = 0;
    let mut n_failed_reads = 0;
    let mut progress_bar = ProgressBar::new_spinner();
    progress_bar.set_style(
        ProgressStyle::default_spinner()
            .tick_strings(&["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"])
            .template("{spinner} [{elapsed_precise}] Processed {pos} reads | {msg}")                    
            .unwrap()            
    );

    for read in pod5_index.reads() {
        let (file_path, read_id, mut pod5_read) = match read {
            Ok(v) => v,
            Err(e) => {
                log::error!("Failed to load pod5 read: {e}");
                update_progress_fail(&mut progress_bar, &n_successful_reads, &mut n_failed_reads);
                continue;
            }
        };
        log::info!("Starting alignment for read {read_id} from file {}", file_path.display());

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

        match output_writer.write_record(
            &read_id, 
            sig_map_refiner.refined_query_to_sig(), 
            sig_map_refiner.refined_ref_to_sig()
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

    if let Err(e) = output_writer.finalize() {
        eprintln!("Failed to write the remaining buffer to file: {e}");
        std::process::exit(1);
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