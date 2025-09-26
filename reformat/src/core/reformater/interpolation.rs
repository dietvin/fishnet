use helper::interpolation::{interpolate, linspace};

use crate::{core::reformater::reformated::ReformatedInterp, error::core::reformat::InterpolationError};

pub(super) fn reformat_interpolate(
    sequence_slice: &[u8],
    alignment_slice: &[usize],
    dwells_slice: &[f64],
    full_signal: &[f64],
    target_len: usize
) -> Result<ReformatedInterp, InterpolationError> {
    let mut output_row = ReformatedInterp::new(sequence_slice, dwells_slice);

    for i in 0..sequence_slice.len() {
        let signal_start_index = alignment_slice[i];
        let signal_end_index = alignment_slice[i+1];
        let signal_slice = &full_signal[signal_start_index..signal_end_index];

        let original_x = linspace(0.0, 1.0, signal_slice.len())?;
        let target_x = linspace(0.0, 1.0, target_len)?;

        let interpolated_signal = interpolate(
            &original_x, 
            signal_slice, 
            &target_x)?;
        
        output_row.push_signal(interpolated_signal);
    }
    Ok(output_row)
}

pub(crate) struct InterpOutputRow {
    bases: Vec<u8>,
    length: usize,
    interpolated_signal: Vec<Vec<f32>>
}
