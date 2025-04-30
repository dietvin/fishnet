use crate::{cli::args_to_input::WhichToAlign, core::{alignment::aligned_read::AlignedRead, loader::{bam::BamFileLazy, pod5::Pod5Index}, refinement::{kmer_table::KmerTable, signal_map_refiner::SigMapRefiner}}, error::FishnetError, logger::setup_logger};

use super::{args_to_input::Config, init_cli::parse_command_line};

pub fn execute() {
    let command_line_input = parse_command_line();

    let input_data = match Config::from_argmatches(command_line_input) {
        Ok(input) => input,
        Err(e) => {
            println!("Failed to parse input data: {e}");
            std::process::exit(1);
        }
    };

    match run_alignment_single_threaded(input_data) {
        Ok(_) => println!("Finished sucessfully"),
        Err(e) => {
            println!("Failed to perform alignment: {e}");
            std::process::exit(1);
        }
    }
}


pub fn run_alignment_single_threaded(input: Config) -> Result<(), FishnetError> {
    if let Err(e) = setup_logger(
        input.debug_path(), 
        *input.debug_level(), 
        vec![], 
        false
    ) {
        println!("Failed to initialize logger: {e}");
        std::process::exit(1);
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

    for read in pod5_index.reads() {
        let (file_path, read_id, mut pod5_read) = match read {
            Ok(v) => v,
            Err(e) => {
                println!("Failed to load pod5 read: {e}");
                log::error!("Failed to load pod5 read: {e}");
                continue;
            }
        };
        log::info!("Starting alignment for read {read_id} from file {}", file_path.display());

        let bam_read = match bam_file.get(&read_id) {
            Ok(v) => v,
            Err(e) => {
                println!("Failed to load bam read {read_id}: {e}");
                log::error!("Failed to load bam read {read_id}: {e}");
                continue;
            }
        };

        let mut aligned_read = match AlignedRead::new(
            &mut pod5_read, 
            &bam_read, 
            input.is_drna()
        ) {
            Ok(v) => v,
            Err(e) => {
                println!("Failed to set up aligned read for {read_id}: {e}");
                log::error!("Failed to set up aligned read for {read_id}: {e}");
                continue;
            }
        };

        if let Err(e) = aligned_read.align_query_to_signal() {
            println!("Query to sequence alignment failed for {read_id}: {e}");
            log::error!("Query to sequence alignment failed for {read_id}: {e}");
            continue;
        };

        if *input.alignment_type() == WhichToAlign::Both || *input.alignment_type() == WhichToAlign::Reference {
            if aligned_read.is_mapped() {
                if let Err(e) = aligned_read.align_reference_to_signal() {
                    println!("Reference to sequence alignment failed for {read_id}: {e}");
                    log::error!("Reference to sequence alignment failed for {read_id}: {e}");
                    continue;
                }
            } else {
                println!("Reference to sequence alignment not possible for {read_id}: Read is unmapped.");
                log::error!("Reference to sequence alignment not possible for {read_id}: Read is unmapped.");
                continue;
            }
        }

        let mut sig_map_refiner = match SigMapRefiner::new(
            &kmer_table, 
            &aligned_read, 
            refine_settings
        ) {
            Ok(v) => v,
            Err(e) => {
                println!("Failed to initialize signal mapping refiner for {read_id}: {e}");
                log::error!("Failed to initialize signal mapping refiner for {read_id}: {e}");
                continue;
            }
        };

        if let Err(e) = sig_map_refiner.start() {
            println!("Mapping refinement failed for {read_id}: {e}");
            log::error!("Mapping refinement failed for {read_id}: {e}");
        }

        let refined_query_map = match sig_map_refiner.refined_query_to_sig() {
            Ok(v) => v,
            Err(e) => {
                println!("Failed to retrieve refined query map for {read_id}: {e}");
                log::error!("Failed to retrieve refined query map for {read_id}: {e}");
                continue;
            }
        };
        println!("Refine query map: {:?}", &refined_query_map[..20]);

        if *input.alignment_type() == WhichToAlign::Both || *input.alignment_type() == WhichToAlign::Reference {
            let refined_ref_map = match sig_map_refiner.refined_ref_to_sig() {
                Ok(v) => v,
                Err(e) => {
                    println!("Failed to retrieve refined reference map for {read_id}: {e}");
                    log::error!("Failed to retrieve refined reference map for {read_id}: {e}");
                    continue;
                }
            };
            println!("Refine ref map: {:?}", &refined_ref_map[..20]);
        }

        log::info!("Successfully processed read {read_id}");
    }

    Ok(())
}