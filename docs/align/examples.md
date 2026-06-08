# Examples

The following examples assume that the current working directory is the repository root. It used the data in [`example_data`](../../example_data/).

Aligning **DNA** reads to their basecalled (**query**) sequences:
```bash
fishnet align \
    --bam example_data/DNA_R10/mappings.bam \
    --pod5 example_data/DNA_R10/reads.pod5 \
    --out alignments.parquet
```

Aligning the same DNA data to the mapped **reference** sequences:
```bash
fishnet align \
    --bam example_data/DNA_R10/mappings.bam \
    --pod5 example_data/DNA_R10/reads.pod5 \
    --out alignments.parquet \
    --alignment-type reference
```

Aligning **direct RNA** reads to their **query** sequences:
```bash
fishnet align \
    --bam example_data/RNA004/mappings.bam \
    --pod5 example_data/RNA004/reads.pod5 \
    --out alignments.parquet \
    --rna
``` 

