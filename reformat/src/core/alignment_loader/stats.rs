use crate::error::core::loader::StatsError;

/// Calculates the mean of an i16 Vector.
/// 
/// # Returns
/// * `Ok(f64)` - Mean value
/// * `Err(StatsError)` - If the vector is empty
pub(super) fn mean_i16(values: &[i16]) -> Result<f64, StatsError> {
    if values.is_empty() {
        return Err(StatsError::ZeroDivision);
    }
    let sum = values.iter().map(|&x| x as f64).sum::<f64>();
    let n = values.len() as f64;
    Ok(sum / n)
}

/// Calculates the standard deviation of an i16 Vector.
/// 
/// # Returns
/// * `Ok(f64)` - The standard deviation
/// * `Err(StatsError)` - If the vector is empty
pub(super) fn std_i16(values: &[i16]) -> Result<f64, StatsError> {
    if values.is_empty() {
        return Err(StatsError::ZeroDivision);
    }

    let mean = values.iter().map(|&x| x as f64).sum::<f64>() / values.len() as f64;
    let variance = values.iter()
        .map(|&el| {
            let diff = el as f64 - mean;
            diff * diff
        })
        .sum::<f64>() / values.len() as f64;
    Ok(variance.sqrt())
}