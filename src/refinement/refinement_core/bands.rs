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

/// Represents a band with start and end indices.
#[derive(Debug)]
pub struct Band {
    band_type: BandType,
    start: Vec<usize>,
    end: Vec<usize>
}

impl Band {
    /// Computes a signal band given a sequence-to-signal map and optional banding constraints.
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
        expected_levels: &[f32],
        half_bandwidth: &usize,
        is_banded: &bool    
    ) -> Result<Self, SignalBandError> {
        if *is_banded && *half_bandwidth == 0 {
            return Err(SignalBandError::InvalidOptions(*half_bandwidth, *is_banded));
        }

        let map_len = map.len();
        let sequence_len = expected_levels.len();
        if sequence_len != map_len {
            return Err(SignalBandError::LengthMismatch(map_len, sequence_len));
        }

        let signal_len = map[map_len - 1] - map[0];

        let mut sequence_indices = Vec::new();
        for i in 0..sequence_len {
            // for each base repeat the index by the map length (map_end-map_start) 
            sequence_indices.extend(vec![i as usize; map[i + 1] - map[i]]);
        }

        let mut start = vec![0 as usize; signal_len];
        let mut end = vec![sequence_len; signal_len];

        if *is_banded {
            for (i, sequence_idx) in sequence_indices.iter().enumerate() {
                start[i] = (sequence_idx - half_bandwidth).max(0);
                end[i] = (sequence_idx + half_bandwidth + 1).min(sequence_len);
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
                // index doesn't need to be corrected (i.e. -1) as we skipped the first position
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

    pub fn band_type(&self) -> &BandType {
        &self.band_type
    }

    pub fn start(&self) -> &Vec<usize> {
        &self.start
    }

    pub fn end(&self) -> &Vec<usize> {
        &self.end
    }

}