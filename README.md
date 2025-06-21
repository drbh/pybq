# pybq

Python bindings for high-performance BQ and VBQ sequence file processing with zero-copy NumPy/PyTorch integration.

## Installation

Build from source using uv and maturin.

```bash
uv sync --force-reinstall
```

## Basic Usage

Read BQ or VBQ files with automatic threading and pattern matching.

```python
import pybq

# For regular BQ files
with pybq.open_bq("file.bq", n_threads=4) as reader:
    print(f"Records: {len(reader):,}")
    matches = reader.count_matches("ACGT")
    print(f"ACGT matches: {matches:,}")

# For VBQ files
with pybq.open_vbq("file.vbq", n_threads=4) as reader:
    print(f"Records: {len(reader):,}")
    matches = reader.count_matches("ACGT")
    print(f"ACGT matches: {matches:,}")
    
    # Population count (number of 1 bits)
    total_bits = reader.popcnt()
    print(f"Total 1 bits: {total_bits:,}")
```

## Zero-Copy Arrays

Convert records directly to NumPy arrays without memory copying.

```python
with pybq.open_bq("file.bq") as reader:
    for record in reader:
        array = np.asarray(record)  # Zero copy
        tensor = torch.from_numpy(array)  # Zero copy
```

## Performance

Multi-threaded processing scales with available CPU cores.

```bash
python my_demo.py test.bq
```

```
file         test.bq
records      24,890
size         995,632 bytes
efficiency   40.0 bytes/record

       A    24,890  100.0%      1ms
       T    24,890  100.0%      1ms
       G    24,890  100.0%      1ms
       C    24,890  100.0%      1ms
    ACGT    24,259   97.5%      2ms

threads=1       2ms    1.0x
threads=2       1ms    2.0x
threads=4       1ms    3.4x
threads=8       0ms    5.6x
```

## Array Integration

Demonstrate zero-copy integration with scientific Python libraries.

```bash
python np_demo.py test.bq
```

```
pybq Array Integration
======================
file: test.bq
records: 24,890

record 1: CGGTATTGTTAGCGCCGTCATTATCCAA
  numpy: shape=(28,), dtype=uint8
  data: [67 71 71 ... 65]
  zero-copy: True

batch: shape=(3, 28), mean=72.4

PyTorch Integration
===================
record 1: CGGTATTGTT...
  tensor: dtype=torch.uint8, shape=torch.Size([28])
  sum: 2041
  memory shared: True
```

## API Reference

Core classes and methods for BQ/VBQ file processing.

```python
# Convenience functions
reader = pybq.open_bq(path, n_threads=1)     # Open BQ file
reader = pybq.open_vbq(path, n_threads=1)    # Open VBQ file

# Reader (same interface for both BQ and VBQ)
reader = pybq.BqReader(path, n_threads=1, is_vbq=False)
reader.len()                    # Total records
reader.count_matches(pattern)   # Pattern matches
reader.set_n_threads(n)         # Change thread count
reader.is_vbq()                 # Check if VBQ format

# Record
record.get_sequence()           # Decoded string
record.data_ptr()               # Memory address
record.shape()                  # Array dimensions
record.popcnt()                 # Population count (1 bits)

# Population Count
reader.popcnt()                 # Total 1 bits in all sequences
```
