# Command line arguments
The table below show all arguments that are available in the `reformat` module.

| Long flag | Short flag | Type | Default | Description
|-|-|-|-|-
| `--help` | `-h` | Flag | - | Print help message explaining all flags. `-h` shows a more compact version than `--help`

## Required Input/Output

| Long flag | Short flag | Type | Default | Description
|-|-|-|-|-
| `--alignment <alignment>` | `-a <alignment>`| Path(file) | - | Path to `.parquet` or `.jsonl` file produced by `fishnet align`
| `--out <out>` | `-o <out>` | Path (file) | - | Output path. Extension determines format (`.parquet` or `.tsv`)

## Pod5 Input

| Long flag | Short flag | Type | Default | Description
|-|-|-|-|-
| `--pod5 <pod5>...` | `-p <pod5>...` | Path(s) (file or directory) | - | POD5 input(s). Required if alignment file lacks raw signal. Multiple allowed
| `--rna` | - | Flag | - | Set if POD5 file(s) are provided and the contained reads are direct RNA (reverse signal)

## Data filtering

**One filtering flag is required.**

| Long flag | Short flag | Type | Default | Description
|-|-|-|-|-
| `--ref-regions <ref-regions>...` | `-r <ref-regions>...` | String(s) | - | Filter input data for one or more reference region(s). Each must be in the format `<REF-NAME>:<REF-START>-<REF-END>` (Start and end are 1-based coordinates and inclusive). Multiple can be provided space-separated
| `--bed-file <bed-file>` | `-R <bed-file>` | Path (file) | - | Filter input data for reference regions from bed file. Must follow bed-style coordinate conventions (0-based, start inclusive, end exclusive)
| `--positions-of-interest <poi>...` | `-P <poi>...` | String(s) | - | Filter input data for one or more positions of interest. Each must be in the format `<REF-NAME>:<REF-SITE>-<HALF-SIZE>`, where <HALF-SIZE> determines the number of bases up- and downstream from the site that are of interest. Site coordinate is 1-based
| `--motifs <motif>...` | `-m <motif>...` | String(s) | - | Filter for motif sequence(s). Sequences must contain only 'A', 'C', 'G' and 'T'/'U'. Multiple can be provided space-separated
| `--motifs-file <motifs-file>` | `-M <motifs-file>` | Path (file) | - | Filter input data for reference regions from a FASTA file. Each motif must be a separate entry. Sequences must contain only 'A', 'C', 'G' and 'T'/'U'

## Processing strategy

| Long flag | Short flag | Type | Default | Description
|-|-|-|-|-
| `--strategy <strategy>` | `-s <strategy>` | Enum (`stats`, `interpolate`) | `stats` | Processing strategy: compute statistics or interpolate signal
| `--alignment-type <alignment-type>` | - | Enum (`reference`, `query`) | - | Only required if the alignment file contains both query- and reference-to-signal alignments. Set to determine which type will be processed
| `--skip-signal-norm` | - | Flag | - | Skip z-standardization of signal intensity
| `--skip-dwell-norm` | - | Flag | - | Skip z-standardization of dwell times

### Strategy 1: `stats`

The following flag is only regarded if `--strategy stats` is set.

| Long flag | Short flag | Type | Default | Description
|-|-|-|-|-
| `--stats <stats>...` | - | Enum(s) (`mean`, `median`, `std`, `dwell`, `signal-to-noise`) | `mean std dwell` | Statistics to compute per base when using `--strategy stats`. Multiple can be provided space-separated

### Strategy 2: `interpolate`

The following flag is only regarded if `--strategy interpolate` is set.
| Long flag | Short flag | Type | Default | Description
|-|-|-|-|-|
| `--target-size <n>` | - | Integer | `30` | Target size for interpolated signal chunks per base

## Threading settings

| Long flag | Short flag | Type | Default | Description
|-|-|-|-|-
| `--threads <n>` | `-t <n>` | Integer | `8` | Number of parallel threads
| `--queue-size <queue-size>` | - | Integer | `8000` | Queue size for worker communication. Increase for higher memory-usage and possibly faster processing


## Input/Output settings

| Long flag | Short flag | Type | Default | Description
|-|-|-|-|-
| `--input-chunk-size <n>` | - | Integer | `4000` | Number of alignments read per iteration. Larger = faster, more memory
| `--force-overwrite` | `-f` | Flag | - | Overwrite existing output file if it exists
| `--output-shape <shape>` | - | Enum (`melted`, `exploded`, `nested`) | `nested` | Determines output structure (nested only for Parquet)
| `--output-batch-size <n>` | - | Integer | `4000` | Number of processed alignments to buffer before writing to output file

## Logging settings

| Long flag | Short flag | Type | Default | Description
|-|-|-|-|-
| `--log-level <log-level>` | - | Enum (`off`, `error`, `warn`, `info`, `debug`, `trace`) | `off` | Controls verbosity of logging
| `--log-path <log-path>` | Path (file) | `log.txt` | Log file path (used if `--log-level` != `off`)