/*!
 * This module contains custom error types for different aspects of the signal-to-sequence
 * alignment process.
 */

use alignment_errors::AlignmentCoreError;
use loader_errors::LoaderError;
use refinement_errors::RefinementError;

pub mod loader_errors;
pub mod alignment_errors;
pub mod refinement_errors;
pub mod cli_errors;
pub mod output_errors;

/// Top level error that handles all custom sub-types
#[derive(Debug, thiserror::Error)]
pub enum AlignmentError {
    #[error("Loader error: {0}")]
    LoaderError(#[from] LoaderError),
    #[error("Alignment error: {0}")]
    AlignmentError(#[from] AlignmentCoreError),
    #[error("Refinement error: {0}")]
    RefinementError(#[from] RefinementError),
    #[error("Failed to set up the logger")]
    LogInitError
}