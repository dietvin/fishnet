use std::io::Error;

#[derive(Debug, thiserror::Error)]
pub enum Pod5PathHandlerError {
    #[error("IoError: {0}")]
    IoError(#[from] Error),
    #[error("No valid pod5 files found")]
    NoValidFilesFound,   
}