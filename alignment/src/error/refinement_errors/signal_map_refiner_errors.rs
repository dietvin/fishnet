use kmer_table::error::KmerTableError;
use pod5_reader_api::error::read::Pod5ReadError;

use crate::error::alignment_errors::AlignedReadError;

use super::refine_errors::RefineError;
use super::rescale_errors::{RescaleError, RoughRescaleError};
#[derive(Debug, thiserror::Error)]
pub enum SigMapRefineError {
    #[error("Failed to initialize the kmer table: {0}")]
    KmerTableError(#[from] KmerTableError),
    #[error("Rough rescaling failed: {0}")]
    RoughRescalingError(#[from] RoughRescaleError),
    #[error("Failed to calculate scaling factors: {0}")]
    RescalingError(#[from] RescaleError),
    #[error("Query-to-signal alignment not present")]
    QueryToSigNotFound,
    #[error("Reference-to-signal alignment not present")]
    RefToSigNotFound,
    #[error("AlignedRead error: {0}")]
    AlignedReadError(#[from] AlignedReadError),
    #[error("Refinement error: {0}")]
    RefineError(#[from] RefineError),
    #[error("Refined query-to-signal alignment not present")]
    RefinedQueryToSigNotFound,
    #[error("Refined reference-to-signal alignment not present")]
    RefinedRefToSigNotFound,
    #[error("Pod5Read error: {0}")]
    Pod5ReadError(#[from] Pod5ReadError)
}