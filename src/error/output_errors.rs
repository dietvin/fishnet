use std::path::PathBuf;

use super::refinement_errors::signal_map_refiner_errors::SigMapRefineError;

#[derive(Debug, thiserror::Error)]
pub enum OutputError {
    #[error("Htslib error: {0}")]
    HtslibError(#[from] rust_htslib::errors::Error),
    #[error("SigMapRefiner error: {0}")]
    SigMapRefinerError(#[from] SigMapRefineError),
    #[error("Output file exists and force overwrite is disabled: {0}")]
    OutFileExists(PathBuf),
    #[error("Invalid source path: {0}")]
    InvalidSourcePath(PathBuf)
}