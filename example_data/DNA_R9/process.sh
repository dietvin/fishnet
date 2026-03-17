#!/bin/bash
set -euo pipefail
BASEDIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# Minimal script for retrieving and processing the DNA R9 data

DORADO_BIN="/home/vincent/tools/dorado-0.9.0-linux-x64/bin/dorado"
DORADO_MODEL="/home/vincent/tools/dorado-0.9.0-linux-x64/models/dna_r9.4.1_e8_sup@v3.6"

data_archive="${BASEDIR}/tmp_data.tar.gz"
fast5_directory="${BASEDIR}/fast5"
mkdir -p "$fast5_directory"

# Download and extract a selection of pod5 files
wget -O "$data_archive" "ftp://ftp.sra.ebi.ac.uk/vol1/run/ERR912/ERR9127551/ecoli_r9.tar.gz"
tar -xzf "$data_archive" \
    -C "$fast5_directory" \
    --wildcards \
    --strip-components=5 \
    "r9/f5s/RefStrains210914_NK/f5s/barcode02/barcode02_r0barcode02b10_0.fast5"

# Convert fast5 files to a pod5 file
pod5_full="${BASEDIR}/all_reads.pod5"
pod5 convert fast5 "$fast5_directory" \
    --threads 24 \
    --force-overwrite \
    --output "$pod5_full" 

# Download and decompress the reference
REF="${BASEDIR}/ref.fa"
wget -O - "https://ftp.ncbi.nlm.nih.gov/genomes/all/GCF/000/005/845/GCF_000005845.2_ASM584v2/GCF_000005845.2_ASM584v2_genomic.fna.gz" \
    | gzip --decompress --stdout - \
    > "$REF"

# Identify reads that map to the reference
read_ids="${BASEDIR}/mapped_read_ids.txt"
set +o pipefail
"$DORADO_BIN" basecaller "$DORADO_MODEL" "$pod5_full" --reference "$REF" \
    | samtools view -F 2308 \
    | cut -f1 \
    | head -n 100 \
    > "$read_ids"
set -o pipefail

# Subset the pod5 data to at most 100 reads that map
pod5_filtered="${BASEDIR}/reads.pod5"
pod5 filter "${pod5_full}" \
    --ids "$read_ids" \
    --threads 24 \
    --force-overwrite \
    --output "$pod5_filtered"

# Rebasecall and -map the subset data
mappings="${BASEDIR}/mappings.bam"
"$DORADO_BIN" basecaller "$DORADO_MODEL" "$pod5_filtered" \
    --reference "$REF" \
    --emit-moves \
    > "$mappings"

# Clean up
rm -r "$data_archive" "$fast5_directory" "$pod5_full" "$read_ids" "$REF"