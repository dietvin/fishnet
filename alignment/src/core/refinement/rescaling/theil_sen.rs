use crate::core::refinement::rescaling::helpers::{median, random_subset};
use crate::core::refinement::rescaling::{RescaleAlgo, RescaleFilterOptions};
use crate::error::core::refinement::rescale::{RescaleError, TheilSenError};


/// Theil–Sen-based rescaling algorithm.
///
/// Uses the Theil–Sen estimator to robustly fit a linear relationship between
/// normalized signal and expected levels. This method is resistant to outliers
/// by computing the median of pairwise slopes.
///
/// # Fields
/// * `filter_options` - Configuration controlling base filtering prior to fitting
/// * `max_points` - Maximum number of points used in slope estimation to bound
///                  computational complexity
///
/// # Construction
/// Use [`TheilSen::new`] to create an instance with explicit filtering parameters
/// and computational limits.
///
/// # Behavior
/// 1. Fits the model:
///    `levels ≈ scale_est * norm_signal + shift_est`
///    using a robust median-based estimator
/// 2. Converts the estimated parameters into updated normalization parameters:
///    * `new_scale = prev_scale / scale_est`
///    * `new_shift = prev_shift - prev_scale * shift_est / scale_est`
///
/// # Edge Cases
/// * If `scale_est == 0.0`, the previous parameters are returned unchanged
///   to avoid division by zero.
///
/// # When to Use
/// * Preferred when data contains outliers or heavy-tailed noise
/// * More computationally expensive than least squares
#[derive(Clone)]
pub struct TheilSen {
    filter_options: RescaleFilterOptions,
    max_points: usize
}

impl TheilSen {
    /// Creates a new Theil–Sen rescaling algorithm instance.
    ///
    /// # Arguments
    /// * `dwell_filter_lower_percentile` - Lower percentile bound for dwell filtering
    /// * `dwell_filter_upper_percentile` - Upper percentile bound for dwell filtering
    /// * `min_abs_level` - Minimum deviation from mean level required to retain a base
    /// * `n_bases_truncate` - Number of bases to remove from each end of the sequence
    /// * `min_num_filtered_levels` - Minimum number of bases required after filtering
    /// * `max_points` - Maximum number of points used in the estimator to limit
    ///                  computational cost
    ///
    /// # Returns
    /// A configured [`TheilSen`] instance.
    ///
    /// # Notes
    /// * `max_points` constrains the number of pairwise comparisons and prevents
    ///   quadratic blow-up for large datasets.
    /// * Filtering parameters are identical in semantics to those used by
    ///   [`LeastSquares`].
    pub fn new(
        dwell_filter_lower_percentile: f32,
        dwell_filter_upper_percentile: f32,
        min_abs_level: f32,
        n_bases_truncate: usize,
        min_num_filtered_levels: usize,
        max_points: usize
    ) -> Self {
        let filter_options = RescaleFilterOptions {
            dwell_filter_lower_percentile,
            dwell_filter_upper_percentile,
            min_abs_level,
            n_bases_truncate,
            min_num_filtered_levels
        };

        Self { filter_options, max_points }
    }
}


impl RescaleAlgo for TheilSen {
    fn rescale(
            &self,
            norm_signal: &[f32],
            levels_filtered: &[f32],
            prev_scale: f32,
            prev_shift: f32,
        ) -> Result<(f32, f32), RescaleError> {
        let (scale_est, shift_est) = theil_sen(
            norm_signal,
            levels_filtered,
            self.max_points
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

/// Computes linear regression using the Theil–Sen estimator.
///
/// Fits a model of the form: `y ≈ intercept + slope * x`
///
/// The slope is estimated as the median of pairwise slopes, and the intercept
/// as the median of residual intercepts. This method is robust to outliers.
///
/// # Arguments
/// * `x` - Input (independent variable) samples
/// * `y` - Output (dependent variable) samples
/// * `max_points` - Optional limit for subsampling input points (0 = use all)
///
/// # Returns
/// * `(slope, intercept)` - Parameters of the fitted line
///
/// # Errors
/// * `TheilSenError::LengthMismatch` - If `x` and `y` differ in length
/// * `TheilSenError::AllSlopesZero` - If no valid slopes can be computed
/// * `TheilSenError::MedianSlopeZero` - If the estimated slope is zero
/// * `TheilSenError::MedianCalcEmptyVec` - If median computation fails
pub(crate) fn theil_sen(
    x: &[f32], 
    y: &[f32],
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

    if median_slope.abs() < f32::EPSILON {
        return Err(TheilSenError::MedianSlopeZero);
    }

    // Compute the median intercept
    let mut intercepts = x.iter()
        .zip(y.iter())
        .map(|(x, y)| y - median_slope * x)
        .collect::<Vec<f32>>();
    
    intercepts.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let median_intercept = median(&intercepts)?;

    Ok((median_slope, median_intercept))
}