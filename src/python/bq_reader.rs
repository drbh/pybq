use pyo3::prelude::*;
use std::collections::HashMap;
use std::path::Path;

use crate::core::{BqRecord, KmerCounter};
use crate::python::reader::{ReaderError, ReaderVariant};

/// Error conversion for Python integration
impl From<ReaderError> for PyErr {
    fn from(err: ReaderError) -> Self {
        match err {
            ReaderError::Io(e) => PyErr::new::<pyo3::exceptions::PyIOError, _>(e.to_string()),
            ReaderError::Binseq(e) => PyErr::new::<pyo3::exceptions::PyIOError, _>(e),
            ReaderError::Runtime(e) => PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e),
        }
    }
}

/// BQ file reader with Python integration
/// Supports context manager protocol and iteration
#[pyclass]
pub struct BqReader {
    reader: Option<ReaderVariant>,
    path: String,
    current_index: usize,
    n_threads: usize,
    is_vbq: bool,
}

#[pymethods]
impl BqReader {
    /// Create a new BQ reader for the given file path
    #[new]
    #[pyo3(signature = (path, n_threads=1, is_vbq=false))]
    pub fn new(path: &str, n_threads: Option<usize>, is_vbq: bool) -> PyResult<Self> {
        println!("Creating BQ reader for path: {}", path);

        let n_threads = n_threads.unwrap_or(1);
        println!("Using {} threads", n_threads);

        // Validate file exists
        if !Path::new(path).exists() {
            return Err(PyErr::new::<pyo3::exceptions::PyIOError, _>(format!(
                "File not found: {}",
                path
            )));
        }

        let reader = ReaderVariant::new(path, is_vbq)?;

        Ok(BqReader {
            reader: Some(reader),
            path: path.to_string(),
            current_index: 0,
            n_threads,
            is_vbq,
        })
    }

    /// Get total number of records in the file
    pub fn len(&self) -> PyResult<usize> {
        let reader = self.get_reader()?;
        Ok(reader.count_records(&self.path, self.n_threads)?)
    }

    /// Count records matching a specific pattern
    pub fn count_matches(&self, pattern: &str) -> PyResult<usize> {
        let reader = self.get_reader()?;
        let pattern_bytes = pattern.as_bytes();
        Ok(reader.count_matches(&self.path, pattern_bytes, self.n_threads)?)
    }

    /// Compute population count (number of 1 bits) across all sequences
    pub fn popcnt(&self) -> PyResult<u64> {
        let reader = self.get_reader()?;
        Ok(reader.count_popbits(&self.path, self.n_threads)?)
    }

    /// Count k-mers across all records in parallel
    ///
    /// This method reads all records in the file in parallel and builds a complete
    /// k-mer count dictionary for the entire dataset. This is much faster than
    /// iterating through records sequentially.
    ///
    /// Args:
    ///     k: Length of k-mers to count
    ///
    /// Returns:
    ///     Dictionary mapping k-mer strings to their counts across all sequences
    pub fn count_kmers_parallel(&self, k: usize) -> PyResult<HashMap<String, usize>> {
        if k == 0 {
            return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(
                "k must be greater than 0",
            ));
        }

        let reader = self.get_reader()?;
        let kmer_counter = KmerCounter::new(k);

        // Use the existing parallel processing infrastructure
        reader.process_parallel(&self.path, kmer_counter.clone(), self.n_threads)?;

