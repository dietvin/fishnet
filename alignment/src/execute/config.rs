use std::{path::PathBuf, vec};

use clap::ArgMatches;
use helper::{errors::CliError, file_handling::{check_and_get_pod5_input, check_input_file, check_output_file}, io::OutputFormat};
use log::LevelFilter;

use crate::execute::config::helpers::calc_quantiles;

mod helpers;


/// Top-level configuration struct built from parsed CLI arguments.
///
/// Every field corresponds to a validated, strongly-typed representation of a
/// CLI flag. `Config` is constructed once via [`Config::from_argmatches`] and
/// then passed through the dispatch layer to the concrete pipeline implementation.
pub struct Config {
    /// Path to the input BAM file containing (aligned) reads.
    pub bam_path: PathBuf,
    /// Paths to one or more input POD5 files (or directories)
    /// containing raw nanopore signal data.
    pub pod5_paths: Vec<PathBuf>,
    /// Controls where and how pipeline results are written to disk.
    pub output_config: OutputConfig,
    /// Optional path to an external k-mer signal-level table together with a
    /// normalisation flag. When `None` an embedded the table is used.
    pub kmer_table_config: Option<KmerTableConfig>,
    /// `true` when the input data originates from a direct-RNA sequencing run;
    /// affects strand orientation handling during alignment.
    pub is_drna: bool,
    /// Number of parallel worker threads used for alignment and refinement.
    pub n_threads: usize,
    /// Capacity (in items) of the inter-thread data channels.
    pub queue_size: usize,
    /// Selects which sequence(s) (query/reference/both) are aligned
    /// against the raw signal.
    pub alignment_type: AlignmentType,
    /// Logging destination and verbosity level.
    pub log_config: LogConfig,
    /// Optional configuration for the rough (coarse) rescaling step that
    /// is applied before fine rescaling. `None` disables rough rescaling.
    pub rough_rescale_config: Option<RoughRescaleConfig>,
    /// Algorithm and parameters used for fine signal rescaling.
    pub rescale_algo: RescaleAlgoOptions,
    /// Number of refinement iterations to run.
    pub refine_iters: usize,
    /// Algorithm used for the DP-based signal-to-sequence refinement step.
    pub refine_algo: RefineAlgoOptions,
    /// Half-bandwidth and minimum step size for the banded DP alignment.
    pub band_config: BandConfig
}

/// Path and normalisation flag for an external k-mer signal-level table.
pub(crate) struct KmerTableConfig {
    /// Filesystem path to the plain-text k-mer table file.
    pub path: PathBuf,
    /// When `true`, level values in the table are normalised before use.
    pub normalize_levels: bool
}

/// Selects which sequence(s) are aligned to the raw nanopore signal.
pub(crate) enum AlignmentType {
    /// Align only the query (basecalled) sequence to the signal.
    Query,
    /// Align only the reference sequence to the signal.
    Reference,
    /// Align both the query and reference sequences to the signal.
    Both
}

/// Logging destination and verbosity level.
pub(crate) struct LogConfig {
    /// Filesystem path of the log file that will be written.
    pub path: PathBuf,
    /// Maximum log level that will be emitted (e.g. `Info`, `Debug`).
    pub level: LevelFilter
} 

/// Controls where and how pipeline output is written.
pub(crate) struct OutputConfig {
    /// Filesystem path of the output file.
    pub path: PathBuf,
    /// Determines which fields are included in each output record.
    /// (Only basic information / with sequence(s) / with sequence(s)
    /// and signal)
    pub level: OutputLevel,
    /// When `true`, an existing output file at `path` will be overwritten
    /// without prompting.
    pub force_overwrite: bool,
    /// Serialisation format of the output file (Parquet or JSON Lines)
    pub format: OutputFormat,
    /// Approximate size threshold (in bytes) at which buffered records are
    /// flushed and written as a single batch.
    pub batch_size_bytes: usize
}

/// Controls the verbosity / richness of each output record.
pub(crate) enum OutputLevel {
    /// Only positional and per-base alignment statistics; no sequence or signal
    /// data is included.
    Minimal,
    /// Minimal fields plus the aligned base sequence(s).
    WithSeq,
    /// All fields including the aligned base sequence(s) and the raw signal trace.
    WithSeqAndSig
}

