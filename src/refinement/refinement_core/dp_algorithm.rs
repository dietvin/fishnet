pub mod forward_pass;
mod traceback;
pub mod forward_step;
mod forward_step_dwell_penalty;

use forward_pass::forward_pass;
use traceback::banded_traceback;

use crate::refinement::signal_map_refiner::settings::RefineAlgo;

use super::bands::Band;

pub fn banded_dp(
    signal: &[f32],
    levels: &Vec<f32>,
    band: &Band,
    method: &RefineAlgo
) -> Vec<usize> {
    let mut base_offsets = Vec::with_capacity(band.len());
    base_offsets.push(0);
    let mut offset_cumsum = 0;
    for (start, end) in band {
        offset_cumsum += end - start;
        base_offsets.push(offset_cumsum);
    }

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