use rand::seq::IteratorRandom;
use rand::rng;

use crate::{error::refinement_errors::rescale_errors::{LstsqError, QuantileCalcError, RescaleError, RoughRescaleError, TheilSenError}, logger::get_log_vector_sample, core::refinement::settings::RescaleAlgo};

/// Rescales a signal using least squares estimation.
///
/// This function adjusts the scaling and shift parameters of a signal by computing
/// quantiles of the signal and expected levels, then finding the best linear fit
/// using least squares regression.
///
/// # Arguments
/// * `scale` - The initial scale factor
/// * `shift` - The initial shift factor
/// * `seq_to_signal_map` - Mapping of sequence positions to signal indices
/// * `levels` - Expected levels for each position
/// * `signal` - The signal values to be rescaled
/// * `quantiles` - The quantile values to use for comparison (values between 0.0 and 1.0)
/// * `clip_bases` - Number of bases to clip from the beginning and end
/// * `use_base_center` - Whether to use the center of each base for quantile calculation
///
/// # Returns
/// A tuple of the optimized (shift, scale) values or an error
pub fn rough_rescale_lstsq(
    scale: f32, 
    shift: f32,
    seq_to_signal_map: &Vec<usize>,
    levels: &Vec<f32>,
    signal: &Vec<f32>,
    quantiles: &Vec<f32>,
    clip_bases: usize,
    use_base_center: bool
) -> Result<(f32, f32), RoughRescaleError> {
    log::trace!(
        "rough_rescale_lstsq input:  scale = {}, shift = {}, seq_to_signal_map = {}, levels = {}, signal = {}, quantiles = {}, clip_bases = {}, use_base_center = {}",
        scale, shift, 
        get_log_vector_sample(seq_to_signal_map, 10), 
        get_log_vector_sample(levels, 10), 
        get_log_vector_sample(signal, 10), 
        get_log_vector_sample(quantiles, 10), 
        clip_bases, use_base_center
    );
    let (norm_signal_quantiles, level_quantiles) = prep_rough_rescale(
        scale, 
        shift, 
        seq_to_signal_map, 
        levels, 
        signal, 
        quantiles, 
        clip_bases, 
        use_base_center
    )?;

    let (new_shift, new_scale) = least_squares(
        &norm_signal_quantiles, 
        &level_quantiles,
        shift,
        scale
    )?;

    log::debug!(
        "rough_rescale_lstsq output: new_scale = {}, new_shift = {}",
        new_scale, new_shift
    );

    Ok((new_shift, new_scale))
}


/// Rescales a signal using Theil-Sen estimation, which is more robust to outliers
/// than least squares.
///
/// This function adjusts the scaling and shift parameters of a signal by computing
/// quantiles of the signal and expected levels, then finding the best linear fit
/// using the Theil-Sen estimator.
///
/// # Arguments
/// * `scale` - The initial scale factor
/// * `shift` - The initial shift factor
/// * `seq_to_signal_map` - Mapping of sequence positions to signal indices
/// * `levels` - Expected levels for each position
/// * `signal` - The signal values to be rescaled
/// * `quantiles` - The quantile values to use for comparison (values between 0.0 and 1.0)
/// * `clip_bases` - Number of bases to clip from the beginning and end
/// * `use_base_center` - Whether to use the center of each base for quantile calculation
/// * `max_points` - Maximum number of points to use in Theil-Sen estimation
///
/// # Returns
/// A tuple of the optimized (shift, scale) values or an error
pub fn rough_rescale_theil_sen(
    scale: f32, 
    shift: f32,
    seq_to_signal_map: &Vec<usize>,
    levels: &Vec<f32>,
    signal: &Vec<f32>,
    quantiles: &Vec<f32>,
    clip_bases: usize,
    use_base_center: bool,
) -> Result<(f32, f32), RoughRescaleError> {
    log::trace!(
        "rough_rescale_theil_sen input:  scale = {}, shift = {}, seq_to_signal_map = {}, levels = {}, signal = {}, quantiles = {}, clip_bases = {}, use_base_center = {}",
        scale, shift, 
        get_log_vector_sample(seq_to_signal_map, 10), 
        get_log_vector_sample(levels, 10), 
        get_log_vector_sample(signal, 10), 
        get_log_vector_sample(quantiles, 10), 
        clip_bases, use_base_center
    );

    let (norm_signal_quantiles, level_quantiles) = prep_rough_rescale(
        scale, 
        shift, 
        seq_to_signal_map, 
        levels, 
        signal, 
        quantiles, 
        clip_bases, 
        use_base_center
    )?;

    let (new_shift, new_scale) = theil_sen(
        &norm_signal_quantiles, 
        &level_quantiles, 
        shift,
        scale,
        // 0 to prevent subsetting (not wanted in rough 
        // rescaling since here only a handfull of 
        // values (quantiles) are used)
        0 
    )?;

    log::debug!(
        "rough_rescale_theil_sen output: new_scale = {}, new_shift = {}",
        new_scale, new_shift
    );

    Ok((new_shift, new_scale))
}


