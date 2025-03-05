use super::kmer_table_errors::KmerTableError;

#[derive(Debug, thiserror::Error)]
pub enum SigMapRefineError {
    #[error("Failed to initialize the kmer table: {0}")]
    KmerTableError(#[from] KmerTableError)
}