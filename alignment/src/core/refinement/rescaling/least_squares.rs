use crate::{
    core::refinement::rescaling::{RescaleAlgo, RescaleFilterOptions}, error::core::refinement::rescale::{LeastSquaresError, RescaleError},
};


/// Least-squares-based rescaling algorithm.
///
/// Estimates a linear relationship between normalized signal and expected levels
/// using ordinary least squares (OLS). This method minimizes squared residuals
/// and is efficient but sensitive to outliers.
///
/// # Fields
/// * `filter_options` - Configuration controlling base filtering prior to fitting
///
/// # Construction
/// Use [`LeastSquares::new`] to create an instance with explicit filtering parameters.
///
/// # Behavior
/// 1. Fits the model:
///    `levels ≈ scale_est * norm_signal + shift_est`
/// 2. Converts the estimated parameters into updated normalization parameters:
///    * `new_scale = prev_scale / scale_est`
///    * `new_shift = prev_shift - prev_scale * shift_est / scale_est`
///
/// # Edge Cases
/// * If `scale_est == 0.0`, the previous parameters are returned unchanged
///   to avoid division by zero.
///
/// # When to Use
/// * Suitable when noise is approximately Gaussian
/// * Not robust to outliers in signal or level estimates
#[derive(Clone)]
pub struct LeastSquares {
    filter_options: RescaleFilterOptions
}

impl LeastSquares {
    /// Creates a new least-squares rescaling algorithm instance.
    ///
    /// # Arguments
    /// * `dwell_filter_lower_percentile` - Lower percentile bound for dwell filtering
    /// * `dwell_filter_upper_percentile` - Upper percentile bound for dwell filtering
    /// * `min_abs_level` - Minimum deviation from mean level required to retain a base
    /// * `n_bases_truncate` - Number of bases to remove from each end of the sequence
    /// * `min_num_filtered_levels` - Minimum number of bases required after filtering
    ///
    /// # Returns
    /// A configured [`LeastSquares`] instance.
    ///
    /// # Notes
    /// These parameters directly populate [`RescaleFilterOptions`] and control
    /// preprocessing prior to regression.
    pub fn new(
        dwell_filter_lower_percentile: f32,
        dwell_filter_upper_percentile: f32,
        min_abs_level: f32,
        n_bases_truncate: usize,
        min_num_filtered_levels: usize
    ) -> Self {
        let filter_options = RescaleFilterOptions {
            dwell_filter_lower_percentile,
            dwell_filter_upper_percentile,
            min_abs_level,
            n_bases_truncate,
            min_num_filtered_levels
        };
        Self { filter_options }
    }
}

impl RescaleAlgo for LeastSquares {
    fn rescale(
        &self,
        norm_signal: &[f32],
        levels_filtered: &[f32],
        prev_scale: f32,
        prev_shift: f32,
    ) -> Result<(f32, f32), RescaleError> {
        let (scale_est, shift_est) = least_squares(
            norm_signal,
            levels_filtered
        )?;

        // Return original values if scale_est is zero
        if scale_est == 0.0 {
            return Ok((prev_scale, prev_shift));
        }

        // Calculate new shift and scale
        let new_scale = prev_scale / scale_est;
        let new_shift = prev_shift - prev_scale * shift_est / scale_est;

        Ok((new_scale, new_shift))
    }

    fn filter_options(&self) -> &RescaleFilterOptions {
        &self.filter_options
    }
}


/// Computes a simple linear regression using ordinary least squares.
///
/// Fits a model of the form:
///     y ≈ intercept + slope * x
///
/// # Arguments
/// * `x` - Input (independent variable) samples
/// * `y` - Output (dependent variable) samples
///
/// # Returns
/// * `(slope, intercept)` - Parameters of the fitted line
///
/// # Errors
/// * `LeastSquaresError::LengthMismatch` - If `x` and `y` differ in length
/// * `LeastSquaresError::ZeroDivision` - If variance of `x` is zero (all x are equal)
pub(crate) fn least_squares(
    x: &[f32], 
    y: &[f32]
) -> Result<(f32, f32), LeastSquaresError> {
    // Ensure we only process the overlapping portion of x and y
    if x.len() != y.len() {
        return Err(LeastSquaresError::LengthMismatch(x.len(), y.len()));
    }
    let n = x.len();

    // Calculate the means of x and y values
    let x_mean = x.iter().sum::<f32>() / n as f32;
    let y_mean = y.iter().sum::<f32>() / n as f32;

    // Calculate the slope using the formula:
    // slope = sum((x_i - x_mean) * (y_i - y_mean)) / sum((x_i - x_mean)²)
    let mut numerator = 0.0;
    let mut denominator = 0.0;

    for i in 0..n {
        let x_diff = x[i] - x_mean;
        numerator += x_diff * (y[i] - y_mean);
        denominator += x_diff * x_diff;
    }

    if denominator.abs() < f32::EPSILON {
        return Err(LeastSquaresError::ZeroDivision);
    }
    let slope = numerator / denominator;

    // Calculate the y-intercept using the formula: intercept = y_mean - slope * x_mean
    let intercept = y_mean - slope * x_mean;

    Ok((slope, intercept))
}
