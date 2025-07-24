# Usage

## Table of contents
- [Usage](#usage)
  - [Table of contents](#table-of-contents)
  - [Input data](#input-data)
  - [Examples](#examples)
  - [Comman line interface](#comman-line-interface)
    - [Options](#options)
    - [Required arguments](#required-arguments)
    - [Optional arguments](#optional-arguments)
      - [General settings](#general-settings)
      - [Output settings](#output-settings)
      - [Threading settings](#threading-settings)
      - [Logging settings](#logging-settings)
      - [Refinement - Dynamic programming](#refinement---dynamic-programming)
      - [Refinement - Rescaling](#refinement---rescaling)
      - [Refinement - Rough rescaling](#refinement---rough-rescaling)

## Input data

Three types of data are needed to perform the signal-to-sequence alignment:

1. Raw sequencing data. Must be stored in **POD5** format.
2. Basecalled data. Must be stored in a single **BAM** file, as produced by Dorado (Note that it must contain the move-table, so base-call with the `--emit-moves` flag!)
3. Expected current intensities. Must be stored in a **kmer level table**, as [provided by ONT](https://github.com/nanoporetech/kmer_models)

## Examples

Aligning mapped **DNA** data to the reference:
```bash
fishnet \
    --bam <bam-file> \
    --pod5 <pod5-file1> <pod5-file2> <pod5-file3> \
    --kmer-table <kmer-table-file> \
    --out <output-file-name>.parquet \
    --alignment-type reference
``` 

Aligning **direct RNA** data to the base-called sequence:
```bash
fishnet \
    --bam <bam-file> \
    --pod5 <pod5-directory> \
    --kmer-table <kmer-table-file> \
    --out <output-file-name>.parquet \
    --rna
``` 

## Comman line interface

### Options

| Long flag | Short flag | Description |
|---|---|---|
| --help | -h | Prints the help message explaining all flags (-h shows a more compact message) |
| --version | -v | Prints the version |

### Required arguments

The following arguments set the central input and output parameters and are required.

| Long flag | Short flag | Type | Description |
|---|---|---|---|
| --bam | -b | string | Path to a BAM input file. Only a single file must be provided. |
| --pod5 | -p | string(s) | Path to the POD5 input. Multiple paths can be provided space separated. A path can point to a POD5 file or a directory. If a directory is given all POD5 files in the directory are processed. File and directory paths can be combined. |
| --kmer-table | -k | string | Path to a kmer table file. Kmer tables are provided by ONT in this repository. |
| --out | -o | string | Path to the output file. File extension determines the output format. Must be '.parquet' for Parquet output or '.jsonl' for JSONL output. |

### Optional arguments

There are various parameters available to set more general settings or to fine-tune the algorithm. For the sake of clarity, the optional arguments are split into three sub-groups:

#### General settings

These arguments let the user set important settings for the type of data that is used (direct RNA / DNA) and the alignment that is wanted (query-to-signal / reference-to-signal). 

| Long flag | Short flag | Type (possible values) | Default | Description |
|---|---|---|---|---|
| --rna | -r | bool | false | Whether direct RNA data is provided. If set, reverses the raw sigal (3'->5') to be in 5'->3' orientation to match the base-called/mapped data. |
| --alignment-type | -a | string (query \| reference \| both) | query | Determines the type of alignment that is performed. If set to 'query' the signal is aligned to the base-called query sequence. If set to 'reference' and a given read is mapped to a reference, the signal is aligned to that reference sequence. |

#### Output settings
These arguments control the output behaviour, including what information gets written to file (`output-level`).

| Long flag | Short flag | Type (possible values) | Default | Description |
|---|---|---|---|---|
| --output-level | -l | int (1 \| 2 \| 3) | 1 | The output level determines which data gets written to the output file.<br> With level 1, only the read id and the alignment(s) get written to file. With level 2, the read id, alignment(s) and sequence(s) get written to file. With level 3, the read id, alignment(s), sequence(s) and the signal get written to file.<br> Note that especially when exporting the signal, the file size can get a lot larger. It is recommended to extract the signal separately in subsequent steps and not store it in the output.
| --force-overwrite | -f | bool | false | Whether existing output files should be overwritten. If the provided output path already exists and the flag is set the existing file is overwritten. Otherwise an error is raised. |
| --output-batch-size |  | int | 1000 | Output batch size. Determines the number of alignments that are collected before dumping these to file. Higher values reduce the I/O overhead, potentially increasing speed, while requiring more memory. |


#### Threading settings
These arguments handle multithreading options, including the number of parallel threads (`threads`).

| Long flag | Short flag | Type (possible values) | Default | Description |
|---|---|---|---|---|
| --threads | -t | int | 8 | Set the number of parallel threads used during processing. Set to 1 to disable multithreading. If set to 2 or 3, falls back to single-threaded processing (due to 3 non-worker threads). |
| --queue-size |  | int | 1000 | Sets the queue size for transfering data to and from worker threads. Only regarded if number of threads is larger than 3. Decrease queue size for a reduced memory footprint. |


#### Logging settings
These arguments handle if and how detailled information gets logged (`log-level`) and where it is written to (`log-path`).

| Long flag | Short flag | Type (possible values) | Default | Description |
|---|---|---|---|---|
| --log-level |  | string (off \| error \| warn \| info \| debug \| trace) | off | Sets the logging level. The amount of intermediated information written to the log increases from 'error' to 'trace'. Set to error to get an overview of the reasons why the alignment failed for (some) given reads. Logging is disabled by default. |
| --log-path |  | string | log.txt | Path to the log file. Only regarded if debug-level is other than 'off'. If the log file exists already new logging output gets appended to the file. |


#### Refinement - Dynamic programming

These arguments allow fine-tuning of the refinement process. In practice, the most important argument is `refine-iters` to determine the number of refinement iterations.

| Long flag | Short flag | Type (possible values) | Default | Description |
|---|---|---|---|---|
| --refine-iters | -i | int | 2 | Sets the number of refinement iterations. In each iteration the alignment boundaries are shifed to minimize the difference between the expected and observed signal, followed by a calculation of rescaling parameters based on the shifed alignment. If set to 0 the refinement is skipped. |
| --refine-algo |  | string (viterbi \| dwell-penalty) | dwell-penalty | Refinement algorithm. Viterbi and dwell penalty approaches are available. The dwell penalty approach also performs the viterbi approach internally,while additionally penalizing adjustments in the mapping that result in short dwell times at a given base. |
| --dwell-penalty-target |  | float | 4.0 | Preferred dwell time used in dwell penalty refinement. Only considered if refine-algo is 'dwell-penalty'. |
| --dwell-penalty-limit |  | float | 3.0 | Maximum dwell time that is penalized in dwell penalty algorithm. Only considered if refine-algo is 'dwell-penalty'. |
| --dwell-penalty-weight |  | float | 0.5 | Strength of the penalty applied to short dwell times in dwell penalty algorithm. Only considered if refine-algo is 'dwell-penalty'. |
| --half-bandwidth |  | int | 5 | Half-width of the signal band, meaning that for each signal measurement bases half-bandwidth up- and downstream from the currently assigned one can be considered. |
| --min-band-size |  | int | 2 | The minimum sequence band size that is forced when adjusting the sequence band. This means that a given signal measurement can potentially be assigned to min-band size number of bases. |
| --normalize-levels |  | bool | false | Whether to normalize the expected levels given in the kmer-table. This is equivalent to the `do_fix_gauge` setting in Remora. |

#### Refinement - Rescaling

These arguments allow fine-tuning of the rescaling process, which is performed after shifting the signal alignment boundaries for each base to minimize the distance to the expected signal intensity. During rescaling, the scale and shift parameters are calculated to normalize the signal optimally based on the alignment.

| Long flag | Short flag | Type (possible values) | Default | Description |
|---|---|---|---|---|
| --rescale-algo |  | string (theil-sen \| least-squares) | theil-sen | Which rescaling algorithm to use to calculate shift and scale parameters to normalize the signal measurement (norm_signal = (signal - shift) / scale). Other than the rough rescaling, here the entire signal is used for the estimation. Available algorithms are least-squares and theil-sen. Note that least-squares is not available and tested in Remora. |
| --rescale-dwell-filter-lower-quant |  | float | 0.1 | Lower filtering threshold for dwell times. Signal data for bases with dwell times below this quantile value are filtered out before rescaling. |
| --rescale-dwell-filter-upper-quant |  | float | 0.9 | Upper filtering threshold for dwell times. Signal data for bases with dwell times above this quantile value are filtered out before rescaling. |
| --rescale-min-abs-level |  | float | 0.2 | Minimum absolute (normalized) signal intensity filter threshold. Signal data from bases where the mean signal itensity deviates less than the given value from the expected intensity, is filtered out before rescaling. |
| --rescale-num-bases-truncate |  | int | 10 | Number of bases to truncate before rescaling. Signal data from the first and last given number of bases are filtered out before rescaling. |
| --rescale-min-num-filtered-levels |  | int | 10 | The minimum number of bases that must remain after filtering to be considered valid for rescaling. |
| --rescale-max-len |  | int | 1000 | Maximum number of bases to use for rescaling. If the sequence contains more bases than the given number, the data is randomly subset to contain the given number of data points. Only regarded when rescale-algo is theil-sen. If set to 0 no subsetting is performed. |

#### Refinement - Rough rescaling

These arguments allow fine-tuning of the rough rescaling process, which is performed once before the first refinement iteration. It does the same as the proper rescaling with the difference that the estimation is based on only a few quantiles of the signal instead of the signal itself.

| Long flag | Short flag | Type (possible values) | Default | Description |
|---|---|---|---|---|
| --rough-rescale-algo |  | string (none \| least-squares \| theil-sen) | theil-sen | Which rough rescaling algorithm to use. Calculates shift and scale parameters to normalize the signal measurement (norm_signal = (signal - shift) / scale). Rough rescaling, because only given percentile values are used instead of all measurements. Available algorithms are least-squares and theil-sen. Theil-sen is considered to be more robust against outliers. |
| --rough-rescale-quants-min |  | float | 0.05 | Lowest percentile to calculate from the signal data during rough rescaling. |
| --rough-rescale-quants-max |  | float | 0.95 | Highest percentile to calculate from the signal data during rough rescaling. |
| --rough-rescale-quants-steps |  | int | 19 | Number of percentile values to consider during rough rescaling. rough-rescale-quants steps number of quantiles are considered, increasing evenly from the lowest to the highest quantile. The lowest and highest values are included. Default quantiles are 0.05, 0.10, 0.15, ..., 0.90, 0.95. |
| --rough-rescale-clip-bases |  | int | 10 | Number of bases to truncate before rough rescaling. Signal data from the first and last given number of bases are filtered out before rough rescaling. |
| --rough-rescale-use-all-signal |  | bool | false | Whether to use the entire signal for quantile calculation during rough rescaling. If set, the quantile values are calculated from all measurements. Otherwise the signal is subset to contain only a single measurement for each base, reducing the computational load. This measurement is taken from the center of the signal assigned to a given base. |

## Output

The output structure depends on two settings: Which alignment type (`--alignment-type`) and which out level (`--output-level`) is chosen. Possible are *query* (default), *reference* and *both* for `alignment-type` and *1* (default), *2* and *3* for `output-level`. 

The table below shows each column with given settings for the alignment type (rows) and output level (columns): 

|  | 1 | 2 | 3 |
|---|---|---|---|
| **query** | read_id, query_to_signal | read_id, query_to_signal, **query_sequence** | read_id, query_to_signal, query_seq, **signal** |
| **reference** | read_id, ref_to_signal, ref_name, ref_start | read_id, ref_to_signal, ref_name, ref_start, **ref_sequence** | read_id, ref_to_signal, ref_name, ref_start, ref_sequence, **signal** |
| **both** | read_id, query_to_signal ref_to_signal, ref_name, ref_start | read_id, query_to_signal ref_to_signal, ref_name, ref_start, **query_sequence**, **ref_sequence** | read_id, query_to_signal ref_to_signal, ref_name, ref_start, query_sequence, ref_sequence, **signal** |

The inidividual columns have the following data types in them:
- `read_id`: String
- `query_to_signal` / `ref_to_signal`: List of 64bit unsigned int
- `ref_name`: String
- `ref_start`: 64bit unsigned int
- `query_sequence` / `ref_sequence`: String
- `signal`: List of 16bit signed int