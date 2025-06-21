use pyo3::prelude::*;
use std::path::Path;
use std::fs::File;
use std::io::BufReader;

use binseq::{ParallelReader, BinseqRecord};
use binseq::vbq::MmapReader as VbqReader;
use crate::core::{BqRecord, GrepCounter, PopcntCounter, RecordCounter};

/// BQ file reader with Python integration
/// Supports context manager protocol and iteration
#[pyclass]
pub struct BqReader {
    reader: Option<binseq::BinseqReader>,
    vbq_reader: Option<VbqReader>,
    path: String,
    current_index: usize,
    n_threads: usize,
    stream_reader: Option<binseq::bq::StreamReader<BufReader<File>>>,
    vbq_block: Option<binseq::vbq::RecordBlock>,
    vbq_block_position: usize,
    is_vbq: bool,
}

#[pymethods]
impl BqReader {
    /// Create a new BQ reader for the given file path
    #[new]
    #[pyo3(signature = (path, n_threads=1, is_vbq=false))]
    pub fn new(path: &str, n_threads: Option<usize>, is_vbq: bool) -> PyResult<Self> {
        println!("Creating BQ reader for path: {}", path);
        println!("Using {} threads", n_threads.unwrap_or(1));
        
        let n_threads = n_threads.unwrap_or(1);
        
        // Validate file exists
        let path_obj = Path::new(path);
        if !path_obj.exists() {
            return Err(PyErr::new::<pyo3::exceptions::PyIOError, _>(format!(
                "File not found: {}",
                path
            )));
        }

        if is_vbq {
            // Open with VBQ reader
            let vbq_reader = VbqReader::new(path).map_err(|e| {
                PyErr::new::<pyo3::exceptions::PyIOError, _>(format!("Failed to open VBQ file: {}", e))
            })?;

            Ok(BqReader {
                reader: None,
                vbq_reader: Some(vbq_reader),
                path: path.to_string(),
                current_index: 0,
                n_threads,
                stream_reader: None,
                vbq_block: None,
                vbq_block_position: 0,
                is_vbq,
            })
        } else {
            // Open with standard BQ reader
            let reader = binseq::BinseqReader::new(path).map_err(|e| {
                PyErr::new::<pyo3::exceptions::PyIOError, _>(format!("Failed to open BQ file: {}", e))
            })?;

            Ok(BqReader {
                reader: Some(reader),
                vbq_reader: None,
                path: path.to_string(),
                current_index: 0,
                n_threads,
                stream_reader: None,
                vbq_block: None,
                vbq_block_position: 0,
                is_vbq,
            })
        }
    }

    /// Get total number of records in the file
    pub fn len(&self) -> PyResult<usize> {
        if self.is_vbq {
            // VBQ implementation
            let vbq_reader = VbqReader::new(&self.path).map_err(|e| {
                PyErr::new::<pyo3::exceptions::PyIOError, _>(format!("Failed to open VBQ file: {}", e))
            })?;
            
            let counter = RecordCounter::new();
            vbq_reader.process_parallel(counter.clone(), self.n_threads).map_err(|e| {
                PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!("Processing failed: {}", e))
            })?;
            
            Ok(counter.count())
        } else {
            // Regular BQ implementation
            let reader = binseq::BinseqReader::new(&self.path).map_err(|e| {
                PyErr::new::<pyo3::exceptions::PyIOError, _>(format!("Failed to open BQ file: {}", e))
            })?;
            
            let counter = RecordCounter::new();
            reader.process_parallel(counter.clone(), self.n_threads).map_err(|e| {
                PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!("Processing failed: {}", e))
            })?;
            
