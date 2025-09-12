use flatbuffers::InvalidFlatbuffer;

use crate::core::footer::embedded_content::EmbeddedContentType;

/// Error types that can occur during pod5 footer parsing operations.
/// Covers I/O failures, flatbuffers parsing errors, missing required fields,
/// and embedded file lookup failures.
#[derive(Debug, thiserror::Error)]
pub enum Pod5FooterError {
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
    #[error("Invalid Flatbuffer: {0}")]
    FlatBuffersError(#[from] InvalidFlatbuffer),
    #[error("Empty file identifier")]
    EmptyFileIdentifier,
    #[error("Empty software entry")]
    EmptySoftware,
    #[error("Empty pod5 version entry")]
    EmptyVersion,
    #[error("Empty contents entry")]
    EmptyContents,
    #[error("Contents vector is empty")]
    EmptyContentsVec,
    #[error("Could not file embedded file of type: {0:?}")]
    EmbeddedFileNotFound(EmbeddedContentType)
}