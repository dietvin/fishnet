use helper::errors::InterpolationError;
use pod5_reader_api::error::read::Pod5ReadError;

#[derive(Debug, thiserror::Error)]
pub enum AlignmentError {
    #[error("BaseRead error: {0}")]
    BaseReadError(#[from] BaseReadError),
    #[error("Query-to-signal alignment error: {0}")]
    QueryToSignalError(#[from] QueryAlignedError),
    #[error("Reference-to-signal alignment error: {0}")]
    RefToSignalError(#[from] RefAlignedError),
}

#[derive(Debug, thiserror::Error)]
pub enum BaseReadError {
    #[error("ID mismatch: {0} (pod5) vs {1} (bam)")]
    IdMismatch(String, String),
    #[error("Pod5Read error: {0}")]
    Pod5Error(#[from] Pod5ReadError),
    #[error("Failed to trim signal: {0}")]
    TrimError(String),
    #[error("CIGAR is None (read is unmapped)")]
    CigarMissing,
    #[error("Reference length is None (read is unmapped)")]
    ReferenceLenNone,
}

#[derive(Debug, thiserror::Error)]
pub enum QueryAlignedError {
    #[error("Length of alignment ({0}) discordant with query length ({1})")]
    DiscordantToSequence(usize, usize),
    #[error("Length of alignment ({0}) discordant with signal length ({1} / {2} = {3})")]
    DiscordantToSignal(usize, usize, usize, usize)
}

#[derive(Debug, thiserror::Error)]
pub enum RefAlignedError {
    #[error("No match ops found in Cigar")]
    NoMatchOps,
    #[error("Length of alignment ({0} - 1) discordant with reference length ({1})")]
    DiscordantToSequence(usize, usize),
    #[error("Interpolation error: {0}")]
    InterpolationError(#[from] InterpolationError),
    #[error("BaseRead error: {0}")]
    BaseReadError(#[from] BaseReadError)
}