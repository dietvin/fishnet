use indicatif::{ProgressBar, ProgressStyle};
use log::LevelFilter;
use crate::{
    cli::{
        parse::args_to_input::{
            Config, WhichToAlign
        },
        output::output_bam::BamWriter
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
    if *input.debug_level() != LevelFilter::Off {
        if let Err(e) = setup_logger(
            input.debug_path(), 
            *input.debug_level(), 
            vec![], 
            false
        ) {
            println!("Failed to initialize logger: {e}");
            std::process::exit(1);
        }
    }

    let bam_path = input.bam_input();
    let mut bam_file = match BamFileLazy::new(bam_path) {
        Ok(v) => v,
        Err(e) => {
            println!("Failed to read Bam file: {e}");
            log::error!("Failed to read Bam file: {e}");
            std::process::exit(1);
        }
    };

    let pod5_paths = input.pod5_input();
    let pod5_index = match Pod5Index::from_files(pod5_paths) {
        Ok(v) => v,
        Err(e) => {
            println!("Failed to read pod5 files: {e}");
            log::error!("Failed to read pod5 files: {e}");
            std::process::exit(1);
        }
    };

    let refine_settings = input.refine_settings();

    let kmer_table_path = input.kmer_table_input();
    let mut kmer_table = match KmerTable::new(kmer_table_path) {
        Ok(v) => v,
        Err(e) => {
            println!("Failed to load kmer table: {e}");
            log::error!("Failed to load kmer table: {e}");
            std::process::exit(1);
        }
    };

    if *refine_settings.normalize_levels() {
        if let Err(e) = kmer_table.fix_gauge() {
            println!("Failed to normalize kmer table levels: {e}");
            log::error!("Failed to normalize kmer table levels: {e}");
            std::process::exit(1);
        }
    }

    let output_path = input.output_dir();
    let mut output_writer = match BamWriter::new(output_path, bam_path, input.force_overwrite()) {
        Ok(v) => v,
        Err(e) => {
            println!("Failed to initialize the output writer: {e}");
            log::error!("Failed to initialize the output writer: {e}");
            std::process::exit(1);
        }
    };


    let total_reads = match pod5_index.num_reads() {
        Ok(v) => v,
        Err(e) => {
            println!("Failed to count number of reads: {e}");
            log::error!("Failed to count number of reads: {e}");
            std::process::exit(1);
        }
    };
    let mut n_successful_reads = 0;
    let mut n_failed_reads = 0;
    let mut progress_bar = ProgressBar::new(total_reads as u64);
    progress_bar.set_style(
        ProgressStyle::default_bar()
        .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} reads ({percent}%) | {msg}")
        .unwrap()
        .progress_chars("#>-")
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

        match output_writer.write_read(
            &mut sig_map_refiner, 
            input.alignment_type()
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