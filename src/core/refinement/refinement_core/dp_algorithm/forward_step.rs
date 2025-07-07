/*!
 * This module implements scoring and dynamic programming logic for signal alignment using 
 * the Viterbi algorithm.
 * 
 * The core components include:
 * - A `score` function that calculates squared error between expected and measured signals.
 * - A `forward_step_viterbi` function that performs a single step of the Viterbi forward pass 
 *   for one base, computing optimal alignment scores and traceback paths across signal bands.
 * 
 * The implementation is adapted from the Nanopore Remora project and is optimized for handling
 * "stay" and "move" transitions within a constrained banded dynamic programming framework.
 */

use crate::logger::get_log_vector_sample;

const LARGE_SCORE: f32 = 100.0;


/// Calculates the squared difference between expected and measured signal levels
///
/// # Arguments
///
/// * `expected` - The expected or reference signal level
/// * `measured` - The actual measured signal level from the data
///
/// # Returns
///
/// The squared difference (error) between the expected and measured values
pub fn score(expected: f32, measured: f32) -> f32 {
    let tmp = measured - expected;
    tmp*tmp
}


/// Processes one base using the Viterbi algorithm with squared error scoring
///
/// This function implements a single forward step in the Viterbi algorithm for signal mapping,
/// calculating optimal paths through a signal matrix using dynamic programming. It computes
/// scores based on squared error between expected signal levels and measured values.
///
/// Adapted from [Nanopore Remora implementation](https://github.com/nanoporetech/remora/blob/0787dae2da818c49a3aaade10515b1e6df88e6bd/src/remora/refine_signal_map_core.pyx#L256)
///
/// # Arguments
///
/// * `current_scores` - Mutable slice to be populated with forward Viterbi scores at each position
///   in the current base's band
/// * `current_traceback` - Mutable slice to be populated with traceback information; each value 
///   indicates the number of signal points backward until the first point assigned to this base
/// * `previous_scores` - Forward Viterbi scores calculated for the previous base's band
/// * `current_level` - Expected signal level for the current base based on reference data
/// * `current_signal` - Slice containing the actual signal values measured for the current base's band
/// * `band_start_diff` - Difference in starting coordinates between the current and previous base's bands;
///   a value of 0 indicates a "stay" transition, while a positive value indicates a "move" transition
///
/// # Behavior
///
/// The function handles three distinct cases:
///
/// 1. Starting position in the band (either "stay" or "move" transition)
/// 2. Overlapping regions where both "stay" and "move" transitions are possible
/// 3. Remaining positions where only "stay" transitions are possible
///
/// At each position, the function computes scores and traceback information for the optimal path.
pub fn forward_step_viterbi(
    current_scores: &mut [f32],
    current_traceback: &mut [i32],
    previous_scores: &[f32],
    current_level: f32,
    current_signal: &[f32],
    band_start_diff: usize
) {
    log::trace!(
        "forward_step_viterbi input: current_scores = {}, current_traceback = {}, previous_scores = {}, current_level = {}, current_signal = {}, band_start_diff = {}",
        get_log_vector_sample(current_scores, 10),
        get_log_vector_sample(current_traceback, 10),
        get_log_vector_sample(previous_scores, 10),
        current_level,
        get_log_vector_sample(current_signal, 10),
        band_start_diff
    );

    // Handle start position in band
    if band_start_diff == 0 {
        // If this is a "stay" band start, set invalid score and traceback
        current_scores[0] = LARGE_SCORE + previous_scores[previous_scores.len()-1];
        current_traceback[0] = -1;
    } else {
        // Compute move score for start of base band
        let base_score = score(current_level, current_signal[0]);
        current_scores[0] = previous_scores[band_start_diff-1] + base_score;
        current_traceback[0] = 0;
    }

    // Create slice of previous_scores starting at the same position as current_scores
    let previous_scores_slice = &previous_scores[band_start_diff..];

    // Determine the length to process based on whether base bands are the same
    let process_len = if previous_scores_slice.len() == current_scores.len() {
        previous_scores_slice.len() -1
    } else {
        previous_scores_slice.len()
    };

    // Compute scores where current and previous base overlap
    for band_pos in 1..=process_len {
        let base_score = score(current_level, current_signal[band_pos]);
        let move_score = previous_scores_slice[band_pos - 1] + base_score;
        let stay_score = current_scores[band_pos - 1] + base_score;

        if move_score < stay_score {
            current_scores[band_pos] = move_score;
            current_traceback[band_pos] = 0;
        } else {
            current_scores[band_pos] = stay_score;
            current_traceback[band_pos] = current_traceback[band_pos - 1] + 1;
        }
    }

    // Stay through rest of the band
    for band_pos in (process_len + 1)..current_scores.len() {
        let base_score = score(current_level, current_signal[band_pos]);
        let stay_score = current_scores[band_pos - 1] + base_score;
        current_scores[band_pos] = stay_score;
        current_traceback[band_pos] = current_traceback[band_pos - 1] + 1;
    }
    
    log::trace!(
        "forward_step_viterbi updated: current_scores = {}, current_traceback = {}",
        get_log_vector_sample(current_scores, 10),
        get_log_vector_sample(current_traceback, 10)
    );
}










