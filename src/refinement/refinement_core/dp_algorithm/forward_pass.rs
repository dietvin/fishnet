use crate::refinement::refinement_core::bands::Band;
use crate::refinement::settings::RefineAlgo;

use super::forward_step::forward_step_viterbi;
use super::forward_step_dwell_penalty::forward_step_dwell_penalty;

/// Performs the forward pass of dynamic programming for signal refinement
///
/// This function implements the forward pass of either the Viterbi algorithm or a dwell penalty
/// algorithm for signal refinement in nanopore sequencing. It processes each base in sequence,
/// calculating optimal paths through the signal data within the constraints of specified bands.
///
/// # Arguments
///
/// * `all_scores` - Mutable vector to be populated with forward scores for all bases. This is
///   pre-allocated with sufficient size to hold scores for all positions within the bands.
/// * `traceback` - Mutable vector to be populated with traceback information for all bases.
///   This will be used in a subsequent backtrace step to reconstruct the optimal path.
/// * `signal` - Slice containing the raw signal values to be processed
/// * `expected_levels` - Vector of expected signal levels for each base in the sequence
/// * `band` - Structure defining the allowed regions (bands) for each base in the signal.
///   These bands constrain the search space of the dynamic programming algorithm.
/// * `base_offsets` - Slice containing offsets into the scores and traceback arrays for each base's information.
///   These offsets enable efficient storage of variable-sized band information in flattened arrays.
/// * `method` - Enum specifying which algorithm to use for the forward pass:
///   - `RefineAlgo::Viterbi`: Standard Viterbi algorithm
///   - `RefineAlgo::DwellPenalty`: Viterbi with additional penalties for deviations from target dwell times
///
/// # Algorithm
///
/// The function processes each base sequentially:
/// 1. Initializes with special handling for the first base
/// 2. For each subsequent base:
///    - Extracts the appropriate band information and slices from the arrays
///    - Calls either `forward_step_viterbi` or `forward_step_dwell_penalty` based on the specified method
///    - Carefully manages array slices to avoid borrowing conflicts
/// 3. Maintains necessary state between bases to ensure proper connectivity in the dynamic programming matrix
///
/// # Note
///
/// This implementation uses a banded approach where only specific regions of the signal
/// are considered for each base, which is more efficient than considering all possible
/// signal positions for each base.
pub fn forward_pass (
    all_scores: &mut Vec<f32>,
    traceback: &mut Vec<i32>,
    signal: &[f32],
    expected_levels: &Vec<f32>,
    band: &Band,
    base_offsets: &[usize],
    method: &RefineAlgo
) {
    let mut short_dwell_penalty_vec = Vec::new();
    let use_dwell_penalty_alg = match method {
        RefineAlgo::DwellPenalty { 
            target, 
            limit, 
            weight 
        } => {
            short_dwell_penalty_vec = calculate_short_dwell_penalty_vec(
                target, 
                limit, 
                weight
            );

            true
        }
        RefineAlgo::Viterbi => false
    };

    let seq_band_start = band.start();
    let seq_band_end = band.end();

    let current_bandwidth = seq_band_end[0];

    let mut previous_scores = vec![f32::INFINITY; current_bandwidth];
    previous_scores[0] = 0.0;

    if use_dwell_penalty_alg {
        forward_step_dwell_penalty(
            &mut all_scores[0..current_bandwidth], 
            &mut traceback[0..current_bandwidth], 
            &previous_scores, 
            expected_levels[0], 
            &signal[0..current_bandwidth], 
            1, 
            &short_dwell_penalty_vec
        );
    } else {
        forward_step_viterbi(
            &mut all_scores[0..current_bandwidth], 
            &mut traceback[0..current_bandwidth], 
            &previous_scores, 
            expected_levels[0], 
            &signal[0..current_bandwidth], 
            1, 
        );
    }
    
    let mut previous_band_start = 0;
    let mut previous_offset = 0;

    for base_idx in 1..expected_levels.len() {
        let current_band_start = seq_band_start[base_idx];
        let current_band_end = seq_band_end[base_idx];
        let current_bandwidth = current_band_end - current_band_start;
        
        let current_offset = base_offsets[base_idx];
        let current_slice_end = current_offset + current_bandwidth;

        // Two references to slices on the same vector is not allowed
        // [0, 1, 2, 3, 4, 5, 6, 7, 8, 9]
        //     |----------||-------|
        //      prev. sl.   curr. sl.
        // => prev. offset = 1, prev. bw = 4 -> prev. bw = 5 (1+4)
        // => curr. offset = 5, curr. bw = 3 -> curr. bw = 8 (5+3)
        //
        // split at 5 (current offset):
        // [0, 1, 2, 3, 4], [5, 6, 7, 8, 9]
        //
        // [0, 1, 2, 3, 4]      [5, 6, 7, 8, 9]
        //     |---------|       |-------|
        //     prev. offset..end    0..curr. bw
        let (scores_prev_slice, scores_current_slice) = all_scores.split_at_mut(current_offset); 

        if use_dwell_penalty_alg {
            forward_step_dwell_penalty(
                &mut scores_current_slice[0..current_bandwidth],
                &mut traceback[current_offset..current_slice_end],
                &mut scores_prev_slice[previous_offset..],
                expected_levels[base_idx],
                &signal[current_band_start..current_band_end],
                current_band_start - previous_band_start,
                &short_dwell_penalty_vec
            )
        } else {
            forward_step_viterbi(
                &mut scores_current_slice[0..current_bandwidth],
                &mut traceback[current_offset..current_slice_end],
                &scores_prev_slice[previous_offset..],
                expected_levels[base_idx],
                &signal[current_band_start..current_band_end],
                current_band_start - previous_band_start
            );
        }

        previous_band_start = current_band_start;
        previous_offset = current_offset;
    }

}


