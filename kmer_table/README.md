# Kmer table

This module handles the kmer levels table handling. It provides the following elements:

- [`KmerTable`](./src/kmer_table.rs): The implementation of a kmer table itself. Stores (standardized) signal levels for each possible kmer.
- [`BinaryKmer`](./src/binary_kmer.rs): A representation of a kmer where the bases are encoded in the bits of a u64 instead of a String or similar.
- [`KmerTableData`](./src/kmer_table_data.rs): A precursor of the KmerTable that allows for serialization.
- [`errors`](./src/error.rs): Various error types for the structs above.

The main reason to move the logic out of the alignment module (it was previously in [alignment::core::refinement](../alignment/src/core/refinement/)) is to embed the standard kmer tables in the binary so the user doesn't need to provide it manually. To this end, a selection of the kmer tables in the added [submodule](../alignment/kmer_models/) get parsed into `KmerTableData` instances and then serialized into binary files. This is done before compiling the alignment module in the [`build.rs`](../alignment/build.rs) file. During compilation, the binary files get deserialized and embedded in the executable.