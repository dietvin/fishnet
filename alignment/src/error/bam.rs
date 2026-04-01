use std::string::FromUtf8Error;

#[derive(Debug, thiserror::Error)]
pub enum BamReadError {
    #[error("Could not extract read id")]
    ReadIdError,
    #[error("Invalid tag: {0} (Must be of length 2)")]
    InvalidTagLength(String),
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
    #[error("Tag '{0}' was found, but could not be extacted: {1}")]
    TagNotExtracted(String, std::io::Error),
    #[error("Tag '{0}' was not found in bam record")]
    TagNotPresent(String),
    #[error("Could not extract cigar operations: {0}")]
    CigarError(std::io::Error),
    #[error("Read is mapped, but alignment information ({0}) was not found")]
    AlignmentInfoMissing(String),
    #[error("Could not transform id to String: {0}")]
    IdConversionError(#[from] std::str::Utf8Error),
    #[error("Could not extract tag '{0}': Expected {1}, got {2}")]
    TagUnexpectedTypeError(String, String, String),
    #[error("Read not mapped - unable to retrieve {0}")]
    NoSuchDataForUnmappedRead(String),
    #[error("Failed to reconstruct the reference sequence: {0}")]
    RefSeqError(#[from] RefSeqReconstructError),
    #[error("Failed to set up the reverse complement (Unexpected ascii value {0}")]
    ReverseComplement(u8),
    #[error("Reference name was not found")]
    RefNameNotFound,
    #[error("No value found in reference sequence index for key {0}")]
    InvalidRefSeqKey(usize),
    #[error("Reference start position was not found")]
    ReferenceStartNotFound,
    #[error("Could not convert Vec<u8> to String: {0}")]
    Utf8Error(#[from] FromUtf8Error)
}

#[derive(Debug, thiserror::Error)]
pub enum BamFileError {
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
    #[error("Could not transform id to String: {0}")]
    IdConversionError(#[from] std::str::Utf8Error),
    #[error("Id not found in index: {0}")]
    IndexError(String),
    #[error("Could not access record: {0}")]
    ValueError(String),
    #[error("Could not initialize BamRead: {0}")]
    BamReadError(#[from] BamReadError),
    #[error("Id of extracted read does not match the wanted id: {0} vs {1}")]
    ReadIdMismatch(String, String),
    #[error("Could not extract read ID from record")]
    RecordAccessError,
}

#[derive(Debug, thiserror::Error, PartialEq)]
pub enum RefSeqReconstructError {
    #[error("Empty query sequence")]
    EmptyQuery,
    #[error("Query sequence index out of bounds: {0} (len={1})")]
    QueryOutOfBounds(usize, usize),
    #[error("Reference sequence index out of bounds: {0} (len={1})")]
    ReferenceOutOfBounds(usize, usize),
    #[error("Invalid char: {0}")]
    InvalidChar(u8)
}