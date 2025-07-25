# Fishnet

## TL;DR

Fishnet implements the signal-to-sequence alignment algorithm used in [Remora](https://github.com/nanoporetech/remora) in a Rust-based command line interface for better accessibility and improved speed.

[Download](https://github.com/dietvin/fishnet/releases/latest) the binary, extract it and run:
```bash
./fishnet --help
```

## Table of contents
- [Fishnet](#fishnet)
  - [TL;DR](#tldr)
  - [Table of contents](#table-of-contents)
  - [Installation](#installation)
  - [Usage](#usage)
    - [Required arguments](#required-arguments)
    - [Optional arguments](#optional-arguments)
  - [Output](#output)
  - [Algorithm details](#algorithm-details)
  - [Code structure](#code-structure)
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

## Usage
Minimal usage:
```bash
fishnet -b <basecalls.bam> -p <raw-signal.pod5> -k <level-table.txt> -o <output-dir>
```
More examples are provided in the [usage documentation](./docs/usage.md#examples). Fishnet requires the following input data:
1. **Raw sequencing data**. Must be stored in **POD5** format.
2. **Basecalled data**. Must be stored in a single **BAM** file, as produced by Dorado (Note that it must contain the move-table, so base-call with the `--emit-moves` flag!)
3. **Expected current intensities**. Must be stored in a **kmer level table**, as [provided by ONT](https://github.com/nanoporetech/kmer_models):
     - DNA R10 (400bps): [9mer_levels_v1.txt](https://raw.githubusercontent.com/nanoporetech/kmer_models/refs/heads/master/dna_r10.4.1_e8.2_400bps/9mer_levels_v1.txt)
     - DNA R10 (260bps): [9mer_levels_v1.txt](https://github.com/nanoporetech/kmer_models/blob/master/dna_r10.4.1_e8.2_260bps/9mer_levels_v1.txt)
     - RNA004: [9mer_levels_v1.txt](https://raw.githubusercontent.com/nanoporetech/kmer_models/refs/heads/master/rna004/9mer_levels_v1.txt)
     - RNA002: [5mer_levels_v1.txt](https://raw.githubusercontent.com/nanoporetech/kmer_models/refs/heads/master/rna_r9.4_180mv_70bps/5mer_levels_v1.txt)

### Required arguments

The following arguments are required:

| Long flag   | Short flag | Explanation                                                                                                                             | Type |
|------------|-----------|-----------------------------------------------------------------------------------------------------------------------------------------|------|
| --bam        | -b         | Path to a bam file (as given by Dorado; must contain **move tables** for each read)                                                                                                                      | str  |
| --pod5       | -p         | Path(s) to one or more pod5 files and/or directories containing pod5 files (separate multiple paths by space) | (multiple) str  |
| --kmer-table | -k         | Path to a [kmer level table](https://github.com/nanoporetech/kmer_models)                                                                                                               | str  |
| --out        | -o         | Path to a directory where the aligned data will be written to                                                                           | str  |

### Optional arguments

The following arguments are the most relevant optional arguments for most users:

| Long flag        | Short flag | Explanation                                                                                                                                                                                                   | Type |
|-----------------|-----------|---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|------|
| --rna             |          | Whether the provided data is direct RNA sequencing data. If set, the signal gets reversed for the alignment (dRNA signals are measured 3'-5')                                                                                                   | bool |
| --alignment-type  | -a         | Which type(s) of alignment to generate. Can be '**query**' (Default) to align the signal to the base-called sequence, '**reference**' to align to the reference sequence (if mapped)or '**both**' to do both. | str  |
| --threads         | -t         | Number of parallel threads to use. Default: **8**                                                                                                                                                             | int  |
| --force-overwrite | -f         | If set and an output file already exists, this file will be overwritten. Raises an error otherwise                                                                                                            | bool |

For the sake of simplicity, the table shows only a subset of the optional arguments. An explanation for all arguments can be found in the [usage documentation](./docs/usage.md#comman-line-interface).

## Output

The output format is determined by the file extension provided in the output file path. Available formats are [Parquet](https://parquet.apache.org/docs/overview/) (`.parquet`) and [JSONL](https://jsonlines.org/) (`.jsonl`) format. Parquet format is recommended as it is more efficient due to compression and chunked writing/reading.

The output structure depends on the given values for the `--alignment-type` and `--output-level` flags. The table below shows the columns that are present with given alignment type (rows) and output level (columns) settings:

|  | 1 | 2 | 3 |
|---|---|---|---|
| **query** | read_id, query_to_signal | read_id, query_to_signal, **query_sequence** | read_id, query_to_signal, query_seq, **signal** |
| **reference** | read_id, ref_to_signal, ref_name, ref_start | read_id, ref_to_signal, ref_name, ref_start, **ref_sequence** | read_id, ref_to_signal, ref_name, ref_start, ref_sequence, **signal** |
| **both** | read_id, query_to_signal ref_to_signal, ref_name, ref_start | read_id, query_to_signal ref_to_signal, ref_name, ref_start, **query_sequence**, **ref_sequence** | read_id, query_to_signal ref_to_signal, ref_name, ref_start, query_sequence, ref_sequence, **signal** |

More information about the output file structure is given in the [usage documentation](./docs/usage.md#output)

## Algorithm details

The sequence-to-signal alignment is calculated in a two step process. An initial alignment is set up from the move table generated during base-calling. Afterwards, the alignment can be refined in an iterative approach where the signal boundaries are shifted to minimize the distance between the observed and expected signal intensities.

A detailed description of all steps in is provided in the [algorithm documentation](./docs/algorithm_details.md).

## Code structure

A detailed overview of the code structure is provided in the [implementation documentation](./docs/implementation_details.md). More detailed explanations are given directly in the scripts.


## License

This project is licensed under the GPL3.0 License. See the [LICENSE](./LICENSE) file for details.