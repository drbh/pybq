use pyo3::prelude::*;
use pyo3::types::PyBytes;

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
}

impl BqRecord {
    /// Create a new BqRecord (internal constructor)
    pub fn new(sequence: String, encoded: Vec<u8>) -> Self {
        Self { sequence, encoded }
    }
}
