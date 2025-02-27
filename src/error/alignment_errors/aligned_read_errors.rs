use super::super::loader_errors::pod5_errors::Pod5ReadError;
use super::query_to_signal_errors::QueryToSignalError;

#[derive(Debug, thiserror::Error)]
pub enum AlignedReadError {
    #[error("ID mismatch: {0} (pod5) vs {1} (bam)")]
    IdMismatch(String, String),
    #[error("Pod5Read error: {0}")]
    Pod5Error(#[from] Pod5ReadError),
    #[error("Query to signal alignment failed: {0}")]
    QueryAlgnmentError(#[from] QueryToSignalError)
}