pub mod helpers;
pub mod args_to_input;

use std::path::PathBuf;
use clap::{value_parser, Arg, ArgAction, ArgMatches, Command};

/// Sets up the following command line interface:
/// 
/// # Required arguments:
/// * `bam` - Path to a single BAM file (Can contain mapped or unmapped reads; mapped reads are needed for reference alignment)
/// * `pod5` - Path to one or multiple pod5 files or directories containing pod5 files
/// * `kmer-table` - Path to a kmer table file
/// * `output-dir` - Path to the 
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
        .about("Perform signal to sequence alignment from Nanopore sequencing data.")
        // Required arguments
        .arg(
            Arg::new("bam")
                .long("bam")
                .help("Path to a single BAM file")
                .required(true)
                .value_parser(value_parser!(PathBuf)),
        )
        .arg(
            Arg::new("pod5")
                .long("pod5")
                .help("Path to one or multiple pod5 files or directories")
                .required(true)
                .num_args(1..)
                .value_parser(value_parser!(PathBuf)),
        )
        .arg(
            Arg::new("kmer-table")
                .long("kmer-table")
                .help("Path to a kmer table file")
                .required(true)
                .value_parser(value_parser!(PathBuf)),
        )
        .arg(
            Arg::new("output-dir")
                .long("output-dir")
                .help("Path to an output directory")
                .required(true)
                .value_parser(value_parser!(PathBuf)),
        )

        // General options
        .arg(
            Arg::new("alignment-type")
                .long("alignment-type")
                .help("Type of alignment to perform")
                .value_parser(["query", "reference", "both"])
                .default_value("query"),
        )
        .arg(
            Arg::new("threads")
                .long("threads")
                .help("Number of threads to use")
                .value_parser(value_parser!(usize))
                .default_value("8"),
        )
        .arg(
            Arg::new("debug-level")
                .long("debug-level")
                .help("Which debug level to use")
                .value_parser(["off", "error", "warn", "info", "debug", "trace"])
                .default_value("off"),
        )
        // Refinement options
        .arg(
            Arg::new("refine-iters")
                .long("refine-iters")
                .help("Number of refinement iterations")
                .value_parser(value_parser!(u32))
                .default_value("2"),
        )
        .arg(
            Arg::new("refine-algo")
                .long("refine-algo")
                .help("Refinement algorithm")
                .value_parser(["viterbi", "dwell-penalty"])
                .default_value("dwell-penalty"),
        )
        .arg(
            Arg::new("dwell-penalty-target")
                .long("dwell-penalty-target")
                .value_parser(value_parser!(f32))
                .default_value("4.0"),
        )
        .arg(
            Arg::new("dwell-penalty-limit")
                .long("dwell-penalty-limit")
                .value_parser(value_parser!(f32))
                .default_value("3.0"),
        )
        .arg(
            Arg::new("dwell-penalty-weight")
                .long("dwell-penalty-weight")
                .value_parser(value_parser!(f32))
                .default_value("0.5"),
        )
        .arg(
            Arg::new("half-bandwidth")
                .long("half-bandwidth")
                .value_parser(value_parser!(usize))
                .default_value("5"),
        )
        .arg(
            Arg::new("min-band-size")
                .long("min-band-size")
                .value_parser(value_parser!(usize))
                .default_value("2"),
        )
        .arg(
            Arg::new("normalize-levels")
                .long("normalize-levels")
                .action(ArgAction::SetTrue)
                .help("Normalize the levels given in the kmer-table"),
        )
        .arg(
            Arg::new("rough-rescale-algo")
                .long("rough-rescale-algo")
                .value_parser(["none", "least-squares", "theil-sen"])
                .default_value("theil-sen"),
        )
        .arg(
            Arg::new("rough-rescale-quants-min")
                .long("rough-rescale-quants-min")
                .value_parser(value_parser!(f32))
                .default_value("0.05"),
        )
        .arg(
            Arg::new("rough-rescale-quants-max")
                .long("rough-rescale-quants-max")
                .value_parser(value_parser!(f32))
                .default_value("0.95"),
        )
        .arg(
            Arg::new("rough-rescale-quants-steps")
                .long("rough-rescale-quants-steps")
                .value_parser(value_parser!(f32))
                .default_value("0.05"),
        )
        .arg(
            Arg::new("rough-rescale-clip-bases")
                .long("rough-rescale-clip-bases")
                .value_parser(value_parser!(usize))
                .default_value("10"),
        )
        .arg(
            Arg::new("rough-rescale-use-all-signal")
                .long("rough-rescale-use-all-signal")
                .action(ArgAction::SetTrue)
                .help("Use the entire signal assigned to a base for rough rescaling"),
        )
        .arg(
            Arg::new("rescale-algo")
                .long("rescale-algo")
                .value_parser(["least-squares", "theil-sen"])
                .default_value("theil-sen"),
        )
        .arg(
            Arg::new("rescale-dwell-filter-lower-percentile")
                .long("rescale-dwell-filter-lower-percentile")
                .value_parser(value_parser!(f32))
                .default_value("0.1"),
        )
        .arg(
            Arg::new("rescale-dwell-filter-upper-percentile")
                .long("rescale-dwell-filter-upper-percentile")
                .value_parser(value_parser!(f32))
                .default_value("0.9"),
        )
        .arg(
            Arg::new("rescale-min-abs-level")
                .long("rescale-min-abs-level")
                .value_parser(value_parser!(f32))
                .default_value("0.2"),
        )
        .arg(
            Arg::new("rescale-num-bases-truncate")
                .long("rescale-num-bases-truncate")
                .value_parser(value_parser!(usize))
                .default_value("10"),
        )
        .arg(
            Arg::new("rescale-min-num-filtered-levels")
                .long("rescale-min-num-filtered-levels")
                .value_parser(value_parser!(usize))
                .default_value("10"),
        )
        .arg(
            Arg::new("rescale-max-points")
                .long("rescale-max-points")
                .value_parser(value_parser!(usize))
                .default_value("1000"),
        )
        .get_matches();
    matches
}