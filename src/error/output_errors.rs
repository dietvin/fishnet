use std::path::PathBuf;

#[derive(Debug, thiserror::Error)]
pub enum OutputError {
    /// Error when attempting to write to an existing file without overwrite permission
    #[error("File {0} already exists and overwrite is not specified.")]
    FileExists(PathBuf),

    /// Error when creating the output file fails
    #[error("Failed to create output file: {0}")]
    FileCreation(#[from] std::io::Error),

    /// Error when attempting to use a writer that has already been finalized
    #[error("Writer already finalized")]
    AlreadyFinalized,

    #[error("Invalid output schema: {0}")]
    InvalidOutputSchema(String),

    // Errors specific to Arrow output format

    /// Error from underlying Parquet library operations
    #[error("Parquet error: {0}")]
    ParquetError(#[from] parquet::errors::ParquetError),

    /// Error from underlying Arrow library operations
    #[error("Arrow error: {0}")]
    ArrowError(#[from] arrow::error::ArrowError),

    // Error specific to Json output format

    /// Error from underlying serde_json operations
    #[error("Serde json error: {0}")]
    SerdeJsonError(#[from] serde_json::Error)

}