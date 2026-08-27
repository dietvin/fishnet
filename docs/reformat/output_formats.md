# Output formats and shapes

Different analyses benefit from different output shapes:

| **Format**          | **Best for**                                                       | **Structure**                        |
| ------------------- | ------------------------------------------------------------------ | ------------------------------------ |
| **Melted (long)**   | Visualization (R ggplot2, Python seaborn)                          | One row per base                     |
| **Exploded (wide)** | Machine learning (clustering, dimensionality reduction, etc.)      | One row per region, columns expanded |
| **Nested**          | Hierarchical data storage (Parquet) for other downstream processes | One row per region, arrays per field |

## 1. Melted (Long)

One row for each base of interest.

### Base-wise statistics

For *N* statistics:

| **Column**                | **Description**                           |
| ------------------------- | ----------------------------------------- |
| `read_id`                 | Unique read ID                            |
| `start_index_on_read`     | Index of first base on the read (0-based) |
| `region_of_interest`      | Region name                               |
| `base_index`              | Position within region                    |
| `base`                    | Base character                            |
| `<STAT-1>`                | First computed statistics for this base   |
| ...                       | ...                                       |
| `<STAT-N>`                | Nth computed statistics for this base     |


### Interpolation

For target size *T*:

| **Column**                    | **Description**                           |
| ----------------------------- | ----------------------------------------- |
| `read_id`                     | Unique read ID                            |
| `start_index_on_read`         | Index of first base on the read (0-based) |
| `region_of_interest`          | Region name                               |
| `base_index`                  | Position within region                    |
| `base`                        | Base character                            |
| `signals`                     | The *T* interpolated signal values        |
| `dwells`                      | Dwell value for the base                  |


## 2. Exploded (Wide)

Each region-read pair becomes one row. All values for all bases appear as separate columns. 
(Requires all regions to have the same length.)

### Base-wise statistics

For regions of length *M* and *N* statistics:

| **Column**                      | **Description**               |
| ------------------------------- | ----------------------------- |
| `read_id`                       | Unique read ID                |
| `start_index_on_read`           | Index of first base (0-based) |
| `region_of_interest`            | Region name                   |
| `base_0 ... base_(M-1)`         | Bases in region               |
| `<STAT-1>_0 ... <STAT-N>_(M-1)` | Per-base statistics           |

### Interpolation

For regions of length *M* and *N* statistics:

| **Column**                                  | **Description**               |
| ------------------------------------------- | ----------------------------- |
| `read_id`                                   | Unique read ID                |
| `start_index_on_read`                       | Index of first base (0-based) |
| `region_of_interest`                        | Region name                   |
| `base_0 ... base_(M-1)`                     | Bases in region               |
| `signal_base0_0 ... signal_base(M-1)_(T-1)` | Interpolated signals          |
| `dwell_0 ... dwell_(M-1)`                   | Per-base dwell times          |




## 3. Nested (Parquet only)

Each row represents one read–region pair. Fields store lists or 2D arrays.

### Base-wise statistics

| **Column**              | **Description**                                                     |
| ----------------------- | ------------------------------------------------------------------- |
| `read_id`               | Unique read ID                                                      |
| `start_index_on_read`   | Index of first base (0-based)                                       |
| `region_of_interest`    | Region name                                                         |
| `bases`                 | Base sequence (string; length = current region lenght)              |
| `<STAT-1> ... <STAT-N>` | Lists of per-base statistic values (length = current region length) |

### Interpolation

With all regions of interest of length M and an interpolation target size of T:

| **Column**            | **Description**                                                 |
| --------------------- | --------------------------------------------------------------- |
| `read_id`             | Unique read ID                                                  |
| `start_index_on_read` | Index of first base (0-based)                                   |
| `region_of_interest`  | Region name                                                     |
| `bases`               | Base sequence (string)                                          |
| `signal`              | 2D array of shape *(M × T)* — interpolated signal for each base |
| `dwell`               | List of *M* dwell values (for each base)                        |

