use crate::core::refinement::{
    band::sequence_band::SequenceBand,
    dp::{
        forward_pass::forward_pass,
        forward_step::RefinementAlgo, traceback::traceback
    }
};

pub mod forward_pass;
pub mod forward_step;
pub mod traceback;

/// Performs banded dynamic programming to find optimal sequence-to-signal alignment.
///
/// This function implements the banded dynamic programming algorithm to align a sequence of
/// expected levels to observed signal measurements. The alignment is constrained to a 
/// predefined band to reduces computational complexity while maintaining alignment accuracy.
///
/// # Arguments
/// * `signal` - Raw nanopore signal measurements
/// * `levels` - Expected reference levels for each sequence position
/// * `band` - Constraint band defining allowed alignment paths
/// * `algo` - Refinement algorithm (struct implementing the RefinementAlgo trait)
///
/// # Returns
/// A vector of signal indices representing the optimal alignment path, where each element
/// corresponds to the signal position aligned to a sequence base. The vector length is
/// `levels.len() + 1` to include both start and end positions.
///
/// # Algorithm Details
/// 1. **Initialization**: Sets up base offset mapping for efficient band indexing
/// 2. **Forward Pass**: Computes optimal scores within the band using dynamic programming
/// 3. **Traceback**: Reconstructs the optimal path from the computed scores
pub fn banded_db<R: RefinementAlgo>(
    signal: &[f32],
    levels: &[f32],
    band: &SequenceBand,
    algo: &R
) -> Vec<usize> {
    let mut base_offsets = Vec::with_capacity(band.len());
    base_offsets.push(0);
    let mut offset_cumsum = 0;
    for (start, end) in band.iter_values() {
        offset_cumsum += end - start;
        base_offsets.push(offset_cumsum);
    }

    let band_len = offset_cumsum;

    let mut all_scores = vec![f32::INFINITY; band_len];
    let mut traceback_vec = vec![0; band_len];

    forward_pass(
        &mut all_scores,
        &mut traceback_vec,
        signal,
        levels,
        band,
        &base_offsets,
        algo
    );

    let mut path = vec![0; levels.len() + 1];

    traceback(
        &mut path,
        band,
        &base_offsets,
        &traceback_vec
    );

    path
}