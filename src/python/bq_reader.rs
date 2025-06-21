use pyo3::prelude::*;
use std::path::Path;

use crate::core::BqRecord;
use crate::python::reader::{ReaderVariant, ReaderError};

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
            return Err(PyErr::new::<pyo3::exceptions::PyIOError, _>(
                format!("File not found: {}", path)
            ));
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
        self.reader.as_ref()
            .ok_or_else(|| ReaderError::Runtime("Reader is closed".to_string()))
    }

    /// Get mutable reference to reader
    fn get_reader_mut(&mut self) -> Result<&mut ReaderVariant, ReaderError> {
        self.reader.as_mut()
            .ok_or_else(|| ReaderError::Runtime("Reader is closed".to_string()))
    }

    /// Close the reader and clean up resources
    fn close_reader(&mut self) {
        if let Some(ref mut reader) = self.reader {
            reader.close();
        }
        self.reader = None;
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
