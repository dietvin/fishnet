# Output formats

The output structure depends on two settings: Which alignment type (`--alignment-type`) and which out level (`--output-level`) is chosen. Possible are *query* (default), *reference* and *both* for `alignment-type` and *1* (default), *2* and *3* for `output-level`. 

The table below shows all columns in an output file with given settings for the alignment type (rows) and output level (columns). Bold column names are the ones that get added over the previous output level. 

|  | 1 | 2 | 3 |
|---|---|---|---|
| **query** | read_id, query_to_signal | read_id, query_to_signal, **query_sequence** | read_id, query_to_signal, query_seq, **signal** |
| **reference** | read_id, ref_to_signal, ref_name, ref_start | read_id, ref_to_signal, ref_name, ref_start, **ref_sequence** | read_id, ref_to_signal, ref_name, ref_start, ref_sequence, **signal** |
| **both** | read_id, query_to_signal ref_to_signal, ref_name, ref_start | read_id, query_to_signal ref_to_signal, ref_name, ref_start, **query_sequence**, **ref_sequence** | read_id, query_to_signal ref_to_signal, ref_name, ref_start, query_sequence, ref_sequence, **signal** |

The inidividual columns have the following data types in them:
- `read_id`: String
- `query_to_signal` / `ref_to_signal`: List of 64bit unsigned int
- `ref_name`: String
- `ref_start`: 64bit unsigned int
- `query_sequence` / `ref_sequence`: String
- `signal`: List of 16bit signed int