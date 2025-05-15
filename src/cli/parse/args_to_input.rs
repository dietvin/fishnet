use std::path::PathBuf;

use clap::ArgMatches;
use log::LevelFilter;

use crate::{core::refinement::settings::{RefineAlgo, RefineSettings, RescaleAlgo, RoughRescaleAlgo, WhichToRefine}, error::cli_errors::CliError};

use super::helpers::{calc_quantiles, check_output_dir, check_and_get_pod5_input, check_input_file};

#[derive(Debug, Clone, PartialEq)]
pub enum WhichToAlign {
    Both,
    Query,
    Reference
}

#[derive(Debug, Clone, PartialEq)]
pub enum OutputFormat {
    Parquet,
    Json
}


impl Default for WhichToAlign {
    fn default() -> Self {
        WhichToAlign::Query
    }
}

#[derive(Debug, Clone)]
pub struct Config {
    bam_input: PathBuf,
    pod5_input: Vec<PathBuf>,
    kmer_table_input: PathBuf,
    output_dir: PathBuf,

    output_format: OutputFormat,
    output_batch_size: usize,
    force_overwrite: bool,
    is_drna: bool,
    alignment_type: WhichToAlign,
    n_threads: usize,
    queue_size: usize,
    log_level: LevelFilter,
    log_path: PathBuf,

    refine_settings: RefineSettings
}

impl Config {
    pub fn from_argmatches(matches: ArgMatches) -> Result<Self, CliError> {
        
        // Required arguments

        let bam_input = matches.get_one::<PathBuf>("bam").ok_or(
            CliError::ArgumentNone("bam".to_string())
        )?.clone();
        check_input_file(&bam_input, "bam")?;

        let pod5_input_raw = matches.get_many::<PathBuf>("pod5").ok_or(
            CliError::ArgumentNone("pod5".to_string()) 
        )?.map(|buf| buf.clone()).collect::<Vec<PathBuf>>();
        let pod5_input = check_and_get_pod5_input(pod5_input_raw)?;

        let kmer_table_input = matches.get_one::<PathBuf>("kmer-table").ok_or(
            CliError::ArgumentNone("kmer-table".to_string())
        )?.clone();
        check_input_file(&kmer_table_input, "txt")?;

        let output_dir = matches.get_one::<PathBuf>("output-dir").ok_or(
            CliError::ArgumentNone("output-dir".to_string()) 
        )?.clone();
        check_output_dir(&output_dir)?;

        let force_overwrite = matches.get_flag("force-overwrite");


        // Optional general arguments

        let output_format_raw = matches.get_one::<String>("output-format").ok_or(
            CliError::ArgumentNone("output-format".to_string()) 
        )?.clone();
        let output_format = match output_format_raw.as_str() {
            "parquet" => OutputFormat::Parquet,
            "jsonl" => OutputFormat::Json,
            _ => unreachable!()
        };

        let output_batch_size = matches.get_one::<usize>("output-batch-size").ok_or(
            CliError::ArgumentNone("alignment-type".to_string()) 
        )?.clone();
        if output_batch_size == 0 {
            CliError::InvalidArgument("output-batch-size".to_string(), 0.to_string());
        }

        let is_drna = matches.get_flag("rna");

        let alignment_type_raw = matches.get_one::<String>("alignment-type").ok_or(
            CliError::ArgumentNone("alignment-type".to_string()) 
        )?.clone();
        let alignment_type = match alignment_type_raw.as_str() {
            "query" => WhichToAlign::Query,
            "reference" => WhichToAlign::Reference,
            "both" => WhichToAlign::Both,
            _ => unreachable!()
        };

        let n_threads = *matches.get_one::<usize>("threads").ok_or(
            CliError::ArgumentNone("threads".to_string()) 
        )?;

        if n_threads == 0 {
            return Err(
                CliError::InvalidArgument("threads".to_string(), 0.to_string())
            );
        }

        let n_threads = if n_threads < 4 {
            1
        } else {
            n_threads
        };

        let queue_size = *matches.get_one::<usize>("queue-size").ok_or(
            CliError::ArgumentNone("queue-size".to_string()) 
        )?;

        if queue_size == 0 {
            return Err(
                CliError::InvalidArgument("queue-size".to_string(), 0.to_string())
            );
        } 

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

        let log_path = matches.get_one::<PathBuf>("log-path").ok_or(
            CliError::ArgumentNone("log-path".to_string()) 
        )?.clone();

        // check_input_file(&log_path, ".txt")?;

        // Optional refinement arguments

        let refine_iters = *matches.get_one::<usize>("refine-iters").ok_or(
            CliError::ArgumentNone("refine-iters".to_string())
        )?;

        let refine_algo_str = matches.get_one::<String>("refine-algo").ok_or(
            CliError::ArgumentNone("refine-algo".to_string())
        )?.clone();

        let refine_algo = match refine_algo_str.as_str() {
            "viterbi" => RefineAlgo::Viterbi,
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

                RefineAlgo::DwellPenalty { 
                    target: dwell_penalty_target, 
                    limit: dwell_penalty_limit, 
                    weight: dwell_penalty_weight
                }
            }
            _ => unreachable!()
        };

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

        let normalize_levels = matches.get_flag("normalize-levels");

