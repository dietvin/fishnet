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