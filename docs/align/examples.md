# Examples

Aligning mapped **DNA** data to the reference:
```bash
fishnet \
    --bam <bam-file> \
    --pod5 <pod5-file1> <pod5-file2> <pod5-file3> \
    --kmer-table <kmer-table-file> \
    --out <output-file-name>.parquet \
    --alignment-type reference
``` 

Aligning **direct RNA** data to the base-called sequence:
```bash
fishnet \
    --bam <bam-file> \
    --pod5 <pod5-directory> \
    --kmer-table <kmer-table-file> \
    --out <output-file-name>.parquet \
    --rna
``` 

