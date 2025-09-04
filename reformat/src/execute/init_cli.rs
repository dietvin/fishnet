use std::path::PathBuf;
use clap::{value_parser, Arg, ArgAction, ArgGroup, Command};

pub fn init_reformat() -> Command {
    let command = Command::new("reformat")
        .about("Process alignments to prepare for analyses")

        // Required IO

        .arg(
            Arg::new("alignment")
                .long("alignment")
                .short('a')
                .required(true)
                .value_parser(value_parser!(PathBuf))
                .help_heading("Required input/output arguments")
                .help("Path to the alignment file")
                .long_help(
"Path to the .parquet/.jsonl file produced by 'fishnet align'"
                )
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
Must be '.parquet' for Parquet output or '.tsv' for text output."
                )
        )

        // Pod5 input

        .arg(
            Arg::new("pod5")
                .long("pod5")
                .short('p')
                .value_parser(value_parser!(PathBuf))
                .num_args(1..)
                .help_heading("Pod5 input")
                .help("Path to the POD5 input")
                .long_help(
"Path to the POD5 input. Multiple paths can be provided space separated.
A path can point to a POD5 file or a directory. If a directory is given 
all POD5 files in the directory are processed. File and directory paths 
can be combined. Required if the alignment file does not contain the raw
signal."
                )
        )

        .arg(
            Arg::new("rna")
                .long("rna")
                .action(ArgAction::SetTrue)
                .help_heading("Pod5 input")
                .help("Set if direct RNA pod5 file(s) are provided")
                .long_help(
"Set if direct RNA pod5 file(s) are provided. Determines whether the signal gets
reversed (stored in 3'-5' direction in dRNA data). Not needed if the signal is 
stored in the alignment input."
                )
        )

        // Filter arguments (only one option can be chosen by the user)

        .arg(
            Arg::new("ref-regions")
                .long("ref-regions")
                .short('r')
                .num_args(1..)
                .value_parser(value_parser!(String))
                .help_heading("Data filter (one is required)")
                .help("Filter input data for one or more reference region(s)")
                .long_help(
"Filter input data for one or more reference region(s). Each must be in the format
'<REF-NAME>:<REF-START>-<REF-END>'. Coordinates frollow samtools-style convention:
REF-START and REF-END are 1-based and inclusive. Multiple reference regions 
can be provided space separated. Only valid if the input data contains reference 
alignments. (For an explanation of the coordinate system check the end of --help)"
                )
        )
        .arg(
            Arg::new("bed-file")
                .long("bed-file")
                .short('R')
                .value_parser(value_parser!(PathBuf))
                .help_heading("Data filter (one is required)")
                .help("Filter input data for reference regions from bed file")
                .long_help(
"Filter input data for reference regions from bed file. The bed file must contain the
reference name, reference start and reference end tab-separated in order. Only valid 
if the input data contains reference alignments. (For an explanation of the coordinate 
system check the end of --help)"
                )
        )
        .arg(
            Arg::new("positions-of-interest")
                .long("positions-of-interest")
                .short('P')
                .num_args(1..)
                .value_parser(value_parser!(String))
                .help_heading("Data filter (one is required)")
                .help("Filter input data for one or more positions of interest")
                .long_help(
"Filter input data for one or more positions of interest using 1-based coordinates. 
Each must be in the format '<REF-NAME>:<REF-SITE>-<HALF-SIZE>', where <HALF-SIZE> 
determines the number of bases up- and downstream from the site that are of interest. 
Multiple reference regions can be provided space separated. Only valid if the input 
data contains reference alignments. (For an explanation of the coordinate system 
check the end of --help)"
                )
        )
        .arg(
            Arg::new("motifs")
                .long("motifs")
                .short('m')
                .num_args(1..)
                .value_parser(value_parser!(String))
                .help_heading("Data filter (one is required)")
                .help("Filter input data for one or more motif sequences")
                .long_help(
"Filter input data for one or more motif sequences. Each must be a string containing
only 'A', 'C', 'G' and 'T'/'U'. Multiple motifs can be provided space separated. 
Valid for both reference and query sequences."
                )

        )
        .arg(
            Arg::new("motifs-file")
                .long("motifs-file")
                .short('M')
                .value_parser(value_parser!(PathBuf))
                .help_heading("Data filter (one is required)")
                .help("Filter input data for motifs from a text file")
                .long_help(
"Filter input data for motifs from a text file. The file must contain one motif per row.
Each motif must be a string containing only 'A', 'C', 'G' and 'T'/'U'. Valid for both 
reference and query sequences."
                )
        )
        .group(
            ArgGroup::new("filter")
                .args(["ref-regions", "bed-file", "positions-of-interest", "motifs", "motifs-file"])
                .required(true)
        )

        // Processing strategy

        .arg(
            Arg::new("strategy")
                .long("strategy")
                .short('s')
                .value_parser(["stats", "interpolate"])
                .default_value("position-wise-stats")
                .help_heading("Processing strategy")
                .help("Set the processing strategy.")
                .long_help(
"Set the processing strategy. There are two options:
1. 'stats': Calculate one or more statistic from the signal chunk for each base. The statistics 
can be set via the --stats flag.
2. 'interpolate': Interpolate the signal chunk into a uniform shape for each base. The number of 
measurements can be set via the --target-size flag."
                )
        )

        .arg(
            Arg::new("alignment-type")
                .long("alignment-type")
                .value_parser(["reference", "query"])
                .help_heading("Processing strategy")
                .help("Sets the alignment that gets parsed.")
                .long_help(
"Sets the alignment that gets parsed. Only relevant if the alignment input contains both
reference and query alignments. Note that 'query' is incompatible with the --ref-regions, 
--bed-file and --positions-of-interest flags."
                )                
        )

        // Processing strategy options

        .arg(
            Arg::new("stats")
                .long("stats")
                .num_args(1..)
                .value_parser(["mean", "median", "std", "dwell", "signal-to-noise"])
                .default_values(["mean", "std", "dwell"])
                .help_heading("Strategy 1: Read wise statisticis settings")
                .help("Determines which statistic(s) are calculated if position-wise-stats was selected")
                .long_help(
"Determines which statistic(s) are calculated if position-wise-stats was selected. These can 
be any combination of the mean/median current intensity ('mean'/'median'), the standard 
deviation of the current intensity ('std'), the dwell time (dwell), or the signal-to-noise 
ratio (mean/std; 'signal-to-noise').

Only regarded if --strategy is set to 'position-wise-stats'"
                )
        )
        .arg(
            Arg::new("target-size")
                .long("target-size")
                .value_parser(value_parser!(usize))
                .default_value("30")
                .help_heading("Strategy 2: Interpolation settings")
                .help("Sets the target size of the signal chunks for each base")
                .long_help(
"Second processing option.
Sets the target size of the signal chunks for each base. Each chunk gets interpolated (if 
the number of measurements in a given chunk is smaller than the target size) or subset (if 
the number of measurements is larger than the target size) to the set target-size

Only regarded if --strategy is set to 'interpolate'"
                )
        )

        // Threading options 

        .arg(
            Arg::new("threads")
                .long("threads")
                .short('t')
                .value_parser(value_parser!(usize))
                .default_value("8")
                .help_heading("Threading settings")
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
                .help_heading("Threading settings")
                .help("Multi-threading queue size")
                .long_help(
"Sets the queue size for transfering data to and from worker threads. Only regarded
if number of threads is larger than 3. Decrease queue size for a reduced memory 
footprint."
                )
        )

        // Output options

        .arg(
            Arg::new("force-overwrite")
                .long("force-overwrite")
                .short('f')
                .action(ArgAction::SetTrue)
                .help_heading("Output settings")
                .help("Whether an existing output file should be overwritten.")
                .long_help(
"Whether existing output files should be overwritten. If the provided output path 
already exists and the flag is set the existing file is overwritten. Otherwise an 
error is raised."
                )
        )
        .arg(
            Arg::new("output-batch-size")
                .long("output-batch-size")
                .value_parser(value_parser!(usize))
                .default_value("4000")
                .help_heading("Output settings")
                .help("Output batch size")
                .long_help(
"Output batch size. Determines the number of alignments that are collected 
before dumping these to file. Higher values reduce the I/O overhead, 
potentially increasing speed, while requiring more memory."
                )
        )

        // Logging options

        .arg(
            Arg::new("log-level")
                .long("log-level")
                .value_parser(["off", "error", "warn", "info", "debug", "trace"])
                .default_value("off")
                .help_heading("Logging settings")
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
                .help_heading("Logging settings")
                .help("Path to the log file")
                .long_help(
"Path to the log file. Only regarded if debug-level is other than 'off'. If the log 
file exists already new logging output gets appended to the file."
                )
        )

        .after_long_help(
"Notes about the coordinate systems for filtering reads:
-------------------------------------------------------

The filtering flag for reference regions follow the following
coordinate systems:

Example sequence chrA:  A C G T A T A C C T

1. BED file (--bed-file):
   - Coordinates follow BED conventions
   - Start is 0-based (inclusive), end is 1-based (exclusive)
   - Example line:   chrA   1   9
   - Covers bases 2 through 9 of:  A C G T A T A C C T
                                     C G T A T A C C

2. Region string (--ref-regions):
   - Coordinates follow samtools-style conventions
   - Both start and end are 1-based (inclusive)
   - Example:        chrA:2-9
   - Covers bases 2 through 9 of:  A C G T A T A C C T
                                     C G T A T A C C

3. Position with window (--positions-of-interest):
   - Coordinates follow samtools-style conventions
   - Site is 1-based; window expands symmetrically
   - Example:        chrA:5-3
   - Covers bases 2 through 8 of:  A C G T A T A C C T
                                     C G T A T A C
"
        );


    command
}
