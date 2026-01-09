//! GPU memory tester implementation.
//!
//! This module provides the core `GpuTester` struct that orchestrates
//! GPU VRAM memory testing using compute shaders.

use crate::error::{FerritestError, GpuError};
use crate::gpu::buffers::{BufferManager, ErrorInfo, ShaderParams};
use crate::gpu::device::GpuInfo;
use crate::gpu::shaders::{ShaderManager, WORKGROUP_SIZE};
use crate::patterns::TestPattern;
use crate::stats::TestStats;
use crate::traits::{MemoryTester, TestConfig, TestResult};
use pollster::block_on;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use wgpu::{Adapter, Device, Queue};

/// GPU VRAM memory tester.
///
/// Tests GPU memory using compute shaders that write and verify
/// test patterns in VRAM.
#[allow(dead_code)] // Used in Issue #15, integrated in Issue #16
pub struct GpuTester {
    /// The wgpu device for GPU operations.
    device: Device,
    /// The command queue for submitting work.
    queue: Queue,
    /// Information about the GPU being tested.
    gpu_info: GpuInfo,
    /// Shader manager with compute pipelines.
    shaders: ShaderManager,
    /// Buffer manager with test and staging buffers.
    buffers: BufferManager,
    /// Timeout for GPU operations.
    timeout: Duration,
    /// Enable verbose output.
    verbose: bool,
}

#[allow(dead_code)] // Used in Issue #15, integrated in Issue #16
impl GpuTester {
    /// Creates a new GPU tester for the specified adapter.
    ///
    /// # Arguments
    ///
    /// * `adapter` - The wgpu adapter (GPU) to use
    /// * `gpu_info` - Information about the GPU
    /// * `memory_mb` - Amount of VRAM to test in megabytes
    /// * `timeout_secs` - Timeout for GPU operations in seconds
    /// * `verbose` - Enable verbose output
    ///
    /// # Errors
    ///
    /// Returns `GpuError::DeviceRequest` if device creation fails.
    pub fn new(
        adapter: Adapter,
        gpu_info: GpuInfo,
        memory_mb: usize,
        timeout_secs: u64,
        verbose: bool,
    ) -> Result<Self, GpuError> {
        // Request device and queue
        let (device, queue) = block_on(adapter.request_device(&wgpu::DeviceDescriptor {
            label: Some("ferritest"),
            required_features: wgpu::Features::empty(),
            required_limits: wgpu::Limits::default(),
            memory_hints: wgpu::MemoryHints::Performance,
            trace: wgpu::Trace::Off,
            experimental_features: wgpu::ExperimentalFeatures::disabled(),
        }))
        .map_err(|e| GpuError::DeviceRequest(e.to_string()))?;

        // Create shader manager
        let shaders = ShaderManager::new(&device)?;

        // Create buffer manager
        let buffers = BufferManager::new(&device, memory_mb)?;

        Ok(Self {
            device,
            queue,
            gpu_info,
            shaders,
            buffers,
            timeout: Duration::from_secs(timeout_secs),
            verbose,
        })
    }

    /// Returns information about the GPU being tested.
    pub fn gpu_info(&self) -> &GpuInfo {
        &self.gpu_info
    }

    /// Returns the amount of memory being tested in bytes.
    pub fn buffer_size(&self) -> u64 {
        self.buffers.buffer_size()
    }

