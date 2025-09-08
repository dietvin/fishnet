//! # Configuration for Signal Reformatting
//!
//! This module provides configuration parsing and validation for the signal reformatting
//! functionality. It handles the complex interactions between different types of alignment
//! data, filtering options, and signal sources.
mod filter_compatibility_validation;

use std::{fs::File, path::PathBuf};
use arrow2::io::parquet::read::{infer_schema, read_metadata};
use clap::ArgMatches;
use log::LevelFilter;

use helper::{errors::CliError, file_handling::{check_and_get_pod5_input, check_input_file, check_output_file}, io::OutputFormat};
use crate::execute::config::filter_compatibility_validation::validate_filter_compatibility;

/// Defines the source of filtering criteria for selecting which reads to process.
///
/// The filtering can be based on genomic coordinates (reference-based) or 
/// sequence motifs (sequence-based).
#[derive(Debug, PartialEq, Eq)]
pub enum FilterSource {
    /// Filter by reference genomic regions provided directly as command line arguments
    RefRegionFromInput {
        regions: Vec<String>
    },
    /// Filter by reference genomic regions loaded from a BED file
    RefRegionFromBed {
        path: PathBuf
    },
    /// Filter by specific genomic positions of interest
    PositionsOfInterest {
        pois: Vec<String>
    },
    /// Filter by sequence motifs provided directly as command line arguments
    MotifFromInput {
        motifs: Vec<String>
    }, 
    /// Filter by sequence motifs loaded from a file
    MotifFromFile {
        path: PathBuf
    }
}

impl FilterSource {
    /// Returns true if this filter type operates on reference coordinates.
    ///
    /// Reference-based filters require reference alignment data to function,
    /// as they need to map genomic coordinates to signal positions.
    fn filters_for_ref(&self) -> bool {
        match self {
            FilterSource::RefRegionFromInput { .. } 
            | FilterSource::RefRegionFromBed { .. } 
            | FilterSource::PositionsOfInterest { .. } => true,
            _ => false
        }
    }
}

/// Defines where the raw signal data will be sourced from.
///
/// Signal data can either be embedded in the alignment file itself,
/// or loaded separately from POD5 files.
#[derive(Debug)]
pub enum SignalSource {
    /// Load signal data from separate POD5 files
    SignalFromFiles {
        paths: Vec<PathBuf>
    },
    /// Use signal data embedded in the alignment file
    SignalFromAlignment
}

/// Specifies which type of alignment data to process.
///
/// Query alignments map query sequences to signal, while reference
/// alignments map reference genome coordinates to signal.
#[derive(Debug, PartialEq, Eq, Clone)]
pub enum AlignmentType {
    /// Process query-to-signal alignments
    Query,
    /// Process reference-to-signal alignments
    Reference
}

/// Statistical measures that can be computed from signal data.
#[derive(Debug)]
pub enum Stats {
    Mean,
    Median,
    Std,
    Dwell,
    SignalToNoise
}

impl Stats {
    /// Parses a string representation into a Stats enum.
    ///
    /// # Panics
    ///
    /// Panics if the string doesn't match any known statistic name.
    /// This should be prevented by proper CLI argument validation.
    fn from_string(s: &String) -> Self {
        match s.as_str() {
            "mean" => Stats::Mean,
            "median" => Stats::Median,
            "std" => Stats::Std,
            "dwell" => Stats::Dwell,
            "signal-to-noise" => Stats::SignalToNoise,
            _ => unreachable!("Invalid statistic name should be caught by CLI validation")
        }
    }
}

/// Defines the strategy for reformatting the signal data.
#[derive(Debug)]
pub enum ReformatStrategy {
    /// Compute statistical summaries from signal 
    /// chunks for each base of each read
    ReadWiseStats {
        stats: Vec<Stats>
    },
    /// Interpolate signal chunks to a fixed length 
    /// for each base of each read
    Interpolation {
        target_len: usize
    }
}

