use crate::{
    core::refinement::{
        rescaling::theil_sen::theil_sen,
        rough_rescaling::{
            RoughRescaleAlgo,
            RoughRescaleOptions,
            prepare::prep_rough_rescale
        }
    },
    error::core::refinement::rescale::RoughRescaleError
};


/// Theil–Sen-based rough rescaling algorithm.
///
/// Uses quantile summaries combined with a robust Theil–Sen estimator to
/// determine scaling parameters. This approach is more resistant to outliers
/// than least squares.
///
/// # Behavior
/// 1. Calls [`prep_rough_rescale`] to compute quantile summaries
/// 2. Fits:
///    `levels ≈ scale_est * norm_signal + shift_est`
///    using median-based slope estimation
/// 3. Converts estimates into updated normalization parameters:
///    * `new_scale = scale / scale_est`
///    * `new_shift = shift - scale * shift_est / scale_est`
///
/// # Parameters
/// * Internally sets `max_points = 0` to disable subsampling, since the number
///   of quantile points is already small
///
/// # Edge Cases
/// * If `scale_est == 0.0`, returns the original `(scale, shift)`
///
/// # Characteristics
/// * More robust to outliers than [`RoughLeastSquares`]
/// * Slightly higher computational cost, but negligible due to small input size
#[derive(Clone)]
pub struct RoughTheilSen {
    options: RoughRescaleOptions
}

impl RoughRescaleAlgo for RoughTheilSen {
    fn new(
        quantiles: Vec<f32>,
        clip_bases: usize,
        use_base_center: bool
    ) -> Self {
        let options = RoughRescaleOptions {
            quantiles,
            clip_bases,
            use_base_center
        };
        Self { options }
    }

    fn rough_rescale(
            &self,
            scale: f32,
            shift: f32,
            seq_to_signal_map: &[usize],
            levels: &[f32],
            signal: &[f32]
        ) -> Result<(f32, f32), RoughRescaleError> {
        let (norm_signal_quantiles, level_quantiles) = prep_rough_rescale(
            scale,
            shift,
            seq_to_signal_map,
            levels,
            signal,
            &self.options.quantiles,
            self.options.clip_bases,
            self.options.use_base_center
        )?;

        let (scale_est, shift_est) = theil_sen(
            &norm_signal_quantiles,
            &level_quantiles,
            0
        )?;

        // Return original values if scale_est is zero
        if scale_est == 0.0 {
            return Ok((scale, shift));
        }

        // Calculate new shift and scale
        let new_shift = shift - scale * shift_est / scale_est;
        let new_scale = scale / scale_est;

        Ok((new_scale, new_shift))
    
    }

    fn options(&self) -> &RoughRescaleOptions {
        &self.options
    }
}