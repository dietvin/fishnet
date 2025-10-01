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


/// Represents all columns that can occur in an alignment input
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum Column {
    ReadId,
    QueryAlignment,
    QuerySequence,
    RefAlignment,
    RefSequence,
    RefName,
    RefStart,
    Signal
}

/// Defines the source of filtering criteria for selecting which reads to process.
///
/// The filtering can be based on genomic coordinates (reference-based) or 
/// sequence motifs (sequence-based).
#[derive(Debug, Clone, PartialEq, Eq)]
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
#[derive(Debug, PartialEq, Eq)]
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
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Stats {
    Mean,
    Median,
    StDev,
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
            "std" => Stats::StDev,
            "dwell" => Stats::Dwell,
            "signal-to-noise" => Stats::SignalToNoise,
            _ => unreachable!("Invalid statistic name should be caught by CLI validation")
        }
    }

    pub(crate) fn to_str(&self) -> &str {
        match self {
            Stats::Mean => "mean",
            Stats::Median => "median",
            Stats::StDev => "std",
            Stats::Dwell => "dwell",
            Stats::SignalToNoise => "signal_to_noise"
        }
    }
}

/// Defines the strategy for reformatting the signal data.
#[derive(Debug, Clone)]
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

/// The available output data shapes
#[derive(Debug, PartialEq, Eq, Clone)]
pub enum OutputShape {
    Melted,
    Exploded,
    Nested
}

/// Complete configuration for nanopore signal reformatting operations.
///
/// This struct encapsulates all parameters required to perform signal reformatting,
/// from input file paths to processing strategies to output formatting. It serves
/// as the single source of truth for configuration throughout the application lifecycle.
///
/// # Configuration Categories
///
/// ## Input/Output
/// - File paths for alignment data, POD5 files, and output
/// - Output format selection (Parquet/TSV)
/// - File overwrite behavior
///
/// ## Data Processing  
/// - Signal source determination (embedded vs external files)
/// - Alignment type selection (query vs reference)
/// - Filtering strategy (genomic regions vs sequence motifs)
/// - Reformatting approach (statistics vs interpolation)
///
/// ## Performance Tuning
/// - Thread count for parallel processing
/// - Chunk sizes for memory management
/// - Queue sizes for data pipeline buffering
///
/// ## Runtime Behavior
/// - Logging configuration and output paths
/// - RNA-specific processing flags
/// - Column selection optimization
///
/// # Lifecycle
///
/// 1. **Construction**: Created via `from_argmatches()` with extensive validation
/// 2. **Validation**: All interdependencies checked before construction completes
/// 3. **Usage**: Immutable configuration passed to processing components
/// 4. **Access**: Getter methods provide safe access to all configuration values
///
/// # Thread Safety
///
/// This struct is `Send + Sync` safe for sharing across threads. All contained
/// data is either owned or safely shareable.
#[derive(Debug)]
pub struct ConfigReformat {
    alignment_input: PathBuf,
    output_file: PathBuf,

    /// Where to source the raw signal data from
    signal_source: SignalSource,
    /// Whether to use dRNA-specific processing (affects POD5 signal extraction)
    is_drna: bool,
    /// Whether to normalize the signal
    norm_signal: bool,
    /// Whether to normalize the dwells
    norm_dwells: bool,
    /// Which alignment type to process
    alignment_type: AlignmentType,
    /// How to reformat the signal data
    filter_source: FilterSource,
    // To determine which reformatting strategy gets perfromed
    reformat_strategy: ReformatStrategy,
    // The columns that are actually needed for processing given the user settings
    columns_of_interest: Vec<Column>,
    /// The number of rows to read in each chunk
    input_chunk_size: usize,
    /// Output file format (Parquet or TSV)
    output_format: OutputFormat,
    /// Output data shape (Melted / exploded / nested)
    output_shape: OutputShape,
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
        let norm_signal = !matches.get_flag("skip-signal-norm");
        let norm_dwells = !matches.get_flag("skip-dwell-norm");

        // === Determining which columns are needed for processing ===

        let columns_of_interest = Self::determine_columns_of_interest(
            &alignment_type, 
            &filter_source, 
            &signal_source
        );


        // === Performance, Input and Output Configuration ===

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

