#[derive(Debug, thiserror::Error)]
pub enum BamReadError {
    #[error("HTSLib error: {0}")]
    HTSLibError(#[from] rust_htslib::errors::Error),
    #[error("Could not transform id to String: {0}")]
    IdConversionError(#[from] std::str::Utf8Error),
    #[error("Could not extract tag '{0}': Expected {1}, got {2}")]
    TagUnexpectedTypeError(String, String, String),
    #[error("Read not mapped - unable to retrieve {0}")]
    NoSuchDataForUnmappedRead(String),
    #[error("Failed to reconstruct the reference sequence: {0}")]
    RefSeqError(#[from] RefSeqReconstructError),
    #[error("Failed to set up the reverse complement (Unexpected ascii value {0}")]
    ReverseComplement(u8)
}

#[derive(Debug, thiserror::Error)]
pub enum BamFileError {
    #[error("HTSLib error: {0}")]
    HTSLibError(#[from] rust_htslib::errors::Error),
    #[error("Could not transform id to String: {0}")]
    IdConversionError(#[from] std::str::Utf8Error),
    #[error("Id not found in index: {0}")]
    IndexError(String),
    #[error("Could not access record: {0}")]
    ValueError(String),
    #[error("Could not initialize BamRead")]
    BamReadError(#[from] BamReadError),
}

#[derive(Debug, thiserror::Error, PartialEq)]
pub enum RefSeqReconstructError {
    #[error("Query sequence index out of bounds: {0} (len={1})")]
    QueryOutOfBounds(usize, usize),
    #[error("Reference sequence index out of bounds: {0} (len={1})")]
    ReferenceOutOfBounds(usize, usize),
    #[error("Invalid char: {0}")]
    InvalidChar(u8)
}