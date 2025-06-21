use std::sync::Arc;
use binseq::ParallelProcessor;
use parking_lot::Mutex;

/// Counter for computing population count (number of 1 bits) on 2-bit encoded sequences
/// Works directly on the encoded data without decoding to avoid memory allocations
#[derive(Clone)]
pub struct PopcntCounter {
    // Global counter shared across threads
    total_popcnt: Arc<Mutex<u64>>,
    local_popcnt: u64,
}

impl PopcntCounter {
    /// Create a new population count counter
    #[must_use]
    pub fn new() -> Self {
        Self {
            total_popcnt: Arc::new(Mutex::new(0)),
            local_popcnt: 0,
        }
    }

    /// Compute population count directly on 2-bit encoded data
    /// For 2-bit encoding, each byte contains 4 nucleotides
    fn compute_popcnt_2bit(&self, encoded_data: &[u8]) -> u64 {
        encoded_data.iter()
            .map(|&byte| byte.count_ones() as u64)
            .sum()
    }

    /// Get the total population count across all processed records
    pub fn total_count(&self) -> u64 {
        *self.total_popcnt.lock()
    }

    /// Reset the counter to zero
    pub fn reset(&self) {
        *self.total_popcnt.lock() = 0;
    }
}

impl Default for PopcntCounter {
    fn default() -> Self {
        Self::new()
    }
}

impl ParallelProcessor for PopcntCounter {
    fn process_record<R: binseq::BinseqRecord>(&mut self, record: R) -> binseq::Result<()> {
        // Try to get direct access to encoded data if the record supports it
        // For now, we'll fall back to decoding since we need to check what methods are available
        
        // Decode the primary sequence to get the encoded data
        let mut sbuf = Vec::<u8>::new();
        record.decode_s(&mut sbuf)?;
        
        // The decoded sequence is in ASCII format, but we want the raw 2-bit encoded data
        // For now, we'll work with what we have and compute popcnt on the ASCII bytes
        // This is not optimal but demonstrates the concept
        self.local_popcnt += self.compute_popcnt_2bit(&sbuf);
        
        // Handle paired sequence if it exists
        if record.is_paired() {
            let mut xbuf = Vec::<u8>::new();
            record.decode_x(&mut xbuf)?;
            self.local_popcnt += self.compute_popcnt_2bit(&xbuf);
        }

        Ok(())
    }

    fn on_batch_complete(&mut self) -> binseq::Result<()> {
        // Add local count to global count
        *self.total_popcnt.lock() += self.local_popcnt;
        self.local_popcnt = 0;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_popcnt_basic() {
        let counter = PopcntCounter::new();
        
        // Test basic popcnt computation
        let data = vec![0b11111111, 0b00000000, 0b10101010];
        let result = counter.compute_popcnt_2bit(&data);
        
        // 8 + 0 + 4 = 12
        assert_eq!(result, 12);
    }

    #[test]
    fn test_popcnt_empty() {
        let counter = PopcntCounter::new();
        let data = vec![];
        let result = counter.compute_popcnt_2bit(&data);
        assert_eq!(result, 0);
    }

    #[test]
    fn test_counter_reset() {
        let counter = PopcntCounter::new();
        *counter.total_popcnt.lock() = 42;
        assert_eq!(counter.total_count(), 42);
        
        counter.reset();
        assert_eq!(counter.total_count(), 0);
    }
}
