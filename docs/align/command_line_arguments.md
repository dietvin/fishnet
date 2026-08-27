# Command line arguments

The table below shows all arguments that are available in the `align` module.

| Long flag | Short flag | Type | Default | Description
|-|-|-|-|-
| `--help` | `-h` | Flag | - | Print help message explaining all flags. `-h` shows a more compact version than `--help`.

## Required Input/Output

| Long flag | Short flag | Type | Default | Description
|-|-|-|-|-
| `--bam <bam>`|  `-b <bam>` | Path (file) | - | Path to a BAM input file. Only a single file must be provided. |
| `--pod5 <pod5>` | `-p <pod5>` | Path(s) (file or directory) | - | Path(s) to the POD5 input. Multiple paths can be provided space-separated. Each path can point to a file or directory. If a directory is provided, all `.pod5` files within it are processed. File and directory paths can be combined. |
| `--out <output>` | `-o <output>` | Path (file) | - | Path to the output file. The file extension determines the format: `.parquet` for Parquet output, `.jsonl` for JSONL output. |

## Optional Arguments

There are various parameters available to set more general settings or to fine-tune the algorithm. For clarity, optional arguments are grouped by function.

### General settings
| Long flag | Short flag | Type | Default | Description
|-|-|-|-|-
| `--rna` | `-r` | Flag | - | Whether direct RNA data is provided. If set, reverses the raw signal (3'->5') to match the base-called/mapped 5'->3' orientation. |
| `--kmer-table <kmer_table>` | `-k <kmer_table>` | Path (file) | - | Path to a [kmer level table](https://github.com/nanoporetech/kmer_models). This is only required if no embedded kmer table can be matched to given data ([more information](kmer_table_matching.md)) |
| `--alignment-type <type>` | `-a <type>` | Enum (`query`, `reference`, `both`) | `query` | Determines the type of alignment to perform. `query`: aligns signal to base-called query sequence. `reference`: aligns signal to the reference sequence (if mapped). |

### Output settings
| Long flag | Short flag | Type | Default | Description
|-|-|-|-|-
| `--output-level <level>` | `-l <level>` | Enum (`1`, `2`, `3`) | `2` | Controls the output detail level: **1:** read ID + alignment(s); **2:** + sequences; **3:** + signal. Note: including signal greatly increases file size. |
| `--force-overwrite` | `-f` | Flag | - | Overwrite existing output files if set. Otherwise, raises an error if the output file already exists. |
| `--output-batch-size <size>` | - | Integer | `64000000` | The maximum estimated data size in bytes that is buffered before flushing the data to file. |

### Threading settings
| Long flag | Short flag | Type | Default | Description
|-|-|-|-|-
| `--threads <n>` |`-t <n>` | Integer | `8` | Number of parallel worker threads used. In addition to the main thread and the specified worker threads, Fishnet spawns one output threads that collects processed data, and one progress thread that handles the progress bar. |
| `--queue-size <size>` | - | Integer | `16000` | Size of the queue for transferring data to and from worker threads. Reducing the queue size lowers memory usage. |

### Logging settings
| Long flag | Short flag | Type | Default | Description
|-|-|-|-|-
| `--log-level <level>` | - | Enum (`off`, `error`, `warn`, `info`, `debug`, `trace`) | `off` | Sets logging verbosity. The amount of intermediate information increases from `error` to `trace`. Use `error` for a summary of failed alignments. |
| `--log-path <path>` | - | Path (file) | `log.txt` | Path to the log file. Only used if `log-level` is not `off`. Existing logs are appended to. |

### Refinement – Dynamic Programming
| Long flag | Short flag | Type | Default | Description
|-|-|-|-|-
| `--refine-iters <n>` | `-i <n>` | Integer | `2` | Number of refinement iterations. In each iteration, alignment boundaries are adjusted to minimize signal differences, and rescaling parameters are recalculated. Set to `0` to skip refinement. |
| `--refine-algo <algo>` | - | Enum (`viterbi`, `dwell-penalty`) | `dwell-penalty` | Refinement algorithm. `dwell-penalty` internally uses Viterbi but penalizes short dwell times. |
| `--dwell-penalty-target <value>` | - | Float | `4.0` | Target dwell time used by the dwell-penalty algorithm. |
| `--dwell-penalty-limit <value>` | - | Float | `3.0` | Maximum dwell time considered for dwell-time penalty. |
| `--dwell-penalty-weight <value>` | - | Float | `0.5` | Penalty weight applied to short dwell times in dwell-penalty refinement. |
| `--half-bandwidth <n>` | - | Integer | `5` | Half-width of the signal band (number of neighboring bases considered during alignment). |
| `--min-band-size <n>` | - | Integer | `2` | Minimum enforced sequence band size when adjusting the sequence band. |
| `--normalize-levels` | - | Flag | - | Normalize expected levels from the k-mer table (equivalent to `do_fix_gauge` in Remora). |

### Refinement – Rescaling
| Long flag | Short flag | Type | Default | Description
|-|-|-|-| -
| `--rescale-algo <algo>` | - | Enum (`theil-sen`, `least-squares`) | `theil-sen` | Rescaling algorithm for normalizing signal (`norm_signal = (signal - shift) / scale`). Uses the full signal for estimation.     |
| `--rescale-dwell-filter-lower-quant <q>` | - | Float | `0.1` | Lower dwell-time quantile filter. Bases with dwell times below this are excluded. |
| `--rescale-dwell-filter-upper-quant <q>` | - | Float | `0.9` | Upper dwell-time quantile filter. Bases above this dwell time are excluded. |
| `--rescale-min-abs-level <value>` | - | Float | `0.2` | Minimum normalized signal intensity difference required for inclusion. |
| `--rescale-num-bases-truncate <n>` | - | Integer | `10` | Number of bases trimmed from each end before rescaling. |
| `--rescale-min-num-filtered-levels <n>` | - | Integer | `10` | Minimum number of bases required after filtering for valid rescaling. |
| `--rescale-max-len <n>` | - | Integer | `1000` | Maximum number of bases used for rescaling. If exceeded, a random subset is used (only for `theil-sen`). Set to `0` to use all. |

### Refinement – Rough Rescaling
| Long flag | Short flag | Type | Default | Description
|-|-|-|-|-
| `--rough-rescale-algo <algo>` | - | Enum (`none`, `least-squares`, `theil-sen`) | `theil-sen` | Algorithm for rough pre-refinement rescaling using signal percentiles instead of full signal data. |
| `--rough-rescale-quants-min <q>` | - | Float | `0.05` | Lowest signal percentile used in rough rescaling. |
| `--rough-rescale-quants-max <q>` | - | Float | `0.95` | Highest signal percentile used in rough rescaling. |
| `--rough-rescale-quants-steps <n>` | - | Integer | `19` | Number of percentile steps between `min` and `max`. Default percentiles: 0.05, 0.10, ..., 0.95. |
| `--rough-rescale-clip-bases <n>` | - | Integer | `10` | Number of bases trimmed from each end before rough rescaling. |
| `--rough-rescale-use-all-signal` | - | Flag | - | Whether to use the entire signal for percentile calculation. If unset, one measurement per base (center) is used to reduce computational load. |
