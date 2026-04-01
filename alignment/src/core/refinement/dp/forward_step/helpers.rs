pub(super) const LARGE_SCORE: f32 = 100.0;

/// Calculates the squared difference between expected and measured signal levels
///
/// # Arguments
///
/// * `expected` - The expected or reference signal level
/// * `measured` - The actual measured signal level from the data
///
/// # Returns
///
/// The squared difference (error) between the expected and measured values
pub(super) fn score(expected: f32, measured: f32) -> f32 {
    let tmp = measured - expected;
    tmp*tmp
}
