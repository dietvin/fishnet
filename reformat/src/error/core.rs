pub(crate) mod loader {
    use pod5_reader_api::error::{dataset::Pod5DatasetError, read::Pod5ReadError};

    use crate::execute::config::Column;

    #[derive(Debug, thiserror::Error)]
    pub(crate) enum ColumnIndexError {
        #[error("Unexpected field in schema: {0}")]
        UnexpectedFieldName(String),
        #[error("Need column '{1:?}' for {0} data, but column was not found")]
        MissingColumn(&'static str, Column)
    }

    #[derive(Debug, thiserror::Error)]
    pub(crate) enum AlignmentChunkError {
        #[error("No data found for column {0} at index {1}")]
        ColumnIndexError(&'static str, usize),
        #[error("Failed to downcast to {0}")]
        DowncastError(&'static str),
        #[error("Value is None")]
        ValueNone,
        #[error("Uuid error: {0}")]
        UuidError(#[from] uuid::Error),
        #[error("Pod5Dataset is needed, but is None")]
        Pod5DatasetMissing,
        #[error("Invalid index {0} with length {1}")]
        InvalidIndex(usize, usize),
        #[error("Pod5Dataset error: {0}")]
        Pod5DatasetError(#[from] Pod5DatasetError),
        #[error("Pod5Read error: {0}")]
        Pod5ReadError(#[from] Pod5ReadError),
        #[error("Row error: {0}")]
        RowError(#[from] RowError)
    }

    #[derive(Debug, thiserror::Error)]
    pub(crate) enum RowError {
        #[error("Reference region error: {0}")]
        ReferenceRegionError(#[from] ReferenceRegionError)
    }

    #[derive(Debug, thiserror::Error)]
    pub(crate) enum RowIteratorError {
        #[error("IO error: {0}")]
        IoError(#[from] std::io::Error),
        #[error("Arrow2 error: {0}")]
        ArrowError(#[from] arrow2::error::Error),
        #[error("Not a single chunk found")]
        NoChunks,
        #[error("Column index error: {0}")]
        ColumnIndexError(#[from] ColumnIndexError),
        #[error("Alignment chunk error: {0}")]
        AlignmentChunkError(#[from] AlignmentChunkError)
    }

    #[derive(Debug, thiserror::Error)]
    pub(crate) enum ReferenceRegionError {

    }

    #[derive(Debug, thiserror::Error)]
    pub(crate) enum ReferenceRegionsError {

    }
}