# pybq

Python bindings for high-performance BQ sequence file processing with zero-copy NumPy/PyTorch integration.

## Installation

Build from source using uv and maturin.

```bash
uv sync --force-reinstall
```

## Basic Usage

Read BQ files with automatic threading and pattern matching.

```python
import pybq

with pybq.open_bq("file.bq", n_threads=4) as reader:
    print(f"Records: {len(reader):,}")
    matches = reader.count_matches("ACGT")
    print(f"ACGT matches: {matches:,}")
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

Core classes and methods for BQ file processing.

```python
# Reader
reader = pybq.BqReader(path, n_threads=1)
reader.len()                    # Total records
reader.count_matches(pattern)   # Pattern matches
reader.set_n_threads(n)        # Change thread count

# Record
record.get_sequence()          # Decoded string
record.data_ptr()             # Memory address
record.shape()                # Array dimensions
```
