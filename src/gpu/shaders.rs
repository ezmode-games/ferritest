//! WGSL shader management for GPU memory testing.
//!
//! This module provides shader loading and validation for the compute
//! shaders used in GPU memory testing.

#![allow(dead_code)] // Used in Issue #14

use crate::error::GpuError;
use wgpu::{BindGroupLayout, ComputePipeline, Device};

/// WGSL source for the pattern write shader.
pub const PATTERNS_WGSL: &str = include_str!("../shaders/patterns.wgsl");

/// WGSL source for the pattern verify shader.
pub const VERIFY_WGSL: &str = include_str!("../shaders/verify.wgsl");

/// Workgroup size used by compute shaders.
/// Must match the @workgroup_size in WGSL files.
pub const WORKGROUP_SIZE: u32 = 256;

/// Pattern IDs that must match the constants in WGSL shaders.
/// These correspond to the TestPattern enum order.
pub mod pattern_ids {
    pub const WALKING_ONES: u32 = 0;
    pub const WALKING_ZEROS: u32 = 1;
    pub const CHECKERBOARD: u32 = 2;
    pub const INVERSE_CHECKERBOARD: u32 = 3;
    pub const RANDOM: u32 = 4;
    pub const ALL_ZEROS: u32 = 5;
    pub const ALL_ONES: u32 = 6;
    pub const SEQUENTIAL: u32 = 7;
}

/// Manages shader modules and compute pipelines for GPU memory testing.
pub struct ShaderManager {
    /// Pipeline for writing test patterns to memory.
    write_pipeline: ComputePipeline,
    /// Pipeline for verifying test patterns in memory.
    verify_pipeline: ComputePipeline,
    /// Bind group layout for the write pipeline.
    write_bind_group_layout: BindGroupLayout,
    /// Bind group layout for the verify pipeline.
    verify_bind_group_layout: BindGroupLayout,
}

