#!/usr/bin/env python3
import sys
import os
import time
import pybq

# Query sequence to align
# query_sequence = "ACGTACGTACGT"
query_sequence = "ACGCGGTTAGCACGTAGCACAGTCTGCTTGCTATGCACTCTTGTGCTTAGCTCTGAAACTCGCACCCTCGCCTCCGCCCCGGTCCT"

if len(sys.argv) != 2:
    print("Usage: python optimal_align.py <file.bq>", file=sys.stderr)
    sys.exit(1)

file_path = sys.argv[1]
is_vbq = file_path.endswith(".vbq")

print("pybq Fast Alignment")
print("===================")

start_time = time.perf_counter_ns()
count = 0
with pybq.BqReader(file_path, n_threads=32, is_vbq=is_vbq) as reader:
    print(f"file: {os.path.basename(file_path)}")
    print(f"records: {len(reader):,}")
    print(f"query: {query_sequence}")
    
    # Single line replaces the entire loop - parallel processing!
    result = reader.find_best_alignment_parallel(query_sequence)

    # note: this is a bit expensive and adds ~1-2ms to the total time..
    count = len(reader)

if result:
    print(f"Best match: record {result['record_index']:,}, position {result['position']}, distance {result['distance']}")
else:
    print("No alignment found")

end_time = time.perf_counter_ns()
elapsed_ms = (end_time - start_time) / 1_000_000
elapsed_time = (end_time - start_time) / 1_000_000_000
print(f"\nTime: {elapsed_ms:.2f} ms")
print(f"Throughput: {count / elapsed_time:.2f} records/second")
