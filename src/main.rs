pub mod core;
pub mod error;
pub mod logger;
pub mod alignment_functions;
pub mod cli;

use core::alignment::aligned_read::AlignedRead;
use core::loader::bam::BamFileLazy;
use core::loader::pod5::Pod5Index;
use core::refinement::{kmer_table::KmerTable, settings::{RefineAlgo, RefineSettings, RescaleAlgo, RoughRescaleAlgo, WhichToRefine}, signal_map_refiner::SigMapRefiner};

use cli::execute_input::execute;


fn main() {
    execute();
}