/// Parameters for the coarse rescaling step applied before fine rescaling.
pub(crate) struct RoughRescaleConfig {
    /// Algorithm used to fit the rough linear rescaling model.
    pub algo: RoughRescaleAlgoOptions,
    /// Quantile values used to build the signal-level histogram for rescaling.
    /// Generated from `rough-rescale-quants-min`, `rough-rescale-quants-max`,
    /// and `rough-rescale-quants-steps`.
    pub quantiles: Vec<f32>,
    /// Number of bases to clip from each end of a read before computing
    /// signal-level statistics (reduces edge effects).
    pub clip_bases: usize,
    /// When `true`, only the centre sample of each base's dwell is used;
    /// when `false`, all samples within the dwell are included.
    pub use_base_center: bool
}

/// Algorithm used for the coarse (rough) rescaling step.
pub(crate) enum RoughRescaleAlgoOptions {
    /// Ordinary least-squares linear regression.
    LeastSquares,
    /// Theil–Sen robust estimator, less sensitive to outlier signal levels.
    TheilSen
}

/// Algorithm and parameters used for the fine signal rescaling step.
///
/// Both variants share a common set of dwell-filtering and level-quality
/// parameters; `TheilSen` additionally exposes a `max_points` cap to keep
/// runtime bounded on very long reads.
pub(crate) enum RescaleAlgoOptions {
    /// Ordinary least-squares linear rescaling.
    LeastSquares {
        /// Lower percentile bound for the per-base dwell-time filter.
        dwell_filter_lower_percentile: f32,
        /// Upper percentile bound for the per-base dwell-time filter.
        dwell_filter_upper_percentile: f32,
        /// Minimum absolute signal level; bases with levels below this
        /// threshold are excluded from the regression.
        min_abs_level: f32,
        /// Number of bases to truncate from each end of the read before
        /// fitting the rescaling model.
        n_bases_truncate: usize,
        /// Minimum number of level observations that must survive filtering
        /// for the rescaling fit to be attempted.
        min_num_filtered_levels: usize
    },
    /// Theil–Sen robust linear rescaling.
    TheilSen {
        /// Lower percentile bound for the per-base dwell-time filter.
        dwell_filter_lower_percentile: f32,
        /// Upper percentile bound for the per-base dwell-time filter.
        dwell_filter_upper_percentile: f32,
        /// Minimum absolute signal level; bases below this threshold are
        /// excluded from the regression
        min_abs_level: f32,
        /// Number of bases to truncate from each end of the read before
        /// fitting the rescaling model.
        n_bases_truncate: usize,
        /// Minimum number of level observations that must survive filtering
        /// for the rescaling fit to be attempted.
        min_num_filtered_levels: usize,
        /// Maximum number of (observed, expected) level pairs passed to the
        /// Theil–Sen solver; random sub-sampling is applied when the filtered
        /// set exceeds this limit.
        max_points: usize
    }
}

/// Algorithm used for the DP-based signal-to-sequence refinement step.
pub(crate) enum RefineAlgoOptions {
    /// Standard Viterbi decoding; selects the globally most probable
    /// segmentation path with no additional penalty terms.
    Viterbi,
    /// Viterbi decoding with an added dwell-time penalty that biases the path
    /// towards a target mean dwell, clamped by `limit` and scaled by `weight`.
    DwellPenalty {
        /// Target mean dwell time (in samples per base) that the penalty term
        /// drives the path towards.
        target: f32,
        /// Maximum penalty magnitude; deviations beyond this value are
        /// clamped.
        limit: f32,
        /// Scaling factor applied to the raw dwell deviation before it is
        /// added to the path cost.
        weight: f32
    }
}

/// Parameters for the banded dynamic-programming alignment.
pub(crate) struct BandConfig {
    /// Half-width of the DP band in samples; larger values increase recall at
    /// the cost of runtime.
    pub half_bandwidth: usize,
    /// Minimum number of signal samples the DP path must advance per base;
    /// prevents degenerate zero-dwell solutions.
    pub min_step: usize
}


