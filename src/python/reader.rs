use pyo3::prelude::*;
use std::path::Path;
use std::fs::File;
use std::io::BufReader;

use binseq::{ParallelReader, BinseqRecord};
use crate::core::{BqRecord, GrepCounter, RecordCounter};

/// BQ file reader with Python integration
/// Supports context manager protocol and iteration
#[pyclass]
pub struct BqReader {
    reader: Option<binseq::BinseqReader>,
    path: String,
    current_index: usize,
    n_threads: usize,
    stream_reader: Option<binseq::bq::StreamReader<BufReader<File>>>,
}

#[pymethods]
impl BqReader {
    /// Create a new BQ reader for the given file path
    #[new]
    #[pyo3(signature = (path, n_threads=1))]
    pub fn new(path: &str, n_threads: Option<usize>) -> PyResult<Self> {
        let n_threads = n_threads.unwrap_or(1);
        
        // Validate file exists
        let path_obj = Path::new(path);
        if !path_obj.exists() {
            return Err(PyErr::new::<pyo3::exceptions::PyIOError, _>(format!(
                "File not found: {}",
                path
            )));
        }

        // Try to open with binseq
        let reader = binseq::BinseqReader::new(path).map_err(|e| {
            PyErr::new::<pyo3::exceptions::PyIOError, _>(format!("Failed to open BQ file: {}", e))
        })?;

        Ok(BqReader {
            reader: Some(reader),
            path: path.to_string(),
            current_index: 0,
            n_threads,
            stream_reader: None,
        })
    }

    /// Get total number of records in the file
    pub fn len(&self) -> PyResult<usize> {
        // Create a new reader for counting
        let reader = binseq::BinseqReader::new(&self.path).map_err(|e| {
            PyErr::new::<pyo3::exceptions::PyIOError, _>(format!("Failed to open BQ file: {}", e))
        })?;
        
        let counter = RecordCounter::new();
        reader.process_parallel(counter.clone(), self.n_threads).map_err(|e| {
            PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!("Processing failed: {}", e))
        })?;
        
        Ok(counter.count())
    }

    /// Count records matching a specific pattern
    pub fn count_matches(&self, pattern: &str) -> PyResult<usize> {
        let pattern_bytes = pattern.as_bytes();
        
        // Create a new reader for counting
        let reader = binseq::BinseqReader::new(&self.path).map_err(|e| {
            PyErr::new::<pyo3::exceptions::PyIOError, _>(format!("Failed to open BQ file: {}", e))
        })?;
        
        let counter = GrepCounter::new(pattern_bytes);
        reader.process_parallel(counter.clone(), self.n_threads).map_err(|e| {
            PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!("Processing failed: {}", e))
        })?;
        
        Ok(counter.count())
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
        self.n_threads = n_threads.max(1); // Ensure at least 1 thread
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
        self.reader = None;
        Ok(false)
    }

    // Iterator protocol
    /// Iterator protocol
    pub fn __iter__(slf: PyRef<'_, Self>) -> PyRef<'_, Self> {
        slf
    }

    /// Iterator next - reads actual records from the file
    pub fn __next__(&mut self) -> PyResult<BqRecord> {
        // Initialize stream reader on first call
        if self.stream_reader.is_none() {
            self.init_stream_reader()?;
        }
        
        if let Some(ref mut stream_reader) = self.stream_reader {
            match stream_reader.next_record() {
                Some(Ok(record)) => {
                    // Decode the record
                    let mut sbuf = Vec::new();
                    if let Err(e) = record.decode_s(&mut sbuf) {
                        return Err(PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(
                            format!("Failed to decode record: {}", e)
                        ));
                    }
                    
                    // Convert ASCII bytes to nucleotide string
                    let sequence = sbuf.iter().map(|&b| match b {
                        65 => 'A',  // ASCII 'A'
                        67 => 'C',  // ASCII 'C'
                        71 => 'G',  // ASCII 'G'
                        84 => 'T',  // ASCII 'T'
                        _ => 'N',   // Unknown
                    }).collect::<String>();
                    
                    // Create BqRecord with the decoded data
                    let bq_record = BqRecord::new(sequence, sbuf);
                    self.current_index += 1;
                    Ok(bq_record)
                },
                Some(Err(e)) => {
                    Err(PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(
                        format!("Error reading record: {}", e)
                    ))
                },
                None => {
                    Err(PyErr::new::<pyo3::exceptions::PyStopIteration, _>(""))
                }
            }
        } else {
            Err(PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(
                "Stream reader not initialized"
            ))
        }
    }
}

impl BqReader {
    /// Initialize the stream reader for iteration
    fn init_stream_reader(&mut self) -> PyResult<()> {
        let file = File::open(&self.path).map_err(|e| {
            PyErr::new::<pyo3::exceptions::PyIOError, _>(format!("Failed to open file: {}", e))
        })?;
        
        let buf_reader = BufReader::new(file);
        let mut stream_reader = binseq::bq::StreamReader::new(buf_reader);
        
        // Read and validate header
        match stream_reader.read_header() {
            Ok(_header) => {
                self.stream_reader = Some(stream_reader);
                Ok(())
            },
            Err(e) => {
                Err(PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(
                    format!("Failed to read header: {}", e)
                ))
            }
        }
    }

    /// Get the current index (for internal use)
    pub fn current_index(&self) -> usize {
        self.current_index
    }

    /// Check if reader is open
    pub fn is_open(&self) -> bool {
        self.reader.is_some()
    }
}
