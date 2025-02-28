#!/bin/python

# The script should be started from the root directory of this repository:
# python tests/expected_alignments/create_expected_alignments.py

import pod5
from remora import io
from pathlib import Path
import numpy as np


test_data_root = Path("example_data")
out_directory = Path("tests/expected_alignments")
pod5_dr = pod5.DatasetReader(test_data_root)
bam_fh = io.ReadIndexedBam(test_data_root / "can_mappings.bam")

for read_id in pod5_dr.read_ids:
    print(read_id)
    pod5_read = pod5_dr.get_read(read_id)
    bam_read = bam_fh.get_first_alignment(read_id)

    remora_read = io.Read.from_pod5_and_alignment(pod5_read, bam_read)
    query_to_signal = remora_read.query_to_signal
    reference_to_signal = remora_read.ref_to_signal

    outpath = out_directory / f"{read_id}_query_to_signal.txt"
    np.savetxt(outpath, query_to_signal, fmt="%i")

    outpath = out_directory / f"{read_id}_ref_to_signal.txt"
    print(outpath)
    np.savetxt(outpath, reference_to_signal, fmt="%i")

