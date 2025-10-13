use std::ffi::OsString;

use uuid::Uuid;

use crate::{
    error::{feather_reader::FeatherReaderError, file::Pod5FileError},
    core::footer::embedded_content::EmbeddedContentType
};

/// Error type for Pod5Dataset operations.
/// 
/// Includes variants for:
/// - Underlying Pod5File errors
/// - Invalid key access
/// - Index out of bounds errors
#[derive(Debug, thiserror::Error)]
pub enum Pod5DatasetError {
    #[error("Paths list is empty")]
    EmptyPathList,
    #[error("Pod5File error: {0}")]
    Pod5FileError(#[from] Pod5FileError),
    #[error("Key {0:?} not found in dataset")]
    InvalidKey(OsString),
    #[error("File index out of bounds: {0} (len={1})")]
    FileIndexError(usize, usize),
    #[error("Read index out of bounds: {0} (len={1})")]
    ReadIndexError(usize, usize),
    #[error("Read id {0} not found in read ids.")]
    ReadIdNotFound(Uuid),
    #[error("Feather reader pool error: {0}")]
    FeatherReaderPoolSharedError(#[from] PoolSharedError)
}


// ==== Errors used for Pod5DatasetThreadSafe operations ====


/// Error type used for initializing SignalReaderConfig
#[derive(Debug, thiserror::Error)]
pub enum SignalReaderConfigError {
    #[error("Embedded content ({0:?}) does not correspond to a signal table")]
    NotSignalTable(EmbeddedContentType)
}

/// Error type for initializing BufferedFeather readers
#[derive(Debug, thiserror::Error)]
pub enum BufferedFeatherReaderError {
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
    #[error("Feather reader error: {0}")]
    FeatherReaderError(#[from] FeatherReaderError)
}

/// Error type for FeatherReaderPoolShared operations
#[derive(Debug, thiserror::Error)]
pub enum PoolSharedError {
    #[error("Buffer size ({0}) can not be smaller than the number of workers ({1})")]
    BufferSizeError(usize, usize),
    #[error("Invalid file id: {0}")]
    FileNotInPool(usize),
    #[error("Buffered feather reader error: {0}")]
    BufferedFeatherReaderError(#[from] BufferedFeatherReaderError),
    #[error("More than the max. number of readers ({0}) are in use")]
    TooManyReadersInUse(usize)
}