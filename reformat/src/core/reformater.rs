mod stats;
mod read_wise_stats;
mod interpolation;
pub(crate) mod reformated;

use crate::{core::{filter::ChunkInfo, reformater::{interpolation::reformat_interpolate, read_wise_stats::reformat_read_wise_stats, stats::{mean, std_dev}}}, error::core::reformat::ReformatError, execute::{config::{ReformatStrategy, Stats}, output::output_data::OutputRow}};

pub(crate) fn reformat(
    sequence: &[u8],
    alignment: &[usize],
    signal: &[f64],
    chunk_info: &ChunkInfo,
    reformat_strategy: &ReformatStrategy,
) -> Result<OutputRow, ReformatError> {
    // Slice sequence and alignment
    let sequence_slice = &sequence[chunk_info.start_index..chunk_info.end_index];
    let alignment_slice = &alignment[chunk_info.start_index..chunk_info.end_index+1];
    
    // Calculate, normalize and slice the dwell times
    let dwells = alignment
        .windows(2)
        .map(|window| (window[1] - window[0]) as f64)
        .collect::<Vec<f64>>();
    let dwells_mean = mean(&dwells)?;
    let dwells_std = std_dev(&dwells)?;
    let dwells_norm_slice = dwells[chunk_info.start_index..chunk_info.end_index]
        .iter()
        .map(|&el| (el - dwells_mean) / dwells_std)
        .collect::<Vec<f64>>();

    let output_line = match reformat_strategy {
        ReformatStrategy::ReadWiseStats { stats } => {
            let row = reformat_read_wise_stats(
                sequence_slice, 
                alignment_slice, 
                &dwells_norm_slice,
                signal,
                stats
            )?;
        }
        ReformatStrategy::Interpolation { target_len } => {
            let row = reformat_interpolate(
                sequence_slice, 
                alignment_slice, 
                &dwells_norm_slice,
                signal, 
                *target_len
            )?;
        }
    };

    Ok(output_line)
}