# Filtering options
Filtering limits processing to **regions or motifs of interest**, reducing both runtime and output size.

## 1. Reference regions
When working with reference-to-signal alignments, bases are processed only if they fall within specified regions. 

There are three ways to provide reference regions:

| **Flag**                                       | **Description**                                                                                                         |
| ---------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------- |
| `--ref-regions <REF:START-END>`                | Specify regions directly (1-based, inclusive). Multiple regions space-separated.                                        |
| `--bed-file <PATH>`                            | Provide regions from a BED file (0-based, start-inclusive, end-exclusive).                                              |
| `--positions-of-interest <REF:POS-HALFWINDOW>` | Define positions with flanking windows. Coordinates are 1-based. See [Positions with windows](#positions-with-windows). |


### Positions with windows
This mode targets a specific site plus a window around it, e.g.: `ChrA:8-4`
```text
1-based index:          1 2 3 4 5 6 7 8 9 0 1 2 3 4 5
Ref sequence:           A C G T A G C T A A A G T C T
Region of interest:          |--------X--------|
```

## 2. Motifs
Motif filtering works for both query- and reference-to-signal alignments. Only alignment segments exactly matching a provided motif are processed. (Requires fishnet align output level 2 or 3 to include sequence information.)

There are two ways to provide motifs:

| **Flag**                 | **Description**                                                          |
| ------------------------ | ------------------------------------------------------------------------ |
| `--motifs <MOTIF-1> ...` | Provide motifs directly. Named automatically (`motif1`, `motif2`, …).    |
| `--motifs-file <FASTA>`  | Provide motifs via a FASTA file. Names are taken from the FASTA headers. |

All motif matches within a read are processed.
