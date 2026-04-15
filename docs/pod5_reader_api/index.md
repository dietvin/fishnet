# POD5 reader API

The `POD5 reader API` provides straight-forward and efficient access to the current signal and corresponding metadata stored in pod5 files. Key features are:

- **Lazy loading** of pod5 files to enable memory-efficient reading
- **Read-wise iteration** to access a large number of reads in straight-forward manner
- **(Thread-safe) random access** to enable targeted access to single reads, optionally in parallel from multiple threads 

The key data structs are [`Pod5File`](pod5_file.md#pod5file) and [`Pod5FileThreadSafe`](pod5_file.md#pod5filethreadsafe) for **single-file access**, [`Pod5Dataset`](pod5_dataset.md#pod5dataset) and [`Pod5DatasetThreadSafe`](pod5_dataset.md#pod5datasetthreadsafe) for **multi-file access**. Reads stored in a given pod5 file are represented by the [`Pod5Read`](pod5_read.md). Follow the links for detailled information about all availalbe functions and examples. 
