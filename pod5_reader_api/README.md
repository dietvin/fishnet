# POD5 Reader API

A library for reading POD5 files. This library provides straight-forward and efficient access to the current signal and corresponding metadata stored in pod5 files. Key features are:
- **Lazy loading** of pod5 files to enable memory-efficient reading
- **Read-wise iteration** to access a large number of reads in straight-forward manner
- **(Thread-safe) random access** to enable targeted access to single reads, optionally in parallel from multiple threads 

The key data structs are `Pod5File` and `Pod5FileThreadSafe` for single-file acces, `Pod5Dataset` and `Pod5DatasetThreadSafe` for multi-file access. Reads stored in a given pod5 file are represented by the `Pod5Read`. 

## Pod5Read
The `Pod5Read` struct grants access to the signal and metadata corresponding to a single read, all of which can be accessed by various getter functions. A read can not be manually initialized, but needs to be retrieved from a pod5 file or dataset.

The following table shows all fields that are accessible from a Pod5Read. The descriptions are taken mostly from the [official pod5 docs](https://github.com/nanoporetech/pod5-file-format/blob/master/docs/tables/reads.toml).
| Field | Description |
|-|-|
| read_id | Globally-unique identifier for the read, can be converted to a string form (using standard routines in other libraries) which matches how reads are identified elsewhere |
| signal | The actual signal |
| signal_indices | A list of zero-indexed row numbers in the Signal table. This must be all the rows in the Signal table that have a matching read_id, in order. It functions as an index for the Signal table |
| read_number | The read number on channel. This is increasing but typically not necessarily consecutive |
| start | How many samples were taken on this channel before the read started (since the data acquisition period began). This can be combined with the sample rate to get a time in seconds for the start of the read relative to the start of data acquisition |
| median_before | "The level of current in the well before this read (typically the open pore level of the well). If the level is not known (eg: due to a mux change), this should be nulled out |
| num_minknow_events | Number of minknow events that the read contains |
| tracked_scaling | Collects tracked_scaling_shift (Shift for tracked read scaling values (based on previous reads shift)) and tracked_scaling_scale (Scale for tracked read scaling values (based on previous reads shift)) |
| predicted_scaling | Collects predicted_scaling_shift (Shift for predicted read scaling values (based on this read's raw signal)) and predicted_scaling_scale (Scale for predicted read scaling values (based on this read's raw signal)) |
| num_reads_since_mux_change | Number of selected reads since the last mux change on this reads channel |
| time_since_mux_change | Time in seconds since the last mux change on this reads channel |
| num_samples | The full length of the signal for this read in samples (equal to the sum of all 'samples' fields of signal chunks) |
| pore | Collects the pore_type (Name of the pore type present in the well), channel (1-indexed channel) and well (1-indexed well (typically 1, 2, 3 or 4)) information |
| calibration | Collects calibration_offset (Calibration offset used to scale raw ADC data into pA readings) and calibration_scale (Calibration scale factor used to scale raw ADC data into pA readings) |
| end_reason | Collects end_reason (The end reason, currently one of: unknown, mux_change, unblock_mux_change, data_service_unblock_mux_change, signal_positive, signal_negative, api_request, device_data_error, analysis_config_change or paused) and end_reason_forced (True if this read was ended 'forcibly' (eg: mux_change, unblock), false if it was a data-driven read break (signal_positive, signal_negative). This allows simple categorisation even in the presence of new reasons that reading code is unaware of) info |
| run_info_id | Id that matches the acquisition_id in the run info stored for the overarching Pod5File | 

The user has the choice to call the standard getter function (e.g. `Pod5Read::read_id()`), which wraps the value in an option, or the *require* function (e.g. `Pod5Read::require_read_id()`) which wraps the value in a Result. All information should be present for a standard read, but the internal arrow schema lists the fields for the data as optional, so they can technically be missing. That's why the values are not directly available.


## Pod5File

The `Pod5File` struct handles access to a single pod5 file. I provides straight-forward iteration over contained reads via the `Pod5File::iter_reads` function, as well as random access to contained reads by the read-id. It implements the following functions:

| Function | Description |
|---|---|
| new | Initializes a new Pod5File from a path to a pod5 file  |
| footer | Returns the footer of a given file |
| get | Returns the read information behind the given read id |
| iter_reads | Iterate efficiently over each read in the file |
| n_reads | Returns the number of reads in the file |
| path | Returns the path it was initialized with |
| read_ids | Returns the read ids contained in the file |
| run_info | Contains metadata that is shared for all reads in the file |

The following code snippet shows an example on how to iterate over all reads in a given file, printing the read id and signal for each:
```rust
use std::path::PathBuf;
use pod5_reader_api::file::Pod5File;

fn main() {
    let path = PathBuf::from("example_data/can_reads.pod5");
    let mut pod5_file = Pod5File::new(&path).unwrap();
    let file_iterator = pod5_file.iter_reads().unwrap();

    for read_res in file_iterator {
        let read = read_res.unwrap();

        let read_id = read.read_id();
        let signal = read.require_signal().unwrap();

        println!("{} | {:?}", read_id, signal);
    }
}
```

Random access to a specific read is provided via the `Pod5File::get` function, which tries to retrieve a read by its unique id. The following code snippet show how to access read `fbf9c81c-fdb2-4b41-85e1-0a2bd8b5a138` by its id:
```rust
use std::{path::PathBuf, str::FromStr};
use pod5_reader_api::file::Pod5File;
use uuid::Uuid;

fn main() {
    let path = PathBuf::from("example_data/can_reads.pod5");
    let read_id = Uuid::from_str("fbf9c81c-fdb2-4b41-85e1-0a2bd8b5a138").unwrap();

    let mut pod5_file = Pod5File::new(&path).unwrap();

    let read = pod5_file.get(&read_id).unwrap();
    let read_id_from_read = read.read_id();
    let num_samples = read.require_num_samples().unwrap();
    println!("{} | {} | {}", read_id, read_id_from_read, num_samples);
}
```

Since both the `iter_reads` and the `get` functions rely on a mutable reference to `Pod5File`, thread-safe access is not given. This is the result of the lazy-loading approach, where the signal of a given read is only read when the read is requested. **For multi-threaded access, use `Pod5FileThreadSafe`**.

### Pod5FileThreadSafe
The `Pod5FileThreadSafe` functions like `Pod5File` with the key difference that it allows for random access to contained reads from multiple threads in parallel. The `iter_reads` function is the only one that is not implemented here. All other functions are the same.

The following example shows how `Pod5FileThreadSafe` can be used to read reads in parallel:
```rust
use std::path::PathBuf;
use std::sync::Arc;
use pod5_reader_api::file::Pod5FileThreadSafe;
use rayon::current_thread_index;
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use uuid::Uuid;

fn main() {
    let path = PathBuf::from("../example_data/can_reads.pod5");
    let n_workers = 4;

    let pod5_file = Arc::new(
        Pod5FileThreadSafe::new(&path, n_workers).unwrap()
    );
    let read_ids: Vec<Uuid> = pod5_file.read_ids().clone();

    read_ids.par_iter().for_each(|read_id| {
        let pod5_file = Arc::clone(&pod5_file);
        let tid = current_thread_index().unwrap();

        let read = pod5_file.get(read_id).unwrap();
        println!(
            "Thread {} processed read {} with {} samples",
            tid,
            read.read_id(),
            read.require_num_samples().unwrap()
        );
    });
}
```

## Pod5Dataset

The `Pod5Dataset` handles access to multiple files at a time. It allows for random access to reads from any contained file and file-wise iteration. It implements the following functions:

| Function | Description |
|---|---|
| new | Initializes a new Pod5Dataset from multiple pod5 paths |
| files | Returns references to all contained pod5 files (&PodFile) |
| get_file | Returns a reference to a specific Pod5File by its path used during initialization |
| get_file_mut | Returns a mutable reference to a specific Pod5File by its path used during initialization |
| get_file_by_index | Returns a reference to a specific Pod5File by its index in the path vector during initialization |
| get_file_by_index_mut | Returns a mutable reference to a specific Pod5File by its index in the path vector during initialization |
| get_read | Returns a read from any file in the dataset by its id |
| iter_files | Returns an iterator over references to all Pod5Files in the dataset |
| iter_files_mut | Returns an iterator over mutable references to all Pod5Files in the dataset |
| n_files | Returns the number of files contained in the dataset |
| n_reads | Returns the number of reads over all files in the dataset |

**TODO: Implement read_ids function**

The following example shows how to iterate over all reads of a dataset:
```rust
use std::path::PathBuf;
use pod5_reader_api::dataset::Pod5Dataset;

fn main() {
    let paths = vec![
        PathBuf::from("example_data/can_reads.pod5"),
        // ...
    ];

    let mut pod5_dataset = Pod5Dataset::new(&paths).unwrap();

    for file in pod5_dataset.iter_files_mut() {
        for read_res in file.iter_reads().unwrap() {
            let read = read_res.unwrap();
            println!("{}", read.read_id());
        }
    }
}
```

Contained `Pod5File`s are accessible via the `get_file`, `get_file_mut`, `get_file_by_index` and `get_file_by_index_mut` functions. Alternatively, read information is directly accessible via the `get_read` and `get_read_mut` functions. The following example shows how to use the latter:
```rust
use std::{path::PathBuf, str::FromStr};
use pod5_reader_api::dataset::Pod5Dataset;
use uuid::Uuid;

fn main() {
    let paths = vec![
        PathBuf::from("example_data/can_reads.pod5"),
        // ...
    ];

    let mut pod5_dataset = Pod5Dataset::new(&paths).unwrap();
    let read_id = Uuid::from_str("fbf9c81c-fdb2-4b41-85e1-0a2bd8b5a138").unwrap();

    let pod5_read = pod5_dataset.get_read(&read_id).unwrap();
    println!("{}", pod5_read.read_id());
    
    // Alternatively the same, but more complicated:
    let pod5_file = pod5_dataset.get_file_by_index_mut(0).unwrap();
    let pod5_read = pod5_file.get(&read_id).unwrap();
    println!("{}", pod5_read.read_id());
}
```

Just like with the `Pod5File`, retrieving read information requires mutable access, and is not thread-safe. Again, **thread-safe access is provided by `Pod5DatasetThreadSafe`**.


## Pod5DatasetThreadSafe
The `Pod5DatasetThreadSafe` functions like `Pod5Dataset` with the key difference that it allows for random access to contained reads from multiple threads in parallel. Key differences are that the functions that retrieve mutable references to contained files are not available here. Other functions that are exclusive here are the following:

| Function | Description |
|---|---|
| get_file_thread_safe | Returns a Pod5FileThreadSafe by its path used during initialization |
| get_file_thread_safe_by_index | Returns a Pod5FileThreadSafe by its index in the path vector during initialization |

Note that all file getter functions (`get_file`, `get_file_by_index`, `get_file_thread_safe`, `get_file_thread_safe_by_index`) construct the file from scratch in the current implementation. As such is pretty inefficient.

The key usage for `Pod5DatasetThreadSafe` is direct access to contained reads from multiple threads in parallel. The following example shows an approach to do just that:
```rust
use std::path::PathBuf;
use std::sync::Arc;
use pod5_reader_api::dataset::Pod5DatasetThreadSafe;
use rayon::current_thread_index;
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use uuid::Uuid;

fn main() {
    let paths = vec![
        PathBuf::from("example_data/can_reads.pod5"),
        // ...
    ];
    let n_workers = 4;

    let pod5_dataset = Arc::new(
        Pod5DatasetThreadSafe::new(&paths, n_workers).unwrap()
    );
    let read_ids: Vec<Uuid> = pod5_dataset.read_ids().clone();

    read_ids.par_iter().for_each(|read_id| {
        let pod5_dataset = Arc::clone(&pod5_dataset);
        let tid = current_thread_index().unwrap();

        let read = pod5_dataset.get_read(read_id).unwrap();
        println!(
            "Thread {} processed read {} with {} samples",
            tid,
            read.read_id(),
            read.require_num_samples().unwrap()
        );
    });
}
``` 

## Pod5Dataset vs Pod5DatasetThreadSafe
The ThreadSafe implementations of Pod5File and Pod5Dataset should only be used when processing data in parallel. All linear operations more efficient when using the non-thread-safe implementations due to less overhead and a much simpler implementation. 

To showcase the differences in processing speed I set up a quick and dirty benchmark when handling 25GB of pod5 data.

The following approaches were tested:
- Random access with Pod5DatasetThreadSafe - 20 threads
- Random access with Pod5DatasetThreadSafe - 8 threads
- Random access with Pod5DatasetThreadSafe - 1 thread
- Random access with Pod5Dataset
- Read-wise iterator with Pod5Dataset

The data was split into a different number of files to test if *fewer but larger*, or *more but smaller* files are more or less efficient for reading:
- 25GB split into 3 files
- 25GB split into 28 files
- 25GB split into 2746 files

In all runs, each read was accessed once. Due to the internal caching of readers for different files, access in a truly random order is slower. To test how much slower, reads were accessed in both random and non-random order.

Here are the times that were measured using the `time` command in bash:

| Approach | 3 files<br>non-random | 28 files<br>non-random | 2746 files<br>non-random | 3 files<br>random | 28 files<br>random | 2746 files<br>random
|-|-|-|-|-|-|-|
| thread-safe, 20 threads | 00:31,9 | 00:17,6 | 00:18,3 | 01:06,5 | 01:10,2 | 01:08,2 |
| thread-safe, 8 threads | 00:34,0 | 00:33,0 | 00:29,8 | 01:07,8 | 01:10,3 | 01:06,8 |
| thread-safe, 1 thread | 03:31,6 | 03:19,7 | 03:18,2 | 07:25,1 | 05:58,1 | 05:23,3 |
| Non thread-safe, random access | 03:14,1 | 03:11,7 | 03:11,3 | NA | NA | NA |
| Non thread-safe, iterative | 01:28,6 | 01:27,1 | 01:28,8 | NA | NA | NA |


## Error Handling

The API provides error handling through a couple custom error types:

- **Pod5DatasetError**: Dataset-level errors (invalid keys, index out of bounds)
- **Pod5FileError**: File-level errors (IO issues, invalid signatures, parsing failures)
- **Pod5ReadError**: Read-level errors (missing required fields)

All error types implement the `thiserror::Error` trait for excellent error messages and debugging support.

## Technical details 


## Performance considerations
