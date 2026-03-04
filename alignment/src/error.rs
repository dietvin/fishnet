/*!
 * This module contains custom error types for different aspects of the signal-to-sequence
 * alignment process.
 */

use alignment_errors::AlignmentCoreError;
use helper::errors::LoggerError;
use kmer_table::error::KmerTableError;
use loader_errors::LoaderError;
use pod5_reader_api::error::dataset::Pod5DatasetError;
use refinement_errors::RefinementError;

use crate::error::{
    loader_errors::bam_errors::BamFileError, 
    output_errors::OutputError, 
    refinement_errors::load_kmer_table_errors::KmerTableLoadingError, 
};

pub mod loader_errors;
pub mod alignment_errors;
pub mod refinement_errors;
pub mod output_errors;

/// Top level error that handles all custom sub-types
#[derive(Debug, thiserror::Error)]
pub enum AlignmentError {
    #[error("Log error: {0}")]
    LogError(#[from] LoggerError),
    #[error("Bam file error: {0}")]
    BamFileError(#[from] BamFileError),
    #[error("Pod5 dataset error: {0}")]
    Pod5DatasetError(#[from] Pod5DatasetError),
    #[error("Kmer table error: {0}")]
    KmerTableError(#[from] KmerTableError),
    #[error("Kmer table loading error: {0}")]
    KmerTableLoadingError(#[from] KmerTableLoadingError),
    #[error("Output error: {0}")]
    OutputError(#[from] OutputError),
    #[error("Loader error: {0}")]
    LoaderError(#[from] LoaderError),
    #[error("Alignment error: {0}")]
    AlignmentError(#[from] AlignmentCoreError),
    #[error("Refinement error: {0}")]
    RefinementError(#[from] RefinementError),
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
    #[error("Failed to join '{0}' thread")]
    ThreadJoinError(&'static str),
}