//! GPU buffer management for memory testing.
//!
//! This module provides buffer allocation and management for GPU memory
//! testing, including the test buffer, parameter uniforms, and error reporting.

use crate::error::GpuError;
use wgpu::{Buffer, BufferUsages, Device, Queue};

/// Parameters structure for shader uniforms.
/// Must match the Params struct in WGSL shaders.
#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct ShaderParams {
    /// Pattern ID (0-7 matching TestPattern enum).
    pub pattern_id: u32,
    /// Random seed for pattern generation.
    pub seed: u32,
    /// Total number of u32 elements in test buffer.
    pub total_elements: u32,
    /// Padding for 16-byte alignment.
    pub _padding: u32,
}

/// Error information returned from verify shader.
/// Must match the ErrorInfo struct in WGSL (excluding atomic wrapper).
#[repr(C)]
#[derive(Debug, Clone, Copy, Default, bytemuck::Pod, bytemuck::Zeroable)]
pub struct ErrorInfo {
    /// Number of errors detected.
    pub error_count: u32,
    /// Index of first error detected.
    pub first_error_index: u32,
    /// Expected value at first error location.
    pub first_error_expected: u32,
    /// Actual value found at first error location.
    pub first_error_actual: u32,
}

/// Manages GPU buffers for memory testing.
///
/// Handles allocation of:
/// - Test buffer: The VRAM being tested (storage buffer)
/// - Params buffer: Shader uniform parameters
/// - Error buffer: GPU-side error tracking
/// - Error staging buffer: CPU-readable copy of errors
#[allow(dead_code)] // Used in Issue #14
pub struct BufferManager {
    /// The main test buffer (GPU memory to test).
    test_buffer: Buffer,
    /// Uniform buffer for shader parameters.
    params_buffer: Buffer,
    /// Storage buffer for error information (GPU-side).
    error_buffer: Buffer,
    /// Staging buffer for reading errors back to CPU.
    error_staging_buffer: Buffer,
    /// Size of test buffer in bytes.
    buffer_size: u64,
    /// Number of u32 elements in test buffer.
    element_count: u32,
}

#[allow(dead_code)] // Methods used in Issue #14
impl BufferManager {
    /// Creates a new BufferManager with the specified memory size.
    ///
    /// # Arguments
    ///
    /// * `device` - The wgpu device for buffer allocation
    /// * `memory_mb` - Amount of VRAM to allocate in megabytes
    ///
    /// # Errors
    ///
    /// Returns `GpuError::BufferAllocation` if buffer creation fails.
    pub fn new(device: &Device, memory_mb: usize) -> Result<Self, GpuError> {
        let buffer_size = (memory_mb * 1024 * 1024) as u64;
        let element_count = (buffer_size / 4) as u32; // u32 elements

        // Create test buffer (storage, read/write by shaders)
        let test_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("test_buffer"),
            size: buffer_size,
            usage: BufferUsages::STORAGE | BufferUsages::COPY_DST | BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });

        // Create params uniform buffer
        let params_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("params_buffer"),
            size: std::mem::size_of::<ShaderParams>() as u64,
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // Create error buffer (GPU-side storage)
        let error_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("error_buffer"),
            size: std::mem::size_of::<ErrorInfo>() as u64,
            usage: BufferUsages::STORAGE | BufferUsages::COPY_DST | BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });

        // Create staging buffer for reading errors back to CPU
        let error_staging_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("error_staging_buffer"),
            size: std::mem::size_of::<ErrorInfo>() as u64,
            usage: BufferUsages::MAP_READ | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        Ok(Self {
            test_buffer,
            params_buffer,
            error_buffer,
            error_staging_buffer,
            buffer_size,
            element_count,
        })
    }

    /// Returns a reference to the test buffer.
    pub fn test_buffer(&self) -> &Buffer {
        &self.test_buffer
    }

    /// Returns a reference to the params buffer.
    pub fn params_buffer(&self) -> &Buffer {
        &self.params_buffer
    }

    /// Returns a reference to the error buffer.
    pub fn error_buffer(&self) -> &Buffer {
        &self.error_buffer
    }

    /// Returns a reference to the error staging buffer.
    pub fn error_staging_buffer(&self) -> &Buffer {
        &self.error_staging_buffer
    }

    /// Returns the size of the test buffer in bytes.
    pub fn buffer_size(&self) -> u64 {
        self.buffer_size
    }

    /// Returns the number of u32 elements in the test buffer.
    pub fn element_count(&self) -> u32 {
        self.element_count
    }

    /// Updates the shader parameters buffer.
    pub fn update_params(&self, queue: &Queue, params: &ShaderParams) {
        queue.write_buffer(&self.params_buffer, 0, bytemuck::bytes_of(params));
    }

    /// Resets the error buffer to zero.
    pub fn reset_errors(&self, queue: &Queue) {
        let zeros = ErrorInfo::default();
        queue.write_buffer(&self.error_buffer, 0, bytemuck::bytes_of(&zeros));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gpu::device::select_gpu;
    use pollster::block_on;

    fn setup_device() -> Option<Device> {
        let adapter = select_gpu(None).ok()?;

        let (device, _queue) = block_on(adapter.request_device(&wgpu::DeviceDescriptor {
            label: Some("test device"),
            required_features: wgpu::Features::empty(),
            required_limits: wgpu::Limits::default(),
            memory_hints: wgpu::MemoryHints::default(),
            trace: wgpu::Trace::Off,
            experimental_features: wgpu::ExperimentalFeatures::disabled(),
        }))
        .ok()?;

        Some(device)
    }

    #[test]
    fn test_shader_params_size() {
        // Must be 16 bytes for GPU alignment
        assert_eq!(std::mem::size_of::<ShaderParams>(), 16);
    }

    #[test]
    fn test_error_info_size() {
        // Must be 16 bytes
        assert_eq!(std::mem::size_of::<ErrorInfo>(), 16);
    }

    #[test]
    fn test_error_info_default() {
        let info = ErrorInfo::default();
        assert_eq!(info.error_count, 0);
        assert_eq!(info.first_error_index, 0);
        assert_eq!(info.first_error_expected, 0);
        assert_eq!(info.first_error_actual, 0);
    }

    #[test]
    fn test_buffer_manager_creation() {
        let Some(device) = setup_device() else {
            println!("No GPU available, skipping buffer manager test");
            return;
        };

        let manager = BufferManager::new(&device, 16);
        assert!(manager.is_ok());
    }

    #[test]
    fn test_buffer_size() {
        let Some(device) = setup_device() else {
            println!("No GPU available, skipping buffer size test");
            return;
        };

        let manager = BufferManager::new(&device, 16).unwrap();
        assert_eq!(manager.buffer_size(), 16 * 1024 * 1024);
    }

    #[test]
    fn test_element_count() {
        let Some(device) = setup_device() else {
            println!("No GPU available, skipping element count test");
            return;
        };

        let manager = BufferManager::new(&device, 16).unwrap();
        // 16 MB = 16 * 1024 * 1024 bytes = 4 * 1024 * 1024 u32 elements
        assert_eq!(manager.element_count(), 4 * 1024 * 1024);
    }

    #[test]
    fn test_buffer_accessors() {
        let Some(device) = setup_device() else {
            println!("No GPU available, skipping buffer accessor test");
            return;
        };

        let manager = BufferManager::new(&device, 16).unwrap();

        // Just verify we can access buffers without panicking
        let _ = manager.test_buffer();
        let _ = manager.params_buffer();
        let _ = manager.error_buffer();
        let _ = manager.error_staging_buffer();
    }
}
