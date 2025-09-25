use crate::{error::core::reformat::InterpolationError, execute::output::output_data::OutputRow};

pub(super) fn reformat_interpolate(
    sequence_slice: &[u8],
    alignment_slice: &[usize],
    dwells_slice: &[f32],
    full_signal: &[f32],
    target_len: usize
) -> Result<OutputRow, InterpolationError> {
    todo!()
}