/// Calculates penalty values for deviations from target dwell times
///
/// This function generates a vector of penalty values used in the dwell penalty algorithm.
/// Each value represents the squared deviation from a target dwell time, weighted by a
/// user-specified factor.
///
/// # Arguments
///
/// * `target` - Target dwell time value (ideal number of signal points per base)
/// * `limit` - Maximum dwell time to consider when calculating penalties
/// * `weight` - Weight factor to scale the penalty values
///
/// # Returns
///
/// A vector of penalty values, where the index corresponds to a specific dwell time
/// and the value is the penalty to apply for that dwell time.
///
/// # Implementation Details
///
/// The function:
/// 1. Ensures the limit doesn't exceed the target (capping it if necessary)
/// 2. Creates a vector with entries for dwell times from 0 to limit
/// 3. Calculates each penalty as: weight * (dwell_time - target)²
///
/// These penalties are designed to penalize both too short and too long dwell times,
/// with the penalty increasing quadratically as the dwell time deviates from the target.
fn calculate_short_dwell_penalty_vec(
    target: &f32, 
    limit: &f32, 
    weight: &f32 
) -> Vec<f32> {
    // Handle the case where limit > target
    let actual_limit = if limit > target {
        target
    } else {
        limit
    };
    
    // Convert actual_limit to usize for array creation
    let size = *actual_limit as usize;
    
    // Create the array and apply the calculation
    let mut result = Vec::with_capacity(size);
    for i in 0..size {
        let i_f32 = i as f32;
        result.push(weight * (i_f32 - target).powi(2));
    }
    
    result
}


#[cfg(test)]
mod test {
    use super::calculate_short_dwell_penalty_vec;

    #[test]
    fn test_calculate_short_dwell_penalty_vec() {
        let vec = calculate_short_dwell_penalty_vec(&4.0, &3.0, &0.5);

        assert_eq!(vec, vec![8.0, 4.5, 2.0])
    }
}