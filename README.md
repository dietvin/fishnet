# Fishnet

<div style="font-size: 25px; font-weight: bold">Fast signal-to-sequence alignment for Nanopore sequencing data in a simple command line interface</div>


## Table of contents
- [Table of contents](#table-of-contents)
- [Description](#description)
- [Installation](#installation)
- [Usage](#usage)
    - [Required arguments](#required-arguments)
    - [Optional arguments](#optional-arguments)
- [Algorithm and backend details](#algorithm-and-backend-details)
- [License](#license)

## Description

Fishnet implements the signal-to-sequence alignment algorithm used in [Remora](https://github.com/nanoporetech/remora) in a Rust-based command line interface for better accessibility and improved speed.

## Installation

No installation is required. Simply download the executable for your operating system:
- Ubuntu: [fishnet]()
- Windows: [fishnet.exe]()

Afterwards the program can be executed from the command line:
```bash
/path/to/fishnet --help
```

To make it more accessible add the executable to the `$PATH` environment variable. This way it can be called at any time:
```bash
fishnet --help
```

More information about the installation and how to build from source can be found in the [documentation](). 

## Usage
Minimal usage:
```bash
fishnet -b <basecalls.bam> -p <raw-signal.pod5> -k <level-table.txt> -o <output-dir>
```

Fishnet requires the following data:
- **Base-called data**: A single BAM file containing the (unmapped) base-calls (including *move tables*)
- **Raw signal data**: One or more POD5 file or directories containing POD5 files 
- **Expected signal intensities**: A k-mer level table obtained from the [kmer_models repository](https://github.com/nanoporetech/kmer_models)
- An output directory

### Required arguments

The following arguments are required:

| Long arg   | Short arg | Explanation                                                                                                                             | Type |
|------------|-----------|-----------------------------------------------------------------------------------------------------------------------------------------|------|
| bam        | b         | Path to a bam file (as given by Dorado; must contain **move tables** for each read)                                                                                                                      | str  |
| pod5       | p         | Path(s) to one or more pod5 files and/or directories containing pod5 files (separate multiple paths by space) | (multiple) str  |
| kmer-table | k         | Path to a [kmer level table](https://github.com/nanoporetech/kmer_models)                                                                                                               | str  |
| output-dir | o         | Path to a directory where the aligned data will be written to                                                                           | str  |
### Optional arguments

The following arguments are the most important optional arguments for most users:

| Long arg        | Short arg | Explanation                                                                                                                                                                                                   | Type |
|-----------------|-----------|---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|------|
| rna             | -         | Whether the provided data is direct RNA sequencing data. If set, the signal gets reversed for the alignment                                                                                                   | bool |
| alignment-type  | a         | Which type(s) of alignment to generate. Can be '**query**' (Default) to align the signal to the base-called sequence, '**reference**' to align to the reference sequence (if mapped)or '**both**' to do both. | str  |
| threads         | t         | Number of parallel threads to use. Default: **8**                                                                                                                                                             | int  |
| output-format   | -         | The output format to which the aligned data will be written. Options: '**parquet**' (Default) or '**jsonl**'                                                                                                  | str  |
| force-overwrite | f         | If set and an output file already exists, this file will be overwritten. Raises an error otherwise                                                                                                            | bool |

For the sake of simplicity, the table shows only a subset of the optional arguments. An explanation for all arguments can be found in the [documentation]().

## Algorithm and backend details

A detailled description of the algorithm that is used and how the algorithm is implemented is provided in the [documentation]().

## License

This project is licensed under the GPL3.0 License. See the [LICENSE](./LICENSE) file for details.