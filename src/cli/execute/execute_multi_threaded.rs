use indicatif::{ProgressBar, ProgressStyle};
use crate::{
    cli::{
        args_to_input::{
            Config, WhichToAlign
        },
        handle_output::BamWriter
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

pub fn run_alignment_multi_threaded(input: Config) -> Result<(), FishnetError> {
    Ok(())
}