impl Config {
    /// Constructs a [`Config`] from the `clap` [`ArgMatches`] produced by the
    /// top-level CLI parser.
    ///
    /// Each section below mirrors a logical group of CLI flags:
    ///
    /// | Section                  | Key flags |
    /// |--------------------------|-----------|
    /// | Input                    | `--bam`, `--pod5` |
    /// | Output                   | `--out`, `--force-overwrite`, `--output-level`, `--output-batch-size` |
    /// | K-mer table              | `--kmer-table`, `--normalize-levels` |
    /// | Chemistry                | `--rna` |
    /// | Threading                | `--threads`, `--queue-size` |
    /// | Alignment type           | `--alignment-type` |
    /// | Logging                  | `--log-path`, `--log-level` |
    /// | Rough rescaling          | `--rough-rescale-algo`, `--rough-rescale-quants-*`, `--rough-rescale-clip-bases`, `--rough-rescale-use-all-signal` |
    /// | Fine rescaling           | `--rescale-algo`, `--rescale-dwell-filter-*`, `--rescale-min-abs-level`, `--rescale-num-bases-truncate`, `--rescale-min-num-filtered-levels`, `--rescale-max-len` |
    /// | Refinement               | `--refine-iters`, `--refine-algo`, `--dwell-penalty-*` |
    /// | Band                     | `--half-bandwidth`, `--min-band-size` |
    ///
    /// # Errors
    /// Returns a [`CliError`] if any required argument is absent
    /// ([`CliError::ArgumentNone`]) or if a numeric value fails a range check
    /// ([`CliError::InvalidArgument`]).  The caller is responsible for
    /// translating the error into a user-facing message.
    pub(crate) fn from_argmatches(matches: &ArgMatches) -> Result<Self, CliError> {

        // BAM input

        let bam_path = matches.get_one::<PathBuf>("bam").ok_or(
            CliError::ArgumentNone("bam".to_string())
        )?.clone();
        check_input_file(&bam_path, "bam")?;

        // POD5 input

        let pod5_input_raw = matches.get_many::<PathBuf>("pod5").ok_or(
            CliError::ArgumentNone("pod5".to_string()) 
        )?.map(|buf| buf.clone()).collect::<Vec<PathBuf>>();
        let pod5_paths = check_and_get_pod5_input(pod5_input_raw)?;

        // Ouput config

        let output_path_raw = matches.get_one::<PathBuf>("out").ok_or(
            CliError::ArgumentNone("out".to_string()) 
        )?.clone();

        let force_overwrite = matches.get_flag("force-overwrite");

        let (output_path, output_format) = check_output_file(
            &output_path_raw,
            force_overwrite,
            vec![
                helper::io::OutputFormat::Parquet,
                helper::io::OutputFormat::Json,
            ]
        )?;

        let output_level_raw = matches.get_one::<String>("output-level").ok_or(
            CliError::ArgumentNone("output-level".to_string()) 
        )?.clone();
        let output_level = match output_level_raw.as_str() {
            "1" => OutputLevel::Minimal,
            "2" => OutputLevel::WithSeq,
            "3" => OutputLevel::WithSeqAndSig,
            _ => unreachable!()
        };

        let output_batch_size = matches.get_one::<usize>("output-batch-size").ok_or(
            CliError::ArgumentNone("alignment-type".to_string()) 
        )?.clone();
        if output_batch_size == 0 {
            return Err(CliError::InvalidArgument("output-batch-size".to_string(), 0.to_string()));
        }

        let output_config = OutputConfig {
            path: output_path,
            level: output_level,
            force_overwrite: force_overwrite,
            format: output_format,
            batch_size_bytes: output_batch_size
        };

        // Kmer levels table config

        let kmer_table_input = matches.get_one::<PathBuf>("kmer-table").cloned();
        let normalize_levels = matches.get_flag("normalize-levels");
        let kmer_table_config = if let Some(kmer_table_path) = kmer_table_input {
            check_input_file(&kmer_table_path, "txt")?;
            Some(KmerTableConfig {
                path: kmer_table_path,
                normalize_levels
            })
        } else {
            None
        };
    
        // RNA input

        let is_drna = matches.get_flag("rna");

        // Number of worker threads

        let n_threads = *matches.get_one::<usize>("threads").ok_or(
            CliError::ArgumentNone("threads".to_string()) 
        )?;

        if n_threads == 0 {
            return Err(
                CliError::InvalidArgument("threads".to_string(), 0.to_string())
            );
        }

        // Threading queue size

        let queue_size = *matches.get_one::<usize>("queue-size").ok_or(
            CliError::ArgumentNone("queue-size".to_string()) 
        )?;

        if queue_size == 0 {
            return Err(
                CliError::InvalidArgument("queue-size".to_string(), 0.to_string())
            );
        }

        // Which sequences to align to the signal

        let alignment_type_raw = matches.get_one::<String>("alignment-type").ok_or(
            CliError::ArgumentNone("alignment-type".to_string()) 
        )?.clone();
        let alignment_type = match alignment_type_raw.as_str() {
            "query" => AlignmentType::Query,
            "reference" => AlignmentType::Reference,
            "both" => AlignmentType::Both,
            _ => unreachable!()
        };

        // Logging config

        let log_path = matches.get_one::<PathBuf>("log-path").ok_or(
            CliError::ArgumentNone("log-path".to_string()) 
        )?.clone();

        let log_level_raw = matches.get_one::<String>("log-level").ok_or(
            CliError::ArgumentNone("log-level".to_string()) 
        )?.clone();
        let log_level = match log_level_raw.as_str() {
            "off" => LevelFilter::Off,
            "error" => LevelFilter::Error,
            "warn" => LevelFilter::Warn,
            "info" => LevelFilter::Info,
            "debug" => LevelFilter::Debug,
            "trace" => LevelFilter::Trace,
            _ => unreachable!()
        };

        let log_config = LogConfig {
            path: log_path,
            level: log_level
        };

        // Rough rescaling config

        let rough_rescale_algo_raw = matches.get_one::<String>("rough-rescale-algo").ok_or(
            CliError::ArgumentNone("rough-rescale-algo".to_string())
        )?.clone();

        let rough_rescale_config = if rough_rescale_algo_raw != "none" {

            let rough_rescale_algo = match rough_rescale_algo_raw.as_str() {
                "least-squares" => RoughRescaleAlgoOptions::LeastSquares,
                "theil-sen" => RoughRescaleAlgoOptions::TheilSen,
                _ => unreachable!()
            };

            let rough_rescale_quants_min = *matches.get_one::<f32>("rough-rescale-quants-min").ok_or(
                CliError::ArgumentNone("rough-rescale-quants-min".to_string())
            )?;
    
            let rough_rescale_quants_max = *matches.get_one::<f32>("rough-rescale-quants-max").ok_or(
                CliError::ArgumentNone("rough-rescale-quants-max".to_string())
            )?;
    
            let rough_rescale_quants_steps = *matches.get_one::<usize>("rough-rescale-quants-steps").ok_or(
                CliError::ArgumentNone("rough-rescale-quants-steps".to_string())
            )?;

            let quantiles = calc_quantiles(
                rough_rescale_quants_min,
                rough_rescale_quants_max,
                rough_rescale_quants_steps
            );

            let rough_rescale_clip_bases = *matches.get_one::<usize>("rough-rescale-clip-bases").ok_or(
                CliError::ArgumentNone("rough-rescale-clip-bases".to_string())
            )?;

            let rough_rescale_use_center_only = !matches.get_flag("rough-rescale-use-all-signal");

            Some(RoughRescaleConfig {
                algo: rough_rescale_algo,
                quantiles: quantiles,
                clip_bases: rough_rescale_clip_bases,
                use_base_center: rough_rescale_use_center_only
            })
        } else {
            None
        };

        // Rescaling config

        let rescale_algo_str = matches.get_one::<String>("rescale-algo").ok_or(
            CliError::ArgumentNone("rescale-algo".to_string())
        )?.clone();

        let rescale_dwell_filter_lower_percentile = *matches.get_one::<f32>("rescale-dwell-filter-lower-quant").ok_or(
            CliError::ArgumentNone("rescale-dwell-filter-lower-quant".to_string())
        )?;

        let rescale_dwell_filter_upper_percentile = *matches.get_one::<f32>("rescale-dwell-filter-upper-quant").ok_or(
            CliError::ArgumentNone("rescale-dwell-filter-upper-quant".to_string())
        )?;

        // TODO: Check that rescale_dwell_filter_lower_percentile < rescale_dwell_filter_upper_percentile

        let rescale_min_abs_level = *matches.get_one::<f32>("rescale-min-abs-level").ok_or(
            CliError::ArgumentNone("rescale-min-abs-level".to_string())
        )?;

        let rescale_num_bases_truncate = *matches.get_one::<usize>("rescale-num-bases-truncate").ok_or(
            CliError::ArgumentNone("rescale-num-bases-truncate".to_string())
        )?;

        let rescale_min_num_filtered_levels = *matches.get_one::<usize>("rescale-min-num-filtered-levels").ok_or(
            CliError::ArgumentNone("rescale-min-num-filtered-levels".to_string())
        )?;
        if rescale_min_num_filtered_levels == 0 {
            return Err(
                CliError::InvalidArgument(
                    "rescale-min-num-filtered-levels".to_string(), 
                    rescale_min_num_filtered_levels.to_string()
                )
            );
        }

        let rescale_algo = match rescale_algo_str.as_str() {
            "least-squares" => {
                RescaleAlgoOptions::LeastSquares {
                    dwell_filter_lower_percentile: rescale_dwell_filter_lower_percentile,
                    dwell_filter_upper_percentile: rescale_dwell_filter_upper_percentile,
                    min_abs_level: rescale_min_abs_level,
                    n_bases_truncate: rescale_num_bases_truncate,
                    min_num_filtered_levels: rescale_min_num_filtered_levels,
                }
            },
            "theil-sen" => {
                let rescale_max_points = *matches.get_one::<usize>("rescale-max-len").ok_or(
                    CliError::ArgumentNone("rescale-max-len".to_string())
                )?;
                RescaleAlgoOptions::TheilSen { 
                    dwell_filter_lower_percentile: rescale_dwell_filter_lower_percentile,
                    dwell_filter_upper_percentile: rescale_dwell_filter_upper_percentile,
                    min_abs_level: rescale_min_abs_level,
                    n_bases_truncate: rescale_num_bases_truncate,
                    min_num_filtered_levels: rescale_min_num_filtered_levels,
                    max_points: rescale_max_points
                }
            },
            _ => unreachable!()
        };

        // Refinement config

        let refine_iters = *matches.get_one::<usize>("refine-iters").ok_or(
            CliError::ArgumentNone("refine-iters".to_string())
        )?;

        let refine_algo_str = matches.get_one::<String>("refine-algo").ok_or(
            CliError::ArgumentNone("refine-algo".to_string())
        )?.clone();

        let refine_algo = match refine_algo_str.as_str() {
            "viterbi" => RefineAlgoOptions::Viterbi,
            "dwell-penalty" => {
                let dwell_penalty_target = *matches.get_one::<f32>("dwell-penalty-target").ok_or(
                    CliError::ArgumentNone("dwell-penalty-target".to_string())
                )?;
                if dwell_penalty_target < 0.0 {
                    return Err(
                        CliError::InvalidArgument("dwell-penalty-target".to_string(), dwell_penalty_target.to_string())
                    );
                }

                let dwell_penalty_limit = *matches.get_one::<f32>("dwell-penalty-limit").ok_or(
                    CliError::ArgumentNone("dwell-penalty-limit".to_string())
                )?;
                if dwell_penalty_limit < 0.0 {
                    return Err(
                        CliError::InvalidArgument("dwell-penalty-limit".to_string(), dwell_penalty_limit.to_string())
                    );
                }
                
                let dwell_penalty_weight = *matches.get_one::<f32>("dwell-penalty-weight").ok_or(
                    CliError::ArgumentNone("dwell-penalty-weight".to_string())
                )?;
                if dwell_penalty_weight < 0.0 {
                    return Err(
                        CliError::InvalidArgument("dwell-penalty-weight".to_string(), dwell_penalty_weight.to_string())
                    );
                }

                RefineAlgoOptions::DwellPenalty { 
                    target: dwell_penalty_target, 
                    limit: dwell_penalty_limit, 
                    weight: dwell_penalty_weight
                }
            }
            _ => unreachable!()
        };

        // Band config
                let half_bandwidth = *matches.get_one::<usize>("half-bandwidth").ok_or(
            CliError::ArgumentNone("half-bandwidth".to_string())
        )?;
        if half_bandwidth == 0 {
            return Err(
                CliError::InvalidArgument("half-bandwidth".to_string(), half_bandwidth.to_string())
            );
        }

        let min_band_size = *matches.get_one::<usize>("min-band-size").ok_or(
            CliError::ArgumentNone("min-band-size".to_string())
        )?;
        if min_band_size == 0 {
            return Err(
                CliError::InvalidArgument("min-band-size".to_string(), min_band_size.to_string())
            );
        }

        let band_config = BandConfig {
            half_bandwidth, 
            min_step: min_band_size
        };

        Ok(Self { 
            bam_path,
            pod5_paths,
            output_config,
            kmer_table_config,
            is_drna,
            n_threads,
            queue_size,
            alignment_type,
            log_config,
            rough_rescale_config,
            rescale_algo,
            refine_iters,
            refine_algo,
            band_config
        })
    }
}