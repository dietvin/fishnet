use std::path::PathBuf;

use crate::{
    error::core::reformat::{
        ReformatedRowInterpError, 
        ReformatedRowStatError
    }, 
    execute::config::Stats
};

#[derive(Debug, thiserror::Error)]
pub enum OutputError {
    #[error("File {0} already exists and overwrite is not specified.")]
    FileExists(PathBuf),
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
    #[error("Arrow2 error: {0}")]
    ArrowError(#[from] arrow2::error::Error),
    #[error("Writer already finalized")]
    AlreadyFinalized,

    #[error("Arrow buffer error: {0}")]
    ArrowBufferError(#[from] ArrowBufferError),

    #[error("TSV output error")]
    TsvOutputError(#[from] TsvOutputError),
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum ArrowBufferError {
    #[error("Reformated row stat error: {0}")]
    ReformatedRowStatError(#[from] ReformatedRowStatError),
    #[error("Reformated row interpolation error: {0}")]
    ReformatedRowInterpError(#[from] ReformatedRowInterpError),
    #[error("Stats from reformated data and buffer do not match")]
    StatsMismatch,
    #[error("Index {0} out of bounds with length {1}")]
    IndexError(usize, usize),
    #[error("Key error: {0:?} not found in stats buffer")]
    KeyError(Stats),
    #[error("Arrow2 error: {0}")]
    ArrowError(#[from] arrow2::error::Error),
    #[error("Found Interpolation buffer with stats data")]
    UnexpectedBufferTypeWithStats,
    #[error("Found Stats buffer with interpolation data")]
    UnexpectedBufferTypeWithInterp,
    #[error("Stat {0:?} from schema not found in buffer")]
    InvalidStat(Stats)
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum TsvOutputError {
    #[error("Data was processed via {0}, but {1} was expected")]
    ReformatStratMismatch(&'static str, &'static str),
    #[error("Reformated row stat error: {0}")]
    ReformatedRowStatError(#[from] ReformatedRowStatError),
    #[error("Nested output shape is not available for TSV output.")]
    NestedOutputNotAvailable,
    #[error("Reformated row interpolation error: {0}")]
    ReformatedRowInterpError(#[from] ReformatedRowInterpError),
    #[error("Index {0} out of bounds with length {1}")]
    IndexError(usize, usize),
    #[error("Key error: {0:?} not found in stats buffer")]
    KeyError(Stats),
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
}