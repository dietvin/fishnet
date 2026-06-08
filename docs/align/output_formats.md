# Output formats

## Available output formats
Alignments and corresponding data can be written to `parquet` and `jsonl` format.

[Parquet](https://parquet.apache.org/) format stores the data in compressed binary form. While this is not human-readable directly, the compression makes it more memory efficient and can be easily loaded for inspection or further processing using [Pandas](https://pandas.pydata.org/pandas-docs/stable/reference/api/pandas.read_parquet.html) in Python or the [Arrow R Package](https://arrow.apache.org/docs/r/reference/read_parquet.html) in R.

Alternatively, data can be written to human-readable [JSONL](https://jsonlines.org/) format. Since storing the data as human-readable strings can be quite inefficient, we recommend writing to Parquet format.

## Output data structure

The output structure depends on two settings: Which **alignment type** (`--alignment-type`) and which **output level** (`--output-level`) is chosen. 

Possible `alignment-type` options are *query* (default), *reference* and *both*. `output-level` accepts *1*, *2* (default) and *3*. 

The table below shows the columns in an output file with given settings for the alignment type (rows) and output level (columns). Bold column names are the ones that get added over the previous output level.

|  | 1 | 2 | 3 |
|---|---|---|---|
| **query** | read_id, query_to_signal | read_id, query_to_signal, **query_sequence** | read_id, query_to_signal, query_seq, **signal** |
| **reference** | read_id, ref_to_signal, ref_name, ref_start | read_id, ref_to_signal, ref_name, ref_start, **ref_sequence** | read_id, ref_to_signal, ref_name, ref_start, ref_sequence, **signal** |
| **both** | read_id, query_to_signal ref_to_signal, ref_name, ref_start | read_id, query_to_signal ref_to_signal, ref_name, ref_start, **query_sequence**, **ref_sequence** | read_id, query_to_signal ref_to_signal, ref_name, ref_start, query_sequence, ref_sequence, **signal** |

## Output data types

The inidividual columns in parquet format have the following data types in them (jsonl data gets parsed to strings):

- `read_id`: String
- `query_to_signal` / `ref_to_signal`: List of 64bit unsigned int
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

read_id | query_to_signal | query_sequence
-- | -- | --
... | ... | ...

### Minimal query-to-signal output
```bash
... --alignment-type query --output-level 1
```

read_id | query_to_signal
-- | --
... | ...

### Minimal reference-to-signal output
```bash
... --alignment-type reference --output-level 1
```

read_id | ref_to_signal | ref_name | ref_start
-- | -- | -- | --
... | ... | ... | ...

### Most comprehensive output
```bash
... --alignment-type both --output-level 3
```
read_id | query_to_signal | ref_to_signal | ref_name | ref_start | query_sequence | ref_sequence | signal
-- | -- | -- | -- | -- | -- | -- | -- 
... | ... | ... | ... | ... | ... | ... | ...