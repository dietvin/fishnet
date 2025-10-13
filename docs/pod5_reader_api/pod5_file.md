# Pod5File

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
