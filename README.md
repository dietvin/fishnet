![fishnet_logo](docs/images/fishnet_logo_icon.png)

## TL;DR

Signal-to-sequence alignments like [Remora](https://github.com/nanoporetech/remora), but faster and more accessible. [Download fishnet](https://github.com/dietvin/fishnet/releases/latest), extract the binary and run the `align` command:
```bash
./fishnet align --help
```

For further processing, run the `reformat` command:
```bash
./fishnet reformat --help
```

## Table of contents

- [TL;DR](#tldr)
- [Table of contents](#table-of-contents)
- [Installation](#installation)
- [Alignment](#alignment)
  - [Minimal usage](#minimal-usage)
  - [Required arguments](#required-arguments)
  - [Optional arguments](#optional-arguments)
  - [Output](#output)
  - [Algorithm details](#algorithm-details)
- [Reformatting](#reformatting)
  - [Minimal usage](#minimal-usage-1)
  - [Required arguments](#required-arguments-1)
    - [Filter arguments (one is required):](#filter-arguments-one-is-required)
  - [Optional arguments](#optional-arguments-1)
  - [Reformatting strategies](#reformatting-strategies)
  - [Output formats](#output-formats)
- [POD5 Reader API](#pod5-reader-api)
- [Repository structure](#repository-structure)
- [License](#license)

## Installation

No installation is required. Simply download the executable for your operating system:
- [Linux x64](https://github.com/dietvin/fishnet/releases/latest/download/fishnet-linux-x86_64.tar.gz)
- [Linux arm64](https://github.com/dietvin/fishnet/releases/latest/download/fishnet-linux-aarch64.tar.gz)
- [Windows](https://github.com/dietvin/fishnet/releases/latest/download/fishnet.exe)

Afterwards the program can be executed from the command line:
```bash
/path/to/fishnet --help
```

To make it more accessible add the executable to the `$PATH` environment variable. This way it can be called at any time:
```bash
fishnet --help
```

More information about the installation and how to build from source can be found in the [installation documentation](./docs/installation.md). 

## Alignment

```bash
fishnet align [...]
```

A signal-to-sequence alignment `A` is an array of signal indices, where the pair `A[i]`, `A[i+1]` corresponds to the start and end indiced on the signal assigned to base `i`. The intervals are half-open (start is included, end is not). 

Here is a minimal example:
```
Signal:
┌──────────────────────────────┐
│ x                      xxxxx │
│x x      xxx           x     x│
│   x    x   xxxx      x       │
│    xxxx        xxxxxx        │
└──────────────────────────────┘
 012345678901234567890123456789  (Signal index)

Sequence:
A C G T A                 (length = 5)

Signal-to-sequence:
[0, 4, 9, 16, 23, 30]     (length = 6)
┌────┬─────┬───────┬───────┬───────┐
│ x  │     │       │       │ xxxxx │
│x x │     │xxx    │       │x     x│
│   x│    x│   xxxx│      x│       │
│    │xxxx │       │xxxxxx │       │
└────┴─────┴───────┴───────┴───────┘
│0123│45678│9012345│6789012│3456789│
│ A  │  C  │   G   │   T   │   A   │
```

With `Fishnet`, signal-to-sequence alignments are created using the `align` command. It is possible to align both the base-called (query) and (if present) the reference sequences to the signal.

The alignment requires the following input data:
1. **Raw sequencing data**. Must be stored in **POD5** format
2. **Basecalled data**. Must be stored in a single **BAM** file, as produced by [Dorado](https://github.com/nanoporetech/dorado/) (Note that it must contain the move-table, so base-call with the `--emit-moves` flag!)
3. **Expected current intensities**. Must be stored in a **kmer level table**, as [provided by ONT](https://github.com/nanoporetech/kmer_models):
     - DNA R10 (400bps): [9mer_levels_v1.txt](https://raw.githubusercontent.com/nanoporetech/kmer_models/refs/heads/master/dna_r10.4.1_e8.2_400bps/9mer_levels_v1.txt)
     - DNA R10 (260bps): [9mer_levels_v1.txt](https://github.com/nanoporetech/kmer_models/blob/master/dna_r10.4.1_e8.2_260bps/9mer_levels_v1.txt)
     - RNA004: [9mer_levels_v1.txt](https://raw.githubusercontent.com/nanoporetech/kmer_models/refs/heads/master/rna004/9mer_levels_v1.txt)
     - RNA002: [5mer_levels_v1.txt](https://raw.githubusercontent.com/nanoporetech/kmer_models/refs/heads/master/rna_r9.4_180mv_70bps/5mer_levels_v1.txt)

### Minimal usage

```bash
fishnet align \
  --bam <basecalls.bam> \
  --pod5 <raw-signal.pod5> \
  --kmer-table <level-table.txt> \
  --out <output-file>
```
More examples are provided in [Examples](docs/align/examples.md).

### Required arguments

The following arguments are required:

| Long flag | Short flag | Explanation | Type |
|-|-|-|-|
| --pod5 | -p | Path(s) to one or more pod5 files and/or directories containing pod5 files (separate multiple paths by space) | Path(s) (file or directory) |
| --bam | -b | Path to a bam file (as given by Dorado; must contain **move tables** for each read) | Path (file) |
| --kmer-table | -k | Path to a [kmer level table](https://github.com/nanoporetech/kmer_models) | Path (file) |
| --out | -o | Path to the output file. Must end with .parquet (recommended) or .jsonl depending on the wanted output format | Path (file) |

### Optional arguments

The following arguments are the most relevant optional arguments for most users:

| Long flag | Short flag |Explanation | Type |
|-|-|-|-|
| --rna | -r | Whether the provided data is direct RNA sequencing data. If set, the signal gets reversed for the alignment (dRNA signals are measured 3'-5') | Flag |
| --alignment-type | -a | Which type(s) of alignment to generate. Can be '**query**' (Default) to align the signal to the base-called sequence, '**reference**' to align to the reference sequence (if mapped)or '**both**' to do both. | Enum (`query`, `reference`, `both`) |
| --threads | -t | Number of parallel threads to use. Default: **8** | int |
| --force-overwrite | -f | If set and an output file already exists, this file will be overwritten. Raises an error otherwise | Flag |

For the sake of simplicity, the table shows only a subset of the optional arguments. For an overview of all arguments, see [Command line arguments](docs/align/command_line_arguments.md).

### Output

The output format is determined by the file extension provided in the output file path. Available formats are [Parquet](https://parquet.apache.org/docs/overview/) (`.parquet`) and [JSONL](https://jsonlines.org/) (`.jsonl`) format. Parquet format is recommended as it is more efficient due to compression and chunked writing/reading.

The exact output structure depends on the given values for the `--alignment-type` and `--output-level` flags. For a detailled overview on which columns are present with which settings, see [Output formats](docs/align/output_formats.md).

### Algorithm details

The sequence-to-signal alignment is calculated in a two step process. An initial alignment is set up from the move table generated during base-calling. Afterwards, the alignment can be refined in an iterative approach where the signal boundaries are shifted to minimize the distance between the observed and expected signal intensities.

For a detailed description of all steps, see [Algorithm details](docs/align/algorithm_details.md).


## Reformatting

```bash
fishnet reformat [...]
```

After aligning signals to sequences, the alignments consists only of signal indices, not the actual signal chunks. Fishnet provides the `reformat` command to process previously calculated alignments with the signals into formats that can easily used for further downstream processing or analyses. 

### Minimal usage

```bash
fishnet reformat \
  --alignment <alignments.parquet> \
  --pod5 <raw-signal.pod5> \
  --motifs <motif> \
  --out <output-file>
```
More examples are provided in [Examples](docs/reformat/examples.md).

### Required arguments

| Long flag | Short flag | Explanation | Type |
|-|-|-|-|
| --alignment | -a | Path to a parquet file produced by `fishnet align` | Path (file) |
| --bam | -b | Path to a bam file (as given by Dorado; must contain **move tables** for each read) | Path (file) |
| --out | -o | Path to the output file. Must end with .parquet (recommended) or .tsv depending on the wanted output format | Path (file) |

#### Filter arguments (one is required):

To reduce the amount of processing required and focus only on bases of interest, the `reformat` module implements different filtering options. 

Reference-to-signal alignments can be filtered by **reference regions of interest**. Alternatively, both query-to-signal and reference-to-signal alignments can be filtered by **motifs of interest**.

**Only parts of a read that overlap with a region of interest are further processed.**
| Long flag | Short flag | Explanation | Type |
|-|-|-|-|
| --ref-regions | -r | Filter input data for one or more reference region(s). Each must be in the format `<REF-NAME>:<REF-START>-<REF-END>` (Start and end are 1-based coordinates and inclusive). | String(s) |
| --bed-file | -R | Filter input data for reference regions from bed file. Must follow bed-style coordinate conventions (0-based, start inclusive, end exclusive) | Path (file) |
| --positions-of-interest | -P | Filter input data for one or more positions of interest. Each must be in the format `<REF-NAME>:<REF-SITE>-<HALF-SIZE>`, where <HALF-SIZE> determines the number of bases up- and downstream from the site that are of interest. Site coordinate is 1-based | String(s) |
| --motifs | -m | Filter input data for reference regions from bed file. Must follow bed-style coordinate conventions (0-based, start inclusive, end exclusive) | String(s) |
| --bed-file | -R | Filter input data for reference regions from a FASTA file. Each motif must be a separate entry. Sequences must contain only 'A', 'C', 'G' and 'T'/'U' | Path (file) |

### Optional arguments

The following arguments are the most relevant optional arguments for most users:

| Long flag | Short flag | Explanation | Type |
|-|-|-|-|
| `--pod5` | `-p` | POD5 input(s). Required if alignment file lacks raw signal. Multiple are allowed | Path(s) (file or directory) |
| `--rna` | - | Set if direct RNA POD5 file(s) are provided (reverse signal) | Flag |
| `--alignment-type` | - | Set only if the alignment file contains both query- and reference to signal alignments. Set to determine which type will be processed. `query` and `reference` are allowed | Enum (`query`, `reference`) |
| `--strategy` | `-s` | How to reformat the data. See [Reformatting strategies](#reformatting-strategies) for more information | Enum (`stats`, `interpolate`) |
| `--output-shape` | - | How to shape the output data. See [Output formats](#output-formats) for more information | Enum (`melted`, `exploded`, `nested`) |
| `--threads` | `-t` | Number of parallel threads to use. Default: **8** | int |
| `--force-overwrite` | `-f` | If set and an output file already exists, this file will be overwritten. Raises an error otherwise | Flag |

For the sake of simplicity, the table shows only a subset of the optional arguments. For an overview of all arguments, see [Command line arguments](docs/reformat/command_line_arguments.md).

### Reformatting strategies

There are two reformatting strategies implement: 
1. **Base-wise statistics**: Calculates statistics that represents the signal assigned to a given base. 
    - This is the default strategy. Can be exlicitly set via the `--strategy "stats"` flag
    - One or more statistics can be specified via the `--stats <stats>...` flag (default: `mean std dwell`). 
    - Available statistics are:
      - Mean signal intensity
      - Median signal intensity
      - Standard deviation of the signal intensity  
      - Dwell time (number of measurements assigned to the base)
      - Signal-to-noise ratio (mean / std. dev.)
2. **Interpolation**: Reshapes the signal for each base into a uniform number of samples using linear interpolation.
    - Can be chosen via the `--strategy "interpolate"` flag
    - The number of interpolated samples can be set via the `--target-size <target-size>` flag (default: `30`)

See [Reformatting strategies](docs/reformat/reformatting_strategies.md) for more details.

### Output formats

The reformatted data can be written to compressed `parquet` or simple `TSV` format. Since TSV is uncompressed, parquet format is recommended. To account for different downstream processing and analyses, there are three output formats to choose from:
1. **Melted**: Long format containing one row for each base. Useful for visualization with ggplot2/seaborn.
2. **Exploded**: Wide format containing one row for each read-region pair. All values for all bases appear as separate columns. Here all regions need to have the same length. Useful for machine-learning task, e.g. clustering.
3. **Nested**: One row for each read-region pair. Fields store lists or 2D arrays. Only available for `parquet` output

See [Output formats](docs/reformat/output_formats.md) for more details. 


## POD5 Reader API

The `POD5 reader API` provides straight-forward and efficient access to the current signal and corresponding metadata stored in pod5 files. Key features are:
- **Lazy loading** of pod5 files to enable memory-efficient reading
- **Read-wise iteration** to access a large number of reads in straight-forward manner
- **(Thread-safe) random access** to enable targeted access to single reads, optionally in parallel from multiple threads 

The key data structs are [`Pod5File`](docs/pod5_reader_api/pod5_file.md#pod5file) and [`Pod5FileThreadSafe`](docs/pod5_reader_api/pod5_file.md#pod5filethreadsafe) for **single-file access**, [`Pod5Dataset`](docs/pod5_reader_api/pod5_dataset.md#pod5dataset) and [`Pod5DatasetThreadSafe`](docs/pod5_reader_api/pod5_dataset.md#pod5datasetthreadsafe) for **multi-file access**. Reads stored in a given pod5 file are represented by the [`Pod5Read`](docs/pod5_reader_api/pod5_read.md). Follow the links for detailled information about all availalbe functions and examples. 


## Repository structure

The code-base is split into different libraries:
- [`fishnet`](fishnet/): Contains the entry point to the command line interface
- [`alignment`](alignment/): Contains the signal-to-sequence alignment logic
- [`reformat`](reformat/): Contains the reformatting logic
- [`pod5_reader_api`](pod5_reader_api/): Contains the logic for accessing pod5 data
- [`helper`](helper/): Contains helper scripts used in the `alignment` and `reformat` libraries

Beyond the code the repo contains [example data](example_data/) to test fishnet, and detailled [documentation](docs/) of all libraries.

## License

This project is licensed under the GPL3.0 License. See the [LICENSE](./LICENSE) file for details.
