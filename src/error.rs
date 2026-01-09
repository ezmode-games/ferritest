//! Error types for memory testing.
//!
//! This module provides unified error handling for both CPU and GPU memory testing.

use crate::patterns::TestPattern;
use thiserror::Error;

/// Information about a detected memory error.
#[derive(Debug, Clone)]
#[allow(dead_code)] // Will be used in Phase 2 when integrating with MemoryTester trait
pub struct MemoryErrorInfo {
    /// The test pattern that detected the error.
    pub pattern: TestPattern,
    /// Byte offset where the error was detected.
    pub offset: usize,
    /// Thread or GPU that detected the error.
    pub source_id: usize,
    /// Expected value (if available).
    pub expected: Option<u64>,
    /// Actual value read (if available).
    pub actual: Option<u64>,
}

#[allow(dead_code)] // Will be used in Phase 2
impl MemoryErrorInfo {
    /// Creates a new MemoryErrorInfo with minimal information.
    pub fn new(pattern: TestPattern, offset: usize, source_id: usize) -> Self {
        Self {
            pattern,
            offset,
            source_id,
            expected: None,
            actual: None,
        }
    }

    /// Creates a MemoryErrorInfo with expected and actual values.
    pub fn with_values(
        pattern: TestPattern,
        offset: usize,
        source_id: usize,
        expected: u64,
        actual: u64,
    ) -> Self {
        Self {
            pattern,
            offset,
            source_id,
            expected: Some(expected),
            actual: Some(actual),
        }
    }
}

/// Unified error type for ferritest operations.
#[derive(Error, Debug)]
#[allow(dead_code)] // Will be used in Phase 2 when integrating with MemoryTester trait
pub enum FerritestError {
    /// Memory error detected during testing.
    #[error("Memory error: {pattern} at offset 0x{offset:X} (source {source_id})")]
    Memory {
        pattern: String,
        offset: usize,
        source_id: usize,
    },

    /// Configuration error.
    #[error("Configuration error: {0}")]
    Config(String),

    /// I/O error.
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// GPU-specific errors (for future use).
    #[error("GPU error: {0}")]
    Gpu(#[from] GpuError),
}

/// GPU-specific errors.
#[derive(Error, Debug)]
#[allow(dead_code)] // Will be used in Phase 3/4 for GPU testing
pub enum GpuError {
    /// No GPU adapter found.
    #[error("No GPU adapter found")]
    NoAdapter,

    /// Specified GPU adapter not found.
    #[error("GPU adapter {index} not found (available: {available:?})")]
    AdapterNotFound {
        index: usize,
        available: Vec<String>,
    },

    /// Failed to request GPU device.
    #[error("Failed to request GPU device: {0}")]
    DeviceRequest(String),

    /// Buffer allocation failed.
    #[error("Buffer allocation failed: requested {requested_mb}MB, available ~{available_mb}MB")]
    BufferAllocation {
        requested_mb: u64,
        available_mb: u64,
    },

    /// Shader compilation failed.
    #[error("Shader compilation failed: {0}")]
    ShaderCompilation(String),

    /// GPU device lost during testing.
    #[error("GPU device lost during testing (possible driver crash or timeout)")]
    DeviceLost,

    /// Buffer mapping failed.
    #[error("Buffer mapping failed: {0}")]
    BufferMapping(String),

    /// GPU operation timed out.
    #[error("GPU operation timed out after {0} seconds")]
    Timeout(u64),

    /// Insufficient VRAM.
    #[error("Insufficient VRAM: need {needed_mb}MB, GPU reports {available_mb}MB")]
    InsufficientVram { needed_mb: u64, available_mb: u64 },
}

impl From<MemoryErrorInfo> for FerritestError {
    fn from(info: MemoryErrorInfo) -> Self {
        FerritestError::Memory {
            pattern: info.pattern.name().to_string(),
            offset: info.offset,
            source_id: info.source_id,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_memory_error_info_new() {
        let info = MemoryErrorInfo::new(TestPattern::AllZeros, 1024, 0);
        assert_eq!(info.pattern, TestPattern::AllZeros);
        assert_eq!(info.offset, 1024);
        assert_eq!(info.source_id, 0);
        assert!(info.expected.is_none());
        assert!(info.actual.is_none());
    }

    #[test]
    fn test_memory_error_info_with_values() {
        let info = MemoryErrorInfo::with_values(TestPattern::AllOnes, 2048, 1, 0xFF, 0x00);
        assert_eq!(info.pattern, TestPattern::AllOnes);
        assert_eq!(info.offset, 2048);
        assert_eq!(info.source_id, 1);
        assert_eq!(info.expected, Some(0xFF));
        assert_eq!(info.actual, Some(0x00));
    }

    #[test]
    fn test_ferritest_error_display() {
        let err = FerritestError::Memory {
            pattern: "All Zeros".to_string(),
            offset: 0x1000,
            source_id: 0,
        };
        assert!(err.to_string().contains("Memory error"));
        assert!(err.to_string().contains("0x1000"));
    }

    #[test]
    fn test_gpu_error_display() {
        let err = GpuError::NoAdapter;
        assert!(err.to_string().contains("No GPU adapter"));

        let err = GpuError::Timeout(30);
        assert!(err.to_string().contains("30 seconds"));
    }

    #[test]
    fn test_adapter_not_found_display() {
        let err = GpuError::AdapterNotFound {
            index: 5,
            available: vec!["GPU 0".into(), "GPU 1".into()],
        };
        let msg = err.to_string();
        assert!(msg.contains("5"));
        assert!(msg.contains("GPU 0"));
    }

    #[test]
    fn test_buffer_allocation_display() {
        let err = GpuError::BufferAllocation {
            requested_mb: 8192,
            available_mb: 4096,
        };
        let msg = err.to_string();
        assert!(msg.contains("8192"));
        assert!(msg.contains("4096"));
    }

    #[test]
    fn test_gpu_error_to_ferritest_error() {
        let gpu_err = GpuError::NoAdapter;
        let err: FerritestError = gpu_err.into();
        assert!(matches!(err, FerritestError::Gpu(_)));
    }

    #[test]
    fn test_memory_error_info_into_ferritest_error() {
        let info = MemoryErrorInfo::new(TestPattern::Checkerboard, 512, 2);
        let err: FerritestError = info.into();

        match err {
            FerritestError::Memory {
                pattern,
                offset,
                source_id,
            } => {
                assert_eq!(pattern, "Checkerboard");
                assert_eq!(offset, 512);
                assert_eq!(source_id, 2);
            }
            _ => panic!("Expected Memory error"),
        }
    }
}
