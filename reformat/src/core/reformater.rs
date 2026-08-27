mod stats;
mod read_wise_stats;
mod interpolation;
pub(crate) mod reformated;

use uuid::Uuid;

use crate::{
    core::{
        filter::MatchedFilterInfo, 
        reformater::{
            interpolation::reformat_interpolate, read_wise_stats::reformat_read_wise_stats, reformated::ReformatedData, stats::{
                mean, 
                std_dev
            }
        }
    }, 
    error::core::reformat::ReformatError, 
    execute::{config::ReformatStrategy, output::output_data::OutputData}
};

/// Main reformating entry function.
/// 
/// Gets reference to the entier sequence, alignment,
pub(crate) fn reformat(
    read_id: &Uuid,
    sequence: &[u8],
    alignment: &[usize],
    signal: &[f32],
    chunk_info: &MatchedFilterInfo,
    reformat_strategy: &ReformatStrategy,
    norm_dwells: bool
) -> Result<OutputData, ReformatError> {
    // Slice sequence and alignment
    let sequence_slice = &sequence[chunk_info.start_index..chunk_info.end_index];
    let alignment_slice = &alignment[chunk_info.start_index..chunk_info.end_index+1];
    
    // Calculate, normalize (unless specified otherwise) and slice the dwell times
    // This could also be moved to the main function in case a read contains multiple
    // regions. But this seems a bit cleaner so I'll leave it here for now
    let dwells = alignment
        .windows(2)
        .map(|window| (window[1] - window[0]) as f32)
        .collect::<Vec<f32>>();

    let dwells = if norm_dwells {
        let dwells_mean = mean(&dwells)?;
        let dwells_std = std_dev(&dwells)?;
        if dwells_std == 0.0 {
            return Err(ReformatError::ZeroDivision);
        }

        dwells[chunk_info.start_index..chunk_info.end_index]
            .iter()
            .map(|&el| (el - dwells_mean) / dwells_std)
            .collect::<Vec<f32>>()
    } else {
        dwells[chunk_info.start_index..chunk_info.end_index].to_vec()
    };

    let reformated_data = match reformat_strategy {
        ReformatStrategy::ReadWiseStats { stats } => {
            let row = reformat_read_wise_stats(
                sequence_slice, 
                alignment_slice, 
                &dwells,
                signal,
                stats
            )?;

            ReformatedData::from_basestat(row)
        }
        ReformatStrategy::Interpolation { target_len } => {
            let row = reformat_interpolate(
                sequence_slice, 
                alignment_slice, 
                &dwells,
                signal, 
                *target_len
            )?;

            ReformatedData::from_interp(row)
        }
    };

    let reformated_row = OutputData::new(
        *read_id, 
        chunk_info.clone(), 
        reformated_data
    );

    Ok(reformated_row)
}