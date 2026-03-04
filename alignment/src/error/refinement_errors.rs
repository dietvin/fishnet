use kmer_table::error::KmerTableError;
use refine_errors::RefineError;
use rescale_errors::{RescaleError, RoughRescaleError};

pub mod load_kmer_table_errors;
pub mod signal_map_refiner_errors;
pub mod refine_errors;
pub mod rescale_errors;
pub mod band_errors;

#[derive(Debug, thiserror::Error)]
pub enum RefinementError {
    #[error("Kmer table error: {0}")]
    KmerTableError(#[from] KmerTableError),
    #[error("Rough rescale error: {0}")]
    RoughRescaleError(#[from] RoughRescaleError),
    #[error("Rescale error: {0}")]
    RescaleError(#[from] RescaleError),
    #[error("Refinement error: {0}")]
    RefineError(#[from] RefineError),
}