/// Prepares data for rough rescaling by calculating quantiles for both
/// the normalized signal and expected levels.
///
/// # Arguments
/// * `scale` - The scale factor to apply to the signal
/// * `shift` - The shift factor to apply to the signal
/// * `seq_to_signal_map` - Mapping of sequence positions to signal indices
/// * `levels` - Expected levels for each position
/// * `signal` - The signal values to be rescaled
/// * `quantiles` - The quantile values to use for comparison (values between 0.0 and 1.0)
/// * `clip_bases` - Number of bases to clip from the beginning and end
/// * `use_base_center` - Whether to use the center of each base for quantile calculation
///
/// # Returns
/// A tuple of vectors containing the quantiles for normalized signal and expected levels,
/// or an error if the preparation fails
fn prep_rough_rescale(
    scale: f32, 
    shift: f32,
    seq_to_signal_map: &[usize],
    levels: &Vec<f32>,
    signal: &Vec<f32>,
    quantiles: &Vec<f32>,
    clip_bases: usize,
    use_base_center: bool
) -> Result<(Vec<f32>, Vec<f32>), RoughRescaleError> {
    let (clip_start, clip_end) = if clip_bases > 0 && levels.len() > clip_bases * 2 {
        (clip_bases, levels.len() - clip_bases)
    } else {
        (0, signal.len())
    };

    let norm_signal = if use_base_center {
        seq_to_signal_map
            .windows(2) // Iterate over the start & end indx of each base
            .map(|window| (window[0] + window[1]) / 2) // Calculate the center index
            .filter(|&idx| idx < signal.len()) // Ignore indices that are out of bounds
            .map(|idx| (signal[idx] - shift) / scale) // Normalize the measurements
            .skip(clip_start) // Clip the start
            .take(clip_end - clip_start) // Clip the end
            .collect::<Vec<f32>>()
    } else if !seq_to_signal_map.is_empty() {
        let start = seq_to_signal_map[0];
        let end = seq_to_signal_map[seq_to_signal_map.len() - 1].min(signal.len());
        
        signal[start..end].iter()
            .map(|&val| (val - shift) / scale)
            .skip(clip_start)
            .take(clip_end - clip_start)
            .collect::<Vec<f32>>()
    } else {
        return Err(RoughRescaleError::PrepError("Empty seq_to_sig_map".to_string()));
    };

    // Clip unwanted information from the expected levels
    let clipped_levels = &levels[clip_start.min(levels.len())..clip_end.min(levels.len())];

    // Calculate quantiles for normalized signal
    let norm_signal_quantiles = calculate_quantiles(&norm_signal, quantiles)?;

    // Calculate quantiles for levels
    let level_quantiles = calculate_quantiles(clipped_levels, quantiles)?;
    
    Ok((norm_signal_quantiles, level_quantiles))
}


