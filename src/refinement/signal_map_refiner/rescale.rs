use super::super::super::error::refinement_errors::signal_map_refiner_errors::RescaleError;

pub fn rough_rescale_lstsq(
    scale: f32, 
    shift: f32,
    seq_to_signal_map: &Vec<usize>,
    levels: &Vec<f32>,
    signal: &Vec<i16>,
    quantiles: &Vec<f32>,
    clip_bases: usize,
    use_base_center: bool
) -> Result<(f32, f32), RescaleError> {
    todo!()
}

pub fn rough_rescale_theil_sen(
    scale: f32, 
    shift: f32,
    seq_to_signal_map: &Vec<usize>,
    levels: &Vec<f32>,
    signal: &Vec<i16>,
    quantiles: &Vec<f32>,
    clip_bases: usize,
    use_base_center: bool,
    max_points: usize
) -> Result<(f32, f32), RescaleError> {
    todo!()
}

pub fn rescale_lstsq() -> Result<(f32, f32), RescaleError> {
    todo!()
}

pub fn rescale_theil_sen() -> Result<(f32, f32), RescaleError> {
    todo!()
}