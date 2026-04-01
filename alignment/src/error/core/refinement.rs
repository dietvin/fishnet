use kmer_table::error::KmerTableError;
use pod5_reader_api::error::read::Pod5ReadError;

use crate::error::{bam::BamReadError, core::{
    alignment::AlignmentError,
    refinement::{
        band::{SequenceBandError, SignalBandError},
        rescale::{RescaleError, RoughRescaleError}
    }
}};

pub mod band;
pub mod rescale;

#[derive(Debug, thiserror::Error)]
pub enum RefinementError {
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
    AlignedReadError(#[from] AlignmentError),
    #[error("Signal band error: {0}")]
    SignalBandError(#[from] SignalBandError),
    #[error("Sequence band error: {0}")]
    SequenceBandError(#[from] SequenceBandError),
    #[error("Refined query-to-signal alignment not present")]
    RefinedQueryToSigNotFound,
    #[error("Refined reference-to-signal alignment not present")]
    RefinedRefToSigNotFound,
    #[error("Pod5Read error: {0}")]
    Pod5ReadError(#[from] Pod5ReadError),
    #[error("BamRead error: {0}")]
    BamReadError(#[from] BamReadError)
}