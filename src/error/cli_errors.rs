use std::{io::Error, path::PathBuf};


#[derive(Debug, thiserror::Error)]
pub enum PathError {
    #[error("Provided path '{0}' is not a file")]
    IsNotFile(PathBuf),
    #[error("Provided path '{0}' does not exist")]
    DoesNotExist(PathBuf),
    #[error("Provided path '{0}' has an invalid extension (expected '{1}')")]
    InvalidExtension(PathBuf, String),
    #[error("Provided path '{0}' is not a directory")]
    IsNotDir(PathBuf),
    #[error("Failed to create directory '{0}'")]
    FailedToCreateDir(PathBuf),
    #[error("Base directory does not exist for '{0}'")]
    BaseDirNotExist(PathBuf),
    #[error("File '{0}' already exists and force overwrite is disabled")]
    FileExists(PathBuf),
    #[error("Io error: {0}")]
    IoError(#[from] std::io::Error)
}


#[derive(Debug, thiserror::Error)]
pub enum Pod5PathError {
    #[error("IoError: {0}")]
    IoError(#[from] Error),
    #[error("No valid pod5 files found")]
    NoValidFilesFound,   
}


#[derive(Debug, thiserror::Error)]
pub enum CliError {
    #[error("Invalid argument for '{0}'")]
    ArgumentNone(String),
    #[error("PathError: {0}")]
    PathError(#[from] PathError),
    #[error("Pod5PathError: {0}")]
    Pod5PathError(#[from] Pod5PathError),
    #[error("Invalid value for argument {0}: {1}")]
    InvalidArgument(String, String),
    #[error("Io error: {0}")]
    IoError(#[from] std::io::Error)
}