use std::fs::File;
use std::io::BufReader;

use binseq::{ParallelReader, BinseqRecord, ParallelProcessor};
use binseq::vbq::MmapReader as VbqReader;
use crate::core::{BqRecord, GrepCounter, PopcntCounter, RecordCounter};

// Constants for nucleotide mapping
const NUCLEOTIDE_A: u8 = 65; // ASCII 'A'
const NUCLEOTIDE_C: u8 = 67; // ASCII 'C'
const NUCLEOTIDE_G: u8 = 71; // ASCII 'G'
const NUCLEOTIDE_T: u8 = 84; // ASCII 'T'

/// Error types for better error handling
#[derive(Debug)]
pub enum ReaderError {
    Io(std::io::Error),
    Binseq(String),
    Runtime(String),
}

impl From<std::io::Error> for ReaderError {
    fn from(err: std::io::Error) -> Self {
        ReaderError::Io(err)
    }
}

/// Reader variant to eliminate branching
pub enum ReaderVariant {
    Standard {
        _reader: binseq::BinseqReader,
        stream_reader: Option<binseq::bq::StreamReader<BufReader<File>>>,
    },
    Vbq {
        reader: VbqReader,
        block: Option<binseq::vbq::RecordBlock>,
        block_position: usize,
    },
}

impl ReaderVariant {
    /// Create a new reader variant based on format
    pub fn new(path: &str, is_vbq: bool) -> Result<Self, ReaderError> {
        if is_vbq {
            let reader = VbqReader::new(path)
                .map_err(|e| ReaderError::Binseq(format!("Failed to open VBQ file: {}", e)))?;
            Ok(ReaderVariant::Vbq {
                reader,
                block: None,
                block_position: 0,
            })
        } else {
            let reader = binseq::BinseqReader::new(path)
                .map_err(|e| ReaderError::Binseq(format!("Failed to open BQ file: {}", e)))?;
            Ok(ReaderVariant::Standard {
                _reader: reader,
                stream_reader: None,
            })
        }
    }

    /// Count total records using parallel processing
    pub fn count_records(&self, path: &str, n_threads: usize) -> Result<usize, ReaderError> {
        let counter = RecordCounter::new();
        self.process_parallel(path, counter.clone(), n_threads)?;
        Ok(counter.count())
    }

    /// Count pattern matches using parallel processing
    pub fn count_matches(&self, path: &str, pattern: &[u8], n_threads: usize) -> Result<usize, ReaderError> {
        let counter = GrepCounter::new(pattern);
        self.process_parallel(path, counter.clone(), n_threads)?;
        Ok(counter.count())
    }

    /// Count population bits using parallel processing
    pub fn count_popbits(&self, path: &str, n_threads: usize) -> Result<u64, ReaderError> {
        let counter = PopcntCounter::new();
        self.process_parallel(path, counter.clone(), n_threads)?;
        Ok(counter.total_count())
    }

    /// Generic parallel processing method
    fn process_parallel<T>(&self, path: &str, processor: T, n_threads: usize) -> Result<(), ReaderError>
    where
        T: ParallelProcessor + Clone + 'static,
    {
        match self {
            ReaderVariant::Standard { .. } => {
                let reader = binseq::BinseqReader::new(path)
                    .map_err(|e| ReaderError::Binseq(format!("Failed to open BQ file: {}", e)))?;
                reader.process_parallel(processor, n_threads)
                    .map_err(|e| ReaderError::Runtime(format!("Processing failed: {}", e)))?;
            }
            ReaderVariant::Vbq { .. } => {
                let reader = VbqReader::new(path)
                    .map_err(|e| ReaderError::Binseq(format!("Failed to open VBQ file: {}", e)))?;
                reader.process_parallel(processor, n_threads)
                    .map_err(|e| ReaderError::Runtime(format!("Processing failed: {}", e)))?;
            }
        }
        Ok(())
    }

