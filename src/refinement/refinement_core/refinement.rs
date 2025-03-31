use crate::{error::refinement_errors::refine_errors::RefineError, refinement::refinement_core::bands::Band};
use super::{super::signal_map_refiner::settings::RefineSettings, dp_algorithm::banded_dp};

pub fn refinement(
    signal_to_sequence_map: Vec<usize>,
    signal: &Vec<f32>,
    expected_levels: &Vec<f32>,
    settings: &RefineSettings
) -> Result<Vec<usize>, RefineError> {
    // trim the signal and adjust the boundaries in the map so it starts at signal index 0
    let sig_map_start = signal_to_sequence_map[0];
    let sig_map_end = signal_to_sequence_map[signal_to_sequence_map.len() - 1];

    let signal_trimmed = &signal[sig_map_start..sig_map_end];
    let mut signal_to_sequence_map_zeroed = signal_to_sequence_map
        .iter()
        .map(|el| el-sig_map_start)
        .collect::<Vec<usize>>();

    let mut band = Band::compute_signal_band(
        &signal_to_sequence_map_zeroed,
        expected_levels.len(),
        *settings.half_bandwidth(),
        true
    )?;
    band.convert_to_sequence_band()?;

    let optimized_map = banded_dp(
        signal_trimmed, 
        expected_levels, 
        &band, 
        settings.refinement_algo()
    );

    Ok(
        optimized_map.iter().map(|el| el + sig_map_start).collect::<Vec<usize>>()
    )
}