use crate::{
    core::refinement::{rescaling::least_squares::least_squares, rough_rescaling::{
        RoughRescaleAlgo,
        RoughRescaleOptions,
        prepare::prep_rough_rescale
    }}, error::core::refinement::rescale::RoughRescaleError, 
};


/// Least-squares-based rough rescaling algorithm.
///
/// Uses quantile summaries of normalized signal and expected levels, followed by
/// ordinary least squares (OLS) regression to estimate scaling parameters.
///
/// # Behavior
/// 1. Calls [`prep_rough_rescale`] to compute:
///    * quantiles of normalized signal
///    * quantiles of expected levels
/// 2. Fits:
///    `levels ≈ scale_est * norm_signal + shift_est`
/// 3. Converts estimates into updated normalization parameters:
///    * `new_scale = scale / scale_est`
///    * `new_shift = shift - scale * shift_est / scale_est`
///
/// # Edge Cases
/// * If `scale_est == 0.0`, returns the original `(scale, shift)`
///
/// # Characteristics
/// * Fast and simple
/// * Sensitive to outliers in quantiles (less robust than Theil–Sen)
#[derive(Clone)]
pub struct RoughLeastSquares {
    options: RoughRescaleOptions
}

impl RoughRescaleAlgo for RoughLeastSquares {
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

        let (scale_est, shift_est) = least_squares(
            &norm_signal_quantiles,
            &level_quantiles
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