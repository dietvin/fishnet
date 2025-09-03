use std::path::PathBuf;

#[derive(Debug, thiserror::Error)]
pub enum DirHandlingError {
    #[error("Directory not found: {0}")]
    DirectoryNotFound(PathBuf),
    #[error("WalkDir error: {0}")]
    WalkDirError(#[from] walkdir::Error),
    #[error("IO Error: {0}")]
    IOError(#[from] std::io::Error),
    #[error("Failed to convert PathBuf to String: {0}")]
    PathConvError(PathBuf),
    #[error("No files of type '{0}' found in {1}")]
    NoFilesFound(String, PathBuf),
}

#[derive(Debug, thiserror::Error)]
pub enum FileHandlingError {
    #[error("File not found: {0}")]
    FileNotFound(PathBuf),
    #[error("Invalid file type: {0} (expected '{1}')")]
    InvalidFileType(PathBuf, String)
}