            Ok(counter.count())
        }
    }

    /// Count records matching a specific pattern
    pub fn count_matches(&self, pattern: &str) -> PyResult<usize> {
        let pattern_bytes = pattern.as_bytes();
        
        if self.is_vbq {
            // VBQ implementation
            let vbq_reader = VbqReader::new(&self.path).map_err(|e| {
                PyErr::new::<pyo3::exceptions::PyIOError, _>(format!("Failed to open VBQ file: {}", e))
            })?;
            
            let counter = GrepCounter::new(pattern_bytes);
            vbq_reader.process_parallel(counter.clone(), self.n_threads).map_err(|e| {
                PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!("Processing failed: {}", e))
            })?;
            
            Ok(counter.count())
        } else {
            // Regular BQ implementation
            let reader = binseq::BinseqReader::new(&self.path).map_err(|e| {
                PyErr::new::<pyo3::exceptions::PyIOError, _>(format!("Failed to open BQ file: {}", e))
            })?;
            
            let counter = GrepCounter::new(pattern_bytes);
            reader.process_parallel(counter.clone(), self.n_threads).map_err(|e| {
                PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!("Processing failed: {}", e))
            })?;
            
            Ok(counter.count())
        }
    }

    /// Compute population count (number of 1 bits) across all sequences
    /// Returns the total number of 1 bits in the 2-bit encoded sequence data
    pub fn popcnt(&self) -> PyResult<u64> {
        if self.is_vbq {
            // VBQ implementation
            let vbq_reader = VbqReader::new(&self.path).map_err(|e| {
                PyErr::new::<pyo3::exceptions::PyIOError, _>(format!("Failed to open VBQ file: {}", e))
            })?;
            
            let counter = PopcntCounter::new();
            vbq_reader.process_parallel(counter.clone(), self.n_threads).map_err(|e| {
                PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!("Processing failed: {}", e))
            })?;
            
            Ok(counter.total_count())
        } else {
            // Regular BQ implementation
            let reader = binseq::BinseqReader::new(&self.path).map_err(|e| {
                PyErr::new::<pyo3::exceptions::PyIOError, _>(format!("Failed to open BQ file: {}", e))
            })?;
            
            let counter = PopcntCounter::new();
            reader.process_parallel(counter.clone(), self.n_threads).map_err(|e| {
                PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!("Processing failed: {}", e))
            })?;
            
            Ok(counter.total_count())
        }
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
        // Clean up resources
        self.reader = None;
        self.vbq_reader = None;
        self.stream_reader = None;
        self.vbq_block = None;
        Ok(false)
    }

    // Iterator protocol
    /// Iterator protocol
    pub fn __iter__(slf: PyRef<'_, Self>) -> PyRef<'_, Self> {
        slf
    }

    /// Iterator next - reads actual records from the file
    pub fn __next__(&mut self) -> PyResult<BqRecord> {
        if self.is_vbq {
            // VBQ reading
            if self.vbq_block.is_none() {
                self.init_vbq_block()?;
            }
            
            if let Some(ref mut block) = self.vbq_block {
                // If we've reached the end of the current block, try to load another
                if self.vbq_block_position >= block.n_records() {
                    // Reset block position
                    self.vbq_block_position = 0;
                    
                    // Read next block
                    let vbq_reader = self.vbq_reader.as_mut().ok_or_else(|| {
                        PyErr::new::<pyo3::exceptions::PyRuntimeError, _>("VBQ reader not initialized")
                    })?;
                    
                    // Clear block and read next chunk
                    let more_data = vbq_reader.read_block_into(block).map_err(|e| {
                        PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!("Failed to read VBQ block: {}", e))
                    })?;
                    
                    if !more_data || block.n_records() == 0 {
                        return Err(PyErr::new::<pyo3::exceptions::PyStopIteration, _>(""));
                    }
                }
                
                // Get the record from the current block
                let record = block.iter().nth(self.vbq_block_position).ok_or_else(|| {
                    PyErr::new::<pyo3::exceptions::PyRuntimeError, _>("VBQ record not found")
                })?;
                self.vbq_block_position += 1;
                
                // Decode the record
                let mut sbuf = Vec::new();
                if let Err(e) = record.decode_s(&mut sbuf) {
                    return Err(PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(
                        format!("Failed to decode VBQ record: {}", e)
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
            } else {
                Err(PyErr::new::<pyo3::exceptions::PyRuntimeError, _>("VBQ block not initialized"))
            }
        } else {
            // Standard BQ reading
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
                    format!("Failed to read BQ header: {}", e)
                ))
            }
        }
    }
    
    /// Initialize the VBQ block for iteration
    fn init_vbq_block(&mut self) -> PyResult<()> {
        // Create a new VBQ reader if not already available
        if self.vbq_reader.is_none() {
            let vbq_reader = VbqReader::new(&self.path).map_err(|e| {
                PyErr::new::<pyo3::exceptions::PyIOError, _>(format!("Failed to open VBQ file: {}", e))
            })?;
            self.vbq_reader = Some(vbq_reader);
        }
        
        // Create a new block for reading
        let vbq_reader = self.vbq_reader.as_mut().ok_or_else(|| {
            PyErr::new::<pyo3::exceptions::PyRuntimeError, _>("VBQ reader not initialized")
        })?;
        
        // Create a new block and read the first chunk
        let mut block = vbq_reader.new_block();
        let more_data = vbq_reader.read_block_into(&mut block).map_err(|e| {
            PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!("Failed to read VBQ block: {}", e))
        })?;
        
        if !more_data || block.n_records() == 0 {
            return Err(PyErr::new::<pyo3::exceptions::PyRuntimeError, _>("VBQ file is empty"));
        }
        
        self.vbq_block = Some(block);
        self.vbq_block_position = 0;
        
        Ok(())
    }

    /// Get the current index (for internal use)
    pub fn current_index(&self) -> usize {
        self.current_index
    }

    /// Check if reader is open
    pub fn is_open(&self) -> bool {
        if self.is_vbq {
            self.vbq_reader.is_some()
        } else {
            self.reader.is_some()
        }
    }
    
    /// Check if this is a VBQ reader
    pub fn is_vbq(&self) -> bool {
        self.is_vbq
    }
}
