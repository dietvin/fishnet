use std::{fs::File, path::PathBuf};

use arrow2::io::parquet::read::{infer_schema, read_metadata};
use clap::ArgMatches;
use helper::{errors::CliError, file_handling::{check_and_get_pod5_input, check_input_file, check_output_file}, io::OutputFormat};
use log::LevelFilter;


pub enum FilterSource {
    RefRegionFromInput {
        regions: Vec<String>
    },
    RefRegionFromBed {
        path: PathBuf
    },
    PositionsOfInterest {
        pois: Vec<String>
    },
    MotifFromInput {
        motifs: Vec<String>
    }, 
    MotifFromFile {
        path: PathBuf
    }
}

pub enum SignalSource {
    SignalFromFiles {
        paths: Vec<PathBuf>
    },
    SignalFromAlignment
}

#[derive(Debug, PartialEq, Eq)]
pub enum AlignmentType {
    Query,
    Reference
}

pub enum Stats {
    Mean,
    Median,
    Std,
    Dwell,
    SignalToNoise
}

impl Stats {
    fn from_string(s: &String) -> Self {
        match s.as_str() {
            "mean" => Stats::Mean,
            "median" => Stats::Median,
            "std" => Stats::Std,
            "dwell" => Stats::Dwell,
            "signal-to-noise" => Stats::SignalToNoise,
            _ => unreachable!()
        }
    }
}

pub enum ReformatStrategy {
    ReadWiseStats {
        stats: Vec<Stats>
    },
    Interpolation {
        target_len: usize
    }
}


pub struct ConfigReformat {
    alignment_input: PathBuf,
    output_file: PathBuf,
    // To determine where the signal information gets parsed from.
    // If the user provides an alignment file with the signal stored 
    // within it, there is no need to provide pod5 file(s).
    signal_source: SignalSource,
    // To determine if dRNA is used in case pod5 data is provided.
    is_drna: bool,
    // to determine which alignment type should be reformatted
    // if only one type is in the alignment file, this is determined automatically
    // if both are in the alignment file the user needs to set it manually
    alignment_type: Option<AlignmentType>,
    // To determine based on which options the data gets filtered
    filter_source: FilterSource,
    // To determine which reformatting strategy gets perfromed
    reformat_strategy: ReformatStrategy,

    output_batch_size: usize,
    force_overwrite: bool,
    n_threads: usize,
    queue_size: usize,
    log_level: LevelFilter,
    log_path: PathBuf,
}

