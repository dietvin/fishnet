use crate::error::refinement_errors::band_errors::{BandValidationError, SequenceBandError, SignalBandError};
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
    fn compute_signal_band(
        map: &[usize], // sequence_to_signal_map
        sequence_len: usize,
        half_bandwidth: usize,
        is_banded: bool    
    ) -> Result<Self, SignalBandError> {
        if is_banded && half_bandwidth == 0 {
            return Err(SignalBandError::InvalidOptions(half_bandwidth, is_banded));
        }

        let map_len = map.len();
        if sequence_len != map_len {
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
                    start[signal_idx] = (sequence_idx - half_bandwidth).max(0);
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

        Ok(band)
    }

    /// Transforms a signal band into a sequence band.
    ///
    /// # Returns
    /// * `Ok(())` if successful, or an error if validation fails
    /// or the band at hand is already a sequence band.
    fn convert_to_sequence_band(&mut self) -> Result<(), SequenceBandError> {
        if self.band_type == BandType::SequenceBand {
            return Err(SequenceBandError::AlreadySequenceBand);
        }

        let signal_len = self.start.len();
        let sequence_len = self.end[self.end.len() - 1];

        let mut sequence_start = vec![0; sequence_len];
        let mut sequence_end = vec![signal_len; sequence_len];

        let mut prev_e = self.end[0];
        let mut prev_s = self.start[0];
        
        for (signal_idx, (e, s)) in self.end.iter()
            .zip(self.start.iter())
            .skip(1)
            .enumerate() {
            // fill the start values
            if prev_e != *e {
                // Index doesn't need to be corrected (i.e. -1) as we skipped 
                // the first position and enumerate is called afterwards
                let lower_signal_pos = signal_idx;
                let lower_sequence_pos = self.end[lower_signal_pos];

                sequence_start[lower_sequence_pos] = lower_signal_pos;
                
                prev_e = *e;
            }
            // fill the end values
            if prev_s != *s {
                let upper_signal_pos = signal_idx + 1;
                let upper_sequence_pos = self.start[upper_signal_pos];

                sequence_end[upper_sequence_pos] = upper_signal_pos;

                prev_s = *s;
            }
        }
        
        self.band_type = BandType::SequenceBand;
        self.start = sequence_start;
        self.end = sequence_end;

        Band::validate_sequence_band(self, signal_len, sequence_len)?;

        Ok(())
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
        if *band.band_type() != BandType::SignalBand {
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
}