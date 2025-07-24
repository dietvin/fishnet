use uuid::Uuid;

use crate::tables::reads_table::{Calibration, EndReason, Pore, PredictedScaling, TrackedScaling};

/// Represents a single read entry in a POD5 file, holding metadata parsed 
/// from the reads table. The actual signal is not loaded initially to 
/// avoid unnecessary memory usage. Instead, signal_indices refer to positions 
/// in a separate signal table used to reconstruct the signal on demand.
#[derive(Debug, Clone)]
pub struct Pod5Read {
    read_id: Uuid,
    signal_indices: Vec<u64>,
    read_number: Option<u32>,
    start: Option<u64>,
    median_before: Option<f32>,
    num_minknow_events: Option<u64>,
    tracked_scaling: TrackedScaling,
    predicted_scaling: PredictedScaling,
    num_reads_since_mux_change: Option<u32>,
    time_since_mux_change: Option<f32>,
    num_samples: Option<u64>,
    pore: Pore,
    calibration: Calibration,
    end_reason: EndReason,
    run_info_id: Option<String>,
    signal: Option<Vec<i16>>
}

impl Pod5Read {
    pub(crate) fn new(
        read_id: Uuid,
        signal_indices: Vec<u64>,
        read_number: Option<u32>,
        start: Option<u64>,
        median_before: Option<f32>,
        num_minknow_events: Option<u64>,
        tracked_scaling: TrackedScaling,
        predicted_scaling: PredictedScaling,
        num_reads_since_mux_change: Option<u32>,
        time_since_mux_change: Option<f32>,
        num_samples: Option<u64>,
        pore: Pore,
        calibration: Calibration,
        end_reason: EndReason,
        run_info_id: Option<String>,
        signal: Option<Vec<i16>>
    ) -> Self {
        Pod5Read {
            read_id,
            signal_indices,
            read_number,
            start,
            median_before,
            num_minknow_events,
            tracked_scaling,
            predicted_scaling,
            num_reads_since_mux_change,
            time_since_mux_change,
            num_samples,
            pore,
            calibration,
            end_reason,
            run_info_id,
            signal    
        }
    }

    /// Returns the unique identifier for this read.
    pub fn read_id(&self) -> &Uuid {
        &self.read_id
    }

    pub fn read_id_string(&self) -> String {
        self.read_id.to_string()
    }

    /// Returns the indices pointing to signal chunks and rows for this read.
    pub fn signal_indices(&self) -> &Vec<u64> {
        &self.signal_indices
    }

    /// Returns the read number if present.
    pub fn read_number(&self) -> Option<&u32> {
        self.read_number.as_ref()
    }

    /// Returns the start sample index if present.
    pub fn start(&self) -> Option<&u64> {
        self.start.as_ref()
    }

    /// Returns the median before value if present.
    pub fn median_before(&self) -> Option<&f32> {
        self.median_before.as_ref()
    }

    /// Returns the number of events detected by MinKNOW for this read if present.
    pub fn num_minknow_events(&self) -> Option<&u64> {
        self.num_minknow_events.as_ref()
    }

    /// Returns the tracked scaling parameters used by MinKNOW for this read.
    pub fn tracked_scaling(&self) -> &TrackedScaling {
        &self.tracked_scaling
    }

    /// Returns the predicted scaling parameters used by Guppy for this read.
    pub fn predicted_scaling(&self) -> &PredictedScaling {
        &self.predicted_scaling
    }

    /// Returns the number of reads since the last mux change if present.
    pub fn num_reads_since_mux_change(&self) -> Option<&u32> {
        self.num_reads_since_mux_change.as_ref()
    }

    /// Returns the time in seconds since the last mux change if present.
    pub fn time_since_mux_change(&self) -> Option<&f32> {
        self.time_since_mux_change.as_ref()
    }

    /// Returns the total number of signal samples for this read if present.
    /// The value is taken from the reads table entry if the signal is not
    /// yet present. Otherwise the length of the signal is used directly.
    pub fn num_samples(&self) -> Option<u64> {
        match self.signal() {
            Some(s) => Some(s.len() as u64),
            None => self.num_samples
        }
    }

    /// Returns the pore metadata for this read.
    pub fn pore(&self) -> &Pore {
        &self.pore
    }

    /// Returns the calibration data for this read.
    pub fn calibration(&self) -> &Calibration {
        &self.calibration
    }