/// Describes what types of data are present in an alignment file.
///
/// This is determined by inspecting the column names in the Parquet file schema.
pub struct AlignmentContent {
    /// Whether query-to-signal alignment data is present
    pub has_query_alignment: bool,
    /// Whether reference-to-signal alignment data is present
    pub has_ref_alignment: bool,
    /// Whether query sequence data is present
    pub has_query_sequence: bool,
    /// Whether reference sequence data is present
    pub has_ref_sequence: bool,
    /// Whether raw signal data is embedded in the file
    pub has_signal: bool
}

/// Complete configuration for the signal reformatting operation.
///
/// This struct contains all the parameters needed to perform signal reformatting,
/// including input/output file paths, processing options, and validation settings.
#[derive(Debug)]
pub struct ConfigReformat {
    alignment_input: PathBuf,
    output_file: PathBuf,

    /// Where to source the raw signal data from
    signal_source: SignalSource,
    /// Whether to use dRNA-specific processing (affects POD5 signal extraction)
    is_drna: bool,
    /// Which alignment type to process (None means auto-detect if possible)
    alignment_type: AlignmentType,
    /// How to reformat the signal data
    filter_source: FilterSource,
    // To determine which reformatting strategy gets perfromed
    reformat_strategy: ReformatStrategy,
    /// Output file format (Parquet or TSV)
    output_format: OutputFormat,
    /// Number of records to collect before writing an output batch
    output_batch_size: usize,
    /// Whether to overwrite existing output files
    force_overwrite: bool,
    /// Number of processing threads to use
    n_threads: usize,
    /// Size of the processing queue
    queue_size: usize,
    /// Logging verbosity level
    log_level: LevelFilter,
    /// Path for log file output
    log_path: PathBuf,
}

