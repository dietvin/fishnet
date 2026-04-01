use crate::core::refinement::{band::sequence_band::SequenceBand, dp::forward_step::RefinementAlgo};

/// Performs the forward pass of dynamic programming for signal refinement
///
/// This function implements the forward pass of either the Viterbi algorithm or a dwell penalty
/// algorithm for signal refinement in nanopore sequencing. It processes each base in sequence,
/// calculating optimal paths through the signal data within the constraints of specified bands.
///
/// # Arguments
///
/// * `all_scores` - Mutable vector to be populated with forward scores for all bases. This is
///                  pre-allocated with sufficient size to hold scores for all positions within
///                  the bands.
/// * `traceback` - Mutable vector to be populated with traceback information for all bases.
///                 This will be used in a subsequent backtrace step to reconstruct the optimal 
///                 path.
/// * `signal` - Slice containing the raw signal values to be processed
/// * `expected_levels` - Vector of expected signal levels for each base in the sequence
/// * `band` - Structure defining the allowed regions (bands) for each base in the signal.
///            These bands constrain the search space of the dynamic programming algorithm.
/// * `base_offsets` - Slice containing offsets into the scores and traceback arrays for each base's information.
///                    These offsets enable efficient storage of variable-sized band information in flattened 
///                    arrays.
/// * `algo` - A struct implementing the `RefinementAlgo` trait. Available options are
///            `Viterbi` and `DwellPenalty`
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
pub fn forward_pass<A: RefinementAlgo>(
    all_scores: &mut [f32],
    traceback: &mut [i32],
    signal: &[f32],
    expected_levels: &[f32],
    band: &SequenceBand,
    base_offsets: &[usize],
    algo: &A
) {
    let seq_band_start = band.start();
    let seq_band_end = band.end();

    let current_bandwidth = seq_band_end[0];

    let mut previous_scores = vec![f32::INFINITY; current_bandwidth];
    previous_scores[0] = 0.0;

    algo.forward_step(
        &mut all_scores[0..current_bandwidth],
        &mut traceback[0..current_bandwidth],
        &previous_scores,
        expected_levels[0],
        &signal[0..current_bandwidth],
        1
    );

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

        algo.forward_step(
            &mut scores_current_slice[0..current_bandwidth],
            &mut traceback[current_offset..current_slice_end],
            &mut scores_prev_slice[previous_offset..],
            expected_levels[base_idx],
            &signal[current_band_start..current_band_end],
            current_band_start - previous_band_start
        );

        previous_band_start = current_band_start;
        previous_offset = current_offset;
    }
}