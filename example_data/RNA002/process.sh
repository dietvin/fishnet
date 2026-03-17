#!/bin/bash
set -euo pipefail
BASEDIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# Minimal script for retrieving and processing the RNA002 data


DORADO_BIN="/home/vincent/tools/dorado-0.9.0-linux-x64/bin/dorado"
DORADO_MODEL="/home/vincent/tools/dorado-0.9.0-linux-x64/models/rna002_70bps_hac@v3"

data_archive="${BASEDIR}/tmp_data.tar.gz"
pod5_directory="${BASEDIR}/pod5"

# Download and extract a selection of pod5 files
wget -O "$data_archive" "ftp://ftp.sra.ebi.ac.uk/vol1/run/ERR152/ERR15278462/RNA002_UHRR_1.tar.gz"

files=()
for i in $(seq 0 20); do
    files+=("mnt/ssd_share_01/new_folder/RESEARCH/20230714_UHRR_R9_DRS/UHRR_R9_DRS/20230714_1508_3A_PAG68321_9f572de7/pod5/PAG68321_9f572de7_b1ea23d9_${i}*.pod5")
done
tar -xzf "$data_archive" \
    -C "$pod5_directory" \
    --wildcards \
    --strip-components=8 \
    "${files[@]}"

# Download and decompress the reference
REF="${BASEDIR}/ref.fa"
wget -O - "https://ftp.ebi.ac.uk/pub/databases/gencode/Gencode_human/release_49/GRCh38.primary_assembly.genome.fa.gz" \
    | gzip --decompress --stdout - \
    > "$REF"

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

# Clean up
rm -r "$data_archive" "$pod5_directory" "$read_ids" "$REF"