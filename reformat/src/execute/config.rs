pub mod ref_region;

use std::path::PathBuf;


pub enum AlignmentType {
    Query,
    Reference
}

pub enum RegionFilter {
    Motifs {
        motifs: Vec<String>
    },
    RefRegions {
        ref_name: String,
        ref_start: usize,
        ref_end: usize
    }
}

pub enum SignalSource {
    Pod5Files {
        file_paths: Vec<PathBuf>
    },
    AlignmentTable
}

pub struct ConfigReformat {
    align_input: PathBuf,
    pod5_input: Option<Vec<PathBuf>>,
    output_file: PathBuf,

    // to determine which alignment type should be reformatted
    // if only one type is in the alignment file, this is determined automatically
    // if both are in the alignment file the user needs to set it manually
    alignment_type: AlignmentType,
    // To determine where the signal information gets parsed from.
    // If the user provides an alignment file with the signal stored 
    // within it, there is no need to provide pod5 file(s).
    signal_data: SignalSource,

    force_overwrite: bool,
    n_threads: usize,
    queue_size: usize,
    // log_level: LevelFilter,

    log_path: PathBuf,

}