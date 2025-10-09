# Reformat module

```bash
fishnet reformat ...
```

The **reformat** module combines alignments (which contain only signal indices) with the corresponding signal values. By calculating **base-wise statistics** or **interpolating** signal segments into a uniform shape, downstream analyses of sequence-to-signal alignments become more accessible.
See [Reformatting strategies](README.md#reformatting-strategies) for details.

To keep processing efficient, reformatting is limited to **bases of interest** — those within provided **reference regions** or **motifs**.
See [Filtering options](README.md#filtering-options).

The reformatted data can be exported in *melted* (long), *exploded* (wide), or *nested* formats, each optimized for different analysis workflows.
See [Output formats and shapes](README.md#output-formats-and-shapes).

All options described below and be set using the modules command line interface. 
See [Command line arguments](README.md#command-line-arguments). 


## Reformatting strategies
Two main strategies are available:
### 1. Base-wise Statistics

<img src="/docs/images/base-wise-stats.jpg" alt="Base-wise stats overview" width="500"/>

Each signal segment aligned to a base of interest is summarized into statistics that represent its characteristics.

| **Statistic**     | **Description**                                            |
| ----------------- | ---------------------------------------------------------- |
| `mean`            | Mean signal intensity                                      |
| `median`          | Median signal intensity                                    |
| `std`             | Standard deviation of signal intensity                     |
| `dwell`           | Dwell time (number of signal samples assigned to the base) |
| `signal-to-noise` | Signal-to-noise ratio (`mean / std`)                       |

This is the default strategy, but can be enabled explicitly with `--strategy stats`. By default, `mean`, `std`, and `dwell` are calculated.
You can specify a custom subset via:
```bash
--stats <STAT-A> <STAT-B> ...
```

### 2. Interpolation into uniform shapes

<img src="/docs/images/interpolation.jpg" alt="Interpolation overview" width="500"/>

Instead of condensing each segment into statistics, the signal is reshaped into a **fixed number of samples per base** using linear interpolation.
This allows direct comparison or machine-learning-based analysis across bases.

To select this strategy, set `--strategy interpolate`. The number of samples per base can be tuned using `--target-size <NUM-SAMPLES>` (default: 30)

Upsampling or downsampling is applied as needed.

## Filtering options
Filtering limits processing to **regions or motifs of interest**, reducing both runtime and output size.

### 1. Reference regions
When working with reference-to-signal alignments, bases are processed only if they fall within specified regions. 

There are three ways to provide reference regions:
| **Flag**                                       | **Description**                                                                                                         |
| ---------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------- |
| `--ref-regions <REF:START-END>`                | Specify regions directly (1-based, inclusive). Multiple regions space-separated.                                        |
| `--bed-file <PATH>`                            | Provide regions from a BED file (0-based, start-inclusive, end-exclusive).                                              |
| `--positions-of-interest <REF:POS-HALFWINDOW>` | Define positions with flanking windows. Coordinates are 1-based. See [Positions with windows](#positions-with-windows). |


#### Positions with windows
This mode targets a specific site plus a window around it, e.g.:
```text
ChrA:8-4
```
```text
1-based index:          1 2 3 4 5 6 7 8 9 0 1 2 3 4 5
Ref sequence:           A C G T A G C T A A A G T C T
Region of interest:          |--------X--------|
```

### 2. Motifs
Motif filtering works for both query- and reference-to-signal alignments. Only alignment segments exactly matching a provided motif are processed. (Requires fishnet align output level 2 or 3 to include sequence information.)

There are two ways to provide motifs:
| **Flag**                 | **Description**                                                          |
| ------------------------ | ------------------------------------------------------------------------ |
| `--motifs <MOTIF-1> ...` | Provide motifs directly. Named automatically (`motif1`, `motif2`, …).    |
| `--motifs-file <FASTA>`  | Provide motifs via a FASTA file. Names are taken from the FASTA headers. |
All motif matches within a read are processed.

## Output formats and shapes

Different analyses benefit from different output shapes:

| **Format**          | **Best for**                                                       | **Structure**                        |
| ------------------- | ------------------------------------------------------------------ | ------------------------------------ |
| **Melted (long)**   | Visualization (R ggplot2, Python seaborn)                          | One row per base                     |
| **Exploded (wide)** | Machine learning (clustering, dimensionality reduction, etc.)      | One row per region, columns expanded |
| **Nested**          | Hierarchical data storage (Parquet) for other downstream processes | One row per region, arrays per field |

### 1. Melted (Long)
Each base of interest becomes one row.

#### Base-wise statistics
For *N* statistics:

| **Column**                | **Description**                           |
| ------------------------- | ----------------------------------------- |
| `read_id`                 | Unique read ID                            |
| `start_index_on_read`     | Index of first base on the read (0-based) |
| `region_of_interest`      | Region name                               |
| `base_index`              | Position within region                    |
| `base`                    | Base character                            |
| `<STAT-1>` ... `<STAT-N>` | Computed statistics for this base         |


#### Interpolation
For target size *T*:
| **Column**                    | **Description**                           |
| ----------------------------- | ----------------------------------------- |
| `read_id`                     | Unique read ID                            |
| `start_index_on_read`         | Index of first base on the read (0-based) |
| `region_of_interest`          | Region name                               |
| `base_index`                  | Position within region                    |
| `base`                        | Base character                            |
| `signal_0` ... `signal_(T-1)` | Interpolated signal values                |
| `dwell`                       | Dwell value for the base                  |


### 2. Exploded (Wide)
Each region–read pair becomes one row. All values for all bases appear as separate columns. 
(Requires all regions to have the same length.)

#### Base-wise statistics
For regions of length *M* and *N* statistics:
| **Column**                      | **Description**               |
| ------------------------------- | ----------------------------- |
| `read_id`                       | Unique read ID                |
| `start_index_on_read`           | Index of first base (0-based) |
| `region_of_interest`            | Region name                   |
| `base_0 ... base_(M-1)`         | Bases in region               |
| `<STAT-1>_0 ... <STAT-N>_(M-1)` | Per-base statistics           |

#### Interpolation
For regions of length *M* and *N* statistics:
| **Column**                                  | **Description**               |
| ------------------------------------------- | ----------------------------- |
| `read_id`                                   | Unique read ID                |
| `start_index_on_read`                       | Index of first base (0-based) |
| `region_of_interest`                        | Region name                   |
| `base_0 ... base_(M-1)`                     | Bases in region               |
| `signal_base0_0 ... signal_base(M-1)_(T-1)` | Interpolated signals          |
| `dwell_0 ... dwell_(M-1)`                   | Per-base dwell times          |




### 3. Nested (Parquet only)
Each row represents one read–region pair. Fields store lists or 2D arrays.

#### Base-wise statistics
| **Column**              | **Description**                                                     |
| ----------------------- | ------------------------------------------------------------------- |
| `read_id`               | Unique read ID                                                      |
| `start_index_on_read`   | Index of first base (0-based)                                       |
| `region_of_interest`    | Region name                                                         |
| `bases`                 | Base sequence (string; length = current region lenght)              |
| `<STAT-1> ... <STAT-N>` | Lists of per-base statistic values (length = current region length) |

#### Interpolation
With all regions of interest of length M and an interpolation target size of T:

| **Column**            | **Description**                                                 |
| --------------------- | --------------------------------------------------------------- |
| `read_id`             | Unique read ID                                                  |
| `start_index_on_read` | Index of first base (0-based)                                   |
| `region_of_interest`  | Region name                                                     |
| `bases`               | Base sequence (string)                                          |
| `signal`              | 2D array of shape *(M × T)* — interpolated signal for each base |
| `dwell`               | List of *M* dwell values (for each base)                        |


## Minimal processing example
The following examples shows what gets calculated and how it gets written to file with different output settings. We'll use the following example:
- reference to signal alignment of two reads:
  1. readA maps to chr1:3-8
  2. readB maps to chr1:4-14
- reference regions of interest: 
  - chr1:5-7
  - chr1:12-13
- For base-wise stats, `mean` and `dwell` are used
- For interpolation, a target size of `3` is used  

```text
0-based index:          0 1 2 3 4 5 6 7 8 9 0 1 2 3 4
1-based index:          1 2 3 4 5 6 7 8 9 0 1 2 3 4 5
Ref sequence:           A C G T|A G|C T A A A|G T|C T
                               |   |         |   |
readA:                      G T|A G|C T      |   |
readB:                        T|A G|C T A A A|G T|C
                               |   |         |   |
                               |   |         |   |
regions of interest:          chr1:5-6     chr1:12-13
```

### Base-wise stats
For the example, we'll suppose that `mean` and `dwell` are chosen for stats. Accordingly, both statistics are calculated for readA at the 5th and 6th reference base, and for readB at the 5th, 6th, 12th and 13th base.

The melted output would look like this:

| read_id | start_index_on_read | region_of_interest | base_index | base | mean  | dwell  |
|---------|---------------------|--------------------|------------|------|-------|--------|
| readA   | 2                   | chr1:5-7           | 0          | A    | mA5   | dA5    |
| readA   | 2                   | chr1:5-7           | 1          | G    | mA6   | dA6    |
| readB   | 1                   | chr1:5-7           | 0          | A    | mA5   | dA5    |
| readB   | 1                   | chr1:5-7           | 1          | G    | mA6   | dA6    |
| readB   | 8                   | chr1:12-13         | 0          | G    | mA12  | dA12   |
| readB   | 8                   | chr1:12-13         | 1          | T    | mA13  | dA13   |

The exploded format would look like this:

| read_id | start_index_on_read | region_of_interest | base_0 | base_1 | mean_0 | mean_1 | dwell_0 | dwell_1 |
|---------|---------------------|--------------------|--------|--------|--------|--------|---------|---------|
| readA   | 2                   | chr1:5-7           | A      | G      | mA5    | mA6    | dA5     | dA6     |
| readB   | 1                   | chr1:5-7           | A      | G      | mB5    | mB6    | dB5     | dB6     |
| readB   | 8                   | chr1:12-13         | G      | T      | mB12   | mB13   | dB12    | dB13    |

The nested format would look like this:

| read_id | start_index_on_read | region_of_interest | bases | mean         | dwell        |
|---------|---------------------|--------------------|-------|--------------|--------------|
| readA   | 2                   | chr1:5-7           | AG    | [mA5, mA6]   | [dA5, dA6]   |
| readB   | 1                   | chr1:5-7           | AG    | [mB5, mB6]   | [dB5, dB6]   |
| readB   | 8                   | chr1:12-13         | GT    | [mB12, mB13] | [dB12, dB13] |



### Interpolation
For the example, we'll suppose that interpolation was performed with a target size of `3`. This results in the interpolated signal for readA at the 5th and 6th base, and for readB at the 5th, 6th, 12th and 13th reference base.

Here is a diagram to show what the data would look like:
```text
Raw per-base signal chunks (variable lengths):

  readA
    base 5 →  [ . . . . . ]                   (5 measurements)
    base 6 →  [ . . . . . . . . . . . ]       (11 measurements)

  readB
    base 5  → [ . . . . ]                     (4 measurements)
    base 6  → [ . . . . . . . . . . . . . ]   (13 measurements)
    base 12 → [ . . . . . . . . . . ]         (10 measurements)
    base 13 → [ . . . . . . . ]               (7 measurements)


After interpolation to target size = 3:

  readA
    base 5  → [ sA5_0  sA5_1  sA5_2 ]         (3 measurements)
    base 6  → [ sA6_0  sA6_1  sA6_2 ]         (3 measurements)

  readB
    base 5  → [ sB5_0  sB5_1  sB5_2 ]         (3 measurements)
    base 6  → [ sB6_0  sB6_1  sB6_2 ]         (3 measurements)
    base 12 → [ sB12_0 sB12_1 sB12_2 ]        (3 measurements)
    base 13 → [ sB13_0 sB13_1 sB13_2 ]        (3 measurements)
```

The melted output would look like this:

| read_id | start_index_on_read | region_of_interest | base_index | base | signal_0   | signal_1  | signal_2  | dwell |
|---------|---------------------|--------------------|------------|------|------------|-----------|-----------|-------|
| readA   | 2                   | chr1:5-7           | 0          | A    | sA5_0      | sA5_1     | sA5_2     | dA5   |
| readA   | 2                   | chr1:5-7           | 1          | G    | sA6_0      | sA6_1     | sA6_2     | dA6   |
| readB   | 1                   | chr1:5-7           | 0          | A    | sB5_0      | sB5_1     | sB5_2     | dB5   |
| readB   | 1                   | chr1:5-7           | 1          | G    | sB6_0      | sB6_1     | sB6_2     | dB6   |
| readB   | 8                   | chr1:12-13         | 0          | G    | sB12_0     | sB12_1    | sB12_2    | dB12  |
| readB   | 8                   | chr1:12-13         | 1          | T    | sB13_0     | sB13_1    | sB13_2    | dB13  |

The exploded format would look like this:

| read_id | start_index_on_read | region_of_interest | base_0 | base_1 | signal_base0_0 | signal_base0_1 | signal_base0_2 | signal_base1_0 | signal_base1_1 | signal_base1_2 | dwell_0 | dwell_1 |
|---------|---------------------|--------------------|--------|--------|----------------|----------------|----------------|----------------|----------------|----------------|---------|---------|
| readA   | 2                   | chr1:5-7           | A      | G      | sA5_0          | sA5_1          | sA5_2          | sA6_0          | sA6_1          | sA6_2          | dA5     |  dA6    |
| readB   | 1                   | chr1:5-7           | A      | G      | sB5_0          | sB5_1          | sB5_2          | sB6_0          | sB6_1          | sB6_2          | dB5     |  dB6    |
| readB   | 8                   | chr1:12-13         | G      | T      | sB12_0         | sB12_1         | sB12_2         | sB13_0         | sB13_1         | sB13_2         | dB12    |  dB13   |

The nested format would look like this:

| read_id | start_index_on_read | region_of_interest | bases | signal                                                | dwell        |
|---------|---------------------|--------------------|-------|-------------------------------------------------------|--------------|
| readA   | 2                   | chr1:5-7           | AG    | \[[sA5_0, sA5_1, sA5_2], [sA6_0, sA6_1, sA6_2]]       | [dA5, dA6]   |
| readB   | 1                   | chr1:5-7           | AG    | \[[sB5_0, sB5_1, sB5_2], [sB6_0, sB6_1, sB6_2]]       | [dB5, dB6]   |
| readB   | 8                   | chr1:12-13         | GT    | \[[sB12_0, sB12_1, sB12_2], [sB13_0, sB13_1, sB13_2]] | [dB12, dB13] |


## Command line arguments

| **Argument**                                             | **Type**                                                      | **Default**      | **Description**                                                               |
| -------------------------------------------------------- | ------------------------------------------------------------- | ---------------- | ----------------------------------------------------------------------------- |
| `-h, --help`                                             | Flag                                                          | –                | Print help message.                                                           |
| **Required Input/Output**                                |                                                               |                  |                                                                               |
| `-a, --alignment <alignment>`                            | Path (file)                                                   | –                | Path to `.parquet` or `.jsonl` file produced by `fishnet align`.              |
| `-o, --out <out>`                                        | Path (file)                                                   | –                | Output path. Extension determines format (`.parquet` or `.tsv`).              |
| **Pod5 Input**                                           |                                                               |                  |                                                                               |
| `-p, --pod5 <pod5>...`                                   | Path(s) (file or directory)                                   | –                | POD5 input(s). Required if alignment file lacks raw signal. Multiple allowed. |
| `--rna`                                                  | Flag                                                          | –                | Set if direct RNA POD5 file(s) are provided (reverse signal).                 |
| **Data Filter (one required)**                           |                                                               |                  |                                                                               |
| `-r, --ref-regions <ref-regions>...`                     | String(s)                                                     | –                | Filter by reference region(s) (`<REF>:<START>-<END>`). 1-based inclusive.     |
| `-R, --bed-file <bed-file>`                              | Path (file)                                                   | –                | BED file with reference regions (0-based start, exclusive end).               |
| `-P, --positions-of-interest <positions-of-interest>...` | String(s)                                                     | –                | Filter by positions of interest (`<REF>:<SITE>-<HALF-SIZE>`).                 |
| `-m, --motifs <motifs>...`                               | String(s)                                                     | –                | Filter for motif sequence(s) (A/C/G/T/U). Multiple allowed.                   |
| `-M, --motifs-file <motifs-file>`                        | Path (file)                                                   | –                | File with one motif per line (A/C/G/T/U only).                                |
| **Processing Strategy**                                  |                                                               |                  |                                                                               |
| `-s, --strategy <strategy>`                              | Enum (`stats`, `interpolate`)                                 | `stats`          | Processing strategy: compute statistics or interpolate signal.                |
| `--alignment-type <alignment-type>`                      | Enum (`reference`, `query`)                                   | –                | Select which alignment type to parse.                                         |
| `--skip-signal-norm`                                     | Flag                                                          | –                | Skip z-standardization of signal intensity.                                   |
| `--skip-dwell-norm`                                      | Flag                                                          | –                | Skip z-standardization of dwell times.                                        |
| **Strategy 1: Statistics Settings**                      |                                                               |                  |                                                                               |
| `--stats <stats>...`                                     | Enum(s) (`mean`, `median`, `std`, `dwell`, `signal-to-noise`) | `mean std dwell` | Statistics to compute per base when using `--strategy stats`.                 |
| **Strategy 2: Interpolation Settings**                   |                                                               |                  |                                                                               |
| `--target-size <target-size>`                            | Integer                                                       | `30`             | Target size for interpolated signal chunks per base.                          |
| **Threading Settings**                                   |                                                               |                  |                                                                               |
| `-t, --threads <threads>`                                | Integer                                                       | `8`              | Number of parallel threads (set to `1` for single-threaded).                  |
| `--queue-size <queue-size>`                              | Integer                                                       | `1000`           | Queue size for worker communication (affects memory use).                     |
| **Input/Output Settings**                                |                                                               |                  |                                                                               |
| `--input-chunk-size <input-chunk-size>`                  | Integer                                                       | `4000`           | Number of alignments read per iteration. Larger = faster, more memory.        |
| `-f, --force-overwrite`                                  | Flag                                                          | –                | Overwrite existing output file if it exists.                                  |
| `--output-shape <output-shape>`                          | Enum (`melted`, `exploded`, `nested`)                         | `nested`         | Determines output structure (nested only for Parquet).                        |
| `--output-batch-size <output-batch-size>`                | Integer                                                       | `4000`           | Number of alignments buffered before writing output.                          |
| **Logging Settings**                                     |                                                               |                  |                                                                               |
| `--log-level <log-level>`                                | Enum (`off`, `error`, `warn`, `info`, `debug`, `trace`)       | `off`            | Controls verbosity of logging.                                                |
| `--log-path <log-path>`                                  | Path (file)                                                   | `log.txt`        | Log file path (used if `--log-level` ≠ `off`).                                |


## Usage Examples
### Example 1: Reference-to-signal with positions of interest (no signal in table)
In this example, the alignment file does **not** contain raw signal data, so the corresponding POD5 input must be provided.
We extract the mean, standard deviation, and dwell time of the signal around given reference positions and output a **melted TSV**.
```bash
fishnet reformat \
  --alignment alignments_ref.parquet \
  --pod5 /data/pod5_runs/run1 /data/pod5_runs/run2 \
  --positions-of-interest chr1:100000-10 chr2:250000-15 \
  --strategy stats \
  --stats mean std dwell \
  --out ref_positions_stats.tsv \
  --output-shape melted \
  --threads 8 \
  --force-overwrite
```

**Explanation:**
- `--alignment` provides reference-to-signal mappings.
- `--pod5` supplies raw signal data (since it’s missing in the alignment file).
- `--positions-of-interest` defines windows around base positions (±10 and ±15 bases).
- The `stats` strategy calculates per-base signal statistics.
- Output is written as a **melted TSV table**, one row per base.

### Example 2: Query-to-signal with motif filtering and interpolation
Here, the alignment file **already contains raw signal** and includes **both reference and query alignments**.
We select the **query alignment**, filter by motifs from a FASTA file, and interpolate the signal to a uniform length of 50.
The result is stored as a **nested Parquet file**.

```bash
fishnet reformat \
  --alignment alignments_query_signal.parquet \
  --alignment-type query \
  --motifs-file motifs.fasta \
  --strategy interpolate \
  --target-size 50 \
  --out interpolated_query_signal.parquet \
  --output-shape nested \
  --threads 8 \
  --force-overwrite
```

**Explanation:**
- `--alignment-type query` selects the query-to-signal mappings.
- `--motifs-file` loads motifs (e.g., ATGCGT, TTTAAA, etc.) from a FASTA file.
- `--strategy interpolate 50` creates uniformly sized signal vectors (50 samples per base).
- `nested` output preserves per-base signal arrays in Parquet — ideal for machine learning input.