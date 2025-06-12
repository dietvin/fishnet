use crate::{error::refinement_errors::band_errors::{BandValidationError, SequenceBandError, SignalBandError}, logger::get_log_vector_sample};
use std::fmt;

/// Enum representing the type of a band: SignalBand or SequenceBand.
#[derive(Debug, PartialEq, Clone, Copy)]
pub enum BandType {
    SignalBand,
    SequenceBand
}

impl fmt::Display for BandType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            BandType::SignalBand => write!(f, "SignalBand"),
            BandType::SequenceBand => write!(f, "SequenceBand"),  
        }
    }
}

/// An iterator over Band positions, yielding (start, end) pairs.
///
/// This struct is created by the `iter` method on `Band` or by using
/// a reference to a band directly in a for loop through the `IntoIterator` trait.
pub struct BandIterator<'a> {
    band: &'a Band,
    index: usize
}

impl<'a> Iterator for BandIterator<'a> {
    type Item = (usize, usize);

    /// Returns the next (start, end) pair in the band, or None if iteration is complete.
    fn next(&mut self) -> Option<Self::Item> {
        if self.index < self.band.start.len() {
            let result = (self.band.start[self.index], self.band.end[self.index]);
            self.index += 1;
            Some(result)
        } else {
            None
        }
    }
}


/// Represents a band with start and end indices. This is used during the
/// dynamic programming run to constrain the search range, reducing the 
/// number of needed calculations.
/// 
/// For a **signal band**, entry i corresponds to signal measurement i.
/// start\[i\] shows the first base, end\[i\] the last base that the 
/// measurement may potentially belong to.
/// 
/// For a **sequence band**, entry i corresponds to base i. start\[i\] shows 
/// the first signal measurement, end\[i\] the last signal measurement 
/// that the base may potentially belong to.
#[derive(Debug)]
pub struct Band {
    band_type: BandType,
    start: Vec<usize>,
    end: Vec<usize>
}

impl Band {
    /// Basic initialization function only intended for testing purposes.
    pub fn new(band_type: BandType,start: Vec<usize>, end: Vec<usize>) -> Self {
        Band { 
            band_type, 
            start, 
            end
        }
    }

