#!/usr/bin/env python3
import sys
import os
import time
import pybq

if len(sys.argv) != 2:
    print("Usage: python minimal_demo.py <file.bq>", file=sys.stderr)
    sys.exit(1)

file_path = sys.argv[1]
is_vbq = file_path.endswith(".vbq")

print("pybq Fast K-mer Count")
print("====================")

start_time = time.perf_counter_ns()

with pybq.BqReader(file_path, n_threads=16, is_vbq=is_vbq) as reader:
    print(f"file: {os.path.basename(file_path)}")
    print(f"records: {len(reader):,}")

    # Single line replaces the entire loop - parallel processing!
    kmer_counts = reader.count_kmers_parallel(4)

# Show results
for kmer, count in sorted(kmer_counts.items())[:10]:
    print(f"{kmer}: {count}")

elapsed_time = (time.perf_counter_ns() - start_time) / 1_000_000
print(f"\nTime: {elapsed_time:.2f} ms | K-mers: {len(kmer_counts):,}")
