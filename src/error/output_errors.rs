use super::refinement_errors::signal_map_refiner_errors::SigMapRefineError;

#[derive(Debug, thiserror::Error)]
pub enum OutputError {
    #[error("Htslib error: {0}")]
    HtslibError(#[from] rust_htslib::errors::Error),
    #[error("SigMapRefiner error: {0}")]
    SigMapRefinerError(#[from] SigMapRefineError)
}