    /// Get next record from the appropriate reader
    pub fn next_record(&mut self, path: &str) -> Result<Option<BqRecord>, ReaderError> {
        match self {
            ReaderVariant::Standard { stream_reader, .. } => {
                // Initialize stream reader if needed
                if stream_reader.is_none() {
                    *stream_reader = Some(Self::create_stream_reader(path)?);
                }

                if let Some(ref mut reader) = stream_reader {
                    match reader.next_record() {
                        Some(Ok(record)) => Ok(Some(Self::decode_record(record)?)),
                        Some(Err(e)) => Err(ReaderError::Runtime(format!("Error reading record: {}", e))),
                        None => Ok(None),
                    }
                } else {
                    Err(ReaderError::Runtime("Stream reader not initialized".to_string()))
                }
            }
            ReaderVariant::Vbq { reader, block, block_position } => {
                // Initialize block if needed
                if block.is_none() {
                    let mut new_block = reader.new_block();
                    let has_data = reader.read_block_into(&mut new_block)
                        .map_err(|e| ReaderError::Runtime(format!("Failed to read VBQ block: {}", e)))?;
                    
                    if !has_data || new_block.n_records() == 0 {
                        return Ok(None);
                    }
                    
                    *block = Some(new_block);
                    *block_position = 0;
                }

                if let Some(ref mut current_block) = block {
                    // Check if we need to load the next block
                    if *block_position >= current_block.n_records() {
                        let has_data = reader.read_block_into(current_block)
                            .map_err(|e| ReaderError::Runtime(format!("Failed to read VBQ block: {}", e)))?;
                        
                        if !has_data || current_block.n_records() == 0 {
                            return Ok(None);
                        }
                        *block_position = 0;
                    }

                    // Get record from current block
                    if let Some(record) = current_block.iter().nth(*block_position) {
                        *block_position += 1;
                        Ok(Some(Self::decode_vbq_record(record)?))
                    } else {
                        Err(ReaderError::Runtime("VBQ record not found".to_string()))
                    }
                } else {
                    Err(ReaderError::Runtime("VBQ block not initialized".to_string()))
                }
            }
        }
    }

    /// Create a stream reader for standard BQ files
    fn create_stream_reader(path: &str) -> Result<binseq::bq::StreamReader<BufReader<File>>, ReaderError> {
        let file = File::open(path)?;
        let buf_reader = BufReader::new(file);
        let mut stream_reader = binseq::bq::StreamReader::new(buf_reader);
        
        stream_reader.read_header()
            .map_err(|e| ReaderError::Runtime(format!("Failed to read BQ header: {}", e)))?;
        
        Ok(stream_reader)
    }

    /// Decode a standard BQ record
    fn decode_record(record: impl BinseqRecord) -> Result<BqRecord, ReaderError> {
        let mut sbuf = Vec::new();
        record.decode_s(&mut sbuf)
            .map_err(|e| ReaderError::Runtime(format!("Failed to decode record: {}", e)))?;
        
        let sequence = Self::bytes_to_sequence(&sbuf);
        Ok(BqRecord::new(sequence, sbuf))
    }

    /// Decode a VBQ record
    fn decode_vbq_record(record: impl BinseqRecord) -> Result<BqRecord, ReaderError> {
        let mut sbuf = Vec::new();
        record.decode_s(&mut sbuf)
            .map_err(|e| ReaderError::Runtime(format!("Failed to decode VBQ record: {}", e)))?;
        
        let sequence = Self::bytes_to_sequence(&sbuf);
        Ok(BqRecord::new(sequence, sbuf))
    }

    /// Convert ASCII bytes to nucleotide sequence
    fn bytes_to_sequence(bytes: &[u8]) -> String {
        bytes.iter().map(|&b| match b {
            NUCLEOTIDE_A => 'A',
            NUCLEOTIDE_C => 'C',
            NUCLEOTIDE_G => 'G',
            NUCLEOTIDE_T => 'T',
            _ => 'N',
        }).collect()
    }

    /// Check if the reader is open
    pub fn is_open(&self) -> bool {
        match self {
            ReaderVariant::Standard { .. } => true,
            ReaderVariant::Vbq { .. } => true,
        }
    }

    /// Clean up resources
    pub fn close(&mut self) {
        match self {
            ReaderVariant::Standard { stream_reader, .. } => {
                *stream_reader = None;
            }
            ReaderVariant::Vbq { block, .. } => {
                *block = None;
            }
        }
    }
}