/// The tests use data that stems from read '*6e37823a-9398-4be8-b111-65cab029f4e0*' in example data
#[cfg(test)]
mod tests {
    use super::forward_step_viterbi;

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

        forward_step_viterbi(
            &mut scores[..band_width], 
            &mut tb[..band_width], 
            &prev_scores, 
            level, 
            &signal, 
            band_start_diff
        );

        let expected_scores: Vec<f32> = vec![ 
             0.28620595,  0.5889039 ,  1.0199802 ,  1.3171295 ,  1.672075  ,
             1.9747729 ,  2.260979  ,  2.5054646 ,  2.7971165 ,  3.0219738 ,
             3.4333246 ,  3.4914825 ,  3.654108  ,  3.8334875 ,  4.3775687 ,
             4.612138  ,  4.8516393 ,  5.1599374 ,  5.4911485 ,  5.8108006 ,
             6.0453696 ,  6.489853  ,  6.803802  ,  7.100951  ,  7.3210297 ,
             7.694315  ,  8.125391  ,  8.3903265 ,  8.751334  ,  9.088402  ,
             9.431377  ,  9.628332  ,  9.839007  , 10.136157  , 10.433307  ,
            10.730456  , 11.097577  , 11.332146  , 11.737023  , 12.074091  ,
            12.202023  , 12.456631  , 12.691199  , 12.751826  , 12.802885  ,
            12.809784  , 0.0        , 0.0        , 0.0        , 0.0
        ];

