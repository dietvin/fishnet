#[derive(Debug, thiserror::Error)]
pub enum ReadsTableError {
    #[error("UUID parsing error: {0}")]
    UuidError(#[from] uuid::Error),
    #[error("Invalid UUID bytes: expected 16 bytes, got {0}")]
    InvalidUuidLength(usize),
    #[error("Signal index out of bounds")]
    SignalIndexOutOfBounds,
    #[error("Invalid chunk structure: expected {expected} arrays, got {actual}")]
    InvalidChunkStructure { expected: usize, actual: usize },
    #[error("Failed to cast array at index {index}: {reason}")]
    ArrayCastError { index: usize, reason: String },
    #[error("Could not downcast the signal indices array")]
    SignalIndexArrayCastError,
    
    #[error["Required column {0} is missing in reads table"]]
    MissingRequiredColumn(String),
}

#[derive(Debug, thiserror::Error)]
pub enum RunInfoError {
    #[error("Expected 1 row in chunk, found {0}")]
    InvalidRowCount(usize),
    #[error("Expected at least {expected} arrays in chunk, found {found}")]
    InvalidArrayCount { expected: usize, found: usize },
    #[error("Invalid acquisition id: {0}")]
    InvalidAcquisitionId(&'static str),
    #[error("Field '{0}' is missing or null")]
    MissingField(&'static str),
    #[error("Invalid type for field '{0}'")]
    InvalidType(&'static str),
    #[error("Invalid map array. Could not acces first value")]
    InvalidMapArray,
    #[error("Invalid map structure: {0}")]
    InvalidMapStructure(&'static str),
}

/// Errors that can occur during the construction or access of `SignalTable`.
#[derive(Debug, thiserror::Error)]
pub enum SignalTableError {
    #[error("Failed to cast column {0} to type {1}")]
    DowncastError(&'static str, &'static str),
    #[error("Invalid UUID bytes: expected 16 bytes, got {0}")]
    InvalidUuidLength(usize),
    #[error("Signal index out of bounds ({0} with length {1}")]
    SignalIndexOutOfBounds(usize, usize),
    #[error("Decompress error: {0}")]
    DecompressError(#[from] std::io::Error)
}