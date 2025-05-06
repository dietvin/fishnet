use std::path::PathBuf;
use clap::{value_parser, Arg, ArgAction, ArgMatches, Command};

/// Sets up the following command line interface:
/// 
/// # Required arguments:
/// * `bam` - Path to a single BAM file (Can contain mapped or unmapped reads; mapped reads are needed for reference alignment)
/// * `pod5` - Path to one or multiple pod5 files or directories containing pod5 files
/// * `kmer-table` - Path to a kmer table file
/// * `output-dir` - Path to the output directory
/// 
/// # Optional arguments:
/// 
/// ## General options
/// * `alignment-type` - Whether to perform query (base-called sequence) to signal alignment, reference to
///                      signal alignment, or both (valid options: `query`, `reference`, `both`; default: `query`)
/// * `threads` - Number of threads to use during calculation (default: 8)
/// * `debug-level` - Which debug level to use (valid options: `off`, `error`, `warn`, `info`, `debug`, `trace`; default: `off`)
/// 
/// ## Refinement options
/// * `refine-iters` - Number of refinement iterations (valid options: uint >= 0; set to 0 to skip refinement; default: 2)
/// * `refine-algo` - Whether to use Viterbi or Dwell penalty algorithm for refinement 
///                   (valid options: `viterbi`, `dwell-penalty`; default: `dwell-penalty`)
/// 
/// * `dwell-penalty-target` - The target value for the dwell penalty refinement. Only processed if `refine-algo`
///                            is set to `dwell-penalty` (default: 4.0)
/// * `dwell-penalty-limit` - The limit value for the dwell penalty refinement. Only processed if `refine-algo`
///                            is set to `dwell-penalty` (default: 3.0)
/// * `dwell-penalty-weight` - The weight value for the dwell penalty refinement. Only processed if `refine-algo`
///                            is set to `dwell-penalty` (default: 0.5)
/// 
/// * `half-bandwidth` - Half of the bandwidth to use during the refinement process (default: 5)
/// * `min-band-size` - The minimum band size allowed when adjusting bands (default: 2)
/// * `normalize-levels` - If set, normalize the levels given in the kmer-table file (eq. to `do_fix_gauge`)
/// 
/// * `rough-rescale-algo` - Whether to perform rough rescaling and if so which algorithm to use
///                          (valid options: `none`, `least-squares`, `theil-sen`; default: `theil-sen`)
/// 
/// * `rough-rescale-quants-min` - Minimum quantile to use during rough rescaling (default: 0.05)
/// * `rough-rescale-quants-max` - Maximum quantiles to use during rough rescaling (default: 0.95)
/// * `rough-rescale-quants-step` - Which quantiles to use during rough rescaling (default: 0.05)
/// * `rough-rescale-clip-bases` - The number of bases to clip before rough rescaling (default: 10)
/// * `rough-rescale-use-all-signal` - If set uses uses the entire signal assigned to a given base for quantile
///                                    calculation. Otherwise only the measurement in the center of the base is
///                                    used. 
/// 
/// * `rescale-algo` - Wether to use theil sen or least squares rescaling after each refinement iteration
///                    (valid options: `least-squares`, `theil-sen`; default: `theil-sen`)
/// * `rescale-dwell-filter-lower-percentile` - Lower percentile for filtering bases based on dwell time before rescaling
///                                     (bases with dwell time < lower_percentile value get removed; default: 0.1)
/// * `rescale-dwell-filter-upper-percentile` - Upper percentile for filtering bases based on dwell time before rescaling
///                                     (bases with dwell time > upper_percentile value get removed; default: 0.9)
/// * `rescale-min-abs-level` - The minimum absolute expected signal intensity value needed for rescaling. 
///                             Expected intensities that deviate less than this value from the mean of the 
///                             expected intensity get removed. (default: 0.2)
/// * `rescale-num-bases-truncate` - The number of bases that will be ignored at the start and end before rescaling.
///                                  (default: 10) 
/// * `rescale-min-num-filtered-levels` - Threshold of the minimum number of valid bases that are needed to
///                                       perform the rescaling. (default: 10)
/// * `rescale-max-points` - Maximum number of data points (i.e. bases) used in Theil sen calculation. If 
///                          the number of bases exceeds this threshold a random subset is selected. Gets 
///                          ignored if set to 0. (Only processed if `rescale-algo` is `theil-sen`; default: 1000) 
pub fn parse_command_line() -> ArgMatches {
    let matches = Command::new("fishnet")
        .version("0.1.0")
        .author("Vincent Dietrich")
        .about("Fast signal-to-sequence alignment!")
        // Required arguments

        .arg(
            Arg::new("bam")
                .long("bam")
                .short('b')
                .required(true)
                .value_parser(value_parser!(PathBuf))
                .help_heading("Required input/output arguments")
                .help("Path to a single BAM file")
        )
        .arg(
            Arg::new("pod5")
                .long("pod5")
                .short('p')
                .required(true)
                .num_args(1..)
                .value_parser(value_parser!(PathBuf))
                .help_heading("Required input/output arguments")
                .help("Path to one or multiple pod5 files or directories")
        )
        .arg(
            Arg::new("kmer-table")
                .long("kmer-table")
                .short('k')
                .required(true)
                .value_parser(value_parser!(PathBuf))
                .help_heading("Required input/output arguments")
                .help("Path to a kmer table file")
        )
        .arg(
            Arg::new("output-dir")
                .long("output-dir")
                .short('o')
                .required(true)
                .value_parser(value_parser!(PathBuf))
                .help_heading("Required input/output arguments")
                .help("Path to the output directory")
        )

        // General options

        .arg(
            Arg::new("output-type")
                .long("output-type")
                .value_parser(["bam", "json", "hdf5"])
                .default_value("bam")
                .help_heading("General settings")
                .help("Output format")
        )
        .arg(
            Arg::new("rna")
            .long("rna")
            .action(ArgAction::SetTrue)
            .help_heading("General settings")
            .help("Whether direct RNA data is provided. Reverses the signal for alignment.")
        )        
        .arg(
            Arg::new("force-overwrite")
            .long("force-overwrite")
            .short('f')
            .action(ArgAction::SetTrue)
            .help_heading("General settings")
            .help("Whether existing output files should be overwritten.")
        )
        .arg(
            Arg::new("alignment-type")
                .long("alignment-type")
                .short('a')
                .value_parser(["query", "reference", "both"])
                .default_value("query")
                .help_heading("General settings")
                .help("Type of alignment to perform")
        )
        .arg(
            Arg::new("threads")
                .long("threads")
                .short('t')
                .value_parser(value_parser!(usize))
                .default_value("8")
                .help_heading("General settings")
                .help("Number of threads to use")
        )
        .arg(
            Arg::new("debug-level")
                .long("debug-level")
                .value_parser(["off", "error", "warn", "info", "debug", "trace"])
                .default_value("off")
                .help_heading("General settings")
                .help("Which debug level to use")
        )
        .arg(
            Arg::new("debug-path")
                .long("debug-path")
                .default_value("log.txt")
                .value_parser(value_parser!(PathBuf))
                .help_heading("General settings")
                .help("Path to the log file. Only regarded if debug-level is other than off.")
        )

        // Refinement general options

        .arg(
            Arg::new("refine-iters")
                .long("refine-iters")
                .value_parser(value_parser!(usize))
                .default_value("2")
                .help_heading("Refinement settings (dynamic programming refinement)")
                .help("Number of refinement iterations. If set to 0 the refinement is skipped.")
        )
        .arg(
            Arg::new("refine-algo")
                .long("refine-algo")
                .value_parser(["viterbi", "dwell-penalty"])
                .default_value("dwell-penalty")
                .help_heading("Refinement settings (dynamic programming refinement)")
                .long_help(
"Refinement algorithm. Viterbi and dwell penalty approaches are available. 
The dwell penalty approach also performs the viterbi approach internally,
while additionally penalizing adjustments in the mapping that result in short
dwell times at a given base."
                )
        )
        .arg(
            Arg::new("dwell-penalty-target")
                .long("dwell-penalty-target")
                .value_parser(value_parser!(f32))
                .default_value("4.0")
                .help_heading("Refinement settings (dynamic programming refinement)")
                .long_help(
"Dwell penalty settings. Only considered if refine-algo is dwell-penalty. 
Preferred dwell time."
                )
        )
        .arg(
            Arg::new("dwell-penalty-limit")
                .long("dwell-penalty-limit")
                .value_parser(value_parser!(f32))
                .default_value("3.0")
                .help_heading("Refinement settings (dynamic programming refinement)")
                .long_help(
"Dwell penalty settings. Only considered if refine-algo is dwell-penalty.
Maximum dwell time that is penalized."
                )
        )
        .arg(
            Arg::new("dwell-penalty-weight")
                .long("dwell-penalty-weight")
                .value_parser(value_parser!(f32))
                .default_value("0.5")
                .help_heading("Refinement settings (dynamic programming refinement)")
                .help(
"Dwell penalty settings. Only considered if refine-algo is dwell-penalty. 
Strength of the penalty applied to short dwell times."
                )
        )
        .arg(
            Arg::new("half-bandwidth")
                .long("half-bandwidth")
                .value_parser(value_parser!(usize))
                .default_value("5")
                .help_heading("Refinement settings (dynamic programming refinement)")
                .long_help(
"Half-width of the signal band, meaning that for each signal measurement 
bases half-bandwidth up- and downstream from the currently assigned one 
can be considered."
                )
        )
        .arg(
            Arg::new("min-band-size")
                .long("min-band-size")
                .value_parser(value_parser!(usize))
                .default_value("2")
                .help_heading("Refinement settings (dynamic programming refinement)")
                .help("The minimum band size when adjusting the sequence band.")
        )
        .arg(
            Arg::new("normalize-levels")
                .long("normalize-levels")
                .action(ArgAction::SetTrue)
                .help_heading("Refinement settings (dynamic programming refinement)")
                .long_help(
"Normalize the levels given in the kmer-table. Equivalent to `do_fix_gauge` setting
in Remora."
                )
        )

        // Refinement rescale options

        .arg(
            Arg::new("rescale-algo")
                .long("rescale-algo")
                .value_parser(["least-squares", "theil-sen"])
                .default_value("theil-sen")
                .help_heading("Refinement settings (Rescaling)")
                .help(
"Rescaling algorithm. Calculates shift and scale parameters to normalize
the signal measurement (norm_signal = (signal - shift) / scale). Other than
the rough rescaling, here the entire signal is used for the estimation.
Available algorithms are least-squares and theil-sen. Note that least-squares
is not available and tested in Remora."
                )
        )
        .arg(
            Arg::new("rescale-dwell-filter-lower-quant")
                .long("rescale-dwell-filter-lower-quant")
                .value_parser(value_parser!(f32))
                .default_value("0.1")
                .help_heading("Refinement settings (Rescaling)")
                .long_help(
"Lower dwell filter quantile. Signal data for bases with dwell times below this quantile 
value are filtered out before rescaling."
                )
        )
        .arg(
            Arg::new("rescale-dwell-filter-upper-quant")
                .long("rescale-dwell-filter-upper-quant")
                .value_parser(value_parser!(f32))
                .default_value("0.9")
                .help_heading("Refinement settings (Rescaling)")
                .long_help(
"Upper dwell filter quantile. Signal data for bases with dwell times above this quantile 
value are filtered out before rescaling."
                )
        )
        .arg(
            Arg::new("rescale-min-abs-level")
                .long("rescale-min-abs-level")
                .value_parser(value_parser!(f32))
                .default_value("0.2")
                .help_heading("Refinement settings (Rescaling)")
                .help(
"Minimum absolute (normalized) signal intensity. Signal data from bases, where the mean signal 
itensity deviates less than the given value from the expected intensity, is filtered out before 
rescaling."
                )
        )
        .arg(
            Arg::new("rescale-num-bases-truncate")
                .long("rescale-num-bases-truncate")
                .value_parser(value_parser!(usize))
                .default_value("10")
                .help_heading("Refinement settings (Rescaling)")
                .long_help(
"Number of bases to truncate. Signal data from the first and last given number of bases are
filtered out before rescaling."
                )
        )
        .arg(
            Arg::new("rescale-min-num-filtered-levels")
                .long("rescale-min-num-filtered-levels")
                .value_parser(value_parser!(usize))
                .default_value("10")
                .help_heading("Refinement settings (Rescaling)")
                .long_help(
"Minimum number of bases that must remain after filtering to be considered valid for rescaling."
                )
        )
        .arg(
            Arg::new("rescale-max-len")
                .long("rescale-max-len")
                .value_parser(value_parser!(usize))
                .default_value("1000")
                .help_heading("Refinement settings (Rescaling)")
                .long_help(
"Maximum number of data points (signal data for given bases) to use. If the sequence
contains more bases than the given number, the data is randomly subset to contain the
given number of data points. 
Only regarded when rescale-algo is theil-sen. If set to 0 no subsetting is performed."
                )
        )
        
        // Refinement rough rescale options

        .arg(
            Arg::new("rough-rescale-algo")
                .long("rough-rescale-algo")
                .value_parser(["none", "least-squares", "theil-sen"])
                .default_value("theil-sen")
                .help_heading("Refinement settings (rough rescaling)")
                .long_help(
"Rough rescaling algorithm. Calculates shift and scale parameters to normalize
the signal measurement (norm_signal = (signal - shift) / scale).
Rough rescaling, because only given percentile values are used instead of all
measurements. Available algorithms are least-squares and theil-sen. Theil-sen
is considered to be more robust against outliers."
                )
        )
        .arg(
            Arg::new("rough-rescale-quants-min")
                .long("rough-rescale-quants-min")
                .value_parser(value_parser!(f32))
                .default_value("0.05")
                .help_heading("Refinement settings (rough rescaling)")
                .help("Lowest percentile to calculate from the signal data during rough rescaling.")
        )
        .arg(
            Arg::new("rough-rescale-quants-max")
                .long("rough-rescale-quants-max")
                .value_parser(value_parser!(f32))
                .default_value("0.95")
                .help_heading("Refinement settings (rough rescaling)")
                .help("Highest percentile to calculate from the signal data during rough rescaling.")
        )
        .arg(
            Arg::new("rough-rescale-quants-steps")
                .long("rough-rescale-quants-steps")
                .value_parser(value_parser!(usize))
                .default_value("19")
                .help_heading("Refinement settings (rough rescaling)")
                .long_help(
"Number of percentile values to consider during rough rescaling. This includes the 
lowest and highest values."
                )
        )
        .arg(
            Arg::new("rough-rescale-clip-bases")
                .long("rough-rescale-clip-bases")
                .value_parser(value_parser!(usize))
                .default_value("10")
                .help_heading("Refinement settings (rough rescaling)")
                .help("Number of bases to ignore at the start and end during rough rescaling.")
        )
        .arg(
            Arg::new("rough-rescale-use-all-signal")
                .long("rough-rescale-use-all-signal")
                .action(ArgAction::SetTrue)
                .help_heading("Refinement settings (rough rescaling)")
                .long_help(
"Whether to use the entire signal for quantile calculation during rough rescaling. 
If set, the quantile values are calculated from all measurements. Otherwise the 
signal is subset to contain only a single measurement for each base, reducing the 
computational load. This measurement is taken from the center of the signal assigned 
to a given base."
                )
        )
        .get_matches();

    matches
}