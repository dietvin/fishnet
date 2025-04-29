pub mod forward_pass;
pub mod traceback;
pub mod forward_step;
mod forward_step_dwell_penalty;

use forward_pass::forward_pass;
use traceback::banded_traceback;


use crate::{logger::get_log_vector_sample, core::refinement::settings::RefineAlgo};

use super::bands::Band;

pub fn banded_dp(
    signal: &[f32],
    levels: &Vec<f32>,
    band: &Band,
    method: &RefineAlgo
) -> Vec<usize> {
    log::trace!(
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

    log::trace!(
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

    log::trace!(
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