impl ConfigReformat {
    pub fn from_argmatches(matches: &ArgMatches) -> Result<Self, CliError> {

        // Required IO

        let alignment_input = matches.get_one::<PathBuf>("alignment").ok_or(
            CliError::ArgumentNone("alignment".to_string())
        )?.clone();
        check_input_file(&alignment_input, "parquet")?;

        let force_overwrite = matches.get_flag("force-overwrite");
        let output_file_raw = matches.get_one::<PathBuf>("out").ok_or(
            CliError::ArgumentNone("out".to_string())
        )?.clone();
        let (output_file, output_format)  = check_output_file(
            &output_file_raw, 
            force_overwrite,
            vec![OutputFormat::Parquet, OutputFormat::Tsv]
        )?;

        // Pod5 input

        let pod5_input = match matches.get_many::<PathBuf>("pod5") {
            Some(p5_in) => {
                let p5_in_raw = p5_in
                    .map(|buf| buf.clone())
                    .collect::<Vec<PathBuf>>();
                Some(
                    check_and_get_pod5_input(p5_in_raw)?
                )
            }
            None => None
        };

        // Filter arguments (only one option can be chosen by the user)

        let filter_source = if let Some(ref_regions) = matches.get_many::<String>("ref-regions") {
            FilterSource::RefRegionFromInput { 
                regions: ref_regions.map(|el| el.clone()).collect()
            } 
        } else if let Some(bed_path) = matches.get_one::<PathBuf>("bed-file") {
            FilterSource::RefRegionFromBed { 
                path: bed_path.clone() 
            }
        } else if let Some(pois) = matches.get_many::<String>("positions-of-interest") {
            FilterSource::PositionsOfInterest { 
                pois: pois.map(|el| el.clone()).collect()
            }
        } else if let Some(motifs) = matches.get_many::<String>("motifs") {
            FilterSource::MotifFromInput { 
                motifs: motifs.map(|el| el.clone()).collect()
            }
        } else if let Some(motif_file) = matches.get_one::<PathBuf>("motifs-file") {
            FilterSource::MotifFromFile { 
                path: motif_file.clone()
            }
        } else {
            return Err(CliError::ArgumentNone("Data filter".to_string()));
        };

        // Processing strategy

        let reformat_strategy_raw = matches.get_one::<String>("strategy").ok_or(
            CliError::ArgumentNone("strategy".to_string())
        )?;
        let reformat_strategy = match reformat_strategy_raw.as_str() {
            "stats" => {
                let stats = matches.get_many::<String>("stats").ok_or(
                    CliError::ArgumentNone("stats".to_string())
                )?.map(|el| Stats::from_string(el)).collect::<Vec<Stats>>();
                ReformatStrategy::ReadWiseStats { stats }
            }
            "interpolate" => {
                let target_len = matches.get_one::<usize>("target-size").ok_or(
                    CliError::ArgumentNone("target-size".to_string())
                )?.clone();
                ReformatStrategy::Interpolation { target_len }
            }
            _ => unreachable!()
        };

        let is_drna = matches.get_flag("rna");

        let alignment_type = match matches.get_one::<String>("alignment-type") {
            Some(s) => match s.as_str() {
                "query" => Some(AlignmentType::Query),
                "reference" => Some(AlignmentType::Reference),
                _ => unreachable!()
            }
            None => None
        };

        let signal_source = Self::check_alignment_import(&alignment_input, &filter_source, &pod5_input, &alignment_type)?;

        // Threading options 

        let n_threads = matches.get_one::<usize>("threads").ok_or(
            CliError::ArgumentNone("threads".to_string()) 
        )?.clone();

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

        // Other output options

        let output_batch_size = matches.get_one::<usize>("output-batch-size").ok_or(
            CliError::ArgumentNone("alignment-type".to_string()) 
        )?.clone();
        if output_batch_size == 0 {
            CliError::InvalidArgument("output-batch-size".to_string(), 0.to_string());
        }

        // Logging options

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



        Ok( Self { 
            alignment_input,
            output_file,
            signal_source,
            is_drna,
            alignment_type,
            filter_source,
            reformat_strategy,
            output_batch_size,
            force_overwrite,
            n_threads,
            queue_size,
            log_level,
            log_path
        })
    }

    const COL_NAME_QUERY_ALGN: &'static str = "query_to_signal";
    const COL_NAME_REF_ALGN: &'static str = "ref_to_signal";
    const COL_NAME_QUERY_SEQ: &'static str = "query_sequence";
    const COL_NAME_REF_SEQ: &'static str = "ref_sequence";
    const COL_NAME_SIG: &'static str = "signal";
    
