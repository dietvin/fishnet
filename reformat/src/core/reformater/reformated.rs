use uuid::Uuid;

use crate::{core::filter::{reference_region::ReferenceRegion, ChunkInfo}, error::core::reformat::ReformatedRowStatError, execute::config::Stats};

/// Container for per-base statistics of a read.
///
/// Holds optional vectors of values for each requested statistic.
/// Only the statistics requested by the user are allocated and filled.
///
/// # Fields
/// * `bases` - The sequence of bases associated with this row.
/// * `length` - Number of bases in the sequence.
/// * `mean` - Optional per-base mean values of the signal slice.
/// * `median` - Optional per-base median values of the signal slice.
/// * `std` - Optional per-base standard deviations of the signal slice.
/// * `dwell` - Optional per-base dwell times.
/// * `signal_to_noise` - Optional per-base signal-to-noise ratios.
pub(crate) struct ReformatedBaseStat {
    bases: Vec<u8>,
    length: usize,
    mean: Option<Vec<f64>>,
    median: Option<Vec<f64>>,
    stdev: Option<Vec<f64>>,
    dwell: Option<Vec<f64>>,
    signal_to_noise: Option<Vec<f64>>
}

impl ReformatedBaseStat {
    /// Initializes an empty `ReformatedRowStat` for a given set of statistics.
    ///
    /// Allocates vectors with capacity equal to the number of bases, only
    /// for the statistics requested in `stats`.
    ///
    /// # Arguments
    /// * `stats` - Slice of statistics to initialize.
    /// * `bases` - Sequence of bases for which stats will be computed.
    ///
    /// # Returns
    /// A `ReformatedRowStat` with empty vectors ready to be filled.
    pub(super) fn from_stats_empty(
        stats: &[Stats], 
        bases: &[u8]
    ) -> Self {
        let mut mean: Option<Vec<f64>> = None;
        let mut median: Option<Vec<f64>> = None;
        let mut stdev: Option<Vec<f64>> = None;
        let mut dwell: Option<Vec<f64>> = None;
        let mut signal_to_noise: Option<Vec<f64>> = None;

        let target_length = bases.len();

        for stat in stats {
            match stat {
                Stats::Mean => mean = Some(Vec::with_capacity(target_length)),
                Stats::Median => median = Some(Vec::with_capacity(target_length)),
                Stats::StDev => stdev = Some(Vec::with_capacity(target_length)),
                Stats::Dwell => dwell = Some(Vec::with_capacity(target_length)),
                Stats::SignalToNoise => signal_to_noise = Some(Vec::with_capacity(target_length)),
            }
        }

        Self { 
            bases: bases.to_vec(),
            length: target_length, 
            mean, 
            median, 
            stdev, 
            dwell, 
            signal_to_noise 
        }
    }

    /// Pushes a mean value for the next base.
    ///
    /// # Arguments
    /// * `value` - Computed mean to add.
    ///
    /// # Errors
    /// Returns `ReformatedRowStatError::UnexpectedStat` if mean was not
    /// requested during initialization.
    pub(super) fn push_mean(&mut self, value: f64) -> Result<(), ReformatedRowStatError> {
        if let Some(values) = &mut self.mean {
            values.push(value);
            Ok(())
        } else {
            Err(ReformatedRowStatError::UnexpectedStat(Stats::Mean))
        }
    }

    /// Pushes a median value for the next base.
    ///
    /// # Arguments
    /// * `value` - Computed median to add.
    ///
    /// # Errors
    /// Returns `ReformatedRowStatError::UnexpectedStat` if median was not
    /// requested during initialization.
    pub(super) fn push_median(&mut self, value: f64) -> Result<(), ReformatedRowStatError> {
        if let Some(values) = &mut self.median {
            values.push(value);
            Ok(())
        } else {
            Err(ReformatedRowStatError::UnexpectedStat(Stats::Median))
        }
    }

    /// Pushes a standard deviation value for the next base.
    ///
    /// # Arguments
    /// * `value` - Computed standard deviation to add.
    ///
    /// # Errors
    /// Returns `ReformatedRowStatError::UnexpectedStat` if standard deviation 
    /// was not requested during initialization.
    pub(super) fn push_std(&mut self, value: f64) -> Result<(), ReformatedRowStatError> {
        if let Some(values) = &mut self.stdev {
            values.push(value);
            Ok(())
        } else {
            Err(ReformatedRowStatError::UnexpectedStat(Stats::StDev))
        }
    }

    /// Pushes a dwell value for the next base.
    ///
    /// # Arguments
    /// * `value` - Computed dwell to add.
    ///
    /// # Errors
    /// Returns `ReformatedRowStatError::UnexpectedStat` if dwell was not
    /// requested during initialization.
    pub(super) fn push_dwell(&mut self, value: f64) -> Result<(), ReformatedRowStatError> {
        if let Some(values) = &mut self.dwell {
            values.push(value);
            Ok(())
        } else {
            Err(ReformatedRowStatError::UnexpectedStat(Stats::Dwell))
        }
    }

    /// Pushes a signal-to-noise value for the next base.
    ///
    /// # Arguments
    /// * `value` - Computed signal-to-noise to add.
    ///
    /// # Errors
    /// Returns `ReformatedRowStatError::UnexpectedStat` if signal-to-noise was not
    /// requested during initialization.
    pub(super) fn push_signal_to_noise(&mut self, value: f64) -> Result<(), ReformatedRowStatError> {
        if let Some(values) = &mut self.signal_to_noise {
            values.push(value);
            Ok(())
        } else {
            Err(ReformatedRowStatError::UnexpectedStat(Stats::SignalToNoise))
        }
    }
}


pub(crate) struct ReformatedInterp {
    bases: Vec<u8>,
    length: usize,
    signal_interp: Vec<Vec<f64>>,
    dwells: Vec<f64>
}

impl ReformatedInterp {
    pub(super) fn new(bases: &[u8], dwells: &[f64]) -> Self {
        let length = bases.len();
        let signal_interp = Vec::with_capacity(length);
        Self { 
            bases: bases.to_vec(), 
            length, 
            signal_interp,
            dwells: dwells.to_vec()
        }
    }

    pub(super) fn push_signal(&mut self, singal_interp: Vec<f64>) {
        self.signal_interp.push(singal_interp.to_vec());
    }
}


pub(crate) enum ReformatedData {
    Stats(ReformatedBaseStat),
    Interp(ReformatedInterp)
}

impl ReformatedData {
    pub(super) fn from_basestat(data: ReformatedBaseStat) -> Self {
        Self::Stats(data)
    }

    pub(super) fn from_interp(data: ReformatedInterp) -> Self {
        Self::Interp(data)
    }
}

pub(crate) struct ReformatedRow {
    read_id: Uuid,
    ref_name: Option<String>,
    ref_start: Option<usize>,
    matched_region_name: String,
    matched_region_start: usize,
    reformated_data: ReformatedData
}

impl ReformatedRow {
    pub(super) fn new(
        read_id: Uuid,
        reference_region: Option<ReferenceRegion>,
        chunk_info: ChunkInfo,
        reformated_data: ReformatedData
    ) -> Self {
        let (ref_name, ref_start) = match reference_region {
            Some(region) => (Some(region.name().to_string()), Some(region.start())),
            None => (None, None)
        };

        let matched_region_name = chunk_info.matched_element_name.clone();
        let matched_region_start = chunk_info.start_index;

        Self { 
            read_id: read_id, 
            ref_name,
            ref_start,
            matched_region_name,
            matched_region_start,
            reformated_data
        }
    }
}