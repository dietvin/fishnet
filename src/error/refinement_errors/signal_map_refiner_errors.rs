use super::kmer_table_errors::KmerTableError;
use super::super::alignment_errors::aligned_read_errors::AlignedReadError;
#[derive(Debug, thiserror::Error)]
pub enum SigMapRefineError {
    #[error("Failed to initialize the kmer table: {0}")]
    KmerTableError(#[from] KmerTableError),
    #[error("Failed to calculate scaling factors: {0}")]
    RescalingError(#[from] RescaleError),
    #[error("Query-to-signal alignment not present")]
    QueryToSigNotFound,
    #[error("Reference-to-signal alignment not present")]
    RefToSigNotFound,
    #[error("AlignedRead error: {0}")]
    AlignedReadError(#[from] AlignedReadError)
}

#[derive(Debug, thiserror::Error)]
pub enum RescaleError {
}