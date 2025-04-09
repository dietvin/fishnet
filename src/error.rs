use alignment_errors::AlignmentError;
use loader_errors::LoaderError;
use refinement_errors::RefinementError;

pub mod loader_errors;
pub mod alignment_errors;
pub mod refinement_errors;

/// Top level error that handles all custom sub-types
#[derive(Debug, thiserror::Error)]
pub enum FishnetError {
    #[error("Loader error: {0}")]
    LoaderError(#[from] LoaderError),
    #[error("Alignment error: {0}")]
    AlignmentError(#[from] AlignmentError),
    #[error("Refinement error: {0}")]
    RefinementError(#[from] RefinementError),
    #[error("Failed to set up the logger")]
    LogInitError
}