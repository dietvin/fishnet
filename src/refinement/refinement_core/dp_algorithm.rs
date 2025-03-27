mod forward_pass;
mod traceback;
mod forward_step;
mod forward_step_dwell_penalty;

use forward_pass::forward_pass;
use traceback::banded_traceback;

pub fn banded_dp(
    signal: &Vec<f32>,
    levels: &Vec<f32>,
    seq_band_start: &Vec<u32>,
    seq_band_end: &Vec<u32>,
    short_dwell_penalty: &Vec<f32>,
    core_method: &str
) -> Vec<u32> {
    let mut base_offsets = vec![0];
    let mut offset_cumsum = 0;
    for (start, end) in seq_band_start.iter().zip(seq_band_end) {
        offset_cumsum += end - start;
        base_offsets.push(offset_cumsum);
    }

    let band_len = offset_cumsum as usize;

    let mut all_scores: Vec<f32> = Vec::with_capacity(band_len);
    let mut traceback: Vec<i32> = Vec::with_capacity(band_len);
    forward_pass(
        &mut all_scores,
        &mut traceback,
        &signal,
        &levels,
        seq_band_start,
        seq_band_end,
        &base_offsets,
        short_dwell_penalty,
        core_method
    );

    let mut path: Vec<u32> = Vec::with_capacity(levels.len()+1);
    banded_traceback(
        &mut path, 
        seq_band_start, 
        seq_band_end, 
        &base_offsets, 
        &traceback
    );

    path    
}