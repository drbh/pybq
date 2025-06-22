use crate::core::BqRecord;
use binseq::ParallelProcessor;
use parking_lot::Mutex;
use std::sync::Arc;

/// Result of an alignment operation
#[derive(Clone, Debug)]
pub struct AlignmentResult {
    pub record_index: usize,
    pub best_position: usize,
    pub best_distance: usize,
}

/// Counter for sliding window alignment across multiple sequences
/// Finds the best alignment of a query sequence against all records in a BQ file
#[derive(Clone)]
pub struct AlignmentCounter {
    query_sequence: String,
    query_record: BqRecord,
    local_record_index: usize,
    local_best_result: Option<AlignmentResult>,
    global_best_result: Arc<Mutex<Option<AlignmentResult>>>,
    sbuf: Vec<u8>,
    xbuf: Vec<u8>,
}

impl AlignmentCounter {
    /// Create a new alignment counter for finding the best match to query_sequence
    #[must_use]
    pub fn new(query_sequence: String) -> Self {
        let query_record = BqRecord::from_sequence(query_sequence.clone());

        Self {
            query_sequence,
            query_record,
            local_record_index: 0,
            local_best_result: None,
            global_best_result: Arc::new(Mutex::new(None)),
            sbuf: Vec::new(),
            xbuf: Vec::new(),
        }
    }

    /// Get the best alignment result across all processed records
    pub fn get_best_result(&self) -> Option<AlignmentResult> {
        self.global_best_result.lock().clone()
    }

    /// Get the query sequence being aligned
    pub fn query_sequence(&self) -> &str {
        &self.query_sequence
    }

    /// Clear internal buffers
    fn clear_buffers(&mut self) {
        self.sbuf.clear();
        self.xbuf.clear();
    }

    /// Process a decoded sequence and update local best result
    fn process_sequence(&mut self, rindex: u64, sequence_bytes: &[u8]) -> binseq::Result<()> {
        // Convert bytes to string for alignment
        let sequence = String::from_utf8_lossy(sequence_bytes);

        // Create a temporary BqRecord for this sequence
        let temp_record = BqRecord::from_sequence(sequence.to_string());

        // Perform sliding window alignment
        match temp_record.sliding_window_best_match(&self.query_record) {
            Ok((position, distance)) => {
                let result = AlignmentResult {
                    // record_index: self.local_record_index,
                    record_index: rindex.try_into().unwrap_or(0),
                    best_position: position,
                    best_distance: distance,
                };

                // Update local best if this is better
                match &self.local_best_result {
                    None => self.local_best_result = Some(result),
                    Some(current_best) => {
                        if distance < current_best.best_distance {
                            self.local_best_result = Some(result);
                        }
                    }
                }
            }
            Err(_) => {
                // Alignment failed (e.g., query longer than target), skip this record
            }
        }

        self.local_record_index += 1;
        Ok(())
    }
}

impl ParallelProcessor for AlignmentCounter {
    fn process_record<R: binseq::BinseqRecord>(&mut self, record: R) -> binseq::Result<()> {
        self.clear_buffers();

        // Decode the primary sequence
        record.decode_s(&mut self.sbuf)?;

        // Get global record index
        let rindex = record.index();

        // Clone the buffer to avoid borrowing conflicts
        let sbuf_clone = self.sbuf.clone();
        self.process_sequence(rindex, &sbuf_clone)?;

        // Process the paired sequence if it exists
        if record.is_paired() {
            record.decode_x(&mut self.xbuf)?;
            let xbuf_clone = self.xbuf.clone();
            self.process_sequence(rindex, &xbuf_clone)?;
        }

        Ok(())
    }

    fn on_batch_complete(&mut self) -> binseq::Result<()> {
        // Update global best result with local best
        if let Some(ref local_best) = self.local_best_result {
            let mut global = self.global_best_result.lock();
            match global.as_ref() {
                None => *global = Some(local_best.clone()),
                Some(current_global_best) => {
                    if local_best.best_distance < current_global_best.best_distance {
                        *global = Some(local_best.clone());
                    }
                }
            }
        }

        // Reset local state for next batch
        self.local_best_result = None;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_alignment_counter_new() {
        let counter = AlignmentCounter::new("ACGT".to_string());
        assert_eq!(counter.query_sequence(), "ACGT");
        assert!(counter.get_best_result().is_none());
    }

    #[test]
    fn test_alignment_result() {
        let result = AlignmentResult {
            record_index: 5,
            best_position: 10,
            best_distance: 2,
        };
        assert_eq!(result.record_index, 5);
        assert_eq!(result.best_position, 10);
        assert_eq!(result.best_distance, 2);
    }
}
