use super::band_errors::{SequenceBandError, SignalBandError};

#[derive(Debug, thiserror::Error)]
pub enum RefineError {
    #[error("Signal band error: {0}")]
    SignalBandError(#[from] SignalBandError),
    #[error("Sequence band error: {0}")]
    SequenceBandError(#[from] SequenceBandError)
}