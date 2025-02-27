use super::super::error::alignment_errors::query_to_signal_errors::QueryToSignalError;

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