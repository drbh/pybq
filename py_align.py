import pybq
import time
from collections import defaultdict, deque
import numpy as np
import sys

# new_seq_to_align = "ACGTACGTACGT"
new_seq_to_align = "ACGCGGTTAGCACGTAGCACAGTCTGCTTGCTATGCACTCTTGTGCTTAGCTCTGAAACTCGCACCCTCGCCTCCGCCCCGGTCCT"


# make a new BqRecord
new_seq_bq_record = pybq.BqRecord.from_sequence(new_seq_to_align)
print(new_seq_bq_record)
print(new_seq_bq_record.get_sequence())
# print(new_seq_bq_record.get_encoded_bytearray())
print(new_seq_bq_record.data_ptr())


if len(sys.argv) != 2:
    print("Usage: python np_demo.py <file.bq>", file=sys.stderr)
    sys.exit(1)

file_path = sys.argv[1]

start_time = time.perf_counter_ns()
# file_path = "some.bq"
# file_path = "ref.bq"
# file_path = "output.vbq"

# # convert to numpy array of 2-bit encoded values
# new_seq_np = np.frombuffer(new_seq_to_align.encode('ascii'), dtype='S1')
# print(new_seq_np)


reader_class = pybq.open_vbq if file_path.endswith(".vbq") else pybq.open_bq

best_seen = 100000000
best_position = -1
record_index = -1

count = 0
for record in reader_class(file_path, 32):
    best_position, best_distance = record.sliding_window_best_match(new_seq_bq_record)
    # print(f"Best position: {best_position}, Best distance: {best_distance}, Sequence: {sequence}")
    # print(f"Best position: {best_position}, Best distance: {best_distance}")
    # sequence = record.get_encoded_bytearray()
    sequence = record.get_sequence()
    ptr = record.data_ptr()
    # print(sequence, ptr)

    if best_distance <= best_seen:
        best_seen = best_distance
        best_position = best_position
        record_index = count
        print(f"New best distance: {best_seen} at record {count}")

    count += 1
    if count >= 500000:
        break


end_time = time.perf_counter_ns()
elapsed_ms = (end_time - start_time) / 1_000_000
elapsed_time = (end_time - start_time) / 1_000_000_000
print(f"Processed {count} records in {elapsed_ms:.2f} ms")
print(f"Throughput: {count / elapsed_time:.2f} records/second")
# print(f"Overall best distance: {best_seen} at position {best_position}")

print(f"Best match: record {record_index}, position {best_position}, distance {best_seen}")

