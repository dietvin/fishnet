use std::collections::HashMap;

use crate::error::core::loader::{ReferenceRegionError, ReferenceRegionsError};

pub(crate) struct ReferenceRegion {
    seq_name: String,
    start: usize,
    end: usize
}

impl ReferenceRegion {
    pub(crate) fn from_bed_entry(name: String, start: usize, end: usize) -> Result<Self, ReferenceRegionError> {
        todo!()
    }

    pub(crate) fn from_region_string(region_string: String) -> Result<Self, ReferenceRegionError> {
        todo!()
    }

    pub(crate) fn from_position_with_window(pos_with_window: String) -> Result<Self, ReferenceRegionError> {
        todo!()
    }

    pub(crate) fn from_position(name: String, site: usize) -> Result<Self, ReferenceRegionError> {
        todo!()
    }

    pub(crate) fn from_start_and_length(name: String, start: usize, length: usize) -> Result<Self, ReferenceRegionError> {
        todo!()
    }

    /// Checks if the other reference region is fully contained within self
    fn fully_contains(&self, other: ReferenceRegion) -> bool {
        other.seq_name == self.seq_name || other.start >= self.start && other.end <= self.end
    }

}

pub(crate) struct ReferenceRegions {
    /// Groups regions by their sequence name 
    regions: HashMap<String, Vec<ReferenceRegion>>
}

impl ReferenceRegions {
    pub(crate) fn from_strings(strings: Vec<String>) -> Result<Self, ReferenceRegionsError> {
        todo!()
    }

    pub(crate) fn from_bed(strings: Vec<String>) -> Result<Self, ReferenceRegionsError> {
        todo!()
    }

    pub(crate) fn contains(other_name: String, other_start: usize, other_end: usize) -> bool {
        todo!()
    }
}


