#[derive(Debug, thiserror::Error)]
pub enum SignalBandError {
    #[error("Invalid options: {0}, {1}")]
    InvalidOptions(usize, bool),
    #[error("Length mismatch: {0} (seq. to sig. map) vs {1} (sequence length)")]
    LengthMismatch(usize, usize),
    #[error("Invalid signal band: {0}")]
    ValidationError(#[from] BandValidationError)
}

#[derive(Debug, thiserror::Error)]
pub enum SequenceBandError {
    #[error("Sequence band already present")]
    AlreadySequenceBand,
    #[error("Invalid sequence band: {0}")]
    ValidationError(#[from] BandValidationError),
    #[error("SignalBand error: {0}")]
    SignalBandError(#[from] SignalBandError)
}

#[derive(Debug, thiserror::Error)]
pub enum BandValidationError {
    #[error("Band does not start with 0 coordinate")]
    StartNonZero,
    #[error("Band contains 0-length region")]
    ZeroLenRegion,
    #[error("Unexpected band length (got {0}, expected {1})")]
    InvalidBandLen(usize, usize),
    #[error("Unexpected end coordinate (got {0}, expected {1})")]
    InvalidEndCoord(usize, usize)
}