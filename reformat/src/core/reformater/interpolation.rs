use helper::interpolation::{interpolate, linspace};

use crate::{error::core::reformat::InterpolationError, execute::output::output_data::OutputRow};

pub(super) fn reformat_interpolate(
    sequence_slice: &[u8],
    alignment_slice: &[usize],
    dwells_slice: &[f64],
    full_signal: &[f64],
    target_len: usize
) -> Result<InterpOutputRow, InterpolationError> {
    for i in 0..sequence_slice.len() {
        let signal_start_index = alignment_slice[i];
        let signal_end_index = alignment_slice[i+1];
        let signal_slice = &full_signal[signal_start_index..signal_end_index];
        let dwell_value = dwells_slice[i];

        let original_x = linspace(0.0, 1.0, signal_slice.len())?;
        let target_x = linspace(0.0, 1.0, target_len)?;

        let interpolated_signal = interpolate(
            &original_x, 
            signal_slice, 
            &target_x)?;
    }
    Ok(())
}

pub(crate) struct InterpOutputRow {
    bases: Vec<u8>,
    length: usize,
    interpolated_signal: Vec<Vec<f32>>
}
