#!/bin/bash
set -euo pipefail

# Minimal script for retrieving and processing the RNA004 data


DORADO_BIN="/home/vincent/tools/dorado-1.3.0-linux-x64/bin/dorado"
DORADO_MODEL="/home/vincent/tools/dorado-1.3.0-linux-x64/models/rna004_130bps_sup@v5.2.0"
BASEDIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# Download and extract the pod5 data
data_archive="${BASEDIR}/tmp_data.tar.gz"
pod5_directory="${BASEDIR}/pod5"
mkdir -p "$pod5_directory"

# Download and extract a selection of pod5 files
# wget -O "$data_archive" "ftp://ftp.sra.ebi.ac.uk/vol1/run/ERR152/ERR15278639/RNA004_UHRR_1.tar.gz"

tar -xzf "$data_archive" \
    -C "$pod5_directory" \
    --wildcards \
    --strip-components=7 \
    "mnt/ssd_share_01/new_folder/RESEARCH/TMP_RNA004_PAPER/UPLOAD_RAW_DATA/RNA004_UHRR_1/RNA004_UHRR_1.passed.pod5"

# Download and decompress the reference
REF="${BASEDIR}/ref.fa"
# wget -O - "https://ftp.ebi.ac.uk/pub/databases/gencode/Gencode_human/release_49/GRCh38.primary_assembly.genome.fa.gz" \
#     | gzip --decompress --stdout - \
#     > "$REF"

# Identify reads that map to the reference
read_ids="${BASEDIR}/mapped_read_ids.txt"
set +o pipefail
"$DORADO_BIN" basecaller "$DORADO_MODEL" "$pod5_directory" --reference "$REF" \
    | samtools view -F 2308 \
    | cut -f1 \
    | head -n 100 \
    > "$read_ids"
set -o pipefail

# Subset the pod5 data to at most 100 reads that map
pod5_filtered="${BASEDIR}/reads.pod5"
pod5 filter "${pod5_directory}" \
    --ids "$read_ids" \
    --threads 24 \
    --output "$pod5_filtered"

# Rebasecall and -map the subset data
mappings="${BASEDIR}/mappings.bam"
"$DORADO_BIN" basecaller "$DORADO_MODEL" "$pod5_filtered" \
    --reference "$REF" \
    --emit-moves \
    > "$mappings"

# # Clean up
# rm -r "$data_archive" "$pod5_directory" "$read_ids" "$REF"
