mod helpers;
pub mod viterbi;
pub mod dwell_penalty;

pub trait RefinementAlgo: Clone + Send {
    /// Processes a single base in the banded DP refinement process.
    fn forward_step(
        &self,
        scores: &mut [f32],
        traceback: &mut [i32],
        previous_scores: &[f32],
        level: f32,
        signal: &[f32],
        band_start_diff: usize
    );
}