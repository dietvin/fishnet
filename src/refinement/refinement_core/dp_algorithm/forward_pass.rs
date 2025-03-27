use core::panic;

use super::forward_step::forward_step_viterbi;
use super::forward_step_dwell_penalty::forward_step_dwell_penalty;

pub fn forward_pass (
    all_scores: &mut Vec<f32>,
    traceback: &mut Vec<i32>,
    signal: &Vec<f32>,
    levels: &Vec<f32>,
    seq_band_start: &[u32],
    seq_band_end: &[u32],
    base_offsets: &[u32],
    short_dwell_penalty: &[f32],
    core_method: &str
) {
    let use_dwell_penalty = match core_method {
        "viterbi" => false,
        "dwell_penalty" => true,
        _ => panic!("Invalid core signal mapping refine method: {}", core_method)
    };

    let current_bandwidth = seq_band_end[0] as usize;

    let mut previous_scores = vec![f32::INFINITY; current_bandwidth];
    previous_scores[0] = 0.0;

    if use_dwell_penalty {
        forward_step_dwell_penalty(
            &mut all_scores[..current_bandwidth],
            &mut traceback[..current_bandwidth],
            &previous_scores,
            levels[0],
            &signal[..current_bandwidth],
            1,
            short_dwell_penalty
        );
    } else {
        forward_step_viterbi(
            &mut all_scores[..current_bandwidth],
            &mut traceback[..current_bandwidth],
            &previous_scores,
            levels[0],
            &signal[..current_bandwidth],
            1,
        );
    }
    
    let mut previous_bandwidth = current_bandwidth;
    let mut previous_band_start = 0;
    let mut previous_offset = 0;

    for base_idx in 1..levels.len() {
        let current_band_start = seq_band_start[base_idx] as usize;
        let current_band_end = seq_band_end[base_idx] as usize;
        let current_bandwidth = current_band_end - current_band_start;
        let current_offset = base_offsets[base_idx] as usize;

        if use_dwell_penalty {
        //     forward_step_dwell_penalty(
        //         &mut all_scores[current_offset..current_offset + current_bandwidth],
        //         &mut traceback[current_offset..current_offset + current_bandwidth],
        //         &all_scores[previous_offset..previous_offset + previous_bandwidth],
        //         levels[base_idx],
        //         &signal[current_band_start..current_band_end],
        //         current_band_start - previous_band_start,
        //         short_dwell_penalty
        //     )
        // } else {
        //     forward_step_viterbi(
        //         &mut all_scores[current_offset..current_offset + current_bandwidth],
        //         &mut traceback[current_offset..current_offset + current_bandwidth],
        //         &all_scores[previous_offset..previous_offset + previous_bandwidth],
        //         levels[base_idx],
        //         &signal[current_band_start..current_band_end],
        //         current_band_start - previous_band_start
        //     );
        }

        previous_band_start = current_band_start;
        previous_bandwidth = current_bandwidth;
        previous_offset = current_offset;
    }

}