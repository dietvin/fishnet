/*!
 * This module provides an enhanced Viterbi alignment step, introducing dwell time penalties 
 * to account for variable durations that a signal may spend at a specific base position.
 * 
 * In addition to standard squared error scoring between expected and measured signal levels,
 * the module incorporates dwell-time-aware dynamic programming to better model realistic
 * signal behaviors. It achieves this by:
 * 
 * - Reusing the basic Viterbi forward step as a reference path.
 * - Exploring multiple dwell durations at each signal position.
 * - Applying dwell-specific penalties to guide optimal alignment.
 * - Choosing the most probable alignment path based on signal error and dwell time cost.
 */

use crate::logger::get_log_vector_sample;

use super::forward_step::{forward_step_viterbi, score};

const LARGE_SCORE: f32 = 100.0;

/// Processes one base using Viterbi algorithm with additional dwell time penalties
///
/// This function extends the standard Viterbi algorithm by incorporating dwell time penalties,
/// which account for variable time that signals may spend at a particular base position.
/// It computes optimal paths through a signal matrix while considering both signal level errors
/// and penalties for different dwell times.
///
/// # Arguments
///
/// * `current_scores` - Mutable slice to be populated with forward scores at each position
///   in the current base's band, including dwell penalties
/// * `current_traceback` - Mutable slice to be populated with traceback information; each value
///   indicates the dwell time (number of signal points) assigned to this position
/// * `previous_scores` - Forward scores calculated for the previous base's band
/// * `current_level` - Expected signal level for the current base based on reference data
/// * `current_signal` - Slice containing the actual signal values measured for the current base's band
/// * `band_start_diff` - Difference in starting coordinates between the current and previous base's bands;
///   a value of 0 indicates a "stay" transition, while a positive value indicates a "move" transition
/// * `dwell_penalty` - Slice containing penalty values for different dwell times; index corresponds to
///   the dwell time, and the value is the penalty to apply
///
/// # Algorithm
///
/// The function works in several steps:
/// 1. First calculates standard Viterbi scores without dwell penalties for later reference
/// 2. For each position in the current band:
///    - Handles edge cases for positions beyond the previous band
///    - Calculates scores for all possible dwell times up to the maximum length of the penalty array
///    - Accumulates the signal level error for each potential dwell time
///    - Selects the optimal dwell time with the minimum combined score (error + penalty)
///    - Updates the traceback to record the chosen dwell time
/// 3. For positions beyond the maximum penalized length, considers both penalized and
///    unpenalized paths to find the global optimum
///
/// # Relationship to forward_step_viterbi
///
/// This function builds on the basic Viterbi implementation but adds the concept of variable
/// dwell times with associated penalties, making it more suitable for signals where the time
/// spent at each position may vary and needs to be accounted for in the model.
pub fn forward_step_dwell_penalty(
    current_scores: &mut [f32],
    current_traceback: &mut [i32],
    previous_scores: &[f32],
    current_level: f32,
    current_signal: &[f32],
    band_start_diff: usize,
    dwell_penalty: &[f32]
) {
    log::trace!(
        "forward_step_dwell_penalty input: redirecting to forward_step_viterbi, dwell_penalty = {}",
        get_log_vector_sample(dwell_penalty, 10)
    );
    // Compute un-penalized band position scores for lookup after dwell_penalty range is searched
    let mut unpen_scores = vec![0.0f32; current_scores.len()];
    let mut unpen_tb = vec![0i32; current_traceback.len()];

    forward_step_viterbi(
        &mut unpen_scores, 
        &mut unpen_tb, 
        previous_scores, 
        current_level, 
        current_signal, 
        band_start_diff
    );

    let max_penalized_len = dwell_penalty.len();

    // Loop over signal positions within this base band
    for band_pos in 0..current_scores.len() { 
        // If past the end of the prev band stay until the end
        if band_pos as i32 + band_start_diff as i32 - previous_scores.len() as i32 >= max_penalized_len as i32 {
            current_scores[band_pos] = current_scores[band_pos - 1] + score(current_level, current_signal[band_pos]);
            current_traceback[band_pos] = current_traceback[band_pos - 1] + 1;
            continue;
        }

        // Set spoof values for position (gets overwritten except for the edge case directly below)
        current_scores[band_pos] = LARGE_SCORE + previous_scores[previous_scores.len() - 1];
        current_traceback[band_pos] = -1;

        if band_pos == 0 && band_start_diff == 0 {
            continue;
        }

        let mut running_pos_score = 0.0;
        for dwell_idx in 0..dwell_penalty.len() {
            if dwell_idx > band_pos || (band_start_diff == 0 && band_pos == dwell_idx) {
                break;
            }

            running_pos_score += score(current_level, current_signal[band_pos - dwell_idx]);
            
            let dwell_offset = (band_pos as i32 - dwell_idx as i32 - 1 + band_start_diff as i32) as usize;
            if dwell_offset >= previous_scores.len() {
                continue;
            }

            let pos_score = previous_scores[dwell_offset] 
                + running_pos_score 
                + dwell_penalty[dwell_idx];

            if pos_score < current_scores[band_pos] {
                current_scores[band_pos] = pos_score;
                current_traceback[band_pos] = dwell_idx as i32;
            }
        }

        if band_pos >= max_penalized_len {
            let pos_score = unpen_scores[band_pos - max_penalized_len] + running_pos_score;

            if pos_score < current_scores[band_pos] {
                current_scores[band_pos] = pos_score;
                current_traceback[band_pos] = unpen_tb[band_pos - max_penalized_len] + max_penalized_len as i32;
            }
        }
    }
    log::trace!(
        "forward_step_dwell_penalty updated and penalized: current_scores = {}, current_traceback = {}",
        get_log_vector_sample(current_scores, 10),
        get_log_vector_sample(current_traceback, 10)
    );
}