    /// Runs a single pattern test (write + verify).
    ///
    /// # Arguments
    ///
    /// * `pattern` - The test pattern to use
    /// * `seed` - Random seed for pattern generation
    ///
    /// # Returns
    ///
    /// Returns `ErrorInfo` containing error count and first error details.
    pub fn run_pattern(&self, pattern: TestPattern, seed: u32) -> Result<ErrorInfo, GpuError> {
        // Update params
        let params = ShaderParams {
            pattern_id: pattern.pattern_id(),
            seed,
            total_elements: self.buffers.element_count(),
            _padding: 0,
        };
        self.buffers.update_params(&self.queue, &params);
        self.buffers.reset_errors(&self.queue);

        // Create bind group for write shader
        let write_bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("write_bind_group"),
            layout: self.shaders.write_bind_group_layout(),
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: self.buffers.params_buffer().as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: self.buffers.test_buffer().as_entire_binding(),
                },
            ],
        });

        // Create bind group for verify shader
        let verify_bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("verify_bind_group"),
            layout: self.shaders.verify_bind_group_layout(),
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: self.buffers.params_buffer().as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: self.buffers.test_buffer().as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: self.buffers.error_buffer().as_entire_binding(),
                },
            ],
        });

        // Calculate workgroup count
        let workgroups = self.buffers.element_count().div_ceil(WORKGROUP_SIZE);

        // Create command encoder
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("test_encoder"),
            });

        // Dispatch write shader
        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("write_pass"),
                timestamp_writes: None,
            });
            pass.set_pipeline(self.shaders.write_pipeline());
            pass.set_bind_group(0, &write_bind_group, &[]);
            pass.dispatch_workgroups(workgroups, 1, 1);
        }

        // Dispatch verify shader
        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("verify_pass"),
                timestamp_writes: None,
            });
            pass.set_pipeline(self.shaders.verify_pipeline());
            pass.set_bind_group(0, &verify_bind_group, &[]);
            pass.dispatch_workgroups(workgroups, 1, 1);
        }

        // Copy error buffer to staging
        encoder.copy_buffer_to_buffer(
            self.buffers.error_buffer(),
            0,
            self.buffers.error_staging_buffer(),
            0,
            std::mem::size_of::<ErrorInfo>() as u64,
        );

        // Submit commands
        self.queue.submit(Some(encoder.finish()));

        // Read back errors with timeout
        self.read_errors_with_timeout()
    }

    /// Reads the error buffer with timeout handling.
    fn read_errors_with_timeout(&self) -> Result<ErrorInfo, GpuError> {
        let buffer = self.buffers.error_staging_buffer();
        let slice = buffer.slice(..);

        let (tx, rx) = std::sync::mpsc::channel();
        slice.map_async(wgpu::MapMode::Read, move |result| {
            tx.send(result).ok();
        });

        let start = Instant::now();
        loop {
            let _ = self.device.poll(wgpu::PollType::Poll);

            if let Ok(result) = rx.try_recv() {
                result.map_err(|e| GpuError::BufferMapping(e.to_string()))?;
                break;
            }

            if start.elapsed() > self.timeout {
                return Err(GpuError::Timeout(self.timeout.as_secs()));
            }

            std::thread::sleep(Duration::from_millis(1));
        }

        let data = slice.get_mapped_range();
        let errors: ErrorInfo = *bytemuck::from_bytes(&data);
        drop(data);
        buffer.unmap();

        Ok(errors)
    }
}

