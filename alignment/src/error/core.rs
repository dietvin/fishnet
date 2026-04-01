use crate::error::{core::{
    alignment::AlignmentError,
    refinement::RefinementError
}, output::OutputRecordError};

pub mod alignment;
pub mod refinement;


#[derive(Debug, thiserror::Error)]
pub enum AlignmentCoreError {
    #[error("Alignment error: {0}")]
    AlignementError(#[from] AlignmentError),
    #[error("Refinement error: {0}")]
    RefinementError(#[from] RefinementError),
    #[error("OutputRecord error: {0}")]
    OutputRecordError(#[from] OutputRecordError)
}