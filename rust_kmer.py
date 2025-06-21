import sys
import os
import time
import pybq

if len(sys.argv) != 2:
    print("Usage: python np_demo.py <file.bq>", file=sys.stderr)
    sys.exit(1)

file_path = sys.argv[1]

print("pybq Array Integration")
print("======================")

start_time = time.perf_counter_ns()

reader_class = pybq.open_vbq if file_path.endswith(".vbq") else pybq.open_bq
kmer_count = {}

with reader_class(file_path, 16) as reader:
    print(f"file: {os.path.basename(file_path)}")
    print(f"records: {len(reader):,}")
    print()

    for record in reader:
        for kmer, count in record.kmers(4).items():
            kmer_count[kmer] = kmer_count.get(kmer, 0) + count

print("\nFull Kmer Count:")
for kmer, count in sorted(kmer_count.items())[:10]:
    print(f"{kmer}: {count}")

elapsed_time = (time.perf_counter_ns() - start_time) / 1_000_000
print(f"\nElapsed time: {elapsed_time:.2f} ms")
