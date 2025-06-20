use pyo3::prelude::*;
use crate::core::BqRecord;
use crate::python::BqReader;

/// Convenience function to open a BQ file
#[pyfunction]
#[pyo3(signature = (path, n_threads=1))]
pub fn open_bq(path: &str, n_threads: Option<usize>) -> PyResult<BqReader> {
    BqReader::new(path, n_threads)
}

/// Hello function for testing
#[pyfunction]
fn hello_from_bin() -> String {
    "Hello from pybq!".to_string()
}

/// Main pybq module
#[pymodule]
fn _pybq(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<BqReader>()?;
    m.add_class::<BqRecord>()?;
    m.add_function(wrap_pyfunction!(open_bq, m)?)?;
    Ok(())
}

/// Core module (legacy)
#[pymodule]
fn _core(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(hello_from_bin, m)?)?;
    Ok(())
}
