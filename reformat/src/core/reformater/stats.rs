use crate::error::core::reformat::StatError;

/// Computes the arithmetic mean of the values.
///
/// # Arguments
/// * `values` - Slice of input values.
///
/// # Returns
/// The mean value.
///
/// # Errors
/// Returns `StatError::VecEmpty` if the input is empty.
pub(super) fn mean(values: &[f32]) -> Result<f32, StatError> {
    if values.is_empty() {
        return Err(StatError::VecEmpty);
    }
    let sum = values.iter().sum::<f32>();
    let n = values.len() as f32;
    Ok(sum / n)
}

/// Computes the population standard deviation using Welford’s 
/// algorithm in a single pass.
///
/// # Arguments
/// * `values` - Slice of input values.
///
/// # Returns
/// The standard deviation.
///
/// # Errors
/// Returns `StatError::VecEmpty` if the input is empty.
pub(super) fn std_dev(values: &[f32]) -> Result<f32, StatError> {
    if values.is_empty() {
        return Err(StatError::VecEmpty);
    }

    let mut mean = 0.0f32;
    let mut sum_sq_dev = 0.0f32;
    let mut count = 0usize;

    values.iter().for_each(|&el| {
        count += 1;
        let delta = el - mean;
        mean += delta / count as f32;
        let delta2 = el - mean;
        sum_sq_dev += delta * delta2;
    });

    let variance = sum_sq_dev / count as f32;
    Ok(variance.sqrt())
}

/// Computes the median of the values.
///
/// Sorts the values to determine the middle element(s).
///
/// # Arguments
/// * `values` - Slice of input values.
///
/// # Returns
/// The median value (middle element if odd length, mean of two
/// middle elements if even length).
///
/// # Errors
/// Returns `StatError::VecEmpty` if the input is empty.
pub(super) fn median(values: &[f32]) -> Result<f32, StatError> {
    if values.is_empty() {
        return Err(StatError::VecEmpty);
    }
    
    let mut sorted = values.to_vec();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    
    let n = sorted.len();
    if n % 2 == 1 {
        // Odd number of elements
        Ok(sorted[n / 2])
    } else {
        // Even number of elements - average the two middle values
        Ok((sorted[n / 2 - 1] + sorted[n / 2]) / 2.0)
    }
}

/// Computes the signal-to-noise ratio (SNR) in a single pass 
/// using Welford’s algorithm.
/// 
/// Also returns the mean and standard deviation to potentially reduce 
/// the number of calculations needed if the mean and/or standard deviation
/// is also requested. 
///
/// # Arguments
/// * `values` - Slice of input values.
///
/// # Returns
/// The mean, standard deviation and mean divided by standard deviation.
///
/// # Errors
/// * `StatError::VecEmpty` if input is empty.
/// * `StatError::ZeroDivision` if standard deviation is zero.
pub(super) fn signal_to_noise(values: &[f32]) -> Result<(f32, f32, f32), StatError> {
    if values.is_empty() {
        return Err(StatError::VecEmpty);
    }

    let mut mean = 0.0f32;
    let mut sum_sq_dev = 0.0f32;
    let mut count = 0usize;

    values.iter().for_each(|&el| {
        count += 1;
        let delta = el - mean;
        mean += delta / count as f32;
        let delta2 = el - mean;
        sum_sq_dev += delta * delta2;
    });

    let variance = sum_sq_dev / count as f32;
    let stdev = variance.sqrt();

    if stdev == 0.0 {
        return Err(StatError::ZeroDivision);
    }

    Ok((mean, stdev, mean / stdev))
}

/// Computes the mean and standard deviation in a single pass 
/// using Welford’s algorithm.
/// 
/// Implemented if signal to noise ratio is not needed to 
/// potentially avoid the zero division error while still only
/// requiring one pass for both stats.
/// 
/// Skips calculation of 
/// 
/// # Arguments
/// * `values` - Slice of input values.
///
/// # Returns
/// The mean and standard deviation.
///
/// # Errors
/// * `StatError::VecEmpty` if input is empty.
pub(super) fn mean_and_stdev(values: &[f32]) -> Result<(f32, f32), StatError> {
    if values.is_empty() {
        return Err(StatError::VecEmpty);
    }

    let mut mean = 0.0f32;
    let mut sum_sq_dev = 0.0f32;
    let mut count = 0usize;

    values.iter().for_each(|&el| {
        count += 1;
        let delta = el - mean;
        mean += delta / count as f32;
        let delta2 = el - mean;
        sum_sq_dev += delta * delta2;
    });

    let variance = sum_sq_dev / count as f32;
    let stdev = variance.sqrt();
    
    Ok((mean, stdev))
}

