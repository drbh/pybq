# bqtools encode -B 62239136 -f a GCF_000001405.26_GRCh38_genomic.fna -o output.vbq
import pybq
import time
from collections import defaultdict, deque

# TODO: complete this file to use fast kmer counting
def count_kmers(file_path, k):
    """K-mer counting with global position and queue"""
    kmers = defaultdict(float)
    queue = deque()

    global_position = 0
    # scan = True
    scan = False

    with pybq.open_vbq(file_path, 2) as reader:
        for index, record in enumerate(reader):
            sequence = record.get_sequence()
            print(index, sequence[:10], len(sequence))
            # for i in range(len(sequence) - k + 1):
            #     kmer = sequence[i : i + k]
            #     kmers[kmer] += 1.0
    return kmers


# Main execution
# file_path = "some.bq"
# file_path = "ref.bq"
file_path = "output.vbq"
# k = 12
# k = 8
# k = 5
k = 4

start_time = time.perf_counter_ns()

print(f"Counting k-mers of length {k} in file: {file_path}")
kmers = count_kmers(file_path, k)

print(f"Total unique k-mers: {len(kmers)}")
print(f"Total k-mer count: {sum(kmers.values()):.2f}")

# sort alphabetically instead of by frequency
sorted_kmers = sorted(kmers.items(), key=lambda x: x[0])
# for kmer, count in sorted_kmers[:20]:
for kmer, count in sorted_kmers:
    print(f"{kmer}: {count:.2f}")

end_time = time.perf_counter_ns()
elapsed_time = (end_time - start_time) / 1_000_000  # convert to milliseconds
print(f"Elapsed time: {elapsed_time:.2f} ms")
