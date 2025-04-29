pub mod core;
pub mod error;
pub mod logger;
pub mod alignment_functions;
pub mod cli;

use core::alignment::aligned_read::AlignedRead;
use core::loader::bam::BamFileLazy;
use core::loader::pod5::Pod5Index;
use core::refinement::{kmer_table::KmerTable, settings::{RefineAlgo, RefineSettings, RescaleAlgo, RoughRescaleAlgo, WhichToRefine}, signal_map_refiner::SigMapRefiner};

fn main() {
    let path: &str = "example_data/can_mappings.bam";
    let mut bam_file: BamFileLazy = BamFileLazy::new(path).unwrap();


    let path: &str = "example_data/can_reads.pod5";
    let paths: Vec<String> = vec![path.to_string()];
    let index: Pod5Index = Pod5Index::from_files(&paths).unwrap();

    let refine_settings: RefineSettings = RefineSettings::custom(
        WhichToRefine::Both, 
        RefineAlgo::default(), 
        2, 
        5, 
        2, 
        RescaleAlgo::default(),
        RoughRescaleAlgo::default_theil_sen(), 
        true
    );

    // Set up the kmer table from the provided file path
    let mut kmer_table = KmerTable::new("example_data/levels.txt").unwrap();
    if *refine_settings.normalize_levels() {
        kmer_table.fix_gauge().unwrap();
    }

    for read in index.reads() {
        let (file_path, read_id, mut pod5_read) = read.unwrap();
        println!("Processing {} found in file {}", read_id, file_path);
        let bam_read = bam_file.get(&read_id).unwrap();
        let mut aligned_read: AlignedRead<'_> = AlignedRead::new(
            &mut pod5_read, 
            &bam_read, 
            false
        ).unwrap();

        // Align query to signal
        aligned_read.align_query_to_signal().unwrap();

        // Align reference to signal
        if aligned_read.is_mapped() {
            aligned_read.align_reference_to_signal().unwrap();
        }


        // Initialize the SigMapRefiner
        let mut sig_map_refiner: SigMapRefiner<'_> = SigMapRefiner::new(
            &kmer_table, 
            &aligned_read, 
            &refine_settings
        ).unwrap();

        // Start the refinement
        sig_map_refiner.start().unwrap();

        // Retrieve the refined maps
        let refined_query_map: &Vec<usize> = sig_map_refiner.refined_query_to_sig().unwrap();
        let refined_ref_map: &Vec<usize> = sig_map_refiner.refined_ref_to_sig().unwrap();

        println!("Refine query map: {:?}", &refined_query_map[..20]);
        println!("Refine ref map: {:?}", &refined_ref_map[..20]);
    }
}