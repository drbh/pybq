// Test compilation of main components
use crate::core::{RecordCounter, GrepCounter};
use crate::python::BqReader;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_record_counter() {
        let counter = RecordCounter::new();
        assert_eq!(counter.count(), 0);
    }

    #[test] 
    fn test_grep_counter() {
        let counter = GrepCounter::new(b"ACGT");
        assert_eq!(counter.count(), 0);
    }
}
