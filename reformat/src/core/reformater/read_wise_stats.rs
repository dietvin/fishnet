use crate::{error::core::reformat::ReadWiseStatsError, execute::{config::Stats, output::output_data::OutputRow}};

pub(super) fn reformat_read_wise_stats(
    sequence_slice: &str,
    alignment_slice: &[usize],
    dwells_slice: &[f32],
    full_signal: &[f32],
    stats: &Vec<Stats>
) -> Result<OutputRow, ReadWiseStatsError> {
    todo!()
}