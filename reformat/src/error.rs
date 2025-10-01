use helper::errors::LoggerError;
use pod5_reader_api::error::dataset::Pod5DatasetError;
use crate::error::{core::{filter::FilterError, loader::RowIteratorError}, execute::OutputError};

pub(crate) mod execute;
pub(crate) mod core;

#[derive(Debug, thiserror::Error)]
pub(crate) enum ReformatError {
    #[error("Logger error: {0}")]
    LoggerError(#[from] LoggerError),
    #[error("Pod5 dataset error: {0}")]
    Pod5DatasetError(#[from] Pod5DatasetError),
    #[error("Row iterator error: {0}")]
    RowIteratorError(#[from] RowIteratorError),
    #[error("Filter error: {0}")]
    FilterError(#[from] FilterError),
    #[error("Exploded output format is not available with filters of unequal size")]
    ExplodedWithUnequalFilterLength,
    #[error("Output error: {0}")]
    OutputError(#[from] OutputError)
}