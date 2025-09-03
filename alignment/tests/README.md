# Integration testing

For integration testing we extracted input and output data from intermediate functions from the original Remora implementation, called the corresponding Rust functions with the input data and compared the Rust output with the expected output.

## Test data

We used some of the test data provided by Remora: 
- [can_mappings.bam](https://github.com/nanoporetech/remora/blob/master/tests/data/can_mappings.bam) 
- [can_reads.pod5](https://github.com/nanoporetech/remora/blob/master/tests/data/can_reads.pod5) 
- [levels.txt](https://github.com/nanoporetech/remora/blob/master/tests/data/levels.txt)

### Manually curated test data:
- **alignments**: contains query- and reference-to-signal alignments for each read. The data was obtained by running the provided python script `create_expected_alignments.py`. This simply set up both (unrefined) alignment using the default settings. (loaded in [test_alignments.rs](test_alignments.rs))
- **kmer_tables**: contains valid and invalid kmer table files to test the loading and error handling of the `KmerTable` struct. (loaded in [test_kmer_table.rs](test_kmer_table.rs))

### The test data extracted directly from Remora 
We used the following parameters for testing data extraction:
- `do_rough_rescale`: True,
- `scale_iters`: 2,
- `algo`: "dwell_penalty",
- `half_bandwidth`: 5,
- `sd_params`: (4, 3, 0.5),
- `do_fix_guage`: True

These parameters provide thorough testing data with enabled rough rescaling, kmer-level normalization and multiple refinement iterations.

With these parameters testing data was extracted from four runs:
- test_data_querymap_leastsquares
- test_data_querymap_theilsen
- test_data_refmap_leastsquares
- test_data_refmap_theilsen
  
As their name suggests, these differ by the type of mapping (query / reference) used for refinement and the algorithm used for rough rescaling (least squares / theil sen). This way we could test all underlying functions and paths that are available.

Each test data directory has the same structure:
- **full_process**: Used to test both the unrefined and refined mappings (loaded in [test_full_process.rs](test_full_process.rs))
- **kmer_table_levels**: Used to test the expected level extraction from a KmerTable for a given sequence (loaded in [test_kmer_table.rs](test_kmer_table.rs))
- **refinement_dp**
  - **band**: Used to test the calculation of the signal and sequence bands used in the banded DP algorithm (loaded in [test_band.rs](test_band.rs))
  - **banded_dp**: Used to test the path calculated from forward pass and subsequent traceback (loaded in [test_banded_dp.rs](test_banded_dp.rs))
  - **forward_pass**: Used to test the scores and traceback calculated before the path is retraced (loaded in [test_dp_forward_pass.rs](test_dp_forward_pass.rs))
  - **traceback**: Used to test the path calculated from the traceback (loaded in [test_traceback.rs](test_traceback.rs))
- **rescale**: Used to test the rescaling functions (loaded in [test_rescale.rs](test_rescale.rs))
- **rough_rescale**: Used to test the rough_rescaling functions (loaded in [test_rough_rescale.rs](test_rough_rescale.rs))