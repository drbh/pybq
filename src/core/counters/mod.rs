//! Counter implementations for various sequence analysis tasks
//!
//! This module provides different counter implementations that can be used
//! for parallel processing of BQ sequence files.

pub mod grep;
pub mod kmer;
pub mod popcnt;
pub mod record;

pub use grep::GrepCounter;
pub use kmer::KmerCounter;
pub use popcnt::PopcntCounter;
pub use record::RecordCounter;
