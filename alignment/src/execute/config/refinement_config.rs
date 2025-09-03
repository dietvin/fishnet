/// Provides settings for signal mapping refinement.
/// 
/// This struct defines configurable parameters used in the refinement
/// of signal-to-sequence mappings. These include:
/// 
/// * `which_map_to_refine` - Which of the two alignments gets refined.
/// * `refinement_algo` - The algorithm used for refining the mapping.
/// * `n_refinement_iters` - Number of refinement iterations to perform.
/// * `half_bandwidth` - Half of the bandwidth size used in the refinement algorithm.
/// * `rescale_algo` - Algorithm applied for signal rescaling.
/// * `rough_rescale_algo` - Algorithm used for an initial, coarse signal rescaling step.
/// * `normalize_levels` - Whether to normalize the levels from the k-mer table.
/// 
/// Default settings can be initialized via `RefineSettings::default`, which reproduces the
/// settings used in the original Python implementation. Customized settings can be specified
/// using `RefineSettings::custom`.
#[derive(Debug, Clone)]
pub struct RefineSettings {
    /// Determines which alignment gets refines.
    which_map_to_refine: WhichToRefine,
    /// Algorithm used for mapping refinement.
    refinement_algo: RefineAlgo,
    /// Number of refinement iterations.
    n_refinement_iters: usize,
    /// Half of the bandwidth size used in the refinement algorithm.
    half_bandwidth: usize,
    /// The minimum number of measurements between one base and the next
    /// to enforce during the band adjustment
    adjust_band_min_size: usize,
    /// Algorithm used for signal rescaling.
    rescale_algo: RescaleAlgo,
    /// Algorithm used for an initial rough signal rescaling.
    rough_rescale_algo: RoughRescaleAlgo,
    /// Whether to normalize levels from the k-mer table.
    normalize_levels: bool,
}

impl Default for RefineSettings {
    /// Returns the default settings, which match the original Python implementation.
    /// 
    /// # Returns
    /// 
    /// * `RefineSettings` - The default settings.
    fn default() -> Self {
        RefineSettings {
            which_map_to_refine: WhichToRefine::default(),
            rough_rescale_algo: RoughRescaleAlgo::default(),
            rescale_algo: RescaleAlgo::default(),
            refinement_algo: RefineAlgo::default(),
            normalize_levels: false,
            n_refinement_iters: 1,
            half_bandwidth: 5,
            adjust_band_min_size: 2
        }
    }
}

impl RefineSettings {
    /// Creates a custom settings configuration.
    /// 
    /// This function allows for custom settings for the refinement.
    /// 
    /// # Arguments
    /// 
    /// * `which_map_to_refine` - Whether to refine the query-to-signal,
    ///                           ref-to-signal or both
    /// * `refinement_algo` - Refinement algorithm
    /// * `n_refinement_iters` - Number of refinement iterations
    /// * `half_bandwidth` - Half of the bandwidth size
    /// * `adjust_band_min_step` - Minimum step between one base and the next
    ///                            to enforce in band adjustment
    /// * `rescale_algo` - Signal rescaling algorithm
    /// * `rough_rescale_algo` - Rough signal rescaling algorithm
    /// * `normalize_levels` - Whether to normalize levels
    /// 
    /// # Returns
    /// 
    /// * `RefineSettings` - Custom settings
    pub fn custom(
        which_map_to_refine: WhichToRefine,
        refinement_algo: RefineAlgo,
        n_refinement_iters: usize,
        half_bandwidth: usize,
        adjust_band_min_size: usize,
        rescale_algo: RescaleAlgo,
        rough_rescale_algo: RoughRescaleAlgo,
        normalize_levels: bool
    ) -> Self {
        RefineSettings {
            which_map_to_refine,
            rough_rescale_algo,
            rescale_algo,
            refinement_algo,
            normalize_levels,
            n_refinement_iters,
            half_bandwidth,
            adjust_band_min_size
        }
    }

    pub fn which_map_to_refine(&self) -> &WhichToRefine {
        &self.which_map_to_refine
    }

    pub fn refinement_algo(&self) -> &RefineAlgo {
        &self.refinement_algo
    }

    pub fn n_refinement_iters(&self) -> &usize {
        &self.n_refinement_iters
    }

    pub fn half_bandwidth(&self) -> &usize {
        &self.half_bandwidth
    }

    pub fn adjust_band_min_size(&self) -> &usize {
        &self.adjust_band_min_size
    }

    pub fn rescale_algo(&self) -> &RescaleAlgo {
        &self.rescale_algo
    }

    pub fn rough_rescale_algo(&self) -> &RoughRescaleAlgo {
        &self.rough_rescale_algo
    }

    pub fn normalize_levels(&self) -> &bool {
        &self.normalize_levels
    }
}


