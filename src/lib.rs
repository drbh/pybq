//! pybq - Python bindings for BQ sequence file format
//! 
//! This crate provides efficient Python bindings for reading and processing
//! BQ (Binary sequence) files with parallel processing capabilities.

pub mod core;
pub mod python;

// Re-export main types for convenience
pub use core::{BqRecord, GrepCounter, PopcntCounter, RecordCounter};
pub use python::{BqReader, open_bq};
