use crate::{error::core::reformat::ReadWiseStatsError, execute::{config::Stats, output::output_data::OutputRow}};

pub(super) fn reformat_read_wise_stats(
    sequence_slice: &[u8],
    alignment_slice: &[usize],
    dwells_slice: &[f32],
    full_signal: &[f32],
    stats: &Vec<Stats>
) -> Result<OutputRow, ReadWiseStatsError> {
    for i in 0..sequence_slice.len() {
        let base = sequence_slice[i];
        let signal_start_index = alignment_slice[i];
        let signal_end_index = alignment_slice[i+1];
        let signal_slice = &full_signal[signal_start_index..signal_end_index];

        for stat in stats {

        }
    }
    Ok(())
}