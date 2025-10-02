use std::collections::HashMap;

use crate::{
    error::core::reformat::{
        ReformatedRowInterpError, 
        ReformatedRowStatError
    }, 
    execute::config::Stats
};

/// Container for read data processed via read-wise statistics.
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

    /// Consumes self and returns the contained data.
    /// 
    /// The contained elements are:
    /// 1. The base sequence encoded in a Vec<u8>
    /// 2. The statistics of each base collected in a Hashmap
    ///    where the keys are the calculated statistics. The
    ///    values are Vec<f64> with the same length as the base
    ///    sequence
    pub(crate) fn into_inner(self) -> Result<(
        Vec<u8>,
        HashMap<Stats, Vec<f64>>
    ), ReformatedRowStatError> {
        let mut stats_collection = HashMap::new();
        let bases = self.bases;
        let n_bases = bases.len();

        Self::collect_data(Stats::Mean, self.mean, n_bases, &mut stats_collection)?;
        Self::collect_data(Stats::Median, self.median, n_bases, &mut stats_collection)?;
        Self::collect_data(Stats::StDev, self.stdev, n_bases, &mut stats_collection)?;
        Self::collect_data(Stats::Dwell, self.dwell, n_bases, &mut stats_collection)?;
        Self::collect_data(Stats::SignalToNoise, self.signal_to_noise, n_bases, &mut stats_collection)?;

        Ok((bases, stats_collection))
    }

    fn collect_data(
        stat: Stats,
        vec_opt: Option<Vec<f64>>, 
        n_bases: usize, 
        stats_collection: &mut HashMap<Stats, Vec<f64>>
    ) -> Result<(), ReformatedRowStatError> {
        if let Some(vec) = vec_opt {
            if vec.len() != n_bases {
                return Err(ReformatedRowStatError::InvalidLength(
                    stat, n_bases, vec.len()
                ));
            }
            stats_collection.insert(stat, vec);
        }
        Ok(())
    }
}


/// Container for read data processed via interpolation.
///
/// Interpolated signals are stored in a nested vector,
/// where the outer vector contains the interpolated signals
/// (inner vectors) for each base.
///
/// # Fields
/// * `bases` - The sequence of bases associated with this row.
/// * `signal_interp` - The interpolated signals for each base
/// * `dwells` - The dwells for each base
/// 
/// # Data shape
/// With a region of interest of M bases and an interpolation
/// target size of T:
/// * `bases`: M
/// * `singal_interp`: M x T
/// * `dwells`: M 
pub(crate) struct ReformatedInterp {
    bases: Vec<u8>,
    signal_interp: Vec<Vec<f64>>,
    dwells: Vec<f64>
}

impl ReformatedInterp {
    /// Initializes a new instance for a given read.
    /// 
    /// Bases and dwells are filled directly. The signals
    /// are left empty here, and are filled base by base
    /// afterwards.
    /// 
    /// # Arguments
    /// * `bases` - The u8 encodings of the sequence
    /// * `dwell` - The dwells for each base
    pub(super) fn new(bases: &[u8], dwells: &[f64]) -> Self {
        let length = bases.len();
        let signal_interp = Vec::with_capacity(length);
        Self { 
            bases: bases.to_vec(), 
            signal_interp,
            dwells: dwells.to_vec()
        }
    }

    /// Adds the interpolated signal to the interpolated signals
    /// 
    /// # Arguments
    /// * `signal_interp` - The interpolated signal for the i-th
    ///                     base
    pub(super) fn push_signal(&mut self, singal_interp: Vec<f64>) {
        self.signal_interp.push(singal_interp.to_vec());
    }

    /// Consumes self and returns the contained data.
    /// 
    /// The contained elements are:
    /// 1. The base sequence encoded in a Vec<u8>
    /// 2. The interpolated signal for each base in a nested vector
    ///    where the inner vector at index *i* corresponds to the 
    ///    interpolated signal for base *i*
    /// 3. The dwells for each base
    /// 
    /// # Returns
    /// * `Ok((Vec<u8>, Vec<Vec<f64>>, Vec<f64>))` - A tuple containing 
    ///     the sequence, the interpolated signal and the dwells.
    /// * `Err(ReformatedRowInterpError::LengthMismatch)` - If the length
    ///     of the dwells or the signal (outer) mismatches the length of
    ///     the sequence 
    pub(crate) fn into_inner(self) -> Result<(Vec<u8>, Vec<Vec<f64>>, Vec<f64>), ReformatedRowInterpError> {
        let n_bases = self.bases.len();
        
        if self.dwells.len() != n_bases {
            return Err(ReformatedRowInterpError::LengthMismatch("dwells", self.dwells.len(), n_bases));
        }

        if self.signal_interp.len() != n_bases {
            return Err(ReformatedRowInterpError::LengthMismatch("signal", self.dwells.len(), n_bases));
        }

        Ok((
            self.bases,
            self.signal_interp,
            self.dwells
        ))
    }
}

/// Wrapper enum containing the reformated data
/// from the base-wise statistics or interpolation
/// processing approach
pub(crate) enum ReformatedData {
    /// Contains base-wise statistics data
    Stats(ReformatedBaseStat),
    /// Contains interpolation data
    Interp(ReformatedInterp)
}

impl ReformatedData {
    /// Initialize Reformated data from a [`ReformatedBaseStat`]
    /// instance
    pub(super) fn from_basestat(data: ReformatedBaseStat) -> Self {
        Self::Stats(data)
    }

    /// Initialize Reformated data from a [`ReformatedInterp`]
    /// instance
    pub(super) fn from_interp(data: ReformatedInterp) -> Self {
        Self::Interp(data)
    }
}