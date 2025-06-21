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
uv run demo.py test.vbq
```

```
Creating BQ reader for path: test.vbq
Using 1 threads
file         test.vbq
records      49,780
size         445,271 bytes
efficiency   8.9 bytes/record

Creating BQ reader for path: test.vbq
Using 1 threads
       A    49,367   99.2%      5ms
       T    49,354   99.1%      5ms
       G    49,753   99.9%      5ms
       C    49,316   99.1%      4ms
    ACGT    26,589   53.4%      6ms
    GGCC     8,566   17.2%      6ms
    AATT     3,846    7.7%      6ms
GGAATTCC        11    0.0%      8ms

Creating BQ reader for path: test.vbq
Using 1 threads
threads=1       5ms    1.0x
Creating BQ reader for path: test.vbq
Using 2 threads
threads=2       3ms    1.8x
Creating BQ reader for path: test.vbq
Using 4 threads
threads=4       2ms    3.3x
Creating BQ reader for path: test.vbq
Using 8 threads
threads=8       1ms    4.9x
```

## Array Integration

Demonstrate zero-copy integration with scientific Python libraries.

```bash
uv run np_demo.py test.vbq
```

```
pybq Array Integration
======================
Creating BQ reader for path: test.vbq
Using 1 threads
file: test.vbq
records: 49,780

record 1: CCAACTGGTGTGACCCTAGTTTATGGCT
  numpy: shape=(28,), dtype=uint8
  data: [67 67 65 ... 84]
  zero-copy: True

record 2: ACGCGGTTAGCACGTACGAGCTGTGACTTGCTATGCACTCTTGTGCTTAGCTCTGAAACCCGGGTGAGCTCACCGCCCCCGGTCCTAGCA
  numpy: shape=(90,), dtype=uint8
  data: [65 67 71 ... 65]
  zero-copy: True

record 3: CGATGTTGTAAAGCGCTTTGATGTCTAA
  numpy: shape=(28,), dtype=uint8
  data: [67 71 65 ... 65]
  zero-copy: True

PyTorch Integration
===================
Creating BQ reader for path: test.vbq
Using 1 threads
record 1: CCAACTGGTG...
  tensor: dtype=torch.uint8, shape=torch.Size([28])
  sum: 2047
  memory shared: True

record 2: ACGCGGTTAG...
  tensor: dtype=torch.uint8, shape=torch.Size([90])
  sum: 6451
  memory shared: True
```

### Processing KMERs

The fastest way to count K-mers in VBQ files is using the `optimal_kmer.py` script which relies on the `pybq` backend for efficient parallel processing.

```bash
uv run optimal_kmer.py test.vbq
```

```
pybq Fast K-mer Count
====================
Creating BQ reader for path: test.vbq
Using 16 threads
file: test.vbq
records: 49,780
AAAA: 4512
AAAC: 28211
AAAG: 4609
AAAT: 4058
AACA: 8653
AACC: 16963
AACG: 9738
AACT: 11579
AAGA: 5073
AAGC: 5751

Time: 13.14 ms | K-mers: 256
```

we can try another approach that simply iterates over the records on the rust side

```bash
uv run rust_kmer.py test.vbq
```

```
pybq Array Integration
======================
Creating BQ reader for path: test.vbq
Using 16 threads
file: test.vbq
records: 49,780


Full Kmer Count:
AAAA: 4512
AAAC: 28211
AAAG: 4609
AAAT: 4058
AACA: 8653
AACC: 16963
AACG: 9738
AACT: 11579
AAGA: 5073
AAGC: 5751

Elapsed time: 688.98 ms
```

and another that iterates on the python side (which requires the most copies but may be faster in some small cases)

```bash
uv run py_kmer.py test.vbq
```

```
uv run py_kmer.py test.vbq
Counting k-mers of length 4 in file: test.vbq
Creating BQ reader for path: test.vbq
Using 16 threads
Total unique k-mers: 256
Total k-mer count: 2787680.00
AAAA: 4512.00
AAAC: 28211.00
AAAG: 4609.00
AAAT: 4058.00
AACA: 8653.00
AACC: 16963.00
AACG: 9738.00
AACT: 11579.00
AAGA: 5073.00
AAGC: 5751.00
Elapsed time: 459.34 ms
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

## Reproducible Data

For a reproducible sequence we rely on the `GCF_000001635.27_GRCm39_genomic` assembly from NCBI, which is available in FASTA format. The following `make` or `just` command will download and convert it to BQ format:

*note you can use `just` or `make` interchangeably, depending on your preference.*

```bash
make download-mouse
just encode-mouse
```

### Comparing kmer counting speed

**note** once you have a mouse VBA file, you can dump a 1:1 comparable FASTA file using the following command. This is helpful for debugging and validation since FASTA files may contain more information than BQ files so we want to ensure the comparisons are equivalent.

```bash
bqtools decode mouse.vbq -o mouse.fa
```

now the `.fa` file should have the same excact sequence as the `.vbq` file, but with the bq policy applied to any missing bases - ensuring that the sequences are identical. 

also once you have both mouse files and the refernce tools jellyfish installed, you can run the following command to validate the sequences:

```bash
time sh ref-kmer.sh
```

```
>12370989
AAAA
>8744579
AAAC
>11121426
AAAG
>12041417
AAAT
>9826286
AACA
sh ref-kmer.sh  784.61s user 1.54s system 680% cpu 1:55.46 total
```

and compare it with the pybq kmer count:

```bash
uv run optimal_kmer.py mouse.vbq
```

```
pybq Fast K-mer Count
====================
Creating BQ reader for path: mouse.vbq
Using 16 threads
file: mouse.vbq
records: 61
AAAA: 12370989
AAAC: 8744579
AAAG: 11121426
AAAT: 12041417
AACA: 9826286
AACC: 6803533
AACG: 1174493
AACT: 8293172
AAGA: 10824039
AAGC: 7599175

Time: 8004.36 ms | K-mers: 256
```