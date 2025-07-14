# Fishnet

<div style="font-size: 25px; font-weight: bold">Fast signal-to-sequence alignment for Nanopore sequencing data in a simple command line interface</div>


## Table of contents
- [Fishnet](#fishnet)
  - [Table of contents](#table-of-contents)
  - [Description](#description)
  - [Installation](#installation)
  - [Usage](#usage)
    - [Required arguments](#required-arguments)
    - [Optional arguments](#optional-arguments)
  - [Algorithm details](#algorithm-details)
  - [Code structure](#code-structure)
  - [License](#license)

## Description

Fishnet implements the signal-to-sequence alignment algorithm used in [Remora](https://github.com/nanoporetech/remora) in a Rust-based command line interface for better accessibility and improved speed.

## Installation

No installation is required. Simply download the executable for your operating system:
- Linux x64: [fishnet]()
- Linux arm64: [fishnet]()
- Windows: [fishnet.exe]()

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
3. **Expected current intensities**. Must be stored in a **kmer level table**, as [provided by ONT](https://github.com/nanoporetech/kmer_models)

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

## Algorithm details

The sequence-to-signal alignment is calculated in a two step process. An initial alignment is set up from the move table generated during base-calling. Afterwards, the alignment can be refined in an iterative approach where the signal boundaries are shifted to minimize the distance between the observed and expected signal intensities.

A detailed description of all steps in is provided in the [algorithm documentation](./docs/algorithm_details.md).

## Code structure

A detailed overview of the code structure is provided in the [implementation documentation](./docs/implementation_details.md). More detailed explanations are given directly in the scripts.


## License

This project is licensed under the GPL3.0 License. See the [LICENSE](./LICENSE) file for details.