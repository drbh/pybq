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

    /// Create a BqRecord from a string sequence (2-bit encoding is done internally)
    #[staticmethod]
    pub fn from_sequence(sequence: String) -> Self {
        // Simple 2-bit encoding: A=00, C=01, G=10, T=11
        let mut encoded = Vec::with_capacity((sequence.len() + 3) / 4);
        let mut byte = 0u8;
        for (i, base) in sequence.bytes().enumerate() {
            let bits = match base {
                b'A' | b'a' => 0b00,
                b'C' | b'c' => 0b01,
                b'G' | b'g' => 0b10,
                b'T' | b't' => 0b11,
                _ => 0b00, // Treat unknowns as 'A'
            };
            byte |= bits << (6 - 2 * (i % 4));
            if i % 4 == 3 {
                encoded.push(byte);
                byte = 0;
            }
        }
        if sequence.len() % 4 != 0 {
            encoded.push(byte);
        }
        Self::new(sequence, encoded)
    }

    /// Get the decoded sequence as a string
    pub fn get_sequence(&self) -> &str {
        &self.sequence
    }

    /// Calculate hamming distance to another BqRecord
    pub fn hamming_distance(&self, other: &BqRecord) -> PyResult<usize> {
        if self.sequence.len() != other.sequence.len() {
            return Err(pyo3::exceptions::PyValueError::new_err(
                "Sequences must be of equal length to compute Hamming distance",
            ));
        }
        // Use XOR to find differing bits
        let distance = self
            .encoded
            .iter()
            .zip(other.encoded.iter())
            .map(|(&a, &b)| (a ^ b).count_ones() as usize)
            .sum();
        Ok(distance)
    }

    // /// Sliding window best match with another BqRecord
    // pub fn sliding_window_best_match(&self, other: &BqRecord) -> PyResult<(usize, usize)> {
    //     let len_self = self.sequence.len();
    //     let len_other = other.sequence.len();
    //     // the other sequence must be shorter than self
    //     if len_other > len_self {
    //         return Err(pyo3::exceptions::PyValueError::new_err(
    //             "Other sequence must be shorter than or equal to self for sliding window match",
    //         ));
    //     }
    //     let mut best_distance = usize::MAX;
    //     let mut best_position = 0;
    //     for i in 0..=(len_self - len_other) {
    //         let window = &self.sequence[i..i + len_other];
    //         let distance = window
    //             .bytes()
    //             .zip(other.sequence.bytes())
    //             .filter(|(a, b)| a != b)
    //             .count();
    //         if distance < best_distance {
    //             best_distance = distance;
    //             best_position = i;
    //         }
    //     }
    //     Ok((best_position, best_distance))
    // }

    /// Sliding window best match with another BqRecord
    pub fn sliding_window_best_match(&self, other: &BqRecord) -> PyResult<(usize, usize)> {
        let (longer_seq, shorter_seq, flip_result) = if self.sequence.len() >= other.sequence.len()
        {
            (&self.sequence, &other.sequence, false)
        } else {
            (&other.sequence, &self.sequence, true)
        };

        let len_longer = longer_seq.len();
        let len_shorter = shorter_seq.len();

        let mut best_distance = usize::MAX;
        let mut best_position = 0;

        for i in 0..=(len_longer - len_shorter) {
            let window = &longer_seq[i..i + len_shorter];
            let distance = window
                .bytes()
                .zip(shorter_seq.bytes())
                .filter(|(a, b)| a != b)
                .count();

            if distance < best_distance {
                best_distance = distance;
                best_position = i;
            }
        }

        // If we flipped the roles, return the position as if self was the shorter one
        Ok(if flip_result {
            (0, best_distance) // best position isn't meaningful if self is shorter
        } else {
            (best_position, best_distance)
        })
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
    pub fn __array_interface__<'py>(
        &self,
        py: Python<'py>,
    ) -> PyResult<Bound<'py, pyo3::types::PyDict>> {
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
        self.encoded
            .iter()
            .map(|&byte| byte.count_ones() as u64)
            .sum()
    }

    /// Count k-mers in the sequence (optimized for speed)
    /// Returns a HashMap with k-mers as keys and their counts as values
    /// Uses byte slices internally and only converts to strings at the end
    pub fn kmers(&self, k: usize) -> PyResult<HashMap<String, usize>> {
        if k == 0 {
            return Err(pyo3::exceptions::PyValueError::new_err(
                "k must be greater than 0",
            ));
        }

        if k > self.sequence.len() {
            return Ok(HashMap::new());
        }

        // Use byte slices as keys to avoid string allocations during counting
        let mut kmer_counts: HashMap<&[u8], usize> = HashMap::with_capacity(
            // Rough estimate of unique k-mers (4^k for DNA, but cap it reasonably)
            std::cmp::min(1 << (2 * k), self.sequence.len() - k + 1),
        );

        let seq_bytes = self.sequence.as_bytes();

        // Count using byte slice references (no allocations)
        for i in 0..=(seq_bytes.len() - k) {
            let kmer_slice = &seq_bytes[i..i + k];
            *kmer_counts.entry(kmer_slice).or_insert(0) += 1;
        }

        // Only convert to strings at the end
        let result = kmer_counts
            .into_iter()
            .map(|(slice, count)| {
                // SAFETY: We know the original sequence was valid UTF-8 (String),
                // so any slice of it is also valid UTF-8
                let kmer_string = unsafe { std::str::from_utf8_unchecked(slice) }.to_string();
                (kmer_string, count)
            })
            .collect();

        Ok(result)
    }

    /// Count k-mers and return the total number of unique k-mers
    pub fn kmer_count(&self, k: usize) -> PyResult<usize> {
        let kmers = self.kmers(k)?;
        Ok(kmers.len())
    }

    /// Get the most frequent k-mer
    pub fn most_frequent_kmer(&self, k: usize) -> PyResult<Option<(String, usize)>> {
        let kmers = self.kmers(k)?;

        let max_entry = kmers
            .iter()
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
        ]
        .into_iter()
        .collect::<HashMap<_, _>>();

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