#[cfg(test)]
mod tests {
    use super::forward_step_dwell_penalty;

    /// Round to a specific number of decimal places
    fn round_to(value: f32, decimal_places: u32) -> f32 {
        let multiplier = 10f32.powi(decimal_places as i32);
        (value * multiplier).round() / multiplier
    }

    /// Test the scores calculated for the first iteration (idx=0)
    #[test]
    fn test_first_iter_scores() {
        let band_width = 46;
        let mut scores= vec![0.0; 50];
        let mut tb = vec![-1;50];
        let mut prev_scores = vec![1000000000.0; band_width];
        prev_scores[0] = 0.0;
        let level = 0.0;
        let signal: Vec<f32> = vec![
            0.53498218, 0.55017991, 0.65656397, 0.545114, 0.59577308, 0.55017991, 0.53498218, 
            0.49445492, 0.54004809,  0.47419129,  0.64136625,  0.24115953,
            0.40326858, 0.42353221, 0.7376185, 0.4843231, 0.48938901, 0.55524581,
            0.57550944, 0.56537763, 0.4843231, 0.66669579, 0.56031172, 0.545114,
            0.46912538, 0.6109708, 0.65656397, 0.51471855, 0.60083898, 0.58057535,
            0.58564126, 0.44379584, 0.45899356, 0.545114, 0.545114, 0.545114,
            0.60590489, 0.4843231, 0.63630034, 0.58057535, 0.35767541, 0.50458673,
            0.4843231, 0.24622543, 0.2259618, -0.08305858
        ];
        let band_start_diff = 1; 
        let dwell_penalty  = vec![8., 4.5, 2.];

        forward_step_dwell_penalty(
            &mut scores[..band_width], 
            &mut tb[..band_width], 
            &prev_scores, 
            level, 
            &signal, 
            band_start_diff,
            &dwell_penalty
        );

        let expected_scores: Vec<f32> = vec![ 
            8.28620593,  5.08890386,  3.01998011,  1.31712938,  1.67207494,
            1.97477287,  2.2609788 ,  2.50546447,  2.79711641,  3.02197378,
            3.43332445,  3.49148236,  3.65410791,  3.83348744,  4.37756849,
            4.61213735,  4.85163896,  5.15993687,  5.49114799,  5.81079985,
            6.04536872,  6.48985199,  6.80380122,  7.10095049,  7.32102911,
            7.69431443,  8.12539067,  8.39032586,  8.75133334,  9.08840108,
            9.43137677,  9.62833152,  9.83900661, 10.13615588, 10.43330515,
           10.73045442, 11.09757516, 11.33214403, 11.73702215, 12.07408989,
           12.20202158, 12.45662936, 12.69119822, 12.75182519, 12.80288392,
           12.80978265, 0.0        , 0.0        , 0.0        , 0.0
        ];

        assert_eq!(
            scores.iter().map(|&el| round_to(el, 4)).collect::<Vec<f32>>(), 
            expected_scores.iter().map(|&el| round_to(el, 4)).collect::<Vec<f32>>()
        );
    }

    /// Test the traceback calculated for the first iteration (idx=0)
    #[test]
    fn test_first_iter_tb() {
        let band_width = 46;
        let mut scores= vec![0.0; 50];
        let mut tb = vec![-1;50];
        let mut prev_scores = vec![1000000000.0; band_width];
        prev_scores[0] = 0.0;
        let level = 0.0;
        let signal = vec![
            0.53498218, 0.55017991, 0.65656397, 0.545114, 0.59577308, 0.55017991, 0.53498218, 
            0.49445492, 0.54004809,  0.47419129,  0.64136625,  0.24115953,
            0.40326858, 0.42353221, 0.7376185, 0.4843231, 0.48938901, 0.55524581,
            0.57550944, 0.56537763, 0.4843231, 0.66669579, 0.56031172, 0.545114,
            0.46912538, 0.6109708, 0.65656397, 0.51471855, 0.60083898, 0.58057535,
            0.58564126, 0.44379584, 0.45899356, 0.545114, 0.545114, 0.545114,
            0.60590489, 0.4843231, 0.63630034, 0.58057535, 0.35767541, 0.50458673,
            0.4843231, 0.24622543, 0.2259618, -0.08305858
        ];
        let band_start_diff = 1; 
        let dwell_penalty  = vec![8., 4.5, 2.];

        forward_step_dwell_penalty(
            &mut scores[..band_width], 
            &mut tb[..band_width], 
            &prev_scores, 
            level, 
            &signal, 
            band_start_diff,
            &dwell_penalty
        );

        let expected_tb = vec![
            0,  1,  2,  3,  4,  5,  6,  7,  8,  9, 
            10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 
            20, 21, 22, 23, 24, 25, 26, 27, 28, 29,
            30, 31, 32, 33, 34, 35, 36, 37, 38, 39, 
            40, 41, 42, 43, 44, 45, -1, -1, -1, -1
        ];

        assert_eq!(tb, expected_tb);
    }
}