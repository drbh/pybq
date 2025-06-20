import pybq
import time
import os
import sys

def timer(func):
    def wrapper(*args, **kwargs):
        start = time.time()
        result = func(*args, **kwargs)
        return result, time.time() - start
    return wrapper

if len(sys.argv) != 2:
    print("Usage: python my_demo.py <file.bq>", file=sys.stderr)
    sys.exit(1)

file_path = sys.argv[1]

# Example 1: File Information
try:
    with pybq.open_bq(file_path) as reader:
        records = len(reader)
        size = os.path.getsize(file_path)
        
        print(f"file         {os.path.basename(file_path)}")
        print(f"records      {records:,}")
        print(f"size         {size:,} bytes")
        print(f"efficiency   {size/records:.1f} bytes/record")
        
except Exception as e:
    print(f"error: {e}", file=sys.stderr)
    sys.exit(1)

print()

# Example 2: Pattern Analysis
patterns = ["A", "T", "G", "C", "ACGT", "GGCC", "AATT", "GGAATTCC"]

try:
    with pybq.open_bq(file_path) as reader:
        total_records = len(reader)
        
        for pattern in patterns:
            matches, elapsed = timer(reader.count_matches)(pattern)
            pct = (matches / total_records * 100) if total_records > 0 else 0
            print(f"{pattern:>8s}  {matches:>8,d}  {pct:>5.1f}%  {elapsed*1000:>5.0f}ms")
            
except Exception as e:
    print(f"error: {e}", file=sys.stderr)
    sys.exit(1)

print()

# Example 3: Threading Performance (only test if file is substantial)
if records > 100:  # Only test threading on meaningful datasets
    thread_counts = [1, 2, 4, 8]
    baseline_time = None
    
    try:
        for threads in thread_counts:
            with pybq.open_bq(file_path, n_threads=threads) as reader:
                _, elapsed = timer(reader.count_matches)("ACGT")
                
                if baseline_time is None:
                    baseline_time = elapsed
                    speedup = 1.0
                else:
                    speedup = baseline_time / elapsed if elapsed > 0 else 0
                
                print(f"threads={threads:d}   {elapsed*1000:>5.0f}ms   {speedup:>4.1f}x")
                
    except Exception as e:
        print(f"error: {e}", file=sys.stderr)
        sys.exit(1)
else:
    print("threads=1      0ms   1.0x  (file too small for threading test)")
