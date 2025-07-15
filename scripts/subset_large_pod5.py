#!/bin/python3
import argparse
from pathlib import Path
from pod5 import DatasetReader, Writer
from tqdm import tqdm

def parse_args():
    parser = argparse.ArgumentParser(
        description="Split one or more .pod5 files into smaller chunks by read count."
    )
    parser.add_argument("inputs", type=Path, nargs='+', help="One or more input .pod5 files")
    parser.add_argument("-n", "--reads-per-file", type=int, default=8000,
                        help="Number of reads per output file (default: 8000)")
    parser.add_argument("-o", "--output-dir", type=Path, default=None,
                        help="Optional base output directory (default: next to each input)")
    parser.add_argument("--quiet", action="store_true", help="Suppress progress output")
    return parser.parse_args()

def validate_input(input_path):
    return input_path.exists() and input_path.suffix == ".pod5"

def split_pod5(input_path, output_dir, reads_per_file=8000, quiet=False):
    output_dir = output_dir or input_path.with_name(input_path.stem)
    output_dir.mkdir(exist_ok=True)

    with DatasetReader(str(input_path)) as reader:
        read_ids = list(reader.read_ids)
        total_reads = len(read_ids)
        total_chunks = (total_reads + reads_per_file - 1) // reads_per_file

        if not quiet:
            print(f"\nProcessing '{input_path.name}': {total_reads} reads ➜ {total_chunks} chunks")

        for i in range(0, total_reads, reads_per_file):
            chunk_id = i // reads_per_file + 1
            chunk_read_ids = read_ids[i:i + reads_per_file]
            output_path = output_dir / f"chunk_{chunk_id}.pod5"

            with Writer(str(output_path)) as writer:
                for read in tqdm(reader.reads(chunk_read_ids),
                                 desc=f"Chunk {chunk_id}",
                                 disable=quiet):
                    writer.add_read(read.to_read())

            if not quiet:
                print(f"Wrote {len(chunk_read_ids)} reads to {output_path}")

    if not quiet:
        print(f"Finished: {input_path.name} ➜ {output_dir}")

def main():
    args = parse_args()

    processed_files = 0
    for input_file in args.inputs:
        if not validate_input(input_file):
            print(f"Skipping invalid file: {input_file}")
            continue

        final_output_dir = (
            args.output_dir / input_file.stem
            if args.output_dir
            else input_file.with_name(input_file.stem)
        )

        split_pod5(input_file, final_output_dir, args.reads_per_file, args.quiet)
        processed_files += 1

    if processed_files == 0:
        print("No valid .pod5 files processed.")
    elif not args.quiet:
        print(f"\nDone! Processed {processed_files} file(s).")

if __name__ == "__main__":
    main()
