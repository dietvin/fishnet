#[derive(Debug, thiserror::Error)]
pub enum RoughRescaleError {
    #[error("Signal vector is empty")]
    EmptySignalVector,

    #[error("Levels vector is empty")]
    EmptyLevelsVector,

    #[error("Failed to prepare the data: {0}")]
    PrepError(String),

    #[error("Failed to calculate the quantiles: {0}")]
    QuantileError(#[from] QuantileCalcError),

    #[error("No valid data points found after filtering")]
    NoValidDataPoints,

    #[error("Calculated scale is zero or too small: {0}")]
    InvalidScaleValue(f32),

    #[error("Least squares calculation failed: {0}")]
    LstqFailed(#[from] LstsqError),

    #[error("Thein Sen calculation failed: {0}")]
    TheilSenFailed(#[from] TheilSenError)
}

#[derive(Debug, thiserror::Error)]
pub enum QuantileCalcError {
    #[error("Empty data vec provided")]
    EmptyDataVec,

    #[error("Empty quantiles vec provided")]
    EmptyQuantVec,

    #[error("Invalid quantile value: {0} (must be between 0.0 and 1.0)")]
    InvalidQuant(f32),

}

#[derive(Debug, thiserror::Error)]
pub enum LstsqError {
    #[error("Length mismatch: {0} (x) vs {1} (y)")]
    LengthMismatch(usize, usize),

    #[error("Not enough data points for linear regression (need at least 2)")]
    InsufficientDataPoints(usize),
    
    #[error("Denominator is too close to zero: {0}")]
    ZeroDenominator(f32),
    
    #[error("Data contains NaN or Infinity values")]
    InvalidFloatingPointValues,
}

#[derive(Debug, thiserror::Error)]
pub enum TheilSenError {
    #[error("Length mismatch: {0} (x) vs {1} (y)")]
    LengthMismatch(usize, usize),

    #[error("All computed slopes are zero, unable to perform Theil-Sen estimation")]
    AllSlopesZero,

    #[error("Slope is zero, unable to compute scaling factors")]
    MedianSlopeZero,

    #[error("Cannot compute median of empty vector")]
    MedianCalcEmptyVec,

    #[error("Requested subsample size {0} is larger than the data size {1}")]
    InvalidSubsampleSize(usize, usize),
    
    #[error("Not enough valid slopes could be calculated")]
    InsufficientValidSlopes,
    
    #[error("Data contains NaN or Infinity values")]
    InvalidFloatingPointValues,
}

