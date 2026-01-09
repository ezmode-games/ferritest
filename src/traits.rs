//! Core traits for memory testing.
//!
//! This module defines the `MemoryTester` trait that both CPU and GPU
//! implementations must satisfy for unified testing.

use crate::error::FerritestError;
use crate::patterns::TestPattern;
use crate::stats::TestStats;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use std::time::Duration;

/// Result of a single test pass.
#[derive(Debug, Clone)]
#[allow(dead_code)] // Will be used in Phase 2
pub struct TestResult {
    /// Number of bytes tested in this pass.
    pub bytes_tested: u64,
    /// Number of errors found.
    pub errors_found: u64,
    /// The pattern used for this test.
    pub pattern: TestPattern,
    /// Duration of the test in milliseconds.
    pub duration_ms: u64,
}

/// Configuration for a test run.
#[derive(Debug, Clone)]
#[allow(dead_code)] // Will be used in Phase 2
pub struct TestConfig {
    /// Amount of memory to test in megabytes.
    pub memory_mb: usize,
    /// Patterns to test (defaults to all patterns).
    pub patterns: Vec<TestPattern>,
    /// Whether to run continuously until stopped.
    pub continuous: bool,
    /// Optional timeout duration.
    pub timeout: Option<Duration>,
    /// Number of threads for CPU testing (ignored for GPU).
    pub threads: Option<usize>,
    /// Enable verbose output.
    pub verbose: bool,
}

impl Default for TestConfig {
    fn default() -> Self {
        Self {
            memory_mb: 1024, // 1 GB default
            patterns: TestPattern::all_patterns(),
            continuous: false,
            timeout: None,
            threads: None,
            verbose: false,
        }
    }
}

/// Progress update for UI callbacks.
#[derive(Debug, Clone)]
#[allow(dead_code)] // Will be used in Phase 2
pub struct ProgressUpdate {
    /// Name of the current pattern being tested.
    pub pattern_name: String,
    /// Bytes processed so far.
    pub bytes_processed: u64,
    /// Total bytes to process.
    pub total_bytes: u64,
    /// Current test pass number.
    pub current_pass: u64,
}

/// Trait for memory testing implementations.
///
/// Both CPU (RAM) and GPU (VRAM) testers implement this trait,
/// allowing for polymorphic dispatch and unified test orchestration.
#[allow(dead_code)] // Will be used in Phase 2
pub trait MemoryTester: Send + Sync {
    /// Returns the name of this tester (e.g., "CPU/RAM", "GPU/VRAM").
    fn name(&self) -> &'static str;

    /// Returns information about the device being tested.
    fn device_info(&self) -> String;

    /// Returns the maximum testable memory in bytes.
    fn max_testable_memory(&self) -> u64;

    /// Runs a complete test suite.
    ///
    /// # Arguments
    /// * `config` - Test configuration
    /// * `stats` - Shared statistics tracker
    /// * `should_stop` - Flag to signal early termination
    ///
    /// # Returns
    /// * `Ok(Vec<TestResult>)` - Results for each pattern tested
    /// * `Err(FerritestError)` - If a fatal error occurred
    fn run_tests(
        &mut self,
        config: &TestConfig,
        stats: Arc<TestStats>,
        should_stop: Arc<AtomicBool>,
    ) -> Result<Vec<TestResult>, FerritestError>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_default() {
        let config = TestConfig::default();
        assert_eq!(config.memory_mb, 1024);
        assert_eq!(config.patterns.len(), 8);
        assert!(!config.continuous);
        assert!(config.timeout.is_none());
        assert!(config.threads.is_none());
        assert!(!config.verbose);
    }

    #[test]
    fn test_config_custom() {
        let config = TestConfig {
            memory_mb: 512,
            patterns: vec![TestPattern::AllZeros, TestPattern::AllOnes],
            continuous: true,
            timeout: Some(Duration::from_secs(60)),
            threads: Some(4),
            verbose: true,
        };
        assert_eq!(config.memory_mb, 512);
        assert_eq!(config.patterns.len(), 2);
        assert!(config.continuous);
        assert_eq!(config.timeout, Some(Duration::from_secs(60)));
        assert_eq!(config.threads, Some(4));
        assert!(config.verbose);
    }

    #[test]
    fn test_result_creation() {
        let result = TestResult {
            bytes_tested: 1024 * 1024 * 1024,
            errors_found: 0,
            pattern: TestPattern::WalkingOnes,
            duration_ms: 1500,
        };
        assert_eq!(result.bytes_tested, 1024 * 1024 * 1024);
        assert_eq!(result.errors_found, 0);
        assert_eq!(result.pattern, TestPattern::WalkingOnes);
        assert_eq!(result.duration_ms, 1500);
    }

    #[test]
    fn test_progress_update() {
        let update = ProgressUpdate {
            pattern_name: "All Zeros".to_string(),
            bytes_processed: 512 * 1024 * 1024,
            total_bytes: 1024 * 1024 * 1024,
            current_pass: 1,
        };
        assert_eq!(update.pattern_name, "All Zeros");
        assert_eq!(update.bytes_processed, 512 * 1024 * 1024);
        assert_eq!(update.total_bytes, 1024 * 1024 * 1024);
        assert_eq!(update.current_pass, 1);
    }
}