        assert_eq!(
            scores.iter().map(|&el| round_to(el, 5)).collect::<Vec<f32>>(), 
            expected_scores.iter().map(|&el| round_to(el, 5)).collect::<Vec<f32>>()
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

        forward_step_viterbi(
            &mut scores[..band_width], 
            &mut tb[..band_width], 
            &prev_scores, 
            level, 
            &signal, 
            band_start_diff
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


    /// Test the scores calculated for the second iteration (idx=1)
    #[test]
    fn test_intermediate_iter_scores() {
        let band_width = 49;
        let mut scores= vec![0.0; 49];
        let mut tb = vec![-1;49];
        let prev_scores= vec![ 
            0.28620595,  0.5889039 ,  1.0199802,  1.3171295,   1.672075 ,
            1.9747729 ,  2.260979  ,  2.5054646,  2.7971165,   3.0219738,
            3.4333246 ,  3.4914825 ,  3.654108 ,  3.8334875,   4.3775687,
            4.612138  ,  4.8516393 ,  5.1599374,  5.4911485,   5.8108006,
            6.0453696 ,  6.489853  ,  6.803802 ,  7.100951 ,   7.3210297,
            7.694315  ,  8.125391  ,  8.3903265,  8.751334 ,   9.088402 ,
            9.431377  ,  9.628332  ,  9.839007 ,  10.136157,   10.433307,
            10.730456 ,  11.097577,   11.332146,  11.737023,   12.074091,
            12.202023 ,  12.456631,   12.691199,  12.751826,   12.802885,
            12.809784
        ];
        let level = 0.0;
        let signal = vec![
            0.55017991,  0.65656397,  0.545114,    0.59577308,  0.55017991,  0.53498218,
            0.49445492,  0.54004809,  0.47419129,  0.64136625,  0.24115953,  0.40326858,
            0.42353221,  0.7376185,   0.4843231,   0.48938901,  0.55524581,  0.57550944,
            0.56537763,  0.4843231,   0.66669579,  0.56031172,  0.545114,    0.46912538,
            0.6109708 ,  0.65656397,  0.51471855,  0.60083898,  0.58057535,  0.58564126,
            0.44379584,  0.45899356,  0.545114,    0.545114,    0.545114,    0.60590489,
            0.4843231 ,  0.63630034,  0.58057535,  0.35767541,  0.50458673,  0.4843231,
            0.24622543,  0.2259618,  -0.08305858,  0.87946392,  0.98078207,  1.38098879,
            1.73560234
        ];
        let band_start_diff = 1; 

        forward_step_viterbi(
            &mut scores[..band_width], 
            &mut tb[..band_width], 
            &prev_scores, 
            level, 
            &signal, 
            band_start_diff
        );

        let expected_scores: Vec<f32> = vec![ 
            0.5889039,  1.0199802,  1.3171295,  1.672075,   1.9747729,  2.260979,
            2.5054646,  2.7971165,  3.0219738,  3.4333246,  3.4914825,  3.654108,
            3.8334875,  4.3775687,  4.612138,   4.8516393,  5.1599374,  5.4911485,
            5.8108006,  6.0453696,  6.489853,   6.803802,   7.100951,   7.3210297,
            7.694315 ,  8.125391,   8.3903265,  8.751334,   9.088402,   9.431377,
            9.628332 ,  9.839007,  10.136157,  10.433307,  10.730456,  11.097577,
           11.332146 , 11.737023,  12.074091,  12.202023,  12.456631,  12.691199,
           12.751826 , 12.802885,  12.809784,  13.5832405, 14.545174,  16.452303,
           19.464619 
        ];

        assert_eq!(
            scores.iter().map(|&el| round_to(el, 5)).collect::<Vec<f32>>(), 
            expected_scores.iter().map(|&el| round_to(el, 5)).collect::<Vec<f32>>()
        );
    }

    /// Test the scores calculated for the second iteration (idx=1)
    #[test]
    fn test_intermediate_iter_traceback() {
        let band_width = 49;
        let mut scores= vec![0.0; 49];
        let mut tb = vec![-1;49];
        let prev_scores= vec![ 
            0.28620595,  0.5889039 ,  1.0199802,  1.3171295,   1.672075 ,
            1.9747729 ,  2.260979  ,  2.5054646,  2.7971165,   3.0219738,
            3.4333246 ,  3.4914825 ,  3.654108 ,  3.8334875,   4.3775687,
            4.612138  ,  4.8516393 ,  5.1599374,  5.4911485,   5.8108006,
            6.0453696 ,  6.489853  ,  6.803802 ,  7.100951 ,   7.3210297,
            7.694315  ,  8.125391  ,  8.3903265,  8.751334 ,   9.088402 ,
            9.431377  ,  9.628332  ,  9.839007 ,  10.136157,   10.433307,
            10.730456 ,  11.097577,   11.332146,  11.737023,   12.074091,
            12.202023 ,  12.456631,   12.691199,  12.751826,   12.802885,
            12.809784
        ];
        let level = 0.0;
        let signal = vec![
            0.55017991,  0.65656397,  0.545114,    0.59577308,  0.55017991,  0.53498218,
            0.49445492,  0.54004809,  0.47419129,  0.64136625,  0.24115953,  0.40326858,
            0.42353221,  0.7376185,   0.4843231,   0.48938901,  0.55524581,  0.57550944,
            0.56537763,  0.4843231,   0.66669579,  0.56031172,  0.545114,    0.46912538,
            0.6109708 ,  0.65656397,  0.51471855,  0.60083898,  0.58057535,  0.58564126,
            0.44379584,  0.45899356,  0.545114,    0.545114,    0.545114,    0.60590489,
            0.4843231 ,  0.63630034,  0.58057535,  0.35767541,  0.50458673,  0.4843231,
            0.24622543,  0.2259618,  -0.08305858,  0.87946392,  0.98078207,  1.38098879,
            1.73560234
        ];
        let band_start_diff = 1; 

        forward_step_viterbi(
            &mut scores[..band_width], 
            &mut tb[..band_width], 
            &prev_scores, 
            level, 
            &signal, 
            band_start_diff
        );

        let expected_tb = vec![
            0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 
            10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 
            20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 
            30, 31, 32, 33, 34, 35, 36, 37, 38, 39, 
            40, 41, 42, 43, 44, 45, 46, 47, 48
        ];

        assert_eq!(tb, expected_tb);
    }


}