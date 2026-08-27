# Examples

## General examples

### Example 1: Reference-to-signal with positions of interest (no signal in input table)
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
  --output-shape melted
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
  --output-shape nested
```

**Explanation:**
- `--alignment-type query` selects the query-to-signal mappings.
- `--motifs-file` loads motifs (e.g., ATGCGT, TTTAAA, etc.) from a FASTA file.
- `--strategy interpolate 50` creates uniformly sized signal vectors (50 samples per base).
- `nested` output preserves per-base signal arrays in Parquet — ideal for machine learning input.


## Detailed (minimal) processing example
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

## Base-wise stats
```bash
fishnet reformat \
  --alignment [...] \
  --ref-regions "chr1:5-6" "chr1:12-13" \
  --strategy "stats" \
  --stats "mean" "dwell" \
  --out [...] \
  --output-shape [...]
```
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



## Interpolation
```bash
fishnet reformat \
  --alignment [...] \
  --ref-regions "chr1:5-6" "chr1:12-13" \
  --strategy "interpolate" \
  --target-size 3 \
  --out [...] \
  --output-shape [...]
```
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