/// Calculates quantiles for a given data set.
///
/// # Arguments
/// * `data` - The data set to calculate quantiles for
/// * `quantiles` - The quantile values to calculate (values between 0.0 and 1.0)
///
/// # Returns
/// A vector of calculated quantile values, or an error if the calculation fails
fn calculate_quantiles(data: &[f32], quantiles: &[f32]) -> Result<Vec<f32>, QuantileCalcError> {
    if data.len() == 0 {
        return Err(QuantileCalcError::EmptyDataVec);
    } else if quantiles.len() == 0 {
        return Err(QuantileCalcError::EmptyQuantVec);
    }

    let mut sorted_data = data.to_vec();
    sorted_data.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

    // For each requested quantile value, calculate the corresponding value from sorted data
    quantiles.iter()
        .map(|&q| calculate_single_quantile(&sorted_data, q))
        .collect::<Result<Vec<f32>, QuantileCalcError>>()
}

/// Calculates a single quantile from a sorted vector.
/// 
/// # Arguments
/// * `sorted_data` - The sorted data set to calculate the quantile for
/// * `q` - The quantile calculate (values between 0.0 and 1.0)
/// 
/// # Returns 
/// The calculated quantile value, or an error if the calculation fails
fn calculate_single_quantile(sorted_data: &[f32], q: f32) -> Result<f32, QuantileCalcError> {
    if q > 1.0 || q < 0.0 {
        return Err(QuantileCalcError::InvalidQuant(q));
    }
    
    // Calculate the position in the sorted array (can be fractional)
    let pos = q * (sorted_data.len() - 1) as f32;

    // Get indices for interpolation
    let idx_floor = pos.floor() as usize;
    let idx_ceil = pos.ceil() as usize;

    // If the position is exactly at an index, return that value
    if idx_floor == idx_ceil {
        Ok(sorted_data[idx_floor])
    } 
    // Otherwise, linearly interpolate between the two nearest values
    else {
        let weight_ceil = pos - idx_floor as f32;   // Fractional part of position
        let weight_floor = 1.0 - weight_ceil;       // Complement of fractional part
        // Weighted average of the two values
        Ok(weight_floor * sorted_data[idx_floor] + weight_ceil * sorted_data[idx_ceil])
    }
}


/// Performs least squares linear regression on two sets of data points.
///
/// This function implements a simple linear regression: y = shift_est + scale_est * x
///
/// # Arguments
/// * `x` - The x-coordinates of the data points
/// * `y` - The y-coordinates of the data points
///
/// # Returns
/// A tuple of (new_shift, new_scale) representing the updated shift and scale parameters
fn least_squares(
    x: &Vec<f32>, 
    y: &Vec<f32>, 
    shift: f32, 
    scale: f32
) -> Result<(f32, f32), LstsqError> {
    // Ensure we only process the overlapping portion of x and y
    if x.len() != y.len() {
        return Err(LstsqError::LengthMismatch(x.len(), y.len()));
    }
    let n = x.len();

    // Calculate the means of x and y values
    let x_mean = x[0..n].iter().sum::<f32>() / n as f32;
    let y_mean = y[0..n].iter().sum::<f32>() / n as f32;

    // Calculate the slope using the formula:
    // scale_est = sum((x_i - x_mean) * (y_i - y_mean)) / sum((x_i - x_mean)²)
    let mut numerator = 0.0;
    let mut denominator = 0.0;

    for i in 0..n {
        let x_diff = x[i] - x_mean;
        numerator += x_diff * (y[i] - y_mean);
        denominator += x_diff * x_diff;
    }

    // Account for zero division
    let scale_est = if denominator.abs() < f32::EPSILON {
        0.0
    } else {
        numerator / denominator
    };

    // Calculate the y-intercept using the formula: shift_est = y_mean - scale_est * x_mean
    let shift_est = y_mean - scale_est * x_mean;

    // Return original values if scale_est is zero
    if scale_est == 0.0 {
        return Ok((shift, scale));
    }

    // Calculate new shift and scale
    let new_shift = shift - (scale * shift_est / scale_est);
    let new_scale = scale / scale_est;

    Ok((new_shift, new_scale))
}


