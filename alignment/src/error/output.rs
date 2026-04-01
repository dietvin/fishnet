use std::path::PathBuf;

use pod5_reader_api::error::read::Pod5ReadError;

use crate::error::bam::BamReadError;


#[derive(Debug, thiserror::Error)]
pub enum OutputRecordError {
    #[error("BAM read error: {0}")]
    BamReadError(#[from] BamReadError),
    #[error("POD5 read error: {0}")]
    Pod5ReadError(#[from] Pod5ReadError)
}


#[derive(Debug, thiserror::Error)]
pub enum BufferError {
    #[error("Arrow2 error: {0}")]
    ArrowError(#[from] arrow2::error::Error),
    #[error("Serde json error: {0}")]
    JsonError(#[from] serde_json::Error)
}


#[derive(Debug, thiserror::Error)]
pub enum WriterError {
    #[error("Arrow2 error: {0}")]
    ArrowError(#[from] arrow2::error::Error),
    #[error("File at '{0:?}' exists and overwrite is disabled")]
    FileExists(PathBuf),
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
}