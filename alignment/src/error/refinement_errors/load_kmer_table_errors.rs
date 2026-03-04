#[derive(Debug, thiserror::Error)]
pub enum KmerTableLoadingError {
    #[error("Found varying basecall model names in BAM header: {0} vs {1}")]
    InconsistentBasecallModel(String, String),

    #[error("No basecalling model found in BAM header")]
    BasecallModelNotFound,

    #[error("Could not assign a stored model to basecall model: {0}")]
    UnfittingBasecallModel(String),

    #[error("Failed to deserialize kmer table: {0}")]
    DeserializationError(#[from] Box<bincode::ErrorKind>)
}