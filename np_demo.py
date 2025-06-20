import sys
import os
import numpy as np

try:
    import torch
    HAS_TORCH = True
except ImportError:
    HAS_TORCH = False

import pybq

if len(sys.argv) != 2:
    print("Usage: python np_demo.py <file.bq>", file=sys.stderr)
    sys.exit(1)

file_path = sys.argv[1]

print("pybq Array Integration")
print("======================")

# Basic file info
with pybq.open_bq(file_path) as reader:
    total = len(reader)
    print(f"file: {os.path.basename(file_path)}")
    print(f"records: {total:,}")
    print()

    # Test first 3 records
    records = []
    for i, record in enumerate(reader):
        if i >= 3:
            break
        
        # Zero-copy to NumPy
        np_array = np.asarray(record)
        records.append(np_array)
        
        print(f"record {i+1}: {record.get_sequence()}")
        print(f"  numpy: shape={np_array.shape}, dtype={np_array.dtype}")
        print(f"  data: [{np_array[0]} {np_array[1]} {np_array[2]} ... {np_array[-1]}]")
        print(f"  zero-copy: {np_array.ctypes.data == record.data_ptr()}")
        print()

    # Batch processing
    if records:
        batch = np.stack(records)
        print(f"batch: shape={batch.shape}, mean={batch.mean():.1f}")
        print()

# PyTorch test
if HAS_TORCH:
    print("PyTorch Integration")
    print("===================")
    
    with pybq.open_bq(file_path) as reader:
        for i, record in enumerate(reader):
            if i >= 2:
                break
                
            # Zero-copy chain: BqRecord -> NumPy -> PyTorch
            np_array = np.asarray(record)
            tensor = torch.from_numpy(np_array)
            
            print(f"record {i+1}: {record.get_sequence()[:10]}...")
            print(f"  tensor: dtype={tensor.dtype}, shape={tensor.shape}")
            print(f"  sum: {tensor.sum().item()}")
            print(f"  memory shared: {tensor.data_ptr() == record.data_ptr()}")
            print()
            
else:
    print("PyTorch not available - install with: pip install torch")
