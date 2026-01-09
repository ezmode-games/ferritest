//! Statistics tracking for memory testing.
//!
//! This module provides thread-safe statistics tracking using atomic operations.

use std::sync::atomic::{AtomicU64, Ordering};

/// Thread-safe statistics tracker for memory testing.
///
/// Uses atomic operations for lock-free concurrent updates from multiple threads.
pub struct TestStats {
    bytes_tested: AtomicU64,
    errors_found: AtomicU64,
    tests_completed: AtomicU64,
}

impl TestStats {
    /// Creates a new TestStats instance with all counters at zero.
    pub fn new() -> Self {
        Self {
            bytes_tested: AtomicU64::new(0),
            errors_found: AtomicU64::new(0),
            tests_completed: AtomicU64::new(0),
        }
    }

    /// Adds the specified number of bytes to the bytes tested counter.
    pub fn add_bytes(&self, bytes: u64) {
        self.bytes_tested.fetch_add(bytes, Ordering::Relaxed);
    }

    /// Increments the error counter by one.
    pub fn add_error(&self) {
        self.errors_found.fetch_add(1, Ordering::Relaxed);
    }

    /// Increments the test counter by one.
    pub fn add_test(&self) {
        self.tests_completed.fetch_add(1, Ordering::Relaxed);
    }

    /// Returns the total number of bytes tested.
    pub fn get_bytes(&self) -> u64 {
        self.bytes_tested.load(Ordering::Relaxed)
    }

    /// Returns the total number of errors found.
    pub fn get_errors(&self) -> u64 {
        self.errors_found.load(Ordering::Relaxed)
    }

    /// Returns the total number of tests completed.
    pub fn get_tests(&self) -> u64 {
        self.tests_completed.load(Ordering::Relaxed)
    }
}

impl Default for TestStats {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::thread;

    #[test]
    fn test_stats_new() {
        let stats = TestStats::new();
        assert_eq!(stats.get_bytes(), 0);
        assert_eq!(stats.get_errors(), 0);
        assert_eq!(stats.get_tests(), 0);
    }

    #[test]
    fn test_stats_default() {
        let stats = TestStats::default();
        assert_eq!(stats.get_bytes(), 0);
        assert_eq!(stats.get_errors(), 0);
        assert_eq!(stats.get_tests(), 0);
    }

    #[test]
    fn test_stats_tracking() {
        let stats = TestStats::new();

        stats.add_bytes(1024);
        stats.add_bytes(2048);
        assert_eq!(stats.get_bytes(), 3072);

        stats.add_error();
        stats.add_error();
        assert_eq!(stats.get_errors(), 2);

        stats.add_test();
        assert_eq!(stats.get_tests(), 1);
    }

    #[test]
    fn test_stats_thread_safe() {
        let stats = Arc::new(TestStats::new());
        let mut handles = vec![];

        for _ in 0..10 {
            let stats_clone = Arc::clone(&stats);
            handles.push(thread::spawn(move || {
                for _ in 0..100 {
                    stats_clone.add_bytes(1);
                    stats_clone.add_test();
                }
            }));
        }

        for handle in handles {
            handle.join().unwrap();
        }

        assert_eq!(stats.get_bytes(), 1000);
        assert_eq!(stats.get_tests(), 1000);
    }
}
