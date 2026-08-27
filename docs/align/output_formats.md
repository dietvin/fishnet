# Output formats


## Available output formats
Alignments and corresponding data can be written to `parquet` and `jsonl` format.

[Parquet](https://parquet.apache.org/) format stores the data in compressed binary form. While this is not human-readable directly, the compression makes it more memory efficient and can be easily loaded for inspection or further processing using [Pandas](https://pandas.pydata.org/pandas-docs/stable/reference/api/pandas.read_parquet.html) in Python or the [Arrow R Package](https://arrow.apache.org/docs/r/reference/read_parquet.html) in R.

Alternatively, data can be written to human-readable [JSONL](https://jsonlines.org/) format. Since storing the data as human-readable strings can be quite inefficient, we recommend writing to Parquet format.


## Core output data

Each row (Parquet) / entry (jsonl) corresponds to the data for one read. 

Which alignment type(s) get generated depends on the `--alignment-type` flag. Possible options are *query* (only query-to-signal; **default**), *reference* (only reference-to-signal) and *both* (query- **and** reference-to-signal)

Depending on the selection, the minimal output dataset can contain the following data:

### Read identifyer

- `read_id`: ID for the read at hand; This column is always present

### Query-to-signal info

When query-to-signal alignments are generated, the following colums are always present:

- `query_to_signal`: The alignment boundaries
- `query_shift` + `query_scale`: Signal normalization parameters ($signal_{norm}=\frac{signal - shift}{scale}$, where signal refers to the DACs stored in a POD5 read)

### Reference-to-signal info

When reference-to-signal alignments are generated, the following colums are always present:

- `ref_to_signal`: The alignment boundaries
- `ref_shift` + `ref_scale`: Signal normalization parameters (see above)
- `ref_name`: The sequence name a read mapped to
- `ref_start`: The start coordinate of the mapping (X-indexed)


## Additional output options

The `--output-level` determines which additional data gets written. Valid options are:

- `1`: Only the core data gets written
- `2`: The **(query and/or reference) sequence(s)** get written to file as well
- `3`: The sequence(s) and the **normalized signal** get written to file as well

The table below shows the columns in an output file with given settings for the alignment type (rows) and output level (columns). Bold column names are the ones that get added over the previous output level.

|  | 1 | 2 | 3 |
|---|---|---|---|
| **query** | read_id<br>query_to_signal<br>query_shift<br>query_scale | read_id<br>query_to_signal<br>query_shift<br>query_scale<br>**query_sequence** | read_id<br>query_to_signal<br>query_shift<br>query_scale<br>query_seq<br>**signal** |
| **reference** | read_id<br>ref_to_signal<br>ref_shift<br>ref_scale<br>ref_name<br>ref_start | read_id<br>ref_to_signal<br>ref_shift<br>ref_scale<br>ref_name<br>ref_start<br>**ref_sequence** | read_id<br>ref_to_signal<br>ref_shift<br>ref_scale<br>ref_name<br>ref_start<br>ref_sequence<br>**signal** |
| **both** | read_id<br>query_to_signal<br>query_shift<br>query_scale ref_to_signal<br>ref_shift<br>ref_scale<br>ref_name<br>ref_start | read_id<br>query_to_signal<br>query_shift<br>query_scale ref_to_signal<br>ref_shift<br>ref_scale<br>ref_name<br>ref_start<br>**query_sequence**<br>**ref_sequence** | read_id<br>query_to_signal<br>query_shift<br>query_scale ref_to_signal<br>ref_shift<br>ref_scale<br>ref_name<br>ref_start<br>query_sequence<br>ref_sequence<br>**signal** |

**Note**: When both alignment types are generated (`--alignment-type both`) and the signal gets exported (`--output-level 3`), the signal that was normalized using the reference normalization parameters gets written to file

## Output data types

The inidividual columns in parquet format have the following data types in them (jsonl data gets parsed to strings):

- `read_id`: String
- `query_to_signal` / `ref_to_signal`: List of 64bit unsigned int
- `query_shift` / `query_scale` / `ref_shift` / `ref_scale`: 32bit float
- `ref_name`: String
- `ref_start`: 64bit unsigned int
- `query_sequence` / `ref_sequence`: String
- `signal`: List of 16bit signed int


## Concrete examples

Below are some examples that show the output structure more explicitly with the corresponding flags.

### Default
```bash
... --alignment-type query --output-level 2
```

read_id | query_to_signal | query_shift | query_shift | query_sequence
-- | -- | -- | -- | --
... | ... | ... | ... | ...

### Minimal query-to-signal output
```bash
... --alignment-type query --output-level 1
```

read_id | query_to_signal | query_shift | query_shift
-- | -- | -- | --
... | ... | ... | ...

### Minimal reference-to-signal output
```bash
... --alignment-type reference --output-level 1
```

read_id | ref_to_signal | ref_shift | ref_shift | ref_name | ref_start
-- | -- | -- | -- | -- | --
... | ... | ... | ... | ... | ...

### Most comprehensive output
```bash
... --alignment-type both --output-level 3
```
read_id | query_to_signal | query_shift | query_shift | ref_to_signal | ref_shift | ref_shift | ref_name | ref_start | query_sequence | ref_sequence | signal
-- | -- | -- | -- | -- | -- | -- | -- | -- | -- | -- | -- 
... | ... | ... | ... | ... | ... | ... | ... | ... | ... | ... | ...