use crate::error;

use super::file_handling_errors::{FileHandlingError, DirHandlingError};

#[derive(Debug, thiserror::Error)]
pub enum Pod5ReadError {
    #[error("Failed to trim signal: {0}")]
    TrimError(String)
}

#[derive(Debug, thiserror::Error)]
pub enum Pod5FileError {
    /// IO errors when opening or reading from files
    #[error("IO error: {0}")]
    IOError(#[from] std::io::Error),

    /// Errors from the pod5 crate
    #[error("Pod5 error: {0}")]
    Pod5Error(#[from] pod5::error::Pod5Error),

    /// Errors from the polars crate
    #[error("Polars error: {0}")]
    PolarsError(#[from] pod5::polars::prelude::PolarsError),

    /// UTF-8 conversion errors for read IDs
    #[error("Failed to decode read ID as UTF-8: {0}")]
    Utf8Error(#[from] std::str::Utf8Error),

    /// UTF-8 conversion errors for read IDs
    #[error("Failed to decode binary id to UUID: {0}")]
    UuidError(#[from] uuid::Error),


    /// Column data is null/missing
    #[error("Data is missing in column '{column}' for read ID '{read_id}'")]
    ColumnDataMissingError {
        column: String,
        read_id: String,
    },

    /// Failed to downcast signal to expected array type
    #[error("Failed to downcast signal to expected array type (polars Int16Array) for read {0}")]
    DowncastError(String),

    /// Could not find the requested read in the read collection
    #[error("Read '{0}' not found in reads")]
    ReadNotFound(String)
}

#[derive(Debug, thiserror::Error)]
pub enum Pod5IndexError {
    /// Invalid directory when initialzing with a directory path
    #[error("Invalid directory: {0}")]
    IoInvalidDir(#[from] DirHandlingError),

    /// Invalid file path in file list  when initialzing with file paths
    #[error("Invalid file list: {0}")]
    IoInvalidFileList(#[from] FileHandlingError), 

    /// Invalid file path given to load_file
    #[error("File not found: {0}")]
    FileNotFound(String),

    #[error("Error loading file: {0}")]
    FileLoadingError(#[from] Pod5FileError),

    #[error("Mutex error: {0}")]
    MutexError(String)
} 