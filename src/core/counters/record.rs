use std::sync::Arc;
use binseq::ParallelProcessor;
use parking_lot::Mutex;

/// Simple counter for counting all records in a BQ file
#[derive(Clone)]
pub struct RecordCounter {
    local_count: usize,
    count: Arc<Mutex<usize>>,
}

impl RecordCounter {
    /// Create a new record counter
    #[must_use]
    pub fn new() -> Self {
        Self {
            local_count: 0,
            count: Arc::new(Mutex::new(0)),
        }
    }

    /// Get the total count of records
    pub fn count(&self) -> usize {
        *self.count.lock()
    }
}

impl Default for RecordCounter {
    fn default() -> Self {
        Self::new()
    }
}

impl ParallelProcessor for RecordCounter {
    fn process_record<R: binseq::BinseqRecord>(&mut self, _record: R) -> binseq::Result<()> {
        // Simply count each record
        self.local_count += 1;
        Ok(())
    }

    fn on_batch_complete(&mut self) -> binseq::Result<()> {
        // Add local count to global count
        *self.count.lock() += self.local_count;
        self.local_count = 0;
        Ok(())
    }
}
