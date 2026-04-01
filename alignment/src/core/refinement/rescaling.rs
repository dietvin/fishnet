use crate::{
    core::refinement::rescaling::filter::filter_bases, error::core::refinement::rescale::RescaleError, 
};

pub mod least_squares;
pub mod theil_sen;
mod filter;
mod helpers;


/// Trait defining a rescaling strategy for signal normalization.
///
/// Implementors encapsulate:
/// * A statistical method for estimating linear transformation parameters
/// * The filtering configuration required to prepare input data
///
/// The rescaling operates on a linear model relating normalized signal to
/// expected levels:
///
/// `levels ≈ scale_est * norm_signal + shift_est`
///
/// where `norm_signal = (raw_signal - prev_shift) / prev_scale`.
///
/// The estimated parameters (`scale_est`, `shift_est`) are then used to update
/// the original normalization parameters (`prev_scale`, `prev_shift`).
///
/// # Required Methods
///
/// ## `rescale`
/// Computes updated normalization parameters based on filtered data.
///
/// * `norm_signal` - Mean signal per base, already normalized using previous parameters
/// * `levels_filtered` - Corresponding expected reference levels
/// * `prev_scale` - Previous normalization scale
/// * `prev_shift` - Previous normalization shift
///
/// Returns updated `(scale, shift)` parameters.
///
/// ## `filter_options`
/// Returns the filtering configuration used during preprocessing.
///
/// This allows the caller to decouple filtering from the algorithm while
/// ensuring consistent parameterization.
pub trait RescaleAlgo: Clone + Send {
    fn rescale(
        &self,
        norm_signal: &[f32],
        levels_filtered: &[f32],
        prev_scale: f32,
        prev_shift: f32,
    ) -> Result<(f32, f32), RescaleError>;

    fn filter_options(&self) -> &RescaleFilterOptions;
}


/// Options used primarily for filtering
/// bases before rescaling.
/// 
/// TODO: Move to config script later on
#[derive(Clone)]
pub struct RescaleFilterOptions {
    /// Lower percentile for filtering bases based on dwell time
    /// (bases with dwell time < lower_percentile value get removed)
    pub dwell_filter_lower_percentile: f32,
    /// Upper percentile for filtering bases based on dwell time
    /// (bases with dwell time > upper_percentile value get removed)
    pub dwell_filter_upper_percentile: f32,
    /// The minimum absolute expected signal intensity value. Expected
    /// intensities that deviate less than this value from the mean of
    /// the expected intensity get removed. 
    pub min_abs_level: f32,
    /// The number of bases that will be ignored at the start and end. 
    pub n_bases_truncate: usize,
    /// Threshold of the minimum number of valid bases that are needed
    /// to perform the rescaling.
    pub min_num_filtered_levels: usize
}


/// Recalibrates signal normalization parameters (scale and shift) using a
/// pluggable rescaling algorithm.
///
/// This function orchestrates the rescaling pipeline by:
/// 1. Validating input dimensions
/// 2. Filtering valid bases and computing normalized mean signal values
/// 3. Delegating parameter estimation to the provided `RescaleAlgo` implementation
///
/// The normalization model is:
/// `normalized = (raw_signal - shift) / scale`
///
/// # Type Parameters
/// * `R` - A type implementing [`RescaleAlgo`], defining both filtering
///         configuration and the rescaling algorithm.
///
/// # Arguments
/// * `scale` - Current scale parameter for normalization
/// * `shift` - Current shift parameter for normalization
/// * `seq_to_signal_map` - Mapping from base indices to signal indices
///                         (length = number of bases + 1)
/// * `signal` - Raw signal measurements
/// * `levels` - Expected reference levels per base
/// * `algo` - Rescaling algorithm implementation
///
/// # Returns
/// * `Ok((new_scale, new_shift))` - Updated normalization parameters
/// * `Err(RescaleError)` - If validation, filtering, or rescaling fails
///
/// # Errors
/// * `EmptyMap` - `seq_to_signal_map` is empty
/// * `InvalidLevelsLen` - `levels.len() != seq_to_signal_map.len() - 1`
/// * Propagated errors from:
///     - [`filter_bases`] (converted from `RescaleFilterError`)
///     - `R::rescale` implementation
///
/// # Notes
/// * Filtering and normalization of signal values are performed in a single pass
///   via [`filter_bases`] to minimize memory traversal.
/// * The algorithm operates on *already normalized* mean signal values.
pub fn rescale<R: RescaleAlgo>(
    scale: f32,
    shift: f32,
    seq_to_signal_map: &[usize],
    signal: &[f32],
    levels: &[f32],
    algo: &R
) -> Result<(f32, f32), RescaleError> {

    // Check if map and levels have a valid length
    let seq_to_signal_map_len = seq_to_signal_map.len();
    if seq_to_signal_map_len == 0 {
        return Err(RescaleError::EmptyMap);
    } else if levels.len() != seq_to_signal_map_len - 1 {
        return Err(RescaleError::InvalidLevelsLen(levels.len(), seq_to_signal_map_len - 1));
    }

    let (mut mean_signal_filtered_norm, mut levels_filtered) = filter_bases(
        seq_to_signal_map,
        signal,
        levels,
        scale,
        shift,
        algo.filter_options()
    )?;

    let (new_scale, new_shift) = algo.rescale(
        &mut mean_signal_filtered_norm, 
        &mut levels_filtered,
        scale,
        shift,
    )?;

    Ok((new_scale, new_shift))
}