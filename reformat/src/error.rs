use pod5_reader_api::error::dataset::Pod5DatasetError;
use crate::error::core::{filter::FilterError, loader::RowIteratorError};

pub(crate) mod execute;
pub(crate) mod core;

#[derive(Debug, thiserror::Error)]
pub(crate) enum ReformatError {
    #[error("Row iterator error: {0}")]
    RowIteratorError(#[from] RowIteratorError),
    #[error("Pod5 dataset error: {0}")]
    Pod5DatasetError(#[from] Pod5DatasetError),
    #[error("Filter error: {0}")]
    FilterError(#[from] FilterError),
}