impl ConfigReformat {
    /// Creates a new configuration from parsed command line arguments.
    ///
    /// This method performs extensive validation to ensure that all the configuration
    /// options are compatible with each other and with the input data.
    ///
    /// # Arguments
    ///
    /// * `matches` - Parsed command line arguments from clap
    ///
    /// # Returns
    ///
    /// * `Ok(ConfigReformat)` if the configuration is valid
    /// * `Err(CliError)` if there are validation errors or missing required arguments
    ///
    /// # Validation Steps
    ///
    /// 1. **File validation**: Ensures input files exist and output paths are valid
    /// 2. **Schema parsing**: Examines the alignment file to determine available data
    /// 3. **Signal source determination**: Decides whether to use embedded or external signals
    /// 4. **Filter compatibility**: Validates that filtering options work with available data
    /// 5. **Argument validation**: Checks that numeric arguments are within valid ranges
    pub fn from_argmatches(matches: &ArgMatches) -> Result<Self, CliError> {

        // === File I/O Validation ===

        let alignment_input = matches.get_one::<PathBuf>("alignment").ok_or(
            CliError::ArgumentNone("alignment".to_string())
        )?.clone();
        check_input_file(&alignment_input, "parquet")?;

        // Parse the alignment file schema to understand what data is available
        let alignment_content = Self::parse_alignment_schema(&alignment_input)?;

        let force_overwrite = matches.get_flag("force-overwrite");
        let output_file_raw = matches.get_one::<PathBuf>("out").ok_or(
            CliError::ArgumentNone("out".to_string())
        )?.clone();
        let (output_file, output_format)  = check_output_file(
            &output_file_raw, 
            force_overwrite,
            vec![OutputFormat::Parquet, OutputFormat::Tsv]
        )?;

        // === Signal Source Configuration ===

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

        let signal_source = Self::parse_signal_source(&alignment_content, &pod5_input)?;

        // === Filter Configuration ===

        let filter_source = Self::parse_filter_source(matches)?;
        
        let alignment_type = Self::determine_alignment_type(
            matches,
            &alignment_content
        )?;

        // Validate that the filter and alignment configurations are compatible
        validate_filter_compatibility(
            &filter_source, 
            &alignment_content, 
            &alignment_type
        )?;

        // === Processing Strategy Configuration ===

        let reformat_strategy = Self::parse_reformat_strategy(matches)?;
        let is_drna = matches.get_flag("rna");

        // === Performance and Output Configuration ===

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

        let output_batch_size = matches.get_one::<usize>("output-batch-size").ok_or(
            CliError::ArgumentNone("alignment-type".to_string()) 
        )?.clone();
        if output_batch_size == 0 {
            return Err(CliError::InvalidArgument("output-batch-size".to_string(), 0.to_string()));
        }

        // === Logging Configuration ===

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
            output_format,
            output_batch_size,
            force_overwrite,
            n_threads,
            queue_size,
            log_level,
            log_path
        })
    }

    fn parse_alignment_schema(alignment_input: &PathBuf) -> Result<AlignmentContent, CliError> {
        let col_name_query_algn = "query_to_signal".to_string();
        let col_name_ref_algn = "ref_to_signal".to_string();
        let col_name_query_seq = "query_sequence".to_string();
        let col_name_ref_seq = "ref_sequence".to_string();
        let col_name_signal = "signal".to_string();

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

        Ok(AlignmentContent { 
            has_query_alignment: column_names.contains(&col_name_query_algn), 
            has_ref_alignment: column_names.contains(&col_name_ref_algn), 
            has_query_sequence: column_names.contains(&col_name_query_seq), 
            has_ref_sequence: column_names.contains(&col_name_ref_seq),
            has_signal: column_names.contains(&col_name_signal)
        })
    }

    fn parse_signal_source(
        alignment_content: &AlignmentContent,
        pod5_input: &Option<Vec<PathBuf>>
    ) -> Result<SignalSource, CliError> {
        match (pod5_input, alignment_content.has_signal) {
            // Option 1: Pod5 file(s) provided AND alignment file contains signal
            (Some(_), true) => Ok(SignalSource::SignalFromAlignment),
            // Option 2: Pod5 file(s) provided AND alignment file does not contain signal
            (Some(paths), false) => Ok(SignalSource::SignalFromFiles { paths: paths.clone() }),
            // Option 3: Pod5 file(s) not provided AND alignment file contains signal
            (None, true) => Ok(SignalSource::SignalFromAlignment),
            // Option 4: Pod5 file(s) not provided AND alignment file does not contain signal
            (None, false) => Err(CliError::InvalidArgument(
                "pod5".to_string(), 
                "Alignment file does not contain the signal, and no pod5 file(s) were provided. Please provide some via the '--pod5' flag".to_string()
            ))
        }
    }

    /// Parses the filter source configuration from command line arguments.
    ///
    /// Exactly one filter option must be specified by the user.
    fn parse_filter_source(matches: &ArgMatches) -> Result<FilterSource, CliError> {
        if let Some(ref_regions) = matches.get_many::<String>("ref-regions") {
            Ok(FilterSource::RefRegionFromInput { 
                regions: ref_regions.map(|el| el.clone()).collect()
            })
        } else if let Some(bed_path) = matches.get_one::<PathBuf>("bed-file") {
            Ok(FilterSource::RefRegionFromBed { 
                path: bed_path.clone() 
            })
        } else if let Some(pois) = matches.get_many::<String>("positions-of-interest") {
            Ok(FilterSource::PositionsOfInterest { 
                pois: pois.map(|el| el.clone()).collect()
            })
        } else if let Some(motifs) = matches.get_many::<String>("motifs") {
            Ok(FilterSource::MotifFromInput { 
                motifs: motifs.map(|el| el.clone()).collect()
            })
        } else if let Some(motif_file) = matches.get_one::<PathBuf>("motifs-file") {
            Ok(FilterSource::MotifFromFile { 
                path: motif_file.clone()
            })
        } else {
            Err(CliError::ArgumentNone("Data filter".to_string()))
        }
    }

    /// Determines which alignment type will actually be used for processing.
    ///
    /// When the user doesn't specify an alignment type explicitly, this function
    /// auto-detects based on what's available in the data. If both alignment types
    /// are present, the user must explicitly choose.
    ///
    /// # Logic
    ///
    /// - If user specified a type explicitly -> use that
    /// - If only query alignment present -> use Query
    /// - If only reference alignment present -> use Reference  
    /// - If both present and user didn't specify -> error (ambiguous)
    /// - If neither present -> error (no data)
    fn determine_alignment_type(
        matches: &ArgMatches,
        alignment_content: &AlignmentContent
    ) -> Result<AlignmentType, CliError> {
        match matches.get_one::<String>("alignment-type") {
            Some(s) => match s.as_str() {
                "query" => Ok(AlignmentType::Query),
                "reference" => Ok(AlignmentType::Reference),
                _ => unreachable!("Invalid alignment type should be caught by CLI validation")
            },
            None => {
                match (alignment_content.has_query_alignment, alignment_content.has_ref_alignment) {
                    (true, true) => Err(CliError::InvalidArgument(
                        "alignment-type".to_string(),
                        "Input contains both query and reference alignments. Please specify which to use with '--alignment-type'".to_string()
                    )),
                    (true, false) => Ok(AlignmentType::Query),
                    (false, true) => Ok(AlignmentType::Reference),
                    (false, false) => Err(CliError::InvalidArgument(
                        "alignment".to_string(), 
                        "No alignment data found in input file".to_string()
                    ))
                }
            }
        }
    }


    /// Parses the reformatting strategy from command line arguments.
    fn parse_reformat_strategy(matches: &ArgMatches) -> Result<ReformatStrategy, CliError> {
        let reformat_strategy_raw = matches.get_one::<String>("strategy").ok_or(
            CliError::ArgumentNone("strategy".to_string())
        )?;
        
        match reformat_strategy_raw.as_str() {
            "stats" => {
                let stats = matches.get_many::<String>("stats").ok_or(
                    CliError::ArgumentNone("stats".to_string())
                )?.map(|el| Stats::from_string(el)).collect::<Vec<Stats>>();
                Ok(ReformatStrategy::ReadWiseStats { stats })
            }
            "interpolate" => {
                let target_len = matches.get_one::<usize>("target-size").ok_or(
                    CliError::ArgumentNone("target-size".to_string())
                )?.clone();
                Ok(ReformatStrategy::Interpolation { target_len })
            }
            _ => unreachable!("Invalid strategy should be caught by CLI validation")
        }
    }

    // === Getter Methods ===

    /// Returns the path to the input alignment file.
    pub fn alignment_input(&self) -> &PathBuf {
        &self.alignment_input
    }

    /// Returns the path where output will be written.
    pub fn output_file(&self) -> &PathBuf {
        &self.output_file
    }

    /// Returns the configured signal source (embedded or external files).
    pub fn signal_source(&self) -> &SignalSource {
        &self.signal_source
    }

    /// Returns whether dRNA-specific processing should be used.
    /// 
    /// This affects how POD5 signal data is extracted and processed.
    pub fn is_drna(&self) -> &bool {
        &self.is_drna
    }

    /// Returns the alignment type to process (None means auto-detect).
    pub fn alignment_type(&self) -> &AlignmentType {
        &self.alignment_type
    }

    /// Returns the configured filtering strategy.
    pub fn filter_source(&self) -> &FilterSource {
        &self.filter_source
    }

    /// Returns the configured reformatting strategy.
    pub fn reformat_strategy(&self) -> &ReformatStrategy {
        &self.reformat_strategy
    }

    /// Returns the output file format (Parquet or TSV).
    pub fn output_format(&self) -> &OutputFormat {
        &self.output_format
    }

    /// Returns the number of records to write per output batch.
    pub fn output_batch_size(&self) -> &usize {
        &self.output_batch_size
    }

    /// Returns whether existing output files should be overwritten.
    pub fn force_overwrite(&self) -> &bool {
        &self.force_overwrite
    }

    /// Returns the number of processing threads to use.
    pub fn n_threads(&self) -> &usize {
        &self.n_threads
    }

    /// Returns the size of the processing queue.
    pub fn queue_size(&self) -> &usize {
        &self.queue_size
    }

    /// Returns the configured logging level.
    pub fn log_level(&self) -> &LevelFilter {
        &self.log_level
    }

    /// Returns the path where log files should be written.
    pub fn log_path(&self) -> &PathBuf {
        &self.log_path
    }
}


