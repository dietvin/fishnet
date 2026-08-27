# Reformat

```bash
fishnet reformat \
  --alignment <alignments.parquet> \
  --pod5 <raw-signal.pod5> \          # See "Pod5 input" below
  --motifs <motif> \                  # See "Filter arguments" below
  --out <output-file>
```

After aligning signals to sequences, the alignments consists only of signal indices, not the actual signal chunks. Fishnet provides the `reformat` command to process previously calculated alignments with the signals into formats that can easily used for further downstream processing or analyses. 

## Required arguments

| Long flag | Short flag | Type | Default | Description
|-|-|-|-|-
| `--alignment <alignment>` | `-a <alingment>`| Path(file) | - | Path to `.parquet` or `.jsonl` file produced by `fishnet align`
| `--out <out>` | `-o <out>` | Path (file) | - | Output path. Extension determines format (`.parquet` or `.tsv`)

### Pod5 input (optional, but recommended):

| Long flag | Short flag | Type | Default | Description
|-|-|-|-|-
| `--pod5 <pod5>...` | `-p <pod5>...` | Path(s) (file or directory) | - | POD5 input(s). Required if alignment file lacks raw signal. Multiple allowed

This is required if the alignment file does not contain the raw signal (which is done by setting `--output-level 3` in the `align` command). We recommend not writing the signal to the alignment file to minimize redundancy and file size, since the signals can be easily retrieved again, and the POD5-internal compression is more efficient.

When providing the POD5 data separately for direct RNA data, make sure to also specify the `--rna` flag.

### Filter arguments (one is required):

To reduce the amount of processing required and focus only on bases of interest, the `reformat` module implements filtering by **regions of interest** (reference-to-signal only) or **motifs of interest** (query- & reference-to-signal).

Only parts of a read that overlap with a region of interest are further processed. One of the following options must be chosen for filtering the data:

1. **Manually specifiy region(s) of interest**:
   - `--ref-regions <region>...`
   - Each region must follow the `<NAME>:<START>-<END>` format, where `<START>` and `<END>` are 1-based coordinates and inclusive (SAM-style)
   - Multiple can be specified space-separated
2. **Region(s) of interest in a bed file**:
   - `--bed-file <file>`
   - The provided path must point to a bed file following bed-style coordinate conventions (0-based, start inclusive, end exclusive)
3. **Position(s) around a site of interest**:
   - `--positions-of-interest <poi>...` 
   - Each position must follow the `<NAME>:<SITE>-<HALF-SIZE>` format, where <HALF-SIZE> determines the number of bases up- and downstream from the site that are of interest. `<SITE>` is 1-based
   - Multiple can be specified space-separated
4. **Manually specify motif(s) of interest**:
   - `--motifs <motif>...`
   - Filter for motif sequence(s). Sequences must contain only 'A', 'C', 'G' and 'T'/'U'
   - Multiple can be specified space-separated
5. **Provide motif(s) of interest in a fasta file**:
   - `--motifs-file <file>`
   - The provided path must point to a fasta file, where each motif must be a separate entry. Sequences must contain only 'A', 'C', 'G' and 'T'/'U'

See [CLI - Data filtering](./command_line_arguments.md#data-filtering) for more information about the filtering command-line flags.

## Optional arguments

The following arguments are the most relevant optional arguments for most users:

| Long flag | Short flag | Type | Default | Description
|-|-|-|-|-
| `--rna` | - | Flag | - | Set if direct RNA POD5 file(s) are provided (reverse signal) | Flag |
| `--alignment-type <alignment-type>` | - | Enum (`reference`, `query`) | - | Only required if the alignment file contains both query- and reference-to-signal alignments. Set to determine which type will be processed
| `--strategy <strategy>` | `-s <strategy>` | Enum (`stats`, `interpolate`) | `stats` | Processing strategy: compute statistics or interpolate signal (See [Reformatting strategies](#reformatting-strategies) below)
| `--output-shape <shape>` | - | Enum (`melted`, `exploded`, `nested`) | `nested` | Determines output structure (nested only for Parquet) (See [Output formats](#output-formats) below)
| `--threads <n>` | `-t <n>` | Integer | `8` | Number of parallel threads
| `--force-overwrite` | `-f` | Flag | - | Overwrite existing output file if it exists

For the sake of simplicity, the table shows only a subset of the optional arguments. For an overview of all arguments, see [Command line arguments](command_line_arguments.md).

## Reformatting strategies

There are two reformatting strategies implement: 

1. **Base-wise statistics**: Calculates statistics that represents the signal assigned to a given base. 
    - This is the default strategy. Can be exlicitly set via the `--strategy stats` flag
    - One or more statistics can be specified via the `--stats <stats>...` flag (default: `mean std dwell`). 
    - Available statistics are: **Mean**/**Median**/**Stand. dev.** of the signal intensity, **dwell time** (number of measurements assigned to the base) and **signal-to-noise ratio** (mean / std. dev.)

2. **Interpolation**: Reshapes the signal for each base into a uniform number of samples using linear interpolation.
    - Can be set using the `--strategy interpolate` flag
    - The number of interpolated samples can be set via the `--target-size <target-size>` flag (default: `30`)

See [Reformatting strategies](reformatting_strategies.md) for more details and [CLI - Processing strategy](./command_line_arguments.md#processing-strategy) for all relevant flags.

## Output formats

The reformatted data can be written to compressed `parquet` or simple `TSV` format. Since TSV is uncompressed, parquet format is recommended to minimize file size. To account for different downstream processing and analyses, there are three output formats to choose from:

1. **Melted**: Long format containing one row for each base. Useful for visualization with ggplot2/seaborn.
2. **Exploded**: Wide format containing one row for each read-region pair. All values for all bases appear as separate columns. Here all regions need to have the same length. Useful for machine-learning task, e.g. clustering.
3. **Nested**: One row for each read-region pair. Fields store lists or 2D arrays. Only available for `parquet` output

See [Output formats](output_formats.md) for more details and [CLI - I/O settings](./command_line_arguments.md#inputoutput-settings) for all relevant flags.


## Examples

Usage examples are provided in [Examples](examples.md).