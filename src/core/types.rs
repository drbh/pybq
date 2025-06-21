use pyo3::prelude::*;
use pyo3::types::PyBytes;
use std::collections::HashMap;

/// Represents a single BQ record with sequence data
#[pyclass]
#[derive(Clone)]
pub struct BqRecord {
    pub sequence: String,
    pub encoded: Vec<u8>,
}

#[pymethods]
impl BqRecord {
    /// Create a new BqRecord from Python
    #[new]
    pub fn py_new(sequence: String, encoded: Vec<u8>) -> Self {
        Self::new(sequence, encoded)
    }

    /// Get the decoded sequence as a string
    pub fn get_sequence(&self) -> &str {
        &self.sequence
    }

    /// Get the raw 2-bit encoded data as Python bytes
    pub fn get_encoded_sequence<'py>(&self, py: Python<'py>) -> Bound<'py, PyBytes> {
        PyBytes::new_bound(py, &self.encoded)
    }

    /// Get data pointer for zero-copy operations
    pub fn data_ptr(&self) -> usize {
        self.encoded.as_ptr() as usize
    }

    /// Get data length
    pub fn data_len(&self) -> usize {
        self.encoded.len()
    }

    /// Get shape for array operations
    pub fn shape(&self) -> (usize,) {
        (self.encoded.len(),)
    }

    /// Get strides for array operations  
    pub fn strides(&self) -> (isize,) {
        (1,)
    }

    /// Get data type string
    pub fn dtype(&self) -> &str {
        "uint8"
    }

    /// Get array interface dict for NumPy compatibility
    #[getter]
    pub fn __array_interface__<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, pyo3::types::PyDict>> {
        let dict = pyo3::types::PyDict::new_bound(py);
        dict.set_item("data", (self.data_ptr(), false))?; // (ptr, read_only)
        dict.set_item("shape", self.shape())?;
        dict.set_item("strides", self.strides())?;
        dict.set_item("typestr", "|u1")?; // Little-endian uint8
        dict.set_item("version", 3)?;
        Ok(dict)
    }

    /// Compute population count (number of 1 bits) for this record
    /// Works on the encoded sequence data
    pub fn popcnt(&self) -> u64 {
        self.encoded.iter()
            .map(|&byte| byte.count_ones() as u64)
            .sum()
    }

    /// Count k-mers in the sequence
    /// Returns a HashMap with k-mers as keys and their counts as values
    /// This is a simple, non-optimized implementation
    pub fn kmers(&self, k: usize) -> PyResult<HashMap<String, usize>> {
        if k == 0 {
            return Err(pyo3::exceptions::PyValueError::new_err(
                "k must be greater than 0"
            ));
        }

        if k > self.sequence.len() {
            return Ok(HashMap::new());
        }

        let mut kmer_counts = HashMap::new();
        
        // Simple sliding window approach
        for i in 0..=(self.sequence.len() - k) {
            let kmer = &self.sequence[i..i + k];
            *kmer_counts.entry(kmer.to_string()).or_insert(0) += 1;
        }

        Ok(kmer_counts)
    }

    /// Count k-mers and return the total number of unique k-mers
    pub fn kmer_count(&self, k: usize) -> PyResult<usize> {
        let kmers = self.kmers(k)?;
        Ok(kmers.len())
    }

    /// Get the most frequent k-mer
    pub fn most_frequent_kmer(&self, k: usize) -> PyResult<Option<(String, usize)>> {
        let kmers = self.kmers(k)?;
        
        let max_entry = kmers.iter()
            .max_by_key(|(_, &count)| count)
            .map(|(kmer, &count)| (kmer.clone(), count));
        
        Ok(max_entry)
    }
}

impl BqRecord {
    /// Create a new BqRecord (internal constructor)
    pub fn new(sequence: String, encoded: Vec<u8>) -> Self {
        Self { sequence, encoded }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_kmers_basic() {
        let record = BqRecord::new("ATCGATCG".to_string(), vec![]);
        let kmers = record.kmers(3).unwrap();
        
        let expected = vec![
            ("ATC".to_string(), 2),
            ("TCG".to_string(), 2),
            ("CGA".to_string(), 1),
            ("GAT".to_string(), 1),
        ].into_iter().collect::<HashMap<_, _>>();
        
        assert_eq!(kmers, expected);
    }

    #[test]
    fn test_kmers_empty_sequence() {
        let record = BqRecord::new("".to_string(), vec![]);
        let kmers = record.kmers(3).unwrap();
        assert!(kmers.is_empty());
    }

    #[test]
    fn test_kmers_k_too_large() {
        let record = BqRecord::new("AT".to_string(), vec![]);
        let kmers = record.kmers(5).unwrap();
        assert!(kmers.is_empty());
    }

    #[test]
    fn test_kmers_k_zero() {
        let record = BqRecord::new("ATCG".to_string(), vec![]);
        let result = record.kmers(0);
        assert!(result.is_err());
    }

    #[test]
    fn test_kmer_count() {
        let record = BqRecord::new("ATCGATCG".to_string(), vec![]);
        let count = record.kmer_count(3).unwrap();
        assert_eq!(count, 4); // 4 unique 3-mers
    }

    #[test]
    fn test_most_frequent_kmer() {
        let record = BqRecord::new("ATCGATCG".to_string(), vec![]);
        let most_frequent = record.most_frequent_kmer(3).unwrap();
        
        // Either "ATC" or "TCG" should be returned (both have count 2)
        assert!(most_frequent.is_some());
        let (kmer, count) = most_frequent.unwrap();
        assert_eq!(count, 2);
        assert!(kmer == "ATC" || kmer == "TCG");
    }

    #[test]
    fn test_most_frequent_kmer_empty() {
        let record = BqRecord::new("".to_string(), vec![]);
        let most_frequent = record.most_frequent_kmer(3).unwrap();
        assert!(most_frequent.is_none());
    }
}