impl MemoryTester for GpuTester {
    fn name(&self) -> &'static str {
        "GPU/VRAM"
    }

    fn device_info(&self) -> String {
        format!("{} ({:?})", self.gpu_info.name, self.gpu_info.backend)
    }

    fn max_testable_memory(&self) -> u64 {
        self.buffers.buffer_size()
    }

    fn run_tests(
        &mut self,
        config: &TestConfig,
        stats: Arc<TestStats>,
        should_stop: Arc<AtomicBool>,
    ) -> Result<Vec<TestResult>, FerritestError> {
        let mut results = Vec::new();
        let mut pass = 0u64;
        let start_time = Instant::now();

        loop {
            for pattern in &config.patterns {
                if should_stop.load(Ordering::Relaxed) {
                    return Ok(results);
                }

                // Check timeout
                if let Some(timeout) = config.timeout {
                    if start_time.elapsed() >= timeout {
                        return Ok(results);
                    }
                }

                let pattern_start = Instant::now();
                let seed = (pass * 1000 + pattern.pattern_id() as u64) as u32;

                let errors = self.run_pattern(*pattern, seed)?;

                let duration_ms = pattern_start.elapsed().as_millis() as u64;
                let bytes = self.buffers.buffer_size();

                // Update stats
                stats.add_bytes(bytes);
                stats.add_test();
                if errors.error_count > 0 {
                    stats.add_error();
                }

                results.push(TestResult {
                    bytes_tested: bytes,
                    errors_found: errors.error_count as u64,
                    pattern: *pattern,
                    duration_ms,
                });

                if self.verbose && errors.error_count > 0 {
                    eprintln!(
                        "GPU Error: {} at offset {:#x} - expected {:#x}, got {:#x}",
                        pattern.name(),
                        errors.first_error_index * 4,
                        errors.first_error_expected,
                        errors.first_error_actual
                    );
                }
            }

            pass += 1;

            // Check termination conditions
            if !config.continuous {
                break;
            }
        }

        Ok(results)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gpu::device::{enumerate_gpus, select_gpu};

    fn setup_tester(memory_mb: usize) -> Option<GpuTester> {
        let gpus = enumerate_gpus();
        if gpus.is_empty() {
            return None;
        }

        let gpu_info = gpus[0].clone();
        let adapter = select_gpu(Some(0)).ok()?;

        GpuTester::new(adapter, gpu_info, memory_mb, 30, false).ok()
    }

    #[test]
    fn test_gpu_tester_creation() {
        let Some(_tester) = setup_tester(16) else {
            println!("No GPU available, skipping tester creation test");
            return;
        };
        // If we get here, creation succeeded
    }

    #[test]
    fn test_gpu_info_accessor() {
        let Some(tester) = setup_tester(16) else {
            println!("No GPU available, skipping gpu info test");
            return;
        };

        let info = tester.gpu_info();
        assert!(!info.name.is_empty());
    }

    #[test]
    fn test_buffer_size_accessor() {
        let Some(tester) = setup_tester(16) else {
            println!("No GPU available, skipping buffer size test");
            return;
        };

        assert_eq!(tester.buffer_size(), 16 * 1024 * 1024);
    }

    #[test]
    fn test_run_single_pattern() {
        let Some(tester) = setup_tester(16) else {
            println!("No GPU available, skipping single pattern test");
            return;
        };

        let result = tester.run_pattern(TestPattern::AllZeros, 0);
        assert!(result.is_ok(), "Pattern execution failed: {:?}", result);

        let errors = result.unwrap();
        assert_eq!(
            errors.error_count, 0,
            "Expected no errors on good GPU memory"
        );
    }

    #[test]
    fn test_all_patterns() {
        let Some(tester) = setup_tester(16) else {
            println!("No GPU available, skipping all patterns test");
            return;
        };

        for pattern in TestPattern::all_patterns() {
            let result = tester.run_pattern(pattern, 12345);
            assert!(
                result.is_ok(),
                "Pattern {:?} execution failed: {:?}",
                pattern,
                result
            );

            let errors = result.unwrap();
            assert_eq!(
                errors.error_count, 0,
                "Pattern {:?} reported errors on good GPU memory",
                pattern
            );
        }
    }

    #[test]
    fn test_different_seeds() {
        let Some(tester) = setup_tester(16) else {
            println!("No GPU available, skipping different seeds test");
            return;
        };

        // Test RandomPattern with different seeds
        for seed in [0, 1, 12345, 99999] {
            let result = tester.run_pattern(TestPattern::RandomPattern, seed);
            assert!(
                result.is_ok(),
                "Random pattern with seed {} failed: {:?}",
                seed,
                result
            );
        }
    }

    #[test]
    fn test_memory_tester_trait_name() {
        let Some(tester) = setup_tester(16) else {
            println!("No GPU available, skipping trait name test");
            return;
        };

        assert_eq!(MemoryTester::name(&tester), "GPU/VRAM");
    }

    #[test]
    fn test_memory_tester_trait_device_info() {
        let Some(tester) = setup_tester(16) else {
            println!("No GPU available, skipping trait device info test");
            return;
        };

        let info = MemoryTester::device_info(&tester);
        assert!(!info.is_empty());
        // Should contain the GPU name
        assert!(info.contains(&tester.gpu_info().name));
    }

    #[test]
    fn test_memory_tester_trait_max_memory() {
        let Some(tester) = setup_tester(16) else {
            println!("No GPU available, skipping trait max memory test");
            return;
        };

        assert_eq!(MemoryTester::max_testable_memory(&tester), 16 * 1024 * 1024);
    }

    #[test]
    fn test_memory_tester_trait_run_tests() {
        let Some(mut tester) = setup_tester(16) else {
            println!("No GPU available, skipping trait run_tests test");
            return;
        };

        let config = TestConfig {
            memory_mb: 16,
            patterns: vec![TestPattern::AllZeros, TestPattern::AllOnes],
            continuous: false,
            timeout: None,
            threads: None,
            verbose: false,
        };
        let stats = Arc::new(TestStats::new());
        let should_stop = Arc::new(AtomicBool::new(false));

        let results = tester.run_tests(&config, stats.clone(), should_stop);
        assert!(results.is_ok());

        let results = results.unwrap();
        assert_eq!(results.len(), 2); // Two patterns

        // Stats should be updated
        assert!(stats.get_bytes() >= 16 * 1024 * 1024 * 2);
        assert_eq!(stats.get_tests(), 2);
    }
}