    /// Computes a signal band given a sequence-to-signal map. 
    ///
    /// # Arguments
    /// * `map` - A sequence-to-signal index map.
    /// * `expected_levels` - Expected levels per sequence position.
    /// * `half_bandwidth` - Half-width of the band.
    /// * `is_banded` - Whether to apply banding constraints.
    ///
    /// # Returns
    /// * `Ok(Band)` if successful, or an error if validation fails.
    pub fn compute_signal_band(
        map: &[usize], // sequence_to_signal_map
        sequence_len: usize,
        half_bandwidth: usize,
        is_banded: bool    
    ) -> Result<Self, SignalBandError> {
        log::debug!(
            "compute_signal_band input: sequence_to_signal_map = {}, sequence_len = {}, half_bandwidth = {}, is_banded = {}",
            get_log_vector_sample(map, 10),
            sequence_len,
            half_bandwidth,
            is_banded,
        );
    
        if is_banded && half_bandwidth == 0 {
            return Err(SignalBandError::InvalidOptions(half_bandwidth, is_banded));
        }

        let map_len = map.len();
        if sequence_len != map_len - 1 {
            return Err(SignalBandError::LengthMismatch(map_len, sequence_len));
        }

        let signal_len = map[map_len - 1] - map[0];

        let mut start = vec![0 as usize; signal_len];
        let mut end = vec![sequence_len; signal_len];

        if is_banded {
            for sequence_idx in 0..sequence_len {
                // Iterate over the sequence intervals (i.e. the start end end signal indices for each base) 
                let sequence_start_idx = map[sequence_idx];
                let sequence_end_idx = map[sequence_idx + 1];
                for signal_idx in sequence_start_idx..sequence_end_idx {
                    // Add the sequence boundaries for each signal measurement to the start and end vectors
                    // (i.e. to which base can measurement x potentially belong)
                    if sequence_idx >= half_bandwidth {
                        // start is initialized with 0, so there is no need
                        // to check for the max btw sequence_idx - half_bandwidth and 0
                        start[signal_idx] = sequence_idx - half_bandwidth;
                    } 
                    end[signal_idx] = (sequence_idx + half_bandwidth + 1).min(sequence_len);
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

        let band = Band { 
            band_type: BandType::SignalBand, 
            start, 
            end 
        };
        Band::validate_signal_band(&band, signal_len, sequence_len)?;

        log::debug!(
            "compute_signal_band output: band_start = {}, band_end = {}",
            get_log_vector_sample(&band.start, 10),
            get_log_vector_sample(&band.end, 10)
        );

        Ok(band)
    }

    /// Validates a signal band.
    /// 
    /// # Arguments
    /// * `band` - Reference to a band
    /// * `signal_len` - The number of signal measurements
    /// * `sequence_len` - The number of bases
    /// 
    /// # Returns
    /// Ok(()) if the band is valid. Error if:
    /// * The band is a sequence band
    /// * The band doesn't start with 0
    /// * A band element has a length of 0
    /// * The length is invalid 
    /// * The end coordinate is invalid
    fn validate_signal_band(
        band: &Band, 
        signal_len: usize, 
        sequence_len: usize
    ) -> Result<(), SignalBandError> {
        if *band.band_type() != BandType::SignalBand {
            return Err(SignalBandError::ValidationError(
                BandValidationError::UnexpectedBandType(*band.band_type())
            ));
        }

        let start = band.start();
        let end = band.end();

        Band::validate_general_band(start, end)?;

        if start.len() != signal_len {
            return Err(SignalBandError::ValidationError(
                BandValidationError::InvalidBandLen(start.len(), signal_len)
            ));
        }
        if end[end.len() - 1] != sequence_len {
            return Err(SignalBandError::ValidationError(
                BandValidationError::InvalidEndCoord(end[end.len() - 1], sequence_len)
            ));
        } 
        Ok(())
    }


    /// Transforms a signal band into a sequence band.
    ///
    /// # Arguments
    /// * `min_step` - Minimum step between one base and the next to enforce in band adjustment.
    ///
    /// # Returns
    /// * `Ok(())` if successful, or an error if validation fails
    ///   or the band at hand is already a sequence band.
    pub fn convert_to_sequence_band(&mut self, min_step: usize) -> Result<(), SequenceBandError> {
        log::debug!(
            "convert_to_sequence_band input: self.start = {}, self.end = {}, min_step = {}",
            get_log_vector_sample(self.start(), 10),
            get_log_vector_sample(self.end(), 10),
            min_step
        );

        if self.band_type == BandType::SequenceBand {
            return Err(SequenceBandError::AlreadySequenceBand);
        }

        let signal_len = self.start.len();
        let sequence_len = self.end[self.end.len() - 1];

        let mut sequence_start = vec![0; sequence_len];
        let mut sequence_end = vec![signal_len; sequence_len];

        // Find positions where changes occur in end array (equivalent to lower_sig_pos in Python)
        for (signal_idx, window) in self.end.windows(2).enumerate() {
            if window[0] != window[1] {
                let lower_signal_pos = signal_idx + 1;  // +1 because we're looking at windows
                let lower_base_pos = self.end[signal_idx];  // This is equivalent to sig_band[1, lower_sig_pos - 1]
                sequence_start[lower_base_pos] = lower_signal_pos;
            }
        }

        // Find positions where changes occur in start array (equivalent to upper_sig_pos in Python)
        for (signal_idx, window) in self.start.windows(2).enumerate() {
            if window[0] != window[1] {
                let upper_signal_pos = signal_idx + 1;  // +1 because we're looking at windows
                let upper_base_pos = self.start[upper_signal_pos];
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

        self.band_type = BandType::SequenceBand;
        self.start = sequence_start;
        self.end = sequence_end;

        self.adjust_sequence_band(min_step)?;

        Band::validate_sequence_band(self, signal_len, sequence_len)?;

        log::debug!(
            "convert_to_sequence_band output: self.start = {}, self.end = {}",
            get_log_vector_sample(self.start(), 10),
            get_log_vector_sample(self.end(), 10)
        );

        Ok(())
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
    fn adjust_sequence_band(&mut self, min_step: usize) -> Result<(), SequenceBandError> {
        log::debug!(
            "adjust_sequence_band input: self.start = {}, self.end = {}, min_step = {}",
            get_log_vector_sample(self.start(), 10),
            get_log_vector_sample(self.end(), 10),
            min_step
        );

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

        log::debug!(
            "adjust_sequence_band output: self.start = {}, self.end = {}, min_step = {}",
            get_log_vector_sample(self.start(), 10),
            get_log_vector_sample(self.end(), 10),
            min_step
        );
        
        Ok(())
    }

    /// Validates a sequence band.
    /// 
    /// # Arguments
    /// * `band` - Reference to a band
    /// * `signal_len` - The number of signal measurements
    /// * `sequence_len` - The number of bases
    /// 
    /// # Returns
    /// Ok(()) if the band is valid. Error if:
    /// * The band is a signal band
    /// * The band doesn't start with 0
    /// * A band element has a length of 0
    /// * The length is invalid 
    /// * The end coordinate is invalid
    fn validate_sequence_band(
        band: &Band, 
        signal_len: usize, 
        sequence_len: usize
    ) -> Result<(), SequenceBandError> {
        if *band.band_type() != BandType::SequenceBand {
            return Err(SequenceBandError::ValidationError(
                BandValidationError::UnexpectedBandType(*band.band_type())
            ));
        }

        let start = band.start();
        let end = band.end();

        Band::validate_general_band(start, end)?;

        if start.len() != sequence_len {
            return Err(SequenceBandError::ValidationError(
                BandValidationError::InvalidBandLen(start.len(), sequence_len)
            ));
        }
        if end[end.len() - 1] != signal_len {
            return Err(SequenceBandError::ValidationError(
                BandValidationError::InvalidEndCoord(end[end.len() - 1], signal_len)
            ));
        }
        Ok(())
    }

    /// Validates aspects that need testing for both signal and sequence bands.
    /// 
    /// # Arguments
    /// * `start` - Start values
    /// * `end` - end values
    /// 
    /// # Returns
    /// Ok(()) if the band starts with 0 and doesn't have intervals of length 0.
    /// Error otherwise.
    fn validate_general_band(
        start: &Vec<usize>, 
        end: &Vec<usize>
    ) -> Result<(), BandValidationError> {
        if start[0] != 0 {
            return Err(BandValidationError::StartNonZero);
        }
        if end.iter().zip(start).any(|(e, s)| e <= s) {
            return Err(BandValidationError::ZeroLenRegion);
        }
        // skipping check for monotically increasing, as this is ensured in the functions
        Ok(())
    }

    /// Returns the band type
    pub fn band_type(&self) -> &BandType {
        &self.band_type
    }

    /// Returns the start vector.
    pub fn start(&self) -> &Vec<usize> {
        &self.start
    }

    /// Returns the end vector.
    pub fn end(&self) -> &Vec<usize> {
        &self.end
    }

    /// Returns an iterator over the band's positions.
    /// Each iteration yields a tuple of (start, end) for a position.
    pub fn iter(&self) -> BandIterator {
        BandIterator {
            band: self,
            index: 0
        }
    }

    pub fn len(&self) -> usize {
        self.start.len()
    }
}

impl<'a> IntoIterator for &'a Band {
    type Item = (usize, usize);
    type IntoIter = BandIterator<'a>;

    /// Creates an iterator that yields (start, end) pairs from a reference to a Band.
    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}