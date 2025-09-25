use helper::errors::InterpolationError;
use pod5_reader_api::error::read::Pod5ReadError;

use super::loader_errors::bam_errors::BamReadError;

#[derive(Debug, thiserror::Error)]
pub enum AlignmentCoreError {
    #[error("Aligned read error: {0}")]
    AlignedReadError(#[from] AlignedReadError),
    #[error("Query to signal error: {0}")]
    QueryToSignalError(#[from] QueryToSignalError),
    #[error("Reference to signal error: {0}")]
    RefToSignalError(#[from] RefToSignalError),

}

#[derive(Debug, thiserror::Error)]
pub enum QueryToSignalError {
    #[error("Length of alignment ({0}) discordant with query length ({1})")]
    DiscordantToSequence(usize, usize),
    #[error("Length of alignment ({0}) discordant with signal length ({1} / {2} = {3})")]
    DiscordantToSignal(usize, usize, usize, usize)
}

#[derive(Debug, thiserror::Error)]
pub enum RefToSignalError {
    #[error("No match ops found in Cigar")]
    NoMatchOps,
    #[error("Length of alignment ({0} - 1) discordant with reference length ({1})")]
    DiscordantToSequence(usize, usize),
    #[error("Interpolation error: {0}")]
    InterpolationError(#[from] InterpolationError)
}

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
    RetrieveBamError(#[from] BamReadError),
    #[error("Failed to trim signal: {0}")]
    TrimError(String),
    #[error("Trimmed signal not set yet. Call `update_signal` to trim the signal")]
    TrimmedSignalNotFound,
}