/// Performs Theil-Sen linear regression on two sets of data points.
///
/// Theil-Sen is a robust linear regression method that is less sensitive to outliers
/// than least squares regression.
///
/// # Arguments
/// * `x` - The x-coordinates of the data points
/// * `y` - The y-coordinates of the data points
/// * `max_points` - Maximum number of points to use in the estimation
///
/// # Returns
/// A tuple of (new_shift, new_scale) representing the updated shift and
/// scale parameters
fn theil_sen(
    x: &Vec<f32>, 
    y: &Vec<f32>, 
    shift: f32,
    scale: f32,
    max_points: usize
) -> Result<(f32, f32), TheilSenError> {
    if x.len() != y.len() {
        return Err(TheilSenError::LengthMismatch(x.len(), y.len()));
    }
    let n = x.len();

    let num_slopes = if max_points > 0 && n > max_points {
        max_points * (max_points - 1) / 2
    } else {
        n * (n - 1) / 2
    };

    let mut slopes = Vec::with_capacity(num_slopes);

    if max_points > 0 && n > max_points {
        let subsampled_indices = random_subset(n, max_points);
        for i in 0..max_points {
            
            let xi = x[subsampled_indices[i]];
            let yi = y[subsampled_indices[i]];

            for j in i+1..max_points {
                let delta_x = x[subsampled_indices[j]] - xi;
                if delta_x != 0.0 {
                    slopes.push((y[subsampled_indices[j]] - yi) / delta_x); // delta_y / delta_x
                }
            }
        }
    } else {
        for i in 0..n {
            let xi = x[i];
            let yi = y[i];
            for j in i+1..n {
                let delta_x = x[j] - xi;
                if delta_x != 0.0 {
                    slopes.push((y[j] - yi) / delta_x);
                }
            }
        }
    }

    if slopes.is_empty() {
        return Err(TheilSenError::AllSlopesZero);
    }

    // Compute median slope
    slopes.sort_by(|a: &f32, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let median_slope = median(&slopes)?;

    if median_slope == 0.0 {
        return Err(TheilSenError::MedianSlopeZero);
    }

    // Compute the median intercept
    let mut intercepts = x.iter()
        .zip(y.iter())
        .map(|(x, y)| y - median_slope * x)
        .collect::<Vec<f32>>();
    
    intercepts.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let median_intercept = median(&intercepts)?;

    let shift_est = -median_intercept / median_slope;
    let scale_est = 1.0 / median_slope;
    
    // Calculate new shift and scale
    let new_shift = shift + (shift_est * scale);
    let new_scale = scale * scale_est;
    
    Ok((new_shift, new_scale))
}


/// Returns a random subset of indices from a vector of a given size.
///
/// # Arguments
/// * `vec_len` - The length of the vector to sample from
/// * `downsampled_len` - The number of unique indices to return
///
/// # Returns
/// A vector of unique random indices
fn random_subset(vec_len: usize, downsampled_len: usize) -> Vec<usize> {
    (0..vec_len).choose_multiple(&mut rng(), downsampled_len)
}

/// Calculates the median of a **sorted** vector of floats.
///
/// # Arguments
/// * `vec` - A sorted vector of f32 values
///
/// # Returns
/// The median value, or an error if the calculation fails
fn median(vec: &Vec<f32>) -> Result<f32, TheilSenError> {
    let len = vec.len();
    if len == 0 {
        return Err(TheilSenError::MedianCalcEmptyVec);
    }
    
    if len % 2 == 0 {
        Ok((vec[len / 2 - 1] + vec[len / 2]) / 2.0)
    } else {
        Ok(vec[len / 2])
    }
}


/// Recalibrates signal normalization parameters (shift and scale) based on the relationship
/// between observed signal and expected reference levels.
///
/// This function implements two different rescaling algorithms (least squares and Theil-Sen)
/// to recalibrate the parameters used to convert raw signal measurements to normalized values.
/// It filters bases based on dwell time and signal intensity deviation to ensure robust estimation.
///
/// # Arguments
/// * `scale` - Current scale parameter for signal normalization
/// * `shift` - Current shift parameter for signal normalization
/// * `seq_to_signal_map` - Mapping of sequence positions to signal indices
/// * `levels` - Expected reference levels for each base
/// * `signal` - Raw signal measurements
/// * `rescale_algo` - Algorithm configuration to use for rescaling (TheilSen or LeastSquares)
///
/// # Returns
/// * `Ok((new_shift, new_scale))` - Updated shift and scale parameters for signal normalization
/// * `Err(RescaleError)` - Error indicating why rescaling failed
///
/// # Algorithm Details
/// 1. Calculates dwell time (number of signal points) for each base
/// 2. Filters bases by dwell time to remove areas with poor signal assignment
/// 3. Filters bases with low deviation from mean level to focus on informative signal
/// 4. Calculates mean signal for each filtered base
/// 5. Applies the selected algorithm (Theil-Sen or least squares) to estimate new parameters
///
/// The normalized signal can be calculated as: normalized = (raw_signal - shift) / scale
///
/// # Errors
/// * `EmptyMap` - The sequence to signal map is empty
/// * `InvalidLevelsLen` - The levels vector length doesn't match the expected length
/// * `BelowMinNumFiltered` - Too few bases passed the filtering criteria
/// * Algorithm-specific errors from least_squares or theil_sen functions
pub fn rescale(
    scale: f32, 
    shift: f32,
    seq_to_signal_map: &Vec<usize>,
    levels: &Vec<f32>,
    signal: &Vec<f32>,
    rescale_algo: &RescaleAlgo
) -> Result<(f32, f32), RescaleError> {
    log::debug!(
        "rescale input: scale = {}, shift = {}, seq_to_signal_map = {}, levels = {}, signal = {}, rescale_algo = {:?}",
        scale,
        shift,
        get_log_vector_sample(seq_to_signal_map, 10),
        get_log_vector_sample(levels, 10),
        get_log_vector_sample(signal, 10),
        rescale_algo
    );

    // Check if map and levels have a valid length
    let seq_to_signal_map_len = seq_to_signal_map.len();
    if seq_to_signal_map_len == 0 {
        return Err(RescaleError::EmptyMap);
    } else if levels.len() != seq_to_signal_map_len - 1 {
        return Err(RescaleError::InvalidLevelsLen(levels.len(), seq_to_signal_map_len - 1));
    }

    // Get the used parameters from the RescaleAlgo enum
    let (dwell_filter_lower_percentile,
        dwell_filter_upper_percentile,
        min_abs_level,
        n_bases_truncate,
        min_num_filtered_levels,
        max_points) = match *rescale_algo {
            RescaleAlgo::TheilSen { 
                dwell_filter_lower_percentile, 
                dwell_filter_upper_percentile,
                min_abs_level, 
                n_bases_truncate, 
                min_num_filtered_levels, 
                max_points 
            } => (
                dwell_filter_lower_percentile,
                dwell_filter_upper_percentile,
                min_abs_level,
                n_bases_truncate,
                min_num_filtered_levels,
                max_points
            ),
            RescaleAlgo::LeastSquares { 
                dwell_filter_lower_percentile, 
                dwell_filter_upper_percentile, 
                min_abs_level, 
                n_bases_truncate, 
                min_num_filtered_levels 
            } => (
                dwell_filter_lower_percentile,
                dwell_filter_upper_percentile,
                min_abs_level,
                n_bases_truncate,
                min_num_filtered_levels,
                0
            )
        };

    // Calculate the dwells (number of measurements) for each base
    let dwells = seq_to_signal_map
        .windows(2)
        .map(|window| (window[1] - window[0]) as f32)
        .collect::<Vec<f32>>();

    // Calculate the upper and lower percentile values of the dwells
    let (dwell_lower_percentile_value, dwell_upper_percentile_value) = get_upper_lower_quantiles(
        &dwells, 
        dwell_filter_lower_percentile,
        dwell_filter_upper_percentile
    )?;

    let n_bases = levels.len();
    let levels_mean = levels.iter().sum::<f32>() / (levels.len() as f32);
    
    let mut mean_signal_filtered = Vec::with_capacity(n_bases);
    let mut levels_filtered = Vec::with_capacity(n_bases);
    
    let (start_base_idx, end_base_idx) = if n_bases_truncate == 0 {
        (0, n_bases)
    } else {
        (n_bases_truncate, n_bases - n_bases_truncate)
    };
    
    // Iterate over all bases, filtering out bases that are not fitting bases on the given parameters
    for base_idx in start_base_idx..end_base_idx {
        let dwell = dwells[base_idx];
        // Ignore bases where the dwell time is shorter than the dwell_lower_percentile_value
        // or longer than the dwell_upper_percentile_value
        if dwell <= dwell_lower_percentile_value || dwell >= dwell_upper_percentile_value {
            continue;
        }
        
        // Ignore bases where the expected signal intensity of the current base 
        // doesn't deviate much from the mean of the expected signal intensity
        let expected_intensity = levels[base_idx];
        if (expected_intensity - levels_mean).abs() <= min_abs_level {
            continue;
        }
        
        // Add the information of valid bases
        let mean_signal_intensity = signal[seq_to_signal_map[base_idx]..seq_to_signal_map[base_idx + 1]]
            .iter()
            .sum::<f32>() / dwell;
        mean_signal_filtered.push(mean_signal_intensity);
        levels_filtered.push(expected_intensity);
    }

    // Check if enough bases passed filtering
    if mean_signal_filtered.len() < min_num_filtered_levels {
        return Err(RescaleError::BelowMinNumFiltered(
            mean_signal_filtered.len(), 
            min_num_filtered_levels
        ));
    }
    
    // Normalize the signal with the previous shift and scale parameters
    let norm_signal = mean_signal_filtered
        .iter()
        .map(|el| (el - shift) / scale)
        .collect::<Vec<f32>>();

    // Calculate the new shift and scale parameters and return these
    let (new_shift, new_scale) = match rescale_algo {
        RescaleAlgo::TheilSen { .. } => theil_sen(
            &norm_signal, 
            &levels_filtered, 
            shift, 
            scale, 
            max_points
        )?,
        RescaleAlgo::LeastSquares { .. } => least_squares(
            &norm_signal, 
            &levels_filtered,
            shift,
            scale
        )?
    };

    log::debug!(
        "rescale output: new_scale = {}, new_shift = {}",
        new_scale,
        new_shift
    );

    Ok((new_shift, new_scale))
}


/// Calculate given upper and lower quantile values.
/// 
/// This function calculates two quantiles (an upper and lower) from given data.
/// 
/// # Arguments
/// * `data` - Data from which to calculate the quantile values
/// * `lower_quantiles` - The lower quantile for which to calculate the value
/// * `upper_quantiles` - The upper quantile for which to calculate the value
/// 
/// # Returns
/// A tuple containing the lower and upper quantile values, or an error if the data
/// is empty or the quantile calculation fails
fn get_upper_lower_quantiles(
    data: &Vec<f32>, 
    lower_quantiles: f32, 
    upper_quantiles: f32
) -> Result<(f32, f32), QuantileCalcError> {
    if data.len() == 0 {
        return Err(QuantileCalcError::EmptyDataVec);
    }

    let mut sorted_data = data.to_vec();
    sorted_data.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

    Ok((
        calculate_single_quantile(&sorted_data, lower_quantiles)?,
        calculate_single_quantile(&sorted_data, upper_quantiles)?
    ))
}



#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;

    #[test]
    fn test_calculate_quantiles() {
        // Test case 1: Simple vector with exact quantiles
        let data = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let quantiles = vec![0.0, 0.25, 0.5, 0.75, 1.0];
        let result = calculate_quantiles(&data, &quantiles).unwrap();
        assert_eq!(result, vec![1.0, 2.0, 3.0, 4.0, 5.0]);

        // Test case 2: Vector with interpolated quantiles
        let data = vec![10.0, 20.0, 30.0, 40.0, 50.0];
        let quantiles = vec![0.0, 0.3, 0.6, 1.0];
        let result = calculate_quantiles(&data, &quantiles).unwrap();
        assert_relative_eq!(result[0], 10.0);
        assert_relative_eq!(result[1], 22.0);
        assert_relative_eq!(result[2], 34.0);
        assert_relative_eq!(result[3], 50.0);

        // Test case 3: Unsorted data
        let data = vec![5.0, 3.0, 1.0, 4.0, 2.0];
        let quantiles = vec![0.0, 0.5, 1.0];
        let result = calculate_quantiles(&data, &quantiles).unwrap();
        assert_eq!(result, vec![1.0, 3.0, 5.0]);
        
        // Test case 4: Empty data
        let data: Vec<f32> = vec![];
        let quantiles = vec![0.5];
        let result = calculate_quantiles(&data, &quantiles);
        assert!(result.is_err());
        
        // Test case 5: Empty quantiles
        let data = vec![1.0, 2.0, 3.0];
        let quantiles: Vec<f32> = vec![];
        let result = calculate_quantiles(&data, &quantiles);
        assert!(result.is_err());
    }

    // #[test]
    // fn test_least_squares() {
    //     // Test case 1: Perfect linear relationship
    //     let x = vec![1.0, 2.0, 3.0, 4.0, 5.0];
    //     let y = vec![2.0, 4.0, 6.0, 8.0, 10.0];
    //     let (shift_est, scale_est) = least_squares(&x, &y).unwrap();
    //     assert_relative_eq!(shift_est, 0.0, epsilon = EPSILON);
    //     assert_relative_eq!(scale_est, 2.0, epsilon = EPSILON);

    //     // Test case 2: Linear relationship with intercept
    //     let x = vec![1.0, 2.0, 3.0, 4.0, 5.0];
    //     let y = vec![3.0, 5.0, 7.0, 9.0, 11.0];
    //     let (shift_est, scale_est) = least_squares(&x, &y).unwrap();
    //     assert_relative_eq!(shift_est, 1.0, epsilon = EPSILON);
    //     assert_relative_eq!(scale_est, 2.0, epsilon = EPSILON);

    //     // Test case 3: Negative slope
    //     let x = vec![1.0, 2.0, 3.0, 4.0, 5.0];
    //     let y = vec![10.0, 8.0, 6.0, 4.0, 2.0];
    //     let (shift_est, scale_est) = least_squares(&x, &y).unwrap();
    //     assert_relative_eq!(shift_est, 12.0, epsilon = EPSILON);
    //     assert_relative_eq!(scale_est, -2.0, epsilon = EPSILON);

    //     // Test case 4: Length mismatch
    //     let x = vec![1.0, 2.0, 3.0];
    //     let y = vec![2.0, 4.0];
    //     let result = least_squares(&x, &y);
    //     assert!(result.is_err());
    //     if let Err(LstsqError::LengthMismatch(x_len, y_len)) = result {
    //         assert_eq!(x_len, 3);
    //         assert_eq!(y_len, 2);
    //     } else {
    //         panic!("Expected LengthMismatch error");
    //     }

    //     // Test case 5: Constant x values (zero slope)
    //     let x = vec![5.0, 5.0, 5.0, 5.0];
    //     let y = vec![1.0, 2.0, 3.0, 4.0];
    //     let (shift_est, scale_est) = least_squares(&x, &y).unwrap();
    //     assert_relative_eq!(shift_est, 2.5, epsilon = EPSILON);
    //     assert_relative_eq!(scale_est, 0.0, epsilon = EPSILON);
    // }

    // #[test]
    // fn test_theil_sen() {
    //     // Test case 1: Perfect linear relationship
    //     let x = vec![1.0, 2.0, 3.0, 4.0, 5.0];
    //     let y = vec![2.0, 4.0, 6.0, 8.0, 10.0];
    //     let max_points = 5;
    //     let (shift_est, scale_est) = theil_sen(&x, &y, max_points).unwrap();
    //     assert_relative_eq!(shift_est, 0.0, epsilon = 0.01);
    //     assert_relative_eq!(scale_est, 0.5, epsilon = 0.01);

    //     // Test case 2: Linear relationship with outlier
    //     let x = vec![1.0, 2.0, 3.0, 4.0, 5.0];
    //     let y = vec![3.0, 5.0, 20.0, 9.0, 11.0]; // Outlier at index 2
    //     let max_points = 5;
    //     let (shift_est, scale_est) = theil_sen(&x, &y, max_points).unwrap();
    //     // Theil-Sen should be robust to the outlier
    //     assert_relative_eq!(shift_est, 1.0, epsilon = 0.1);
    //     assert_relative_eq!(scale_est, 0.5, epsilon = 0.1);

    //     // Test case 3: Length mismatch
    //     let x = vec![1.0, 2.0, 3.0];
    //     let y = vec![2.0, 4.0];
    //     let max_points = 3;
    //     let result = theil_sen(&x, &y, max_points);
    //     assert!(result.is_err());
    //     if let Err(TheilSenError::LengthMismatch(x_len, y_len)) = result {
    //         assert_eq!(x_len, 3);
    //         assert_eq!(y_len, 2);
    //     } else {
    //         panic!("Expected LengthMismatch error");
    //     }

    //     // Test case 4: Subsampling
    //     let x = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0];
    //     let y = vec![2.0, 4.0, 6.0, 8.0, 10.0, 12.0, 14.0, 16.0, 18.0, 20.0];
    //     let max_points = 5; // Subsample to 5 points
    //     let (shift_est, scale_est) = theil_sen(&x, &y, max_points).unwrap();
    //     // Should still get reasonable estimates
    //     assert!(shift_est < 2.0 && shift_est > -2.0);
    //     assert!(scale_est > 0.4 && scale_est < 0.6);
    // }

    #[test]
    fn test_random_subset() {
        // Test case 1: Subset of the same size as the vector
        let vec_len = 5;
        let downsampled_len = 5;
        let subset = random_subset(vec_len, downsampled_len);
        assert_eq!(subset.len(), downsampled_len);
        // All elements should be unique and in range
        let mut seen = vec![false; vec_len];
        for &idx in &subset {
            assert!(idx < vec_len);
            assert!(!seen[idx]);
            seen[idx] = true;
        }
        assert!(seen.iter().all(|&x| x));

        // Test case 2: Smaller subset
        let vec_len = 10;
        let downsampled_len = 3;
        let subset = random_subset(vec_len, downsampled_len);
        assert_eq!(subset.len(), downsampled_len);
        // All elements should be unique and in range
        let mut seen = vec![false; vec_len];
        for &idx in &subset {
            assert!(idx < vec_len);
            assert!(!seen[idx]);
            seen[idx] = true;
        }
        assert_eq!(seen.iter().filter(|&&x| x).count(), downsampled_len);

        // Test case 3: Empty subset
        let vec_len = 5;
        let downsampled_len = 0;
        let subset = random_subset(vec_len, downsampled_len);
        assert_eq!(subset.len(), 0);
    }

    #[test]
    fn test_median() {
        // Test case 1: Odd number of elements
        let data = vec![1.0, 3.0, 5.0, 7.0, 9.0];
        let result = median(&data).unwrap();
        assert_eq!(result, 5.0);

        // Test case 2: Even number of elements
        let data = vec![1.0, 3.0, 5.0, 7.0];
        let result = median(&data).unwrap();
        assert_eq!(result, 4.0);

        // Test case 3: Empty vector
        let data: Vec<f32> = vec![];
        let result = median(&data);
        assert!(result.is_err());
        match result {
            Err(TheilSenError::MedianCalcEmptyVec) => {},
            _ => panic!("Expected MedianCalcEmptyVec error"),
        }

        // Test case 4: Single element
        let data = vec![42.0];
        let result = median(&data).unwrap();
        assert_eq!(result, 42.0);

        // Test case 5: Unsorted data (should work correctly only with sorted data)
        let data = vec![5.0, 1.0, 3.0, 2.0, 4.0];
        // Our implementation expects sorted data, so the result will be incorrect
        let result = median(&data).unwrap();
        assert_eq!(result, 3.0); // This is element at index 2, not the actual median
    }
}