    /// Check if the parquet file has the following
    fn check_alignment_import(
        alignment_input: &PathBuf, 
        filter_source: &FilterSource,
        pod5_input: &Option<Vec<PathBuf>>,
        alignment_type: &Option<AlignmentType>
    ) -> Result<SignalSource, CliError> {
        // Extract the column names from the parquet file
        let mut reader = File::open(alignment_input)?;
        let metadata = match read_metadata(&mut reader) {
            Ok(m) => m,
            Err(e) => return Err(CliError::InvalidArgument(
                "alignment".to_string(), 
                format!("Could not read metadata from file '{:?}' ({})", alignment_input, e)
            ))
        };
    
        let schema = match infer_schema(&metadata) {
            Ok(s) => s,
            Err(e) => return Err(CliError::InvalidArgument(
                "alignment".to_string(), 
                format!("Could not read metadata from file '{:?}' ({})", alignment_input, e)
            ))
        };
    
        let column_names = schema.fields.iter()
            .map(|f| f.name.clone())
            .collect::<Vec<String>>();
    
        let contains_both_types = column_names.contains(&Self::COL_NAME_QUERY_ALGN.to_string()) 
            && column_names.contains(&Self::COL_NAME_REF_ALGN.to_string());
        let contains_only_ref = !contains_both_types && 
            column_names.contains(&Self::COL_NAME_REF_ALGN.to_string());
        let contains_only_query = !contains_both_types && !contains_only_ref;
        let manually_set_query = match alignment_type {
            Some(AlignmentType::Query) => true,
            Some(AlignmentType::Reference) => false,
            None => false
        };
        let contains_ref_sequence = column_names.contains(&Self::COL_NAME_REF_SEQ.to_string());
        let contains_query_sequence = column_names.contains(&Self::COL_NAME_QUERY_SEQ.to_string());
    
        // Alignment file contains both alignment types -> alignment type 
        // If the alignment type is set with alignment data that contains 
        // only one, it gets ignored
        if contains_both_types && alignment_type.is_none() {
                return Err(CliError::InvalidArgument(
                    "alignment-type".to_string(), 
                    "'--alignment-type' not set. Must be set with alignment data that contains query and reference alignments".to_string()
                ));
            }
    
        // Filtering by ref positions -> Must be ref alignment
        match filter_source {
            FilterSource::RefRegionFromInput { .. } | 
            FilterSource::RefRegionFromBed { .. } | 
            FilterSource::PositionsOfInterest { .. } => {
                // Ref region filtering only works with reference alignments
                if contains_only_query {
                    return Err(CliError::InvalidArgument(
                        "filter arguments".to_string(), 
                        "Filtering is set for reference coordinates, but the data only contains query to signal alignments".to_string()
                    ));
                }
    
                if contains_both_types && manually_set_query {
                    return Err(CliError::InvalidArgument(
                        "filter arguments".to_string(), 
                        "Filtering is set for reference coordinates, but the alignment to parse is set to 'query' (see --alignment-type)".to_string()
                    ));
                }
            },      
            FilterSource::MotifFromInput { .. } | FilterSource::MotifFromFile { .. } => {
                // Motif filtering only works when the ref/query sequence is included 
                if (contains_only_ref || (contains_both_types && !manually_set_query)) && !contains_ref_sequence {
                    return Err(CliError::InvalidArgument(
                        "filter arguments".to_string(), 
                        "Filtering is set for motif(s) and parsing is targeted for the reference, but the reference sequence is not present".to_string()
                    ));
                }
    
                if ((contains_only_query || (contains_both_types && manually_set_query))) && !contains_query_sequence {
                    return Err(CliError::InvalidArgument(
                        "filter arguments".to_string(), 
                        "Filtering is set for motif(s) and parsing is targeted for the query, but the query sequence is not present".to_string()
                    ));
                }
            }
        }
    
        // No pod5 provided -> Must be contained in alignment file
        if pod5_input.is_none() && !column_names.contains(&Self::COL_NAME_SIG.to_string()) {
            return Err(CliError::InvalidArgument(
                "pod5".to_string(), 
                "Alignment input does not contain the signal and no pod5 file(s) are provided. Please provide a file(s) via the '--pod5' flag".to_string()
            ));
        }
    
        let signal_source = match pod5_input {
            None => SignalSource::SignalFromAlignment,
            Some(paths) => SignalSource::SignalFromFiles { paths: paths.clone() }
        };
    
        Ok(signal_source)
    }
    
    pub fn alignment_input(&self) -> &PathBuf {
        &self.alignment_input
    }

    pub fn output_file(&self) -> &PathBuf {
        &self.output_file
    }

    pub fn signal_source(&self) -> &SignalSource {
        &self.signal_source
    }

    pub fn is_drna(&self) -> &bool {
        &self.is_drna
    }

    pub fn alignment_type(&self) -> &Option<AlignmentType> {
        &self.alignment_type
    }

    pub fn filter_source(&self) -> &FilterSource {
        &self.filter_source
    }

    pub fn reformat_strategy(&self) -> &ReformatStrategy {
        &self.reformat_strategy
    }

    pub fn output_batch_size(&self) -> &usize {
        &self.output_batch_size
    }

    pub fn force_overwrite(&self) -> &bool {
        &self.force_overwrite
    }

    pub fn n_threads(&self) -> &usize {
        &self.n_threads
    }

    pub fn queue_size(&self) -> &usize {
        &self.queue_size
    }

    pub fn log_level(&self) -> &LevelFilter {
        &self.log_level
    }

    pub fn log_path(&self) -> &PathBuf {
        &self.log_path
    }
}


