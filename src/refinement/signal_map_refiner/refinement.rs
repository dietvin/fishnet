use crate::error::refinement_errors::refine_errors::RefineError;
use super::settings::RefineSettings;

pub fn refinement(
    signal_to_sequence_map: Vec<usize>,
    signal: &Vec<f32>,
    expected_levels: &Vec<f32>,
    settings: &RefineSettings
) -> Result<Vec<usize>, RefineError> {
    Ok(())
}