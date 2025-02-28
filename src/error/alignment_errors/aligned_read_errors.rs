use super::super::loader_errors::{pod5_errors::Pod5ReadError, bam_errors::BamReadError};
use super::query_to_signal_errors::QueryToSignalError;
use super::reference_to_signal_errors::RefToSignalError;

#[derive(Debug, thiserror::Error)]
pub enum AlignedReadError {
    #[error("ID mismatch: {0} (pod5) vs {1} (bam)")]
    IdMismatch(String, String),
    #[error("Pod5Read error: {0}")]
    Pod5Error(#[from] Pod5ReadError),
    #[error("Query to signal alignment failed: {0}")]
    QueryAlignmentError(#[from] QueryToSignalError),
    #[error("Reference to signal alignment failed: {0}")]
    RefAlignmentError(#[from] RefToSignalError),
    #[error("Read is unmapped")]
    Unmapped,
    #[error("No query to signal alignment found.")]
    RefBeforeQuery,
    #[error("Failed to get BAM data: {0}")]
    RetrieveBamError(#[from] BamReadError)
}