/// Enumeration of which alignment to refine.
#[derive(Debug, Clone, PartialEq)]
pub enum WhichToRefine {
    /// Refine the query-to-signal and reference-to-signal alignment
    Both,
    /// Refine only the query-to-signal alignment
    Query,
    /// Refine only the reference-to-signal alignment
    Reference
}

impl Default for WhichToRefine {
    /// Returns the default option for which alignment to refine. 
    /// Default: WhichToRefine::Query
    fn default() -> Self {
        Self::Query
    }
}


/// Enumeration of available refinement algorithms.
#[derive(Debug, Clone, PartialEq)]
pub enum RefineAlgo {
    /// Viterbi algorithm (short dwell times are not penalized).
    Viterbi,
    /// Dwell penalty algorithm, which discourages short dwell times.
    /// 
    /// * `target` - Preferred dwell time.
    /// * `limit` - Maximum dwell time that is penalized; dwell times above this value remain unaffected.
    /// * `weight` - Strength of the penalty applied to short dwell times.
    DwellPenalty {
        target: f32,
        limit: f32,
        weight: f32
    }
}

impl RefineAlgo {
    /// Returns the default option for dwell penalty refinement. 
    /// 
    /// Default options are:
    /// * target = 4.0
    /// * limit = 3.0
    /// * weight = 0.5
    pub fn default_dwell_penalty() -> Self {
        Self::DwellPenalty { target: 4.0, limit: 3.0, weight: 0.5 }
    }
}

impl Default for RefineAlgo {
    /// Returns the default option for the refinement algorithm. 
    /// 
    /// Default: Self::DwellPenalty { target: 4.0, limit: 3.0, weight: 0.5 }
    fn default() -> Self {
        Self::default_dwell_penalty()
    }
}


/// Enumeration of algorithms for rough rescaling of signals.
#[derive(Debug, Clone, PartialEq)]
pub enum RoughRescaleAlgo {
    /// No rough rescaling applied.
    NoRoughRescaling,
    /// Least-squares regression-based rescaling.
    /// 
    /// * `quantiles` - The quantiles based on which the scaling factors get calculated.
    ///                 Default values: [0.05, 0.1 , 0.15, 0.2 , 0.25, 0.3 , 0.35, 0.4 , 
    ///                 0.45, 0.5 , 0.55, 0.6 , 0.65, 0.7 , 0.75, 0.8 , 0.85, 0.9 , 0.95]
    /// * `clip_bases` - The number of bases that get clipped from the start and end of
    ///                  the levels. Default: 10
    /// * `use_base_center` - Whether to use only a single data point from the signal for
    ///                       each base. If false, all measurements are used for the 
    ///                       computation. Default: true
    LeastSquares {
        quantiles: Vec<f32>,
        clip_bases: usize,
        use_base_center: bool
    },
    /// Theil-Sen estimator-based rescaling.
    /// 
    /// * `quantiles` - The quantiles based on which the scaling factors get calculated.
    ///                 Default values: [0.05, 0.1 , 0.15, 0.2 , 0.25, 0.3 , 0.35, 0.4 , 
    ///                 0.45, 0.5 , 0.55, 0.6 , 0.65, 0.7 , 0.75, 0.8 , 0.85, 0.9 , 0.95]
    /// * `clip_bases` - The number of bases that get clipped from the start and end of
    ///                  the levels. Default: 10
    /// * `use_base_center` - Whether to use only a single data point from the signal for
    ///                       each base. If false, all measurements are used for the 
    ///                       computation. Default: true
    TheilSen {
        quantiles: Vec<f32>,
        clip_bases: usize,
        use_base_center: bool,
    }
}

impl RoughRescaleAlgo {
    /// Returns the default options for rough rescaling using least squares. 
    /// 
    /// Default options are:
    /// * quantiles = 0.05 to 0.95 in steps of 0.05
    /// * clip_bases = 10
    /// * use_base_center = true
    pub fn default_least_squares() -> Self {
        Self::LeastSquares { 
            quantiles: vec![
                0.05, 0.1, 0.15, 0.2, 0.25, 0.3, 0.35, 0.4,
                0.45, 0.5, 0.55, 0.6, 0.65, 0.7, 0.75, 0.8,
                0.85, 0.9, 0.95,
            ], 
            clip_bases: 10, 
            use_base_center: true 
        }
    }

    /// Returns the default options for rough rescaling using theil sen. 
    /// 
    /// Default options are:
    /// * quantiles = 0.05 to 0.95 in steps of 0.05
    /// * clip_bases = 10
    /// * use_base_center = true
    pub fn default_theil_sen() -> Self {
        RoughRescaleAlgo::TheilSen {
            quantiles: vec![
                0.05, 0.1, 0.15, 0.2, 0.25, 0.3, 0.35, 0.4,
                0.45, 0.5, 0.55, 0.6, 0.65, 0.7, 0.75, 0.8,
                0.85, 0.9, 0.95,
            ],
            clip_bases: 10,
            use_base_center: true,
        }
    }
}

