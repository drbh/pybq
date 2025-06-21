import pybq
import time
from collections import defaultdict, deque


def count_kmers(file_path, k):
    """K-mer counting with global position and queue"""
    kmers = defaultdict(float)
    queue = deque()

    global_position = 0
    # scan = True
    scan = False

    # with pybq.open_vbq(file_path, n_threads=16) as reader:
    with pybq.open_bq(file_path) as reader:

        if scan is False:
            for record in reader:
                sequence = record.get_sequence()
                for i in range(len(sequence) - k + 1):
                    kmer = sequence[i : i + k]
                    kmers[kmer] += 1.0
            return kmers

        while scan:
            # see if queue is long enough so we can read a window of
            # global_position + k
            if len(queue) < k:
                try:
                    record = next(reader)
                    sequence = record.get_sequence()
                    print(sequence)
                    queue.extend(sequence)
                except StopIteration:
                    break

            # if queue is long enough, we can start processing
            if len(queue) >= k:
                window = list(queue)[:k]  # get the first k characters
                kmer = "".join(window)
                kmers[kmer] += 1.0
                # remove the first character from the queue
                global_position += 1
                queue.popleft()

            # exit condition: if we have read all records and the queue is empty
            if not queue:
                break

    return kmers


# Main execution
file_path = "test.bq"
# file_path = "some.bq"
# file_path = "ref.bq"
# file_path = "output.vbq"
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
