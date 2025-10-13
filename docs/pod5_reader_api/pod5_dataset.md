
# Pod5Dataset

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
