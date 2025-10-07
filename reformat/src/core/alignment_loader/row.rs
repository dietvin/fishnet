use uuid::Uuid;

use crate::{
    core::filter::reference_region::ReferenceRegion, 
    error::core::loader::RowError
};

/// Represents a single row of alignment data.
/// 
/// Contains all the information for one sequencing read including its alignment
/// to a reference, sequence data, and raw signal values.
pub(crate) struct Row {
    /// Unique identifier for this sequencing read
    read_id: Uuid,
    /// Alignment coordinates mapping query to signal or reference to signal
    alignment: Vec<usize>,
    /// DNA/RNA sequence (or N-filled placeholder if not available)
    sequence: Vec<u8>,
    /// Reference sequence name and coordinates this read aligns to (if applicable)
    ref_region: Option<ReferenceRegion>,
    /// Z-standardized raw current measurements
    signal: Vec<f64>
}

impl Row {
    /// Creates a new Row with the provided data.
    /// 
    /// # Arguments
    /// * `read_id` - Unique identifier for the read
    /// * `alignment` - Vector of alignment coordinates
    /// * `sequence` - DNA/RNA sequence string
    /// * `signal` - Raw signal data
    /// * `ref_name` - Optional reference name
    /// * `ref_start` - Optional reference start position
    pub(super) fn new(
        read_id: Uuid,
        alignment: Vec<usize>,
        sequence: Vec<u8>,
        signal: Vec<f64>,
        ref_name: Option<String>,
        ref_start: Option<usize> 
    ) -> Result<Self, RowError> {
        let ref_region = match (ref_name, ref_start) {
            (Some(name), Some(start)) => Some(ReferenceRegion::from_start_and_length(
                name, 
                start, 
                sequence.len()
            )?),
            _ => None
        };

        Ok(Self {
            read_id,
            alignment,
            sequence,
            ref_region,
            signal
        })
    }

    /// Returns the read ID.
    pub(crate) fn read_id(&self) -> &Uuid {
        &self.read_id
    }

    /// Returns the alignment coordinates.
    pub(crate) fn alignment(&self) -> &[usize] {
        &self.alignment
    }

    /// Returns the sequence string.
    pub(crate) fn sequence(&self) -> &[u8] {
        &self.sequence
    }

    /// Returns the reference region if available.
    pub(crate) fn ref_region(&self) -> Option<&ReferenceRegion> {
        self.ref_region.as_ref()
    }

    /// Returns the raw signal data.
    pub(crate) fn signal(&self) -> &[f64] {
        &self.signal
    }
}