        let rough_rescale_algo_str = matches.get_one::<String>("rough-rescale-algo").ok_or(
            CliError::ArgumentNone("rough-rescale-algo".to_string())
        )?.clone();

        let rough_rescale_algo = if rough_rescale_algo_str.as_str() == "none" {
            RoughRescaleAlgo::NoRoughRescaling
        } else {
            let rough_rescale_quants_min = *matches.get_one::<f32>("rough-rescale-quants-min").ok_or(
                CliError::ArgumentNone("rough-rescale-quants-min".to_string())
            )?;
    
            let rough_rescale_quants_max = *matches.get_one::<f32>("rough-rescale-quants-max").ok_or(
                CliError::ArgumentNone("rough-rescale-quants-max".to_string())
            )?;
    
            let rough_rescale_quants_steps = *matches.get_one::<usize>("rough-rescale-quants-steps").ok_or(
                CliError::ArgumentNone("rough-rescale-quants-steps".to_string())
            )?;

            // TODO: Check that rough_rescale_quants_min < rough_rescale_quants_max and rough_rescale_quants_steps > 2

            let quantiles = calc_quantiles(
                rough_rescale_quants_min, 
                rough_rescale_quants_max,
                rough_rescale_quants_steps
            );
    
            let rough_rescale_clip_bases = *matches.get_one::<usize>("rough-rescale-clip-bases").ok_or(
                CliError::ArgumentNone("rough-rescale-clip-bases".to_string())
            )?;
    
            let rough_rescale_use_all_signal = matches.get_flag("rough-rescale-use-all-signal");

            match rough_rescale_algo_str.as_str() {
                "least-squares" => {
                    RoughRescaleAlgo::LeastSquares { 
                        quantiles: quantiles, 
                        clip_bases: rough_rescale_clip_bases, 
                        use_base_center: rough_rescale_use_all_signal
                    }
                }
                "theil-sen" => {
                    RoughRescaleAlgo::TheilSen { 
                        quantiles: quantiles, 
                        clip_bases: rough_rescale_clip_bases, 
                        use_base_center: rough_rescale_use_all_signal
                    }
                }
                _ => unreachable!()         
            }

        };

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
            "least-squares" => RescaleAlgo::LeastSquares { 
                dwell_filter_lower_percentile: rescale_dwell_filter_lower_percentile, 
                dwell_filter_upper_percentile: rescale_dwell_filter_upper_percentile, 
                min_abs_level: rescale_min_abs_level, 
                n_bases_truncate: rescale_num_bases_truncate, 
                min_num_filtered_levels: rescale_min_num_filtered_levels 
            },
            "theil-sen" => {
                let rescale_max_points = *matches.get_one::<usize>("rescale-max-len").ok_or(
                    CliError::ArgumentNone("rescale-max-len".to_string())
                )?;
                RescaleAlgo::TheilSen { 
                    dwell_filter_lower_percentile: rescale_dwell_filter_lower_percentile, 
                    dwell_filter_upper_percentile: rescale_dwell_filter_upper_percentile,
                    min_abs_level: rescale_min_abs_level, 
                    n_bases_truncate: rescale_num_bases_truncate, 
                    min_num_filtered_levels: rescale_min_num_filtered_levels, 
                    max_points: rescale_max_points 
                }
            }
            _ => unreachable!()
        };

        let which_to_refine = match alignment_type {
            WhichToAlign::Both => WhichToRefine::Both,
            WhichToAlign::Query => WhichToRefine::Query,
            WhichToAlign::Reference => WhichToRefine::Reference,
        };

        let refine_settings = RefineSettings::custom(
            which_to_refine, 
            refine_algo, 
            refine_iters, 
            half_bandwidth, 
            min_band_size, 
            rescale_algo, 
            rough_rescale_algo, 
            normalize_levels
        );

        Ok(Config { 
            bam_input, 
            pod5_input, 
            kmer_table_input, 
            output_dir, 
            output_format,
            output_batch_size,
            force_overwrite,
            is_drna,
            alignment_type, 
            n_threads,
            queue_size,
            log_level, 
            log_path,
            refine_settings 
        })
    }


    pub fn bam_input(&self) -> &PathBuf {
        &self.bam_input
    }

    pub fn pod5_input(&self) -> &Vec<PathBuf> {
        &self.pod5_input
    }

    pub fn kmer_table_input(&self) -> &PathBuf {
        &self.kmer_table_input
    }

    pub fn output_dir(&self) -> &PathBuf {
        &self.output_dir
    }

    pub fn output_format(&self) -> &OutputFormat {
        &self.output_format
    }

    pub fn output_batch_size(&self) -> usize {
        self.output_batch_size
    }

    pub fn force_overwrite(&self) -> bool {
        self.force_overwrite
    }

    pub fn is_drna(&self) -> bool {
        self.is_drna
    }

    pub fn alignment_type(&self) -> &WhichToAlign {
        &self.alignment_type
    }

    pub fn n_threads(&self) -> usize {
        self.n_threads
    }

    pub fn queue_size(&self) -> usize {
        self.queue_size
    }

    pub fn log_level(&self) -> &LevelFilter {
        &self.log_level
    }

    pub fn log_path(&self) -> &PathBuf {
        &self.log_path
    }

    pub fn refine_settings(&self) -> &RefineSettings {
        &self.refine_settings
    }


}