        let input_chunk_size = matches.get_one::<usize>("input-chunk-size").ok_or(
            CliError::ArgumentNone("input-chunk-size".to_string()) 
        )?.clone();
        if input_chunk_size == 0 {
            return Err(CliError::InvalidArgument("input-chunk-size".to_string(), 0.to_string()));
        }

        let output_shape_raw = matches.get_one::<String>("output-shape").ok_or(
            CliError::ArgumentNone("output-shape".to_string()) 
        )?.clone();
        let output_shape = match output_shape_raw.as_str() {
            "melted" => OutputShape::Melted,
            "exploded" => {
                if output_format != OutputFormat::Parquet {
                    return Err(CliError::InvalidOutputShape);
                }
                OutputShape::Exploded
            }
            "nested" => OutputShape::Nested,
            _ => unreachable!("Constrained by the CLI")
        };

        let output_batch_size = matches.get_one::<usize>("output-batch-size").ok_or(
            CliError::ArgumentNone("output-batch-size".to_string()) 
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
            norm_signal,
            norm_dwells,
            alignment_type,
            filter_source,
            reformat_strategy,
            columns_of_interest,
            input_chunk_size,
            output_format,
            output_shape,
            output_batch_size,
            force_overwrite,
            n_threads,
            queue_size,
            log_level,
            log_path
        })
    }

    /// Parses the Parquet schema of an alignment file to determine available data columns.
    ///
    /// This function inspects the metadata of a Parquet file to identify which types of
    /// alignment data, sequences, and signals are present. This information drives
    /// validation logic and processing decisions throughout the application.
    ///
    /// # Arguments
    ///
    /// * `alignment_input` - Path to the input Parquet alignment file
    ///
    /// # Returns
    ///
    /// * `Ok(AlignmentContent)` - Structure describing available data columns
    /// * `Err(CliError)` - File I/O errors, invalid Parquet format, or schema issues
    ///
    /// # Expected Column Names
    ///
    /// The function looks for these specific column names in the Parquet schema:
    /// - `"query_to_signal"`: Query-to-signal alignment data
    /// - `"ref_to_signal"`: Reference-to-signal alignment data  
    /// - `"query_sequence"`: sequence of the query reads
    /// - `"ref_sequence"`: sequence of the reference genome
    /// - `"signal"`: Raw nanopore signal data
    ///
    /// # Error Conditions
    ///
    /// - File cannot be opened (permissions, doesn't exist)
    /// - Invalid Parquet format or corrupted metadata
    /// - Schema inference failures
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

    /// Parses and validates the signal source configuration.
    ///
    /// Determines where raw signal data should be loaded from based on user input
    /// and the content of the alignment file. This function implements a priority
    /// system where embedded signal data is preferred over external POD5 files
    /// when both are available.
    ///
    /// # Arguments
    ///
    /// * `alignment_content` - Description of data columns present in the alignment file
    /// * `pod5_input` - Optional paths to POD5 files provided by the user
    ///
    /// # Returns
    ///
    /// * `Ok(SignalSource::SignalFromAlignment)` - Use signal data embedded in alignment file
    /// * `Ok(SignalSource::SignalFromFiles)` - Load signal data from the provided POD5 files
    /// * `Err(CliError)` - No signal source is available (neither embedded nor POD5 files)
    ///
    /// # Decision Matrix
    ///
    /// | POD5 Provided | Embedded Signal | Result | Notes |
    /// |---------------|-----------------|---------|-------|
    /// | Yes | Yes | SignalFromAlignment | Embedded takes priority |
    /// | Yes | No | SignalFromFiles | Use external POD5 files |
    /// | No | Yes | SignalFromAlignment | Use embedded data |
    /// | No | No | Error | No signal source available |
    fn parse_signal_source(
        alignment_content: &AlignmentContent,
        pod5_input: &Option<Vec<PathBuf>>
    ) -> Result<SignalSource, CliError> {
        match (pod5_input, alignment_content.has_signal) {
            // Option 1: Pod5 file(s) provided AND alignment file contains signal
            (Some(_), true) => {
                println!("Warning: Pod5 file(s) provided, and alignment input contains signal. Pod5 input will be ignored");
                Ok(SignalSource::SignalFromAlignment)
            }
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

    /// Parses and validates the filtering configuration from command line arguments.
    ///
    /// This function ensures that exactly one filtering method is specified by the user.
    /// Filtering determines which subset of columns will be processed based on either
    /// genomic coordinates or sequence motifs.
    ///
    /// # Arguments
    ///
    /// * `matches` - Parsed command line arguments from clap
    ///
    /// # Returns
    ///
    /// * `Ok(FilterSource)` - The configured filtering method
    /// * `Err(CliError)` - No filter specified or multiple conflicting filters
    ///
    /// # Filter Types
    ///
    /// ## Reference-Based Filtering (Genomic Coordinates)
    /// - `RefRegionFromInput`: Direct command line regions (e.g., "chr1:1000-2000")
    /// - `RefRegionFromBed`: Genomic regions loaded from BED format file
    /// - `PositionsOfInterest`: Specific genomic positions (e.g., "chr1:1500")
    ///
    /// ## Sequence-Based Filtering (Motif Matching)
    /// - `MotifFromInput`: DNA motifs provided directly (e.g., "ATCG", "GCTA")
    /// - `MotifFromFile`: Motifs loaded from a text file (one per line)
    ///
    /// # Validation
    ///
    /// The function enforces mutual exclusivity - exactly one filter type must be specified.
    /// File-based filters (`RefRegionFromBed`, `MotifFromFile`) undergo basic path validation
    /// but detailed file content validation occurs during processing.
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

    /// Resolves the target alignment type from user input and available data.
    ///
    /// This function implements an auto-detection mechanism when users don't explicitly
    /// specify an alignment type. It prevents ambiguous configurations and ensures
    /// that the chosen alignment type is actually present in the input data.
    ///
    /// # Arguments
    ///
    /// * `matches` - Parsed command line arguments containing user preferences
    /// * `alignment_content` - Description of alignment data present in the input file
    ///
    /// # Returns
    ///
    /// * `Ok(AlignmentType)` - The resolved alignment type to use for processing
    /// * `Err(CliError)` - Invalid configuration or ambiguous input
    ///
    /// # Resolution Logic
    ///
    /// ## Explicit User Choice
    /// If user specifies `--alignment-type`:
    /// - Validate that the requested type exists in the data
    /// - Return the requested type or error if unavailable
    ///
    /// ## Auto-Detection
    /// When `--alignment-type` is not specified:
    /// 1. **Single alignment type**: Return the available type
    /// 2. **Both types present**: Error - user must choose explicitly
    /// 3. **No alignments**: Error - no processable data
    ///
    /// # Error Cases
    ///
    /// - User requests query alignment but file only contains reference alignment
    /// - User requests reference alignment but file only contains query alignment  
    /// - File contains both alignment types but user didn't specify preference
    /// - File contains no alignment data
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


    /// Parses the signal reformatting strategy from command line arguments.
    ///
    /// This function determines how raw signal data will be transformed for output.
    /// The strategy affects the shape and content of the final output data.
    ///
    /// # Arguments
    ///
    /// * `matches` - Parsed command line arguments from clap
    ///
    /// # Returns
    ///
    /// * `Ok(ReformatStrategy)` - The configured reformatting approach
    /// * `Err(CliError)` - Missing required arguments or invalid configurations
    ///
    /// # Strategies
    ///
    /// ## Statistical Summary (`"stats"`)
    /// Computes statistical measures for each base position across signal chunks.
    /// Reduces variable-length signal data to fixed statistical summaries.
    /// 
    /// **Required Arguments:** `--stats` (one or more statistics)
    /// **Available Statistics:**
    /// - `mean`: Average signal value
    /// - `median`: Middle signal value  
    /// - `std`: Standard deviation
    /// - `dwell`: Time spent at each base
    /// - `signal-to-noise`: Signal quality metric
    ///
    /// **Output Shape:** One row per base, with columns for each requested statistic
    ///
    /// ## Interpolation (`"interpolate"`) 
    /// Resamples variable-length signal chunks to a fixed length using interpolation.
    /// Preserves signal shape while standardizing length.
    ///
    /// **Required Arguments:** `--target-size` (positive integer)
    /// **Output Shape:** One row per base, with `target-size` signal values per base
    ///
    /// # Validation
    ///
    /// - Strategy must be either "stats" or "interpolate"
    /// - Stats strategy requires at least one statistic
    /// - Interpolation strategy requires target-size > 0
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


    /// Determines which columns from the alignment file are required for processing.
    ///
    /// This function analyzes the user's configuration to identify the minimal set of
    /// columns that must be read from the Parquet file. This optimization reduces I/O
    /// overhead by avoiding unnecessary data loading.
    ///
    /// # Arguments
    ///
    /// * `alignment_type` - Whether to process query or reference alignments
    /// * `filter_source` - How reads will be filtered (affects required metadata columns)
    /// * `signal_source` - Where signal data comes from (affects whether to read signal column)
    ///
    /// # Returns
    ///
    /// A vector of `Column` enums representing the required columns. Always includes
    /// at least `ReadId` and one alignment column.
    ///
    /// # Column Selection Logic
    ///
    /// ## Always Required
    /// - `ReadId`: Essential for tracking individual reads
    /// - Alignment column: Either `QueryAlignment` or `RefAlignment` based on `alignment_type`
    ///
    /// ## Filter-Dependent Columns
    /// - **Reference-based filters** (genomic coordinates):
    ///   - `RefName`: Chromosome/contig identifier
    ///   - `RefStart`: Genomic start position
    /// - **Motif-based filters** (sequence patterns):
    ///   - `QuerySequence`: For query alignment processing
    ///   - `RefSequence`: For reference alignment processing
    ///
    /// ## Signal Data
    /// - `Signal`: Only included if `signal_source == SignalSource::SignalFromAlignment`
    fn determine_columns_of_interest(
        alignment_type: &AlignmentType,
        filter_source: &FilterSource,
        signal_source: &SignalSource,
    ) -> Vec<Column> {
        let mut columns = Vec::new();

        columns.push(Column::ReadId);

        match alignment_type {
            AlignmentType::Query => columns.push(Column::QueryAlignment),
            AlignmentType::Reference => columns.push(Column::RefAlignment)
        }

        match filter_source {
            FilterSource::RefRegionFromInput { .. }
            | FilterSource::RefRegionFromBed { .. }
            | FilterSource::PositionsOfInterest { .. } => {
                columns.push(Column::RefName);
                columns.push(Column::RefStart);
            }
            FilterSource::MotifFromInput { .. }
            | FilterSource::MotifFromFile { .. } => {
                match alignment_type {
                    AlignmentType::Query => columns.push(Column::QuerySequence),
                    AlignmentType::Reference => columns.push(Column::RefSequence),
                }
            }
        }

        if *signal_source == SignalSource::SignalFromAlignment {
            columns.push(Column::Signal);
        }

        columns
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
    pub fn is_drna(&self) -> bool {
        self.is_drna
    }

    /// Returns whether the signal should be z-standardized before
    /// processing.
    pub fn norm_signal(&self) -> bool {
        self.norm_signal
    }

    /// Returns whether the dwell values should be z-standardized 
    /// before processing.
    pub fn norm_dwells(&self) -> bool {
        self.norm_dwells
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

    /// Returns the columns that are needed for processing
    pub fn columns_of_interest(&self) -> &Vec<Column> {
        &self.columns_of_interest
    }

    /// Return the input chunk size
    pub fn input_chunk_size(&self) -> usize {
        self.input_chunk_size
    }

    /// Returns the output file format (Parquet or TSV).
    pub fn output_format(&self) -> &OutputFormat {
        &self.output_format
    }

    /// Returns the output shape (melted / exploded / nested).
    pub fn output_shape(&self) -> &OutputShape {
        &self.output_shape
    }

    /// Returns the number of records to write per output batch.
    pub fn output_batch_size(&self) -> usize {
        self.output_batch_size
    }

    /// Returns whether existing output files should be overwritten.
    pub fn force_overwrite(&self) -> bool {
        self.force_overwrite
    }

    /// Returns the number of processing threads to use.
    pub fn n_threads(&self) -> usize {
        self.n_threads
    }

    /// Returns the size of the processing queue.
    pub fn queue_size(&self) -> usize {
        self.queue_size
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


