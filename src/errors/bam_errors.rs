#[derive(Debug, thiserror::Error)]
pub enum BamReadError {
    #[error("HTSLib error: {0}")]
    HTSLibError(#[from] rust_htslib::errors::Error),
    #[error("Could not transform id to String: {0}")]
    IdConversionError(#[from] std::str::Utf8Error),
    #[error("Could not extract tag '{0}': Expected {1}, got {2}")]
    TagUnexpectedTypeError(String, String, String),
    #[error("Read not mapped - unable to retrieve data")]
    NoSuchDataForUnmappedRead
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
    BamReadError(#[from] BamReadError)
}