impl Default for RoughRescaleAlgo {
    /// Returns the default option for rough rescaling. 
    /// 
    /// Default: RoughRescaleAlgo::NoRoughRescaling
    fn default() -> Self {
        Self::NoRoughRescaling
    }
}


/// Enumeration of algorithms for precise signal rescaling.
#[derive(Debug, Clone, PartialEq)]
pub enum RescaleAlgo {
    /// Least-squares regression-based rescaling. 
    /// 
    /// NOTE: THIS IS NOT IMPLEMENTED IN REMORA!
    /// 
    /// * `dwell_filter_lower_percentile` - Lower percentile for filtering bases based on dwell time
    ///                                     (bases with dwell time < lower_percentile value get removed)
    /// * `dwell_filter_upper_percentile` - Upper percentile for filtering bases based on dwell time
    ///                                     (bases with dwell time > upper_percentile value get removed)
    /// * `min_abs_level` - The minimum absolute expected signal intensity value. Expected intensities that 
    ///                     deviate less than this value from the mean of the expected intensity get removed. 
    /// * `n_bases_truncate` - The number of bases that will be ignored at the start and end. 
    /// * `min_num_filtered_levels` - Threshold of the minimum number of valid bases that are needed to
    ///                               perform the rescaling.
    LeastSquares {
        dwell_filter_lower_percentile: f32,
        dwell_filter_upper_percentile: f32,
        min_abs_level: f32,
        n_bases_truncate: usize,
        min_num_filtered_levels: usize
    },

    /// Theil-Sen estimator-based rescaling.
    /// 
    /// * `dwell_filter_lower_percentile` - Lower percentile for filtering bases based on dwell time
    ///                                     (bases with dwell time < lower_percentile value get removed)
    /// * `dwell_filter_upper_percentile` - Upper percentile for filtering bases based on dwell time
    ///                                     (bases with dwell time > upper_percentile value get removed)
    /// * `min_abs_level` - The minimum absolute expected signal intensity value. Expected intensities that 
    ///                     deviate less than this value from the mean of the expected intensity get removed. 
    /// * `n_bases_truncate` - The number of bases that will be ignored at the start and end. 
    /// * `min_num_filtered_levels` - Threshold of the minimum number of valid bases that are needed to
    ///                               perform the rescaling.
    /// * `max_points` - Limits the number of data points used in the estimation.
    ///                  A subset is randomly selected if the data set exceeds 
    ///                  this threshold. Gets ignored if equal to 0.
    TheilSen {
        dwell_filter_lower_percentile: f32,
        dwell_filter_upper_percentile: f32,
        min_abs_level: f32,
        n_bases_truncate: usize,
        min_num_filtered_levels: usize,
        max_points: usize
    }
}

impl RescaleAlgo {
    /// Returns the default options for rescaling using least squares. 
    /// 
    /// Default options are:
    /// * dwell_filter_lower_percentile = 0.1
    /// * dwell_filter_upper_percentile = 0.9
    /// * min_abs_level = 0.2
    /// * n_bases_truncate = 10
    /// * min_num_filtered_levels = 10
    pub fn default_least_squares() -> Self {
        RescaleAlgo::LeastSquares {
            dwell_filter_lower_percentile: 0.1,
            dwell_filter_upper_percentile: 0.9,
            min_abs_level: 0.2,
            n_bases_truncate: 10,
            min_num_filtered_levels: 10,
        }
    }

    /// Returns the default options for rescaling using theil sen. 
    /// 
    /// Default options are:
    /// * dwell_filter_lower_percentile = 0.1
    /// * dwell_filter_upper_percentile = 0.9
    /// * min_abs_level = 0.2
    /// * n_bases_truncate = 10
    /// * min_num_filtered_levels = 10
    /// * max_points = 1000
    pub fn default_theil_sen() -> Self {
        RescaleAlgo::TheilSen {
            dwell_filter_lower_percentile: 0.1,
            dwell_filter_upper_percentile: 0.9,
            min_abs_level: 0.2,
            n_bases_truncate: 10,
            min_num_filtered_levels: 10,
            max_points: 1000,
        }
    }
}

impl Default for RescaleAlgo {
    /// Returns the default option for rough rescaling. 
    /// 
    /// Default: Theil sen with the following options
    /// * dwell_filter_lower_percentile = 0.1
    /// * dwell_filter_upper_percentile = 0.9
    /// * min_abs_level = 0.2
    /// * n_bases_truncate = 10
    /// * min_num_filtered_levels = 10
    /// * max_points = 1000
    fn default() -> Self {
        Self::default_theil_sen()
    }
}
