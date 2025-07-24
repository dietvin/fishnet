/*!
 * This module implements banded dynamic programming algorithms for optimal sequence-to-signal
 * alignment. It provides efficient path finding through a constrained search space defined by 
 * alignment bands.
 *
 * ## Core Algorithm
 *
 * The banded dynamic programming approach restricts the alignment search space to a predefined
 * band around the expected alignment path. This dramatically reduces computational complexity
 * from O(n*m) to O(n*w) where n is the sequence length, m is the signal length, and w is the
 * band width.
 *
 * ## Key Components
 *
 * ### Forward Pass
 * - Computes optimal alignment scores within the allowed band
 * - Fills dynamic programming matrix with minimum cost paths
 * - Supports various scoring methods and penalty functions
 * - Handles dwell time penalties for improved alignment accuracy
 *
 * ### Traceback
 * - Reconstructs the optimal alignment path from the computed scores
 * - Produces sequence-to-signal mapping as a series of signal indices
 * - Operates within the banded constraint space
 *
 * ## Features
 *
 * - **Memory Efficient**: Only stores scores within the allowed band region
 * - **Configurable Algorithms**: Supports multiple refinement algorithms via `RefineAlgo`
 * - **Flexible Banding**: Works with arbitrary band definitions and widths
 * - **Comprehensive Logging**: Detailed debug information for analysis and troubleshooting
 * - **Optimized Storage**: Pre-allocated vectors for performance-critical operations
 *
 * ## Input Requirements
 *
 * - **Signal**: Raw nanopore signal measurements
 * - **Levels**: Expected reference levels for each sequence position
 * - **Band**: Constraint region defining allowed alignment paths
 * - **Method**: Algorithm configuration specifying scoring and penalty parameters
 *
 * ## Output
 *
 * Returns a vector of signal indices representing the optimal sequence-to-signal alignment
 * path, where each element corresponds to the signal position aligned to a sequence base.
 */

pub mod forward_pass;
pub mod traceback;
pub mod forward_step;
mod forward_step_dwell_penalty;

use forward_pass::forward_pass;
use traceback::banded_traceback;


use crate::{logger::get_log_vector_sample, core::refinement::settings::RefineAlgo};

use super::bands::Band;

/// Performs banded dynamic programming to find optimal sequence-to-signal alignment.
///
/// This function implements a memory-efficient banded dynamic programming algorithm to align
/// a sequence of expected levels to observed signal measurements. The alignment is constrained
/// to a predefined band, which dramatically reduces computational complexity while maintaining
/// alignment accuracy.
///
/// # Arguments
/// * `signal` - Raw nanopore signal measurements
/// * `levels` - Expected reference levels for each sequence position
/// * `band` - Constraint band defining allowed alignment paths
/// * `method` - Algorithm configuration specifying scoring method and parameters
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
pub fn banded_dp(
    signal: &[f32],
    levels: &Vec<f32>,
    band: &Band,
    method: &RefineAlgo
) -> Vec<usize> {
    log::debug!(
        "banded_dp input: signal = {}, levels = {}, band start = {}, band end = {}, method = {:?}",
        get_log_vector_sample(signal, 10), 
        get_log_vector_sample(levels, 10), 
        get_log_vector_sample(band.start(), 10), 
        get_log_vector_sample(band.end(), 10), 
        method
    );
    let mut base_offsets = Vec::with_capacity(band.len());
    base_offsets.push(0);
    let mut offset_cumsum = 0;
    for (start, end) in band {
        offset_cumsum += end - start;
        base_offsets.push(offset_cumsum);
    }

    log::debug!(
        "banded_dp base offsets: base_offsets = {}",
        get_log_vector_sample(&base_offsets, 10)
    );

    let band_len = offset_cumsum;

    // let mut all_scores: Vec<f32> = Vec::with_capacity(band_len);
    // let mut traceback: Vec<i32> = Vec::with_capacity(band_len);

    let mut all_scores = vec![f32::INFINITY; band_len];
    let mut traceback = vec![0; band_len];

    forward_pass(
        &mut all_scores,
        &mut traceback,
        &signal,
        &levels,
        band,
        &base_offsets,
        method
    );

    log::debug!(
        "banded_dp after forward pass: all_scores = {}, traceback = {}",
        get_log_vector_sample(&all_scores, 20),
        get_log_vector_sample(&traceback, 20)
    );

    // let mut path: Vec<usize> = Vec::with_capacity(levels.len()+1);
    let mut path: Vec<usize> = vec![0; levels.len()+1];
    banded_traceback(
        &mut path, 
        band, 
        &base_offsets, 
        &traceback
    );

    path    
}