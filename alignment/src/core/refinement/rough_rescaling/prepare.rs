use helper::quantiles::calculate_quantiles;

use crate::error::core::refinement::rescale::RoughRescaleError;


/// Prepares quantile summaries for rough rescaling.
///
/// This function extracts representative values from both the signal and
/// expected levels, then computes aligned quantiles for regression.
///
/// # Processing Steps
/// 1. Determines clipping range based on `clip_bases`
/// 2. Extracts normalized signal values:
///    * If `use_base_center`:
///        - Uses one sample per base (midpoint of each segment)
///    * Otherwise:
///        - Uses all signal values within the mapped region
/// 3. Applies clipping to both signal and levels
/// 4. Computes quantiles for:
///    * normalized signal
///    * expected levels
///
/// # Arguments
/// * `scale`, `shift` - Normalization parameters used to compute:
///   `norm_signal = (signal - shift) / scale`
/// * `seq_to_signal_map` - Mapping from base indices to signal indices
/// * `levels` - Expected reference levels
/// * `signal` - Raw signal measurements
/// * `quantiles` - Quantiles to compute (values in `[0.0, 1.0]`)
/// * `clip_bases` - Number of bases to exclude from each end
/// * `use_base_center` - Sampling strategy for signal extraction
///
/// # Returns
/// * `Ok((norm_signal_quantiles, level_quantiles))`
///     - Two vectors of equal length corresponding to requested quantiles
/// * `Err(RoughRescaleError)` - If input validation or quantile computation fails
///
/// # Errors
/// * `PrepError` - If `seq_to_signal_map` is empty when required
/// * Errors propagated from `calculate_quantiles`
pub(super) fn prep_rough_rescale(
    scale: f32, 
    shift: f32,
    seq_to_signal_map: &[usize],
    levels: &[f32],
    signal: &[f32],
    quantiles: &[f32],
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
