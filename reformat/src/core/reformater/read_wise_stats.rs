use crate::{
    core::reformater::{reformated::ReformatedBaseStat, stats::{
        mean, mean_and_stdev, median, signal_to_noise, std_dev
    }}, 
    error::core::reformat::ReadWiseStatsError, 
    execute::config::Stats
};

/// Calculates statistics for each base in a region of interst.
///
/// This function takes an alignment and computes requested statistics 
/// (mean, median, std, dwell, signal-to-noise) for each base in the 
/// sequence slice.
///
/// # Arguments
/// * `sequence_slice` - The base sequence as a slice of nucleotides (encoded as `u8`).
/// * `alignment_slice` - The signal alignment indices. Must have length equal to `sequence_slice.len() + 1`.
/// * `dwells_slice` - The dwell times per base (pre-normalized).
/// * `full_signal` - The full normalized nanopore signal trace.
/// * `stats` - The list of statistics to compute.
/// * `matching_filter_name` - Name of the filter or condition associated with this output.
///
/// # Returns
/// A `StatOutputRow` containing per-base statistics, if successful.
///
/// # Errors
/// * `ReadWiseStatsError` if input lengths are inconsistent or if
///   stat computation fails (e.g., empty slices).
pub(super) fn reformat_read_wise_stats(
    sequence_slice: &[u8],
    alignment_slice: &[usize],
    dwells_slice: &[f32],
    full_signal: &[f32],
    stats: &[Stats]
) -> Result<ReformatedBaseStat, ReadWiseStatsError> {
    let mut output_row = ReformatedBaseStat::from_stats_empty(
        stats, 
        sequence_slice
    );

    for i in 0..sequence_slice.len() {
        let signal_start_index = alignment_slice[i];
        let signal_end_index = alignment_slice[i+1];
        let signal_slice = &full_signal[signal_start_index..signal_end_index];
        let dwell_value = dwells_slice[i];

        // To reduce the amount of redundant calculations
        let need_mean = stats.contains(&Stats::Mean);
        let need_stdev = stats.contains(&Stats::StDev);
        let need_snr = stats.contains(&Stats::SignalToNoise);
    
        let (mean_val, stdev_val, signal_to_noise_val) = if need_snr && need_mean && need_stdev {
            let (mean_tmp, stdev_tmp, stn_tmp) = signal_to_noise(signal_slice)?;
            (Some(mean_tmp), Some(stdev_tmp), Some(stn_tmp)) 
        } else if need_mean && need_stdev {
            let (mean_tmp, stdev_tmp) = mean_and_stdev(signal_slice)?;
            (Some(mean_tmp), Some(stdev_tmp), None)
        } else {
            (None, None, None)
        };

        for stat in stats {
            match stat {
                Stats::Mean => {
                    let stat_value = match mean_val {
                        Some(v) => v, // Use previously calculated value
                        None => mean(signal_slice)?
                    };
                    output_row.push_mean(stat_value)?;
                }
                Stats::Median => {
                    let stat_value = median(signal_slice)?;
                    output_row.push_median(stat_value)?;
                }
                Stats::StDev => {
                    let stat_value = match stdev_val {
                        Some(v) => v, // Use previously calculated value
                        None => std_dev(signal_slice)?
                    };
                    output_row.push_std(stat_value)?;
                }
                Stats::Dwell => {
                    // Dwell times don't need to be calculated anymore
                    // (they were calculated and normalized as a whole)
                    output_row.push_dwell(dwell_value)?;
                }
                Stats::SignalToNoise => {
                    let stat_value = match signal_to_noise_val {
                        Some(v) => v, // Use previously calculated value
                        None => signal_to_noise(signal_slice)?.2
                    }; 
                    output_row.push_signal_to_noise(stat_value)?;
                }
            }
        }
    }

    Ok(output_row)
}