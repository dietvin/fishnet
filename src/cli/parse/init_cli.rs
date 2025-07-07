/*!
 * Initializes the command line interface.
 */

use std::path::PathBuf;
use clap::{value_parser, Arg, ArgAction, ArgMatches, Command};
use console::style;

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
        .about(format!("{}", style("Fishnet - Fast signal-to-sequence alignment!").bold().green()))
        // Required arguments

        .arg(
            Arg::new("bam")
                .long("bam")
                .short('b')
                .required(true)
                .value_parser(value_parser!(PathBuf))
                .help_heading("Required input/output arguments")
                .help("Path to the BAM input")
                .long_help(
"Path to a BAM input file. Only a single file must be provided."
                )
        )
        .arg(
            Arg::new("pod5")
                .long("pod5")
                .short('p')
                .required(true)
                .num_args(1..)
                .value_parser(value_parser!(PathBuf))
                .help_heading("Required input/output arguments")
                .help("Path to the POD5 input")
                .long_help(
"Path to the POD5 input. Multiple paths can be provided space separated.
A path can point to a POD5 file or a directory. If a directory is given 
all POD5 files in the directory are processed. File and directory paths 
can be combined."
                )
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
            Arg::new("out")
                .long("out")
                .short('o')
                .required(true)
                .value_parser(value_parser!(PathBuf))
                .help_heading("Required input/output arguments")
                .help("Path to the output file")
                .long_help(
"Path to the output file. File extension determines the output format.
Must be '.parquet' for Parquet output or '.jsonl' for JSONL output."
                )
        )

        // General options
        
        .arg(
            Arg::new("rna")
                .long("rna")
                .short('r')
                .action(ArgAction::SetTrue)
                .help_heading("General settings")
                .help("Whether direct RNA data is provided")
                .long_help(
"Whether direct RNA data is provided. If set, reverses the raw sigal (3'->5') to be 
in 5'->3' orientation to match the base-called/mapped data."
                )
        )        
        .arg(
            Arg::new("force-overwrite")
                .long("force-overwrite")
                .short('f')
                .action(ArgAction::SetTrue)
                .help_heading("General settings")
                .help("Whether an existing output file should be overwritten.")
                .long_help(
"Whether existing output files should be overwritten. If the provided output path 
already exists and the flag is set the existing file is overwritten. Otherwise an 
error is raised."
                )
        )
        .arg(
            Arg::new("alignment-type")
                .long("alignment-type")
                .short('a')
                .value_parser(["query", "reference", "both"])
                .default_value("query")
                .help_heading("General settings")
                .help("Type of alignment to perform")
                .long_help(
"Determines the type of alignment that is performed. If set to 'query' the signal 
is aligned to the base-called query sequence. If set to 'reference' and a given read 
is mapped to a reference, the signal is aligned to that reference sequence."
                )
        )
        .arg(
            Arg::new("threads")
                .long("threads")
                .short('t')
                .value_parser(value_parser!(usize))
                .default_value("8")
                .help_heading("General settings")
                .help("Number of parallel threads")
                .long_help(
"Set the number of parallel threads used during processing. Set to 1 to 
disable multithreading. If set to 2 or 3, falls back to single-threaded
processing (due to 3 non-worker threads)."
                )
        )
        .arg(
            Arg::new("queue-size")
                .long("queue-size")
                .value_parser(value_parser!(usize))
                .default_value("1000")
                .help_heading("General settings")
                .help("Multi-threading queue size")
                .long_help(
"Sets the queue size for transfering data to and from worker threads. Only regarded
if number of threads is larger than 3. Decrease queue size for a reduced memory 
footprint."
                )
        )
        .arg(
            Arg::new("log-level")
                .long("log-level")
                .value_parser(["off", "error", "warn", "info", "debug", "trace"])
                .default_value("off")
                .help_heading("General settings")
                .help("Which log level to use")
                .long_help(
"Sets the logging level. The amount of intermediated information written to the log 
increases from 'error' to 'trace'. Set to error to get an overview of the reasons why
the alignment failed for (some) given reads. Logging is disabled by default."
                )
        )
        .arg(
            Arg::new("log-path")
                .long("log-path")
                .default_value("log.txt")
                .value_parser(value_parser!(PathBuf))
                .help_heading("General settings")
                .help("Path to the log file")
                .long_help(
"Path to the log file. Only regarded if debug-level is other than 'off'. If the log 
file exists already new logging output gets appended to the file."
                )
        )
        .arg(
            Arg::new("output-batch-size")
                .long("output-batch-size")
                .value_parser(value_parser!(usize))
                .default_value("4000")
                .help_heading("General settings")
                .help("Output batch size")
                .long_help(
"Output batch size. Determines the number of alignments that are collected 
before dumping these to file. Higher values reduce the I/O overhead, 
potentially increasing speed, while requiring more memory."
                )
        )


        // Refinement general options

        .arg(
            Arg::new("refine-iters")
                .long("refine-iters")
                .short('i')
                .value_parser(value_parser!(usize))
                .default_value("2")
                .help_heading("Refinement settings (dynamic programming refinement)")
                .help("Number of refinement iterations")
                .long_help(
"Sets the number of refinement iterations. In each iteration the alignment boundaries 
are shifed to minimize the difference between the expected and observed signal, followed 
by a calculation of rescaling parameters based on the shifed alignment. If set to 0 the 
refinement is skipped."
                )
        )
        .arg(
            Arg::new("refine-algo")
                .long("refine-algo")
                .value_parser(["viterbi", "dwell-penalty"])
                .default_value("dwell-penalty")
                .help_heading("Refinement settings (dynamic programming refinement)")
                .help("Which refinement algorithm to use")
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
                .help("Preferred dwell time")
                .long_help(
"Preferred dwell time used in dwell penalty refinement. Only considered if refine-algo 
is 'dwell-penalty'."
                )
        )
        .arg(
            Arg::new("dwell-penalty-limit")
                .long("dwell-penalty-limit")
                .value_parser(value_parser!(f32))
                .default_value("3.0")
                .help_heading("Refinement settings (dynamic programming refinement)")
                .help("Maximum penalized dwell time")
                .long_help(
"Maximum dwell time that is penalized in dwell penalty algorithm. Only considered if 
refine-algo is 'dwell-penalty'."
                )
        )
        .arg(
            Arg::new("dwell-penalty-weight")
                .long("dwell-penalty-weight")
                .value_parser(value_parser!(f32))
                .default_value("0.5")
                .help_heading("Refinement settings (dynamic programming refinement)")
                .help("Short dwell time penalty weight")
                .long_help(
"Strength of the penalty applied to short dwell times in dwell penalty algorithm. Only 
considered if refine-algo is 'dwell-penalty'."
                )
        )
        .arg(
            Arg::new("half-bandwidth")
                .long("half-bandwidth")
                .value_parser(value_parser!(usize))
                .default_value("5")
                .help_heading("Refinement settings (dynamic programming refinement)")
                .help("Dynamic programming half bandwidth")
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
                .help("Minimum sequence band size")
                .long_help(
"The minimum sequence band size that is forced when adjusting the sequence band. 
This means that a given signal measurement can potentially be assigned to min-band-size 
number of bases."
                    )
        )
        .arg(
            Arg::new("normalize-levels")
                .long("normalize-levels")
                .action(ArgAction::SetTrue)
                .help_heading("Refinement settings (dynamic programming refinement)")
                .help("Normalize levels in kmer table")
                .long_help(
"Whether to normalize the expected levels given in the kmer-table. This is equivalent to 
the `do_fix_gauge` setting in Remora."
                )
        )

        // Refinement rescale options

        .arg(
            Arg::new("rescale-algo")
                .long("rescale-algo")
                .value_parser(["least-squares", "theil-sen"])
                .default_value("theil-sen")
                .help_heading("Refinement settings (Rescaling)")
                .help("Which rescaling algorithm to use")
                .long_help(
"Which rescaling algorithm to use to calculate shift and scale parameters 
to normalize the signal measurement (norm_signal = (signal - shift) / scale). 
Other than the rough rescaling, here the entire signal is used for the estimation.
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
                .help("Lower dwell quantile filter threshold")
                .long_help(
"Lower filtering threshold for dwell times. Signal data for bases with dwell times below this quantile 
value are filtered out before rescaling."
                )
        )
        .arg(
            Arg::new("rescale-dwell-filter-upper-quant")
                .long("rescale-dwell-filter-upper-quant")
                .value_parser(value_parser!(f32))
                .default_value("0.9")
                .help_heading("Refinement settings (Rescaling)")
                .help("Upper dwell quantile filter threshold")
                .long_help(
"Upper filtering threshold for dwell times. Signal data for bases with dwell times above this quantile 
value are filtered out before rescaling."
                )
        )
        .arg(
            Arg::new("rescale-min-abs-level")
                .long("rescale-min-abs-level")
                .value_parser(value_parser!(f32))
                .default_value("0.2")
                .help_heading("Refinement settings (Rescaling)")
                .help("Minimum absolute (normalized) signal intensity filter threshold")
                .long_help(
"Minimum absolute (normalized) signal intensity filter threshold. Signal data from bases where the 
mean signal itensity deviates less than the given value from the expected intensity, is filtered out before 
rescaling."
                )
        )
        .arg(
            Arg::new("rescale-num-bases-truncate")
                .long("rescale-num-bases-truncate")
                .value_parser(value_parser!(usize))
                .default_value("10")
                .help_heading("Refinement settings (Rescaling)")
                .help("Number of bases to truncate for rescaling")
                .long_help(
"Number of bases to truncate before rescaling. Signal data from the first and last given number of bases are
filtered out before rescaling."
                )
        )
        .arg(
            Arg::new("rescale-min-num-filtered-levels")
                .long("rescale-min-num-filtered-levels")
                .value_parser(value_parser!(usize))
                .default_value("10")
                .help_heading("Refinement settings (Rescaling)")
                .help("Minimum number of bases needed for rescaling")
                .long_help(
"The minimum number of bases that must remain after filtering to be considered valid for rescaling."
                )
        )
        .arg(
            Arg::new("rescale-max-len")
                .long("rescale-max-len")
                .value_parser(value_parser!(usize))
                .default_value("1000")
                .help_heading("Refinement settings (Rescaling)")
                .help("Maximum number of bases to use for rescaling")
                .long_help(
"Maximum number of bases to use for rescaling. If the sequence contains more bases than 
the given number, the data is randomly subset to contain the given number of data points. 
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
                .help("Which rough rescaling algorithm to use")
                .long_help(
"Which rough rescaling algorithm to use. Calculates shift and scale parameters to normalize
the signal measurement (norm_signal = (signal - shift) / scale). Rough rescaling, because 
only given percentile values are used instead of all measurements. Available algorithms are 
least-squares and theil-sen. Theil-sen is considered to be more robust against outliers."
                )
        )
        .arg(
            Arg::new("rough-rescale-quants-min")
                .long("rough-rescale-quants-min")
                .value_parser(value_parser!(f32))
                .default_value("0.05")
                .help_heading("Refinement settings (rough rescaling)")
                .help("Lowest percentile used for rough rescaling")
                .long_help("Lowest percentile to calculate from the signal data during rough rescaling.")
        )
        .arg(
            Arg::new("rough-rescale-quants-max")
                .long("rough-rescale-quants-max")
                .value_parser(value_parser!(f32))
                .default_value("0.95")
                .help_heading("Refinement settings (rough rescaling)")
                .help("Highest percentile used for rough rescaling")
                .long_help("Highest percentile to calculate from the signal data during rough rescaling.")
        )
        .arg(
            Arg::new("rough-rescale-quants-steps")
                .long("rough-rescale-quants-steps")
                .value_parser(value_parser!(usize))
                .default_value("19")
                .help_heading("Refinement settings (rough rescaling)")
                .help("Number of steps between lowest and highest quantile for rough rescaling")
                .long_help(
"Number of percentile values to consider during rough rescaling. rough-rescale-quants-steps number of 
quantiles are considered, increasing evenly from the lowest to the highest quantile. The lowest and 
highest values are included. Default quantiles are 0.05, 0.10, 0.15, ..., 0.90, 0.95."
                )
        )
        .arg(
            Arg::new("rough-rescale-clip-bases")
                .long("rough-rescale-clip-bases")
                .value_parser(value_parser!(usize))
                .default_value("10")
                .help_heading("Refinement settings (rough rescaling)")
                .help("Number of bases to truncate for rough rescaling")
                .long_help(
"Number of bases to truncate before rough rescaling. Signal data from the first and last given number of bases are
filtered out before rough rescaling."
                )
        )
        .arg(
            Arg::new("rough-rescale-use-all-signal")
                .long("rough-rescale-use-all-signal")
                .action(ArgAction::SetTrue)
                .help_heading("Refinement settings (rough rescaling)")
                .help("Wether to use the entire signal for quantile calculation during rough rescaling")
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