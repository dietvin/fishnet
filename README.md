# Unnamed resquiggle tool (working title: Fishnet)

The goal is to provide a tool for signal-to-sequence alignment (*resquiggling*) of nanopore sequencing data. To provide fast performance the tool is written in Rust.

## Dependencies

At this point it relies on the **experimental** [pod5-rs crate](https://github.com/bsaintjo/pod5-rs) for reading POD5 files and the [rust-htslib crate](https://github.com/rust-bio/rust-htslib) for reading BAM files.

## Approach

The approach follows the implementation found in [Remora](https://github.com/nanoporetech/remora) for the most part. The source code for this can be found here:

- base-called sequence to signal: [query_to_signal](https://github.com/nanoporetech/remora/blob/0787dae2da818c49a3aaade10515b1e6df88e6bd/src/remora/io.py#L2123)
- mapped sequence to signal: [ref_to_signal](https://github.com/nanoporetech/remora/blob/0787dae2da818c49a3aaade10515b1e6df88e6bd/src/remora/io.py#L2075)

The goal is to provide alignments that are as similar as possible to the established approach, with the added benefit that the new tool is **more accessible** and **faster**. It should work via a simple command line interface:

```bash
signal-to-query -t 32 <POD5 directory> <BAM file> <output directory>
signal-to-ref -t 32 <POD5 directory> <BAM file> <output directory>
```

The user should have multiple options for the output file types. Most accessible would be a (tab-separated) text file like this:

| read_id 	| query 	| signal 	| query_to_signal 	|
|---------	|-------	|--------	|-----------------	|
| ...     	| ...   	| ...    	| ...             	|  

| read_id 	| ref_seq 	| ref_start 	| ref_end 	| signal 	| ref_to_signal 	|
|---------	|---------	|-----------	|---------	|--------	|-----------------	|
| ...     	| ...     	| ...       	| ...     	| ...    	| ...             	|

The user should be able to decide whether they want to include columns with large amounts of data (signal, query, ref_seq) to keep the file size to a minimum.

Alternatively the data could be written to a simple binary file of some more advanced and efficient (row-based(?)) file format. Generally an output file should be generated for each POD5 file provided. This would keep the site of the output files in check and enable more straight-forward multithreading. 