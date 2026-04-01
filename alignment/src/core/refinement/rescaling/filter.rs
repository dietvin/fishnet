use helper::quantiles::get_upper_lower_quantiles;

use crate::{
    core::refinement::rescaling::RescaleFilterOptions, error::core::refinement::rescale::RescaleFilterError,
};


/// Filters bases and computes normalized mean signal intensities for rescaling.
///
/// This function performs preprocessing required for parameter estimation:
/// 1. Computes dwell times (signal span per base)
/// 2. Applies percentile-based dwell filtering
/// 3. Removes bases with low informational value (low deviation from mean level)
/// 4. Computes mean signal per base
/// 5. Normalizes signal values using the provided `scale` and `shift` 
///
/// The result is a pair of aligned vectors:
/// * normalized mean signal intensities
/// * corresponding expected levels
///
/// # Arguments
/// * `seq_to_signal_map` - Mapping from base indices to signal indices
/// * `signal` - Raw signal measurements
/// * `levels` - Expected reference levels per base
/// * `scale` - Current scale parameter used for normalization
/// * `shift` - Current shift parameter used for normalization
/// * `options` - Filtering configuration (`RescaleFilterOptions`)
///
/// # Returns
/// * `Ok((mean_signal_filtered_norm, levels_filtered))`
///     - `mean_signal_filtered_norm`: normalized mean signal per retained base
///     - `levels_filtered`: corresponding expected levels
/// * `Err(RescaleFilterError)` - If filtering constraints are not satisfied
///
/// # Errors
/// * `BelowMinNumFiltered` - Too few bases before or after filtering
/// * `TooShortForTruncation` - Sequence too short for requested truncation
/// * `TooShortAfterTruncation` - Not enough bases remain after truncation
/// * Errors from percentile computation (`get_upper_lower_quantiles`)
///
/// # Filtering Criteria
/// A base is retained only if:
/// * Its dwell lies strictly within the configured percentile bounds
/// * Its expected level deviates sufficiently from the global mean:
///   `|level - mean(levels)| > min_abs_level`
///
/// # Implementation Notes
/// * Normalization is performed inline during mean computation to avoid an
///   additional pass over the data.
/// * Output vectors are preallocated to `n_bases` capacity to reduce reallocations.
/// * The function assumes `levels.len() == seq_to_signal_map.len() - 1`.
pub(super) fn filter_bases(
    seq_to_signal_map: &[usize],
    signal: &[f32],
    levels: &[f32],
    scale: f32,
    shift: f32,
    options: &RescaleFilterOptions
) -> Result<(Vec<f32>, Vec<f32>), RescaleFilterError> {

    // Calculate the dwells (number of measurements) for each base
    let dwells = seq_to_signal_map
        .windows(2)
        .map(|window| (window[1] - window[0]) as f32)
        .collect::<Vec<f32>>();

    let n_bases = dwells.len();

    // Make sure that the sequence is long enough considering the minimum
    // number of needed levels and the number of bases to truncate
    if n_bases < options.min_num_filtered_levels {
        return Err(RescaleFilterError::BelowMinNumFiltered(
            n_bases,
            options.min_num_filtered_levels 
        ));
    } else if 2 * options.n_bases_truncate > n_bases {
        // Added check so the next else if can not be <0
        return Err(RescaleFilterError::TooShortForTruncation(
            n_bases,
            options.n_bases_truncate 
        ));
    } else if n_bases - 2 * options.n_bases_truncate < options.min_num_filtered_levels {
        return Err(RescaleFilterError::TooShortAfterTruncation(
            n_bases - 2 * options.n_bases_truncate,
            options.min_num_filtered_levels 
        ));
    }

    // Calculate the upper and lower percentile values of the dwells
    let (dwell_lower_percentile_value, dwell_upper_percentile_value) = get_upper_lower_quantiles(
        &dwells, 
        options.dwell_filter_lower_percentile,
        options.dwell_filter_upper_percentile
    )?;

    let levels_mean = levels.iter().sum::<f32>() / (levels.len() as f32);

    let mut mean_signal_filtered_norm = Vec::with_capacity(n_bases);
    let mut levels_filtered = Vec::with_capacity(n_bases);

    let (start_base_idx, end_base_idx) = if options.n_bases_truncate == 0 {
        (0, n_bases)
    } else {
        (options.n_bases_truncate, n_bases - options.n_bases_truncate)
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
        if (expected_intensity - levels_mean).abs() <= options.min_abs_level {
            continue;
        }
        
        // Calculate the mean current intensity from the current signal chunk
        let mean_signal_intensity = signal[
            seq_to_signal_map[base_idx]..seq_to_signal_map[base_idx + 1]
        ]
            .iter()
            .sum::<f32>() / dwell;

        // Normalization is done in this loop to avoid another pass later on
        let mean_signal_intensity_norm = (mean_signal_intensity - shift) / scale;

        mean_signal_filtered_norm.push(mean_signal_intensity_norm);
        levels_filtered.push(expected_intensity);
    }

    // Check if enough bases passed filtering
    if mean_signal_filtered_norm.len() < options.min_num_filtered_levels {
        return Err(RescaleFilterError::BelowMinNumFiltered(
            mean_signal_filtered_norm.len(), 
            options.min_num_filtered_levels
        ));
    }

    Ok((mean_signal_filtered_norm, levels_filtered))
}