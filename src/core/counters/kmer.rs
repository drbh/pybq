use std::collections::HashMap;
use std::sync::Arc;
use binseq::ParallelProcessor;
use parking_lot::Mutex;

/// Counter for k-mer analysis across multiple sequences
/// Counts k-mers in parallel across all records in a BQ file
#[derive(Clone)]
pub struct KmerCounter {
    k: usize,
    local_kmers: HashMap<String, usize>,
    global_kmers: Arc<Mutex<HashMap<String, usize>>>,
    sbuf: Vec<u8>,
    xbuf: Vec<u8>,
}

impl KmerCounter {
    /// Create a new k-mer counter for k-mers of length k
    #[must_use]
    pub fn new(k: usize) -> Self {
        Self {
            k,
            local_kmers: HashMap::new(),
            global_kmers: Arc::new(Mutex::new(HashMap::new())),
            sbuf: Vec::new(),
            xbuf: Vec::new(),
        }
    }

    /// Count k-mers in a sequence (simple implementation)
    /// This is a helper method for testing
    #[allow(dead_code)]
    fn count_kmers_in_sequence(&mut self, sequence: &[u8]) {
        if self.k == 0 || self.k > sequence.len() {
            return;
        }

        // Convert to string for simplicity (not optimized)
        let seq_str = String::from_utf8_lossy(sequence);
        
        // Count k-mers using sliding window
        for i in 0..=(seq_str.len() - self.k) {
            let kmer = &seq_str[i..i + self.k];
            *self.local_kmers.entry(kmer.to_string()).or_insert(0) += 1;
        }
    }

    /// Get the total k-mer counts across all processed records
    pub fn get_counts(&self) -> HashMap<String, usize> {
        self.global_kmers.lock().clone()
    }

    /// Get the number of unique k-mers found
    pub fn unique_kmer_count(&self) -> usize {
        self.global_kmers.lock().len()
    }

    /// Get the total number of k-mers processed
    pub fn total_kmer_count(&self) -> usize {
        self.global_kmers.lock().values().sum()
    }

    /// Get the most frequent k-mer
    pub fn most_frequent_kmer(&self) -> Option<(String, usize)> {
        self.global_kmers.lock()
            .iter()
            .max_by_key(|(_, &count)| count)
            .map(|(kmer, &count)| (kmer.clone(), count))
    }

    /// Clear internal buffers
    fn clear_buffers(&mut self) {
        self.sbuf.clear();
        self.xbuf.clear();
    }
}

impl ParallelProcessor for KmerCounter {
    fn process_record<R: binseq::BinseqRecord>(&mut self, record: R) -> binseq::Result<()> {
        self.clear_buffers();

        // Decode the primary sequence
        record.decode_s(&mut self.sbuf)?;
        
        // Process the primary sequence - split the borrow to avoid conflicts
        if self.k > 0 && self.k <= self.sbuf.len() {
            let seq_str = String::from_utf8_lossy(&self.sbuf);
            for i in 0..=(seq_str.len() - self.k) {
                let kmer = &seq_str[i..i + self.k];
                *self.local_kmers.entry(kmer.to_string()).or_insert(0) += 1;
            }
        }

        // Decode the paired sequence if it exists
        if record.is_paired() {
            record.decode_x(&mut self.xbuf)?;
            
            // Process the paired sequence
            if self.k > 0 && self.k <= self.xbuf.len() {
                let seq_str = String::from_utf8_lossy(&self.xbuf);
                for i in 0..=(seq_str.len() - self.k) {
                    let kmer = &seq_str[i..i + self.k];
                    *self.local_kmers.entry(kmer.to_string()).or_insert(0) += 1;
                }
            }
        }

        Ok(())
    }

    fn on_batch_complete(&mut self) -> binseq::Result<()> {
        // Merge local k-mer counts with global counts
        let mut global = self.global_kmers.lock();
        for (kmer, count) in self.local_kmers.drain() {
            *global.entry(kmer).or_insert(0) += count;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_kmer_counter_basic() {
        let mut counter = KmerCounter::new(3);
        let sequence = b"ATCGATCG";
        
        counter.count_kmers_in_sequence(sequence);
        
        // Check local counts before batch completion
        assert_eq!(counter.local_kmers.get("ATC"), Some(&2));
        assert_eq!(counter.local_kmers.get("TCG"), Some(&2));
        assert_eq!(counter.local_kmers.get("CGA"), Some(&1));
        assert_eq!(counter.local_kmers.get("GAT"), Some(&1));
    }

    #[test]
    fn test_kmer_counter_empty() {
        let mut counter = KmerCounter::new(3);
        let sequence = b"";
        
        counter.count_kmers_in_sequence(sequence);
        assert!(counter.local_kmers.is_empty());
    }

    #[test]
    fn test_kmer_counter_k_too_large() {
        let mut counter = KmerCounter::new(10);
        let sequence = b"ATCG";
        
        counter.count_kmers_in_sequence(sequence);
        assert!(counter.local_kmers.is_empty());
    }
}