        Ok(kmer_counter.get_counts())
    }

    /// Get k-mer statistics for the entire file in parallel
    ///
    /// This is a convenience method that returns summary statistics about k-mers
    /// without returning the full count dictionary (which can be very large).
    ///
    /// Args:
    ///     k: Length of k-mers to analyze
    ///
    /// Returns:
    ///     Dictionary with keys:
    ///     - "unique_kmers": Number of unique k-mers found
    ///     - "total_kmers": Total number of k-mers processed
    ///     - "most_frequent_kmer": Tuple of (kmer, count) for most frequent k-mer
    pub fn kmer_stats_parallel(&self, k: usize) -> PyResult<HashMap<String, PyObject>> {
        if k == 0 {
            return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(
                "k must be greater than 0",
            ));
        }

        let reader = self.get_reader()?;
        let kmer_counter = KmerCounter::new(k);

        // Use the existing parallel processing infrastructure
        reader.process_parallel(&self.path, kmer_counter.clone(), self.n_threads)?;

        Python::with_gil(|py| {
            let mut stats = HashMap::new();

            stats.insert(
                "unique_kmers".to_string(),
                kmer_counter.unique_kmer_count().into_py(py),
            );

            stats.insert(
                "total_kmers".to_string(),
                kmer_counter.total_kmer_count().into_py(py),
            );

            if let Some((kmer, count)) = kmer_counter.most_frequent_kmer() {
                stats.insert("most_frequent_kmer".to_string(), (kmer, count).into_py(py));
            } else {
                stats.insert("most_frequent_kmer".to_string(), py.None());
            }

            Ok(stats)
        })
    }

    /// Get the top N most frequent k-mers in parallel
    ///
    /// This method efficiently finds the most frequent k-mers without requiring
    /// the caller to handle potentially huge k-mer dictionaries.
    ///
    /// Args:
    ///     k: Length of k-mers to analyze
    ///     n: Number of top k-mers to return (default: 10)
    ///
    /// Returns:
    ///     List of tuples (kmer, count) sorted by count (descending)
    #[pyo3(signature = (k, n=10))]
    pub fn top_kmers_parallel(&self, k: usize, n: usize) -> PyResult<Vec<(String, usize)>> {
        if k == 0 {
            return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(
                "k must be greater than 0",
            ));
        }

        let reader = self.get_reader()?;
        let kmer_counter = KmerCounter::new(k);

        // Use the existing parallel processing infrastructure
        reader.process_parallel(&self.path, kmer_counter.clone(), self.n_threads)?;

        let counts = kmer_counter.get_counts();

        // Get top N k-mers by count
        let mut kmer_vec: Vec<(String, usize)> = counts.into_iter().collect();
        kmer_vec.sort_by(|a, b| b.1.cmp(&a.1)); // Sort by count descending
        kmer_vec.truncate(n);

        Ok(kmer_vec)
    }

    /// Build a complete k-mer profile for the entire file
    ///
    /// This is the highest-level k-mer analysis method that provides comprehensive
    /// k-mer analysis results in a single call. It's optimized for speed using
    /// parallel processing.
    ///
    /// Args:
    ///     k: Length of k-mers to analyze
    ///     include_counts: Whether to include the full k-mer count dictionary
    ///                    (default: False, as this can be very large)
    ///     top_n: Number of top k-mers to include (default: 20)
    ///
    /// Returns:
    ///     Dictionary with comprehensive k-mer analysis results
    #[pyo3(signature = (k, include_counts=false, top_n=20))]
    pub fn kmer_profile_parallel(
        &self,
        k: usize,
        include_counts: bool,
        top_n: usize,
    ) -> PyResult<HashMap<String, PyObject>> {
        if k == 0 {
            return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(
                "k must be greater than 0",
            ));
        }

        let reader = self.get_reader()?;
        let kmer_counter = KmerCounter::new(k);

        // Use the existing parallel processing infrastructure
        reader.process_parallel(&self.path, kmer_counter.clone(), self.n_threads)?;

        let counts = kmer_counter.get_counts();

        Python::with_gil(|py| {
            let mut profile = HashMap::new();

            // Basic statistics
            profile.insert("k".to_string(), k.into_py(py));

            profile.insert("unique_kmers".to_string(), counts.len().into_py(py));

            let total_kmers: usize = counts.values().sum();
            profile.insert("total_kmers".to_string(), total_kmers.into_py(py));

            // Top k-mers
            let mut kmer_vec: Vec<(String, usize)> =
                counts.iter().map(|(k, &v)| (k.clone(), v)).collect();
            kmer_vec.sort_by(|a, b| b.1.cmp(&a.1)); // Sort by count descending

            if !kmer_vec.is_empty() {
                profile.insert(
                    "most_frequent_kmer".to_string(),
                    kmer_vec[0].clone().into_py(py),
                );

                kmer_vec.truncate(top_n);
                profile.insert("top_kmers".to_string(), kmer_vec.into_py(py));
            } else {
                profile.insert("most_frequent_kmer".to_string(), py.None());
                profile.insert(
                    "top_kmers".to_string(),
                    Vec::<(String, usize)>::new().into_py(py),
                );
            }

            // Optional full counts
            if include_counts {
                profile.insert("all_counts".to_string(), counts.clone().into_py(py));
            }

            // Diversity metrics
            if total_kmers > 0 {
                let shannon_entropy = Self::calculate_shannon_entropy(&counts, total_kmers);
                profile.insert("shannon_entropy".to_string(), shannon_entropy.into_py(py));

                let simpson_index = Self::calculate_simpson_index(&counts, total_kmers);
                profile.insert("simpson_diversity".to_string(), simpson_index.into_py(py));
            }

            Ok(profile)
        })
    }

    /// Python len() support
    pub fn __len__(&self) -> PyResult<usize> {
        self.len()
    }

    /// Check if the file is empty
    pub fn is_empty(&self) -> PyResult<bool> {
        Ok(self.len()? == 0)
    }

    /// Get the file path
    pub fn path(&self) -> &str {
        &self.path
    }

    /// Get the number of threads used for processing
    pub fn n_threads(&self) -> usize {
        self.n_threads
    }

    /// Set the number of threads for processing
    pub fn set_n_threads(&mut self, n_threads: usize) {
        self.n_threads = n_threads.max(1);
    }

    /// Get sequence length (placeholder for future implementation)
    pub fn sequence_length(&self) -> PyResult<usize> {
        if self.reader.is_some() {
            // TODO: Implement by reading the first record
            Ok(0)
        } else {
            Err(PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(
                "Reader is closed",
            ))
        }
    }

    // Context manager protocol
    /// Context manager entry
    pub fn __enter__(slf: PyRef<'_, Self>) -> PyRef<'_, Self> {
        slf
    }

    /// Context manager exit
    #[pyo3(signature = (_exc_type=None, _exc_value=None, _traceback=None))]
    pub fn __exit__(
        &mut self,
        _exc_type: Option<&Bound<'_, PyAny>>,
        _exc_value: Option<&Bound<'_, PyAny>>,
        _traceback: Option<&Bound<'_, PyAny>>,
    ) -> PyResult<bool> {
        self.close_reader();
        Ok(false)
    }

    // Iterator protocol
    /// Iterator protocol
    pub fn __iter__(slf: PyRef<'_, Self>) -> PyRef<'_, Self> {
        slf
    }

    /// Iterator next - reads actual records from the file
    pub fn __next__(&mut self) -> PyResult<BqRecord> {
        let path = self.path.clone();
        let reader = self.get_reader_mut()?;

        match reader.next_record(&path)? {
            Some(record) => {
                self.current_index += 1;
                Ok(record)
            }
            None => Err(PyErr::new::<pyo3::exceptions::PyStopIteration, _>("")),
        }
    }
}

impl BqReader {
    /// Get immutable reference to reader
    fn get_reader(&self) -> Result<&ReaderVariant, ReaderError> {
        self.reader
            .as_ref()
            .ok_or_else(|| ReaderError::Runtime("Reader is closed".to_string()))
    }

    /// Get mutable reference to reader
    fn get_reader_mut(&mut self) -> Result<&mut ReaderVariant, ReaderError> {
        self.reader
            .as_mut()
            .ok_or_else(|| ReaderError::Runtime("Reader is closed".to_string()))
    }

    /// Close the reader and clean up resources
    fn close_reader(&mut self) {
        if let Some(ref mut reader) = self.reader {
            reader.close();
        }
        self.reader = None;
    }

    /// Calculate Shannon entropy for diversity analysis
    fn calculate_shannon_entropy(counts: &HashMap<String, usize>, total: usize) -> f64 {
        let total_f = total as f64;
        counts
            .values()
            .map(|&count| {
                let p = count as f64 / total_f;
                if p > 0.0 {
                    -p * p.ln()
                } else {
                    0.0
                }
            })
            .sum()
    }

    /// Calculate Simpson's diversity index
    fn calculate_simpson_index(counts: &HashMap<String, usize>, total: usize) -> f64 {
        let total_f = total as f64;
        1.0 - counts
            .values()
            .map(|&count| {
                let p = count as f64 / total_f;
                p * p
            })
            .sum::<f64>()
    }

    /// Get the current index (for internal use)
    pub fn current_index(&self) -> usize {
        self.current_index
    }

    /// Check if reader is open
    pub fn is_open(&self) -> bool {
        self.reader.as_ref().map_or(false, |r| r.is_open())
    }

    /// Check if this is a VBQ reader
    pub fn is_vbq(&self) -> bool {
        self.is_vbq
    }
}
