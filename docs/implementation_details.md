# Implementation details

This page provides a overview of the general code structure. It lists all modules along with short explanations. Detailed descriptions of each function are provided in the scripts. 

## Fishnet

Fishnet is structured in the following modules:

- [cli](../src/cli.rs): Handles the command line parsing, the execution and output writing.

    - [parse](../src/cli/parse.rs): Parses the command line flags to usable parameters.

    - [execute](../src/cli/execute.rs): Functions as the entry point to the alignment process. Contains scripts to execute the process in single- and multithreaded.
    
    - [output](../src/cli/output.rs): Handles writing the alignments to file. Contains scripts to write the data to `parquet` or `jsonl`.

- [core](../src/core.rs): Contains all logic for loading bam and pod5 files (`loader`), performing an initial alignment (`alignment`) and the refinement of this alignment (`refinement`).
    - [loader](../src/core/loader.rs): Handles bam and pod5 file loading. 

    - [alignment](../src/core/alignment.rs): Handles the initial query/reference to signal alignment.

    - [refinement](../src/core/refinement.rs): Handles the refinement of the inital alignment.

        - [kmer_table](../src/core/refinement/kmer_table.rs): Provides functionality for loading, storing, and querying k-mers with their associated level values. 

        - [settings](../src/core/refinement/settings.rs): Contains logic to set up valid parameter combinations for the refinement.

        - [signal_map_refiner](../src/core/refinement/signal_map_refiner.rs): Provides functionality for refining the alignment between raw nanopore signal data and DNA/RNA sequences. It handles the process of improving initial alignments through iterative refinement and rescaling operations.

        - [refinement_core](../src/core/refinement/refinement_core.rs): Contains the core logic for the refinement, including the band calculation and the banded dynamic programming algorithm.

            - [bands](../src/core/refinement/refinement_core/bands.rs): Provides structures and algorithms for creating and managing bands that constrain the search space during dynamic programming operations. Bands are used to reduce computational complexity by limiting the range of valid alignments between signal measurements and sequence bases.

            - [dp_algorithm](../src/core/refinement/refinement_core/dp_algorithm.rs): Implements banded dynamic programming algorithms for optimal sequence-to-signal alignment. It provides efficient path finding through a constrained search space defined by alignment bands.

- [logger](../src/logger.rs): Provides logging utilities using the `log4rs` crate to configure flexible, file-based logging for Rust applications.

- [error](../src/error.rs): Contains the custom error types implemented in the other modules and allow proper error handling. Custom error types are implemented for the command line interface (`cli_errors`), the bam/pod5 loading (`loader_errors`), the alignment (`alignment_errors`), the refinement (`refinement_errors`) and the output writing (`output_errors`).If an error occurs it is caught in the main execution function and handled according to its severity.
    - [cli_errors](../src/error/cli_errors.rs)
    - [loader_errors](../src/error/loader_errors.rs)
    - [alignment_errors](../src/error/alignment_errors.rs)
    - [refinement_errors](../src/error/refinement_errors.rs)
    - [output_errors](../src/error/output_errors.rs)

## Pod5 reader API

Fishnet initially used the pod5 API from the [pod5-rs](https://github.com/bsaintjo/pod5-rs) crate. This implementation has the major limitation that at the time of writing, it does not support lazy loading of individual reads. As such, working with larger pod5 file (multiple GB) frequently leads to crashes when the machine runs out of memory.

To enable working with large Pod5 files directly, Fishnet uses a [custom pod5 Reader API](../pod5_reader_api/README.md) that implements a more efficient data loading approach. Here, since the signal measurements make up by far the biggest chunk of a pod5 file, only the metadata is read into memory initially. The signal data is only accessed and decompressed once a reads gets requested (either through random access by the read id or when iterating through a file). The decompression functionalities are taken from the [pod5-rs crate](https://github.com/bsaintjo/pod5-rs/blob/main/svb16/src/lib.rs).

The implementation is bare-bones at the moment, but it works as a stand in for Fishnet.