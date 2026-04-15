# Getting started

![fishnet_logo](images/fishnet_logo_wide_cropped.png)

Fishnet performs signal-to-sequence alignment (*resquiggling*) fast and accessibly. It uses the alignment algorithm from ONT's [Remora](https://github.com/nanoporetech/remora).

## Installation

See [Installation](installation.md) for all details.

## Alignment

```bash
fishnet align \
  --bam <basecalls.bam> \
  --pod5 <raw-signal.pod5> \
  --out <output-file>
```

See [Align](align/index.md) for an overview.


## Reformatting

```bash
fishnet reformat \
  --alignment <alignments.parquet> \
  --pod5 <raw-signal.pod5> \
  --motifs <motif> \
  --out <output-file>
```

See [Reformat](reformat/index.md) for an overview.

## POD5 Reader API

See [Pod5 Reader API](pod5_reader_api/index.md) for an overview.

## Citation

Fishnet is currently under review in Genome Biology: 

DOI: [https://doi.org/10.21203/rs.3.rs-8345719/v1](https://doi.org/10.21203/rs.3.rs-8345719/v1)

If you use Fishnet in your work, we would be really happy if you cite us:
```
Vincent Dietrich, Lioba Lehmann, Stefan Pastore et al. Fishnet simplifies and accelerates signal-to-sequence alignment in Nanopore sequencing, 16 January 2026, PREPRINT (Version 1) available at Research Square [https://doi.org/10.21203/rs.3.rs-8345719/v1]
```