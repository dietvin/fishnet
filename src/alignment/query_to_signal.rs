use super::super::error::alignment_errors::query_to_signal_errors::QueryToSignalError;

/// Align the query (base-called) sequence to the signal.
/// 
/// This function aligns the query sequence to the signal using the move table generated 
/// during base-calling. The move table indicates whether a the sequence moved to the next 
/// base during sequencing.
/// 
/// # Arguments
/// 
/// * `move_table` - A slice of boolean values indicating if a step forward is taking place
///                  at a given signal index.
/// * `stride` - The step size to use when mapping positions between query and signal.
/// * `signal_len` - The number of measurement in the signal
/// * `reverse_signal` - Whether to reverse the mapping (True for direct RNA data that runs 
///                      3'->5' through the pore)
/// * `query_length` - The length of the base-called sequence
/// 
/// # Returns
///
/// * `Ok(Vec<usize>)` - A vector mapping query positions to signal positions.
/// * `Err(QueryToSignalError)` - An error if the mapping is inconsistent with query or signal dimensions.
///
/// # Errors
///
/// * `QueryToSignalError::DiscordantToSequence` - If the number of steps in the mapping doesn't match the query length.
/// * `QueryToSignalError::DiscordantToSignal` - If the move table length is inconsistent with signal length and stride.
pub fn align_query_to_signal(
    move_table: &[bool],
    stride: usize, 
    signal_len: usize, 
    reverse_signal: bool,
    query_length: usize
) -> Result<Vec<usize>, QueryToSignalError> {
    let mut query_to_signal = Vec::with_capacity(signal_len+1);
    for (i, step_forward) in move_table.iter().enumerate() {
        if *step_forward {
            query_to_signal.push(i * stride);
        }
    }
    query_to_signal.push(signal_len);

    if reverse_signal {
        query_to_signal = query_to_signal
            .iter()
            .rev()
            .map(|el| signal_len - *el)
            .collect();
    }

    if query_to_signal.len()-1 != query_length {
        return Err(QueryToSignalError::DiscordantToSequence(
            query_to_signal.len(), query_length
        ));
    } else if move_table.len() != (signal_len/stride) {
        return Err(QueryToSignalError::DiscordantToSignal(
            query_to_signal.len(), signal_len, stride, signal_len/stride)
        );
    }

    Ok(query_to_signal)
}