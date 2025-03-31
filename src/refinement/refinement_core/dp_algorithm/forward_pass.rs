use std::env::current_exe;

use crate::refinement::refinement_core::bands::Band;
use crate::refinement::signal_map_refiner::settings::RefineAlgo;

use super::forward_step::forward_step_viterbi;
use super::forward_step_dwell_penalty::forward_step_dwell_penalty;

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