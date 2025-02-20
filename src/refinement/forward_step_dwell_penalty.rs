use super::forward_step::forward_step_viterbi;

pub fn banded_forward_dwell_penalty_step(
    current_scores: &mut [f32],
    current_traceback: &mut [i32],
    previous_scores: &[f32],
    current_level: f32,
    current_signal: &[f32],
    band_start_diff: usize,
    dwell_penalty: &[f32]
) {
    // Compute un-penalized band position scores for lookup after dwell_penalty range is searched
    let mut unpen_scores = vec![0.0f32; current_scores.len()];
    let mut unpen_tb = vec![0i32; current_traceback.len()];

    forward_step_viterbi(
        &mut unpen_scores, 
        &mut unpen_tb, 
        previous_scores, 
        current_level, 
        current_signal, 
        band_start_diff
    );

    // Loop over signal positions within this base band
    for band_pos in 0..curr_scores.len() {
        
    }
}
