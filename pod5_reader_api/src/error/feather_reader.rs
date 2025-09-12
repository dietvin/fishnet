/// Error types that can occur during FeatherReader operations.
/// Covers bounded reader errors, Arrow format errors, I/O failures,
/// and index out of bounds conditions.
#[derive(Debug, thiserror::Error)]
pub enum FeatherReaderError {
    #[error("Embedded feather reader error: {0}")]
    EmbeddedFeatherReaderError(#[from] BoundedReaderError),
    #[error("Arrow2 error: {0}")]
    Arrow2Error(#[from] arrow2::error::Error),
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
    #[error("Index out of bounds: {0}")]
    IndexOutOfBounds(usize),
}

#[derive(Debug, thiserror::Error)]
pub enum BoundedReaderError {
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
}