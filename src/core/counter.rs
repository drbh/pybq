use std::sync::Arc;
use binseq::ParallelProcessor;
use memchr::memmem::Finder;
use parking_lot::Mutex;

/// Counter for parallel processing of BQ sequences
/// Counts sequences matching a given pattern using multiple threads
#[derive(Clone)]
pub struct GrepCounter {
    // Thread-local variables
    sbuf: Vec<u8>,
    xbuf: Vec<u8>,
    local_count: usize,

    // Search pattern stored as bytes
    pattern: Vec<u8>,

    // Global counter shared across threads
    count: Arc<Mutex<usize>>,
}

impl GrepCounter {
    /// Create a new counter for the given pattern
    #[must_use]
    pub fn new(pattern: &[u8]) -> Self {
        Self {
            sbuf: Vec::new(),
            xbuf: Vec::new(),
            pattern: pattern.to_vec(),
            local_count: 0,
            count: Arc::new(Mutex::new(0)),
        }
    }

    /// Check if a sequence matches the pattern
    fn match_sequence(&self, seq: &[u8]) -> bool {
        let finder = Finder::new(&self.pattern);
        finder.find(seq).is_some()
    }

    /// Clear internal buffers for reuse
    fn clear_buffers(&mut self) {
        self.sbuf.clear();
        self.xbuf.clear();
    }

    /// Get the total count of matching sequences
    pub fn count(&self) -> usize {
        *self.count.lock()
    }
}

impl ParallelProcessor for GrepCounter {
    fn process_record<R: binseq::BinseqRecord>(&mut self, record: R) -> binseq::Result<()> {
        self.clear_buffers();

        // Decode the primary sequence
        record.decode_s(&mut self.sbuf)?;
        
        // Decode the paired sequence if it exists
        if record.is_paired() {
            record.decode_x(&mut self.xbuf)?;
        }

        // Check if either sequence matches the pattern
        if self.match_sequence(&self.sbuf) || self.match_sequence(&self.xbuf) {
            self.local_count += 1;
        }

        Ok(())
    }

    fn on_batch_complete(&mut self) -> binseq::Result<()> {
        // Add local count to global count
        *self.count.lock() += self.local_count;
        self.local_count = 0;
        Ok(())
    }
}
