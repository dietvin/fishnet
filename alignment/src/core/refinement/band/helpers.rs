use crate::error::core::refinement::band::BandValidationError;

/// Validates aspects that need testing for both signal and sequence bands.
/// 
/// # Arguments
/// * `start` - Start values
/// * `end` - end values
/// 
/// # Returns
/// Ok(()) if the band starts with 0 and doesn't have intervals of length 0.
/// Error otherwise.
pub(super) fn validate_band(
    start: &Vec<usize>, 
    end: &Vec<usize>
) -> Result<(), BandValidationError> {
    if start[0] != 0 {
        return Err(BandValidationError::StartNonZero);
    }
    if end.iter().zip(start).any(|(e, s)| e <= s) {
        return Err(BandValidationError::ZeroLenRegion);
    }
    // skipping check for monotically increasing, as this is ensured in the functions
    Ok(())
}


/// Validates aspects that need testing for both signal and sequence bands.
/// 
/// # Arguments
/// * `start` - Start values
/// * `end` - end values
/// 
/// # Returns
/// Ok(()) if the band starts with 0 and doesn't have intervals of length 0.
/// Error otherwise.
pub(super) fn validate_sequence_band(
    start: &Vec<usize>, 
    end: &Vec<usize>,
    signal_len: usize,
    sequence_len: usize,
) -> Result<(), BandValidationError> {
    validate_band(start, end)?;

    if start.len() != sequence_len {
        return Err(BandValidationError::InvalidBandLen(start.len(), sequence_len));
    }

    if end[end.len() - 1] != signal_len {
        return Err(BandValidationError::InvalidEndCoord(end[end.len() - 1], signal_len));
    }

    Ok(())
}