impl ShaderManager {
    /// Creates a new `ShaderManager` with compiled shaders and pipelines.
    ///
    /// Returns `Ok(Self)` on success. The `Result` type allows for future
    /// error handling extensions.
    ///
    /// # Panics
    ///
    /// Panics if shader compilation or pipeline creation fails. These failures
    /// are handled internally by wgpu and are not returned as `GpuError`.
    pub fn new(device: &Device) -> Result<Self, GpuError> {
        // Create shader modules
        let write_module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("pattern_write"),
            source: wgpu::ShaderSource::Wgsl(PATTERNS_WGSL.into()),
        });

        let verify_module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("pattern_verify"),
            source: wgpu::ShaderSource::Wgsl(VERIFY_WGSL.into()),
        });

        // Create bind group layout for write pipeline:
        // @group(0) @binding(0) - Uniform buffer (Params)
        // @group(0) @binding(1) - Storage buffer (data, read_write)
        let write_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("write_bind_group_layout"),
                entries: &[
                    // Params uniform buffer
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    // Data storage buffer (read_write)
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: false },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                ],
            });

        // Create bind group layout for verify pipeline:
        // @group(0) @binding(0) - Uniform buffer (Params)
        // @group(0) @binding(1) - Storage buffer (data, read)
        // @group(0) @binding(2) - Storage buffer (errors, read_write)
        let verify_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("verify_bind_group_layout"),
                entries: &[
                    // Params uniform buffer
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    // Data storage buffer (read only)
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    // Errors storage buffer (read_write)
                    wgpu::BindGroupLayoutEntry {
                        binding: 2,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: false },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                ],
            });

        // Create pipeline layouts
        let write_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("write_pipeline_layout"),
                bind_group_layouts: &[&write_bind_group_layout],
                push_constant_ranges: &[],
            });

        let verify_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("verify_pipeline_layout"),
                bind_group_layouts: &[&verify_bind_group_layout],
                push_constant_ranges: &[],
            });

        // Create compute pipelines
        let write_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("write_pattern_pipeline"),
            layout: Some(&write_pipeline_layout),
            module: &write_module,
            entry_point: Some("write_pattern"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            cache: None,
        });

        let verify_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("verify_pattern_pipeline"),
            layout: Some(&verify_pipeline_layout),
            module: &verify_module,
            entry_point: Some("verify_pattern"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            cache: None,
        });

        Ok(Self {
            write_pipeline,
            verify_pipeline,
            write_bind_group_layout,
            verify_bind_group_layout,
        })
    }

    /// Returns a reference to the write pipeline.
    pub fn write_pipeline(&self) -> &ComputePipeline {
        &self.write_pipeline
    }

    /// Returns a reference to the verify pipeline.
    pub fn verify_pipeline(&self) -> &ComputePipeline {
        &self.verify_pipeline
    }

    /// Returns a reference to the write bind group layout.
    pub fn write_bind_group_layout(&self) -> &BindGroupLayout {
        &self.write_bind_group_layout
    }

    /// Returns a reference to the verify bind group layout.
    pub fn verify_bind_group_layout(&self) -> &BindGroupLayout {
        &self.verify_bind_group_layout
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gpu::device::select_gpu;
    use pollster::block_on;

    fn setup_device() -> Option<(wgpu::Device, wgpu::Queue)> {
        let adapter = select_gpu(None).ok()?;

        let (device, queue) = block_on(adapter.request_device(&wgpu::DeviceDescriptor {
            label: Some("test device"),
            required_features: wgpu::Features::empty(),
            required_limits: wgpu::Limits::default(),
            memory_hints: wgpu::MemoryHints::default(),
            trace: wgpu::Trace::Off,
            experimental_features: wgpu::ExperimentalFeatures::disabled(),
        }))
        .ok()?;

        Some((device, queue))
    }

    #[test]
    fn test_patterns_shader_compilation() {
        let Some((device, _queue)) = setup_device() else {
            println!("No GPU available, skipping shader compilation test");
            return;
        };

        // Should not panic
        let _shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("patterns"),
            source: wgpu::ShaderSource::Wgsl(PATTERNS_WGSL.into()),
        });
    }

    #[test]
    fn test_verify_shader_compilation() {
        let Some((device, _queue)) = setup_device() else {
            println!("No GPU available, skipping shader compilation test");
            return;
        };

        // Should not panic
        let _shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("verify"),
            source: wgpu::ShaderSource::Wgsl(VERIFY_WGSL.into()),
        });
    }

    #[test]
    fn test_pattern_ids_match_enum() {
        use crate::patterns::TestPattern;

        // Verify pattern IDs match the TestPattern enum order
        let patterns = TestPattern::all_patterns();
        assert_eq!(
            patterns[pattern_ids::WALKING_ONES as usize],
            TestPattern::WalkingOnes
        );
        assert_eq!(
            patterns[pattern_ids::WALKING_ZEROS as usize],
            TestPattern::WalkingZeros
        );
        assert_eq!(
            patterns[pattern_ids::CHECKERBOARD as usize],
            TestPattern::Checkerboard
        );
        assert_eq!(
            patterns[pattern_ids::INVERSE_CHECKERBOARD as usize],
            TestPattern::InverseCheckerboard
        );
        assert_eq!(
            patterns[pattern_ids::RANDOM as usize],
            TestPattern::RandomPattern
        );
        assert_eq!(
            patterns[pattern_ids::ALL_ZEROS as usize],
            TestPattern::AllZeros
        );
        assert_eq!(
            patterns[pattern_ids::ALL_ONES as usize],
            TestPattern::AllOnes
        );
        assert_eq!(
            patterns[pattern_ids::SEQUENTIAL as usize],
            TestPattern::Sequential
        );
    }

    #[test]
    fn test_workgroup_size_constant() {
        assert_eq!(WORKGROUP_SIZE, 256);
    }

    #[test]
    fn test_shader_manager_creation() {
        let Some((device, _queue)) = setup_device() else {
            println!("No GPU available, skipping shader manager test");
            return;
        };

        let manager = ShaderManager::new(&device);
        assert!(manager.is_ok());
    }

    #[test]
    fn test_pipelines_accessible() {
        let Some((device, _queue)) = setup_device() else {
            println!("No GPU available, skipping pipeline access test");
            return;
        };

        let manager = ShaderManager::new(&device).unwrap();

        // Verify pipelines exist (accessing them shouldn't panic)
        let _ = manager.write_pipeline();
        let _ = manager.verify_pipeline();
        let _ = manager.write_bind_group_layout();
        let _ = manager.verify_bind_group_layout();
    }
}
