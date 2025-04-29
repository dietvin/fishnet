use crate::error::alignment_errors::QueryToSignalError;

/// Aligns the query (base-called) sequence to raw signal measurements.
/// 
/// This function creates a mapping between positions in the base-called sequence and 
/// positions in the raw signal using the move table generated during base-calling.
/// The move table indicates when the sequencer detected a new base as it processed
/// the signal.
/// 
/// # Arguments
/// 
/// * `move_table` - A slice of boolean values where `true` indicates the sequencer detected
///                  a new base at this signal position.
/// * `stride` - The sampling rate factor - number of signal measurements taken per move table position.
/// * `signal_len` - The total number of measurements in the raw signal.
/// * `reverse_signal` - Whether to reverse the mapping direction (set to `true` for direct RNA 
///                      data that runs 3'->5' through the pore).
/// * `query_length` - The length of the base-called sequence in nucleotides.
/// 
/// # Returns
///
/// * `Ok(Vec<usize>)` - A vector where each index represents a query position and the value
///                      represents the corresponding signal position.
/// * `Err(QueryToSignalError)` - An error if the mapping is inconsistent with query or signal dimensions.
///
/// # Errors
///
/// * `QueryToSignalError::DiscordantToSequence` - If the number of steps in the mapping doesn't match the expected query length.
/// * `QueryToSignalError::DiscordantToSignal` - If the move table length is inconsistent with the signal length and stride.
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