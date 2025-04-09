use crate::{error::refinement_errors::refine_errors::RefineError, refinement::{refinement_core::bands::Band, settings::RefineSettings}};
use super::dp_algorithm::banded_dp;


/// Refines a preliminary sequence-to-signal mapping using dynamic programming
///
/// This function takes an initial mapping between a sequence and signal data, then applies
/// a banded dynamic programming algorithm to optimize the boundaries between bases, based on
/// the distance between expected and measured signal intensity. This results in a more 
/// accurate alignment of the signal to the sequence.
///
/// # Arguments
///
/// * `sequence_to_signal_map` - Initial mapping from sequence positions to signal positions.
///   Each element represents the starting position in the signal for the corresponding base.
///   The last element represents the end position of the final base.
/// * `signal` - Vector containing the raw signal values from nanopore sequencing
/// * `expected_levels` - Vector of expected signal levels for each base in the sequence,
///   derived from the kmer table entry for the kmer around a given base
/// * `settings` - Configuration settings for the refinement process, including:
///   - Bandwidth parameters that control the search space
///   - Algorithm selection (Viterbi or DwellPenalty)
///
/// # Returns
///
/// * `Result<Vec<usize>, RefineError>` - On success, returns a vector containing the refined
///   mapping from sequence positions to signal positions. On failure, returns a `RefineError`
///   describing what went wrong.
///
/// # Algorithm
///
/// The refinement process consists of several steps:
/// 1. Trims the signal to the region of interest based on the initial mapping
/// 2. Adjusts the mapping coordinates to be zero-based relative to the trimmed signal
/// 3. Computes a band around the initial mapping to constrain the search space
/// 4. Converts the band from signal-based to sequence-based coordinates
/// 5. Applies banded dynamic programming to find the optimal mapping within this band
/// 6. Adjusts the optimized mapping back to the original signal coordinates
///
/// # Note
///
/// At this point Error handling is not implemented for the dynamic programming part
/// of the refinement.
pub fn refinement(
    sequence_to_signal_map: Vec<usize>,
    signal: &Vec<f32>,
    expected_levels: &Vec<f32>,
    settings: &RefineSettings
) -> Result<Vec<usize>, RefineError> {
    // trim the signal and adjust the boundaries in the map so it starts at signal index 0
    let sig_map_start = sequence_to_signal_map[0];
    let sig_map_end = sequence_to_signal_map[sequence_to_signal_map.len() - 1];

    let signal_trimmed = &signal[sig_map_start..sig_map_end];
    let sequence_to_signal_map_zeroed = sequence_to_signal_map
        .iter()
        .map(|el| el-sig_map_start)
        .collect::<Vec<usize>>();

    let mut band = Band::compute_signal_band(
        &sequence_to_signal_map_zeroed,
        expected_levels.len(),
        *settings.half_bandwidth(),
        true
    )?;
    band.convert_to_sequence_band(*settings.adjust_band_min_size())?;

    let optimized_map = banded_dp(
        signal_trimmed, 
        expected_levels, 
        &band, 
        settings.refinement_algo()
    );

    Ok(
        optimized_map.iter().map(|el| el + sig_map_start).collect::<Vec<usize>>()
    )
}