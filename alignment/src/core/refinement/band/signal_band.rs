use crate::{
    core::refinement::band::helpers::validate_band, error::core::refinement::band::SignalBandError,
};

/// A SignalBand with start and end indices. 
/// 
/// Functions as an intermediate band that is only used to
/// generate a SequenceBand from it.
/// 
/// Here, entry i corresponds to signal measurement i. start\[i\] shows the
/// first base, end\[i\] the last base that the measurement may potentially
/// belong to.
pub struct SignalBand {
    pub start: Vec<usize>,
    pub end: Vec<usize>
}

impl SignalBand {
    /// Initialize a SignalBand given a sequence-to-signal map
    ///
    /// # Arguments
    /// * `seq_to_signal_map` - A signal-to-sequence alignment
    /// * `seq_len` - The number of bases in the sequence
    /// * `half_bandwidth` - Half-width of the band
    /// * `is_banded` - Whether to apply banding constraints
    ///
    /// # Returns
    /// * `Ok(Band)` if successful, or an error if validation fails
    /// 
    /// # Errors
    /// 
    /// * `SignalBandError::InvalidOptions` - If half_bandwidth is 0 with banded enabled
    /// * `SignalBandError::LengthMismatch` - If the singal-to-sequence alignment doesn't
    ///                                       match the sequence length
    pub fn new(
        seq_to_signal_map: &[usize],
        seq_len: usize,
        half_bandwidth: usize,
        is_banded: bool
    ) -> Result<Self, SignalBandError> {
        if is_banded && half_bandwidth == 0 {
            return Err(SignalBandError::InvalidOptions(half_bandwidth, is_banded));
        }

        let map_len = seq_to_signal_map.len();
        if seq_len != map_len - 1 {
            return Err(SignalBandError::LengthMismatch(map_len, seq_len));
        }

        let signal_len = seq_to_signal_map[map_len - 1] - seq_to_signal_map[0];

        let mut start = vec![0 as usize; signal_len];
        let mut end = vec![seq_len; signal_len];

        if is_banded {
            for sequence_idx in 0..seq_len {
                // Iterate over the sequence intervals (i.e. the start end end signal indices for each base) 
                let sequence_start_idx = seq_to_signal_map[sequence_idx];
                let sequence_end_idx = seq_to_signal_map[sequence_idx + 1];
                for signal_idx in sequence_start_idx..sequence_end_idx {
                    // Add the sequence boundaries for each signal measurement to the start and end vectors
                    // (i.e. to which base can measurement x potentially belong)
                    if sequence_idx >= half_bandwidth {
                        // start is initialized with 0, so there is no need
                        // to check for the max btw sequence_idx - half_bandwidth and 0
                        start[signal_idx] = sequence_idx - half_bandwidth;
                    } 
                    end[signal_idx] = (sequence_idx + half_bandwidth + 1).min(seq_len);
                }
            }
        }

        // ensure monotonicity
        for i in 1..signal_len {
            start[i] = start[i].max(start[i - 1]);
        }
        for i in (0..signal_len - 1).rev() {
            end[i] = end[i].min(end[i + 1]);
        }

        let band = Self { start, end };
        validate_band(&band.start, &band.end)?;

        Ok(band)
    }
}