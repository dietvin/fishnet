use crate::{
    core::refinement::band::{
        helpers::{validate_band, validate_sequence_band}, 
        signal_band::SignalBand
    }, error::core::refinement::band::SequenceBandError, 
};

pub struct SequenceBand {
    start: Vec<usize>,
    end: Vec<usize>
}

impl SequenceBand {
    pub fn new(
        seq_to_signal_map: &[usize],
        seq_len: usize,
        half_bandwidth: usize,
        is_banded: bool,
        min_step: usize
    ) -> Result<Self, SequenceBandError> {
        let signal_band = SignalBand::new(
            seq_to_signal_map,
            seq_len,
            half_bandwidth,
            is_banded
        )?;
        Self::from_signal_band(signal_band, min_step)
    }

    /// Generate a SequenceBand from a given SignalBand.
    /// 
    /// # Arguments
    /// 
    /// * `signal_band` - A SignalBand instance that gets adjusted to a SequenceBand
    /// * `min_step` - Minimum step between one base and the next to enforce in band adjustment
    pub fn from_signal_band(
        signal_band: SignalBand,
        min_step: usize
    ) -> Result<Self, SequenceBandError> {
        let signal_len = signal_band.start.len();
        let sequence_len = signal_band.end[signal_band.end.len() - 1];

        let mut sequence_start = vec![0; sequence_len];
        let mut sequence_end = vec![signal_len; sequence_len];

        // Find positions where changes occur in end array (equivalent to lower_sig_pos in Python)
        for (signal_idx, window) in signal_band.end.windows(2).enumerate() {
            if window[0] != window[1] {
                let lower_signal_pos = signal_idx + 1;  // +1 because we're looking at windows
                let lower_base_pos = signal_band.end[signal_idx];  // This is equivalent to sig_band[1, lower_sig_pos - 1]
                sequence_start[lower_base_pos] = lower_signal_pos;
            }
        }

        // Find positions where changes occur in start array (equivalent to upper_sig_pos in Python)
        for (signal_idx, window) in signal_band.start.windows(2).enumerate() {
            if window[0] != window[1] {
                let upper_signal_pos = signal_idx + 1;  // +1 because we're looking at windows
                let upper_base_pos = signal_band.start[upper_signal_pos];
                sequence_end[upper_base_pos - 1] = upper_signal_pos;
            }
        }
        
        let mut max_so_far = 0;
        for idx in 0..sequence_start.len() {
            max_so_far = max_so_far.max(sequence_start[idx]);
            sequence_start[idx] = max_so_far;
        }

        let mut min_so_far = signal_len;
        for idx in (0..sequence_end.len()).rev() {
            min_so_far = min_so_far.min(sequence_end[idx]);
            sequence_end[idx] = min_so_far;
        }

        let mut sequence_band = Self{
            start: sequence_start,
            end: sequence_end
        };
        sequence_band.adjust(min_step)?;

        validate_sequence_band(
            &sequence_band.start,
            &sequence_band.end,
            signal_len,
            sequence_len
        )?;

        Ok(sequence_band)
    }

    /// Initilialization function for testing
    pub fn from_existing_vecs(
        start: Vec<usize>,
        end: Vec<usize>
    ) -> Self {
        Self { start, end }
    }

    /// Adjusts sequence band boundaries to disallow invalid paths.
    /// 
    /// This function ensures each band start and end is properly positioned
    /// relative to adjacent positions. It enforces monotonicity and minimum
    /// step size between consecutive positions.
    ///
    /// # Arguments
    /// * `min_step` - Minimum step between one base and the next to enforce in band adjustment.
    ///
    /// # Returns
    /// * `Ok(())` if successful, or an error if adjustment fails.
    ///
    /// # Details
    /// The function performs the following adjustments:
    /// 1. Ensures each start position is at least `min_step` less than the next position
    /// 2. Enforces monotonically increasing start positions
    /// 3. Ensures each end position is at least `min_step` more than the previous position
    /// 4. Enforces monotonically increasing end positions
    /// 
    /// The first start position and last end position are preserved from the original band.
    fn adjust(
        &mut self,
        min_step: usize
    ) -> Result<(), SequenceBandError> {
        // Remember the initial values for first start and last end
        let band_min = self.start[0];
        let band_max = self.end[self.end.len() - 1];
        let sequence_len = self.start.len();
        
        // Fix starts to make sure each start is at least min_step less than the next
        for seq_pos in (0..sequence_len - 1).rev() {
            if self.start[seq_pos] > self.start[seq_pos + 1].saturating_sub(min_step) {
                self.start[seq_pos] = self.start[seq_pos + 1].saturating_sub(min_step);
            }
        }
        
        // Restore the first start position
        self.start[0] = band_min;
        
        // Proceed through beginning of band ensuring only valid positions
        let mut seq_pos = 1;
        while seq_pos < sequence_len && self.start[seq_pos] <= self.start[seq_pos - 1] {
            self.start[seq_pos] = self.start[seq_pos - 1] + 1;
            seq_pos += 1;
        }
        
        // Fix ends to make sure each end is at least min_step more than the previous
        for seq_pos in 1..sequence_len {
            if self.end[seq_pos] < self.end[seq_pos - 1] + min_step {
                self.end[seq_pos] = self.end[seq_pos - 1] + min_step;
            }
        }
        
        // Restore the last end position
        self.end[sequence_len - 1] = band_max;
        
        // Proceed through end of band ensuring only valid positions
        if sequence_len > 1 {
            let mut seq_pos = sequence_len - 2;
            while self.end[seq_pos] >= self.end[seq_pos + 1] {
                self.end[seq_pos] = self.end[seq_pos + 1] - 1;
                if seq_pos == 0 {
                    break;
                }
                seq_pos -= 1;
            }
        }
        
        Ok(())
    }

    pub fn start(&self) -> &Vec<usize> {
        &self.start
    }

    pub fn end(&self) -> &Vec<usize> {
        &self.end
    }

    pub fn len(&self) -> usize {
        self.start.len()
    }

    pub fn iter_values(&self) -> SequenceBandIter<'_> {
        SequenceBandIter {
            band_len: self.len(),
            band: self,
            index: 0
        }
    }
}

pub struct SequenceBandIter<'a> {
    band_len: usize,
    band: &'a SequenceBand,
    index: usize
}

impl<'a> Iterator for SequenceBandIter<'a> {
    type Item = (usize, usize);
    fn next(&mut self) -> Option<Self::Item> {
        if self.index < self.band_len {
            let items = (self.band.start[self.index], self.band.end[self.index]);
            self.index += 1;
            Some(items)
        } else {
            None
        }
    }
}