    /// Returns the reason the read ended (e.g. signal drop, mux change, etc).
    pub fn end_reason(&self) -> &EndReason {
        &self.end_reason
    }

    /// Returns the run information ID this read is associated with, if available.
    pub fn run_info_id(&self) -> Option<&String> {
        self.run_info_id.as_ref()
    }

    /// Returns the actual signal data if it has been loaded.
    pub fn signal(&self) -> Option<&Vec<i16>> {
        self.signal.as_ref()
    }
    
    /// Returns the actual signal data as a mutable reference
    /// if it has been loaded.
    pub fn signal_mut(&mut self) -> Option<&mut Vec<i16>> {
        self.signal.as_mut()
    }

    /// Returns the read number or an error if it is not available.
    pub fn require_read_number(&self) -> Result<&u32, Pod5ReadError> {
        Ok(self.read_number().ok_or(Pod5ReadError::MissingField("read_number"))?)       
    }

    /// Returns the start sample index or an error if it is not available.
    pub fn require_start(&self) -> Result<&u64, Pod5ReadError> {
        Ok(self.start().ok_or(Pod5ReadError::MissingField("start"))?)
    }

    /// Returns the median before value or an error if it is not available.
    pub fn require_median_before(&self) -> Result<&f32, Pod5ReadError> {
        Ok(self.median_before().ok_or(Pod5ReadError::MissingField("median_before"))?)
    }

    /// Returns the number of MinKNOW events or an error if it is not available.
    pub fn require_num_minknow_events(&self) -> Result<&u64, Pod5ReadError> {
        Ok(self.num_minknow_events().ok_or(Pod5ReadError::MissingField("num_minknow_events"))?)
    }

    /// Returns the number of reads since the last mux change or an error if not available.
    pub fn require_num_reads_since_mux_change(&self) -> Result<&u32, Pod5ReadError> {
        Ok(self.num_reads_since_mux_change().ok_or(Pod5ReadError::MissingField("num_reads_since_mux_change"))?)
    }

    /// Returns the time since the last mux change or an error if not available.
    pub fn require_time_since_mux_change(&self) -> Result<&f32, Pod5ReadError> {
        Ok(self.time_since_mux_change().ok_or(Pod5ReadError::MissingField("time_since_mux_change"))?)
    }

    /// Returns the number of samples in the signal or an error if not available.
    pub fn require_num_samples(&self) -> Result<u64, Pod5ReadError> {
        Ok(self.num_samples().ok_or(Pod5ReadError::MissingField("num_samples"))?)
    }

    pub fn require_calibration_offset(&self) -> Result<&f32, Pod5ReadError> {
        match &self.calibration.offset {
            Some(offset) => Ok(offset),
            None => Err(Pod5ReadError::MissingField("calibration offset"))
        }
    }

    pub fn require_calibration_scale(&self) -> Result<&f32, Pod5ReadError> {
        match &self.calibration.scale {
            Some(scale) => Ok(scale),
            None => Err(Pod5ReadError::MissingField("calibration scale"))
        }
    }

    /// Returns the run info ID or an error if it is not available.
    pub fn require_run_info_id(&self) -> Result<&String, Pod5ReadError> {
        Ok(self.run_info_id().ok_or(Pod5ReadError::MissingField("run_info_id"))?)
    }

    /// Returns the signal data or an error if it is not loaded.
    pub fn require_signal(&self) -> Result<&Vec<i16>, Pod5ReadError> {
        Ok(self.signal().ok_or(Pod5ReadError::MissingField("signal"))?)
    }

    /// Returns the signal data as a mutable reference or an error if it is not loaded.
    pub fn require_signal_mut(&mut self) -> Result<&mut Vec<i16>, Pod5ReadError> {
        Ok(self.signal.as_mut().ok_or(Pod5ReadError::MissingField("signal"))?)
    }

    /// Sets the signal data for this read. Used after loading the signal from disk.
    pub(crate) fn set_signal(&mut self, signal: Vec<i16>) {
        self.signal = Some(signal)
    } 
}


/// Enumerates possible errors that can occur when attempting to access required 
/// fields of a Pod5Read. Currently includes a variant for missing or unset fields.
#[derive(Debug, thiserror::Error)]
pub enum Pod5ReadError {
    #[error("Field '{0}' is missing or null")]
    MissingField(&'static str),
}
