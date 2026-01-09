//! GPU/VRAM memory testing implementation.
//!
//! This module provides GPU memory testing using wgpu for cross-platform
//! support (Vulkan, Metal, DX12). Tests VRAM using compute shaders that
//! run the same test patterns as CPU testing.

pub mod device;

#[allow(unused_imports)] // Re-exports for Phase 4 (Issues #14, #15)
pub use device::{enumerate_gpus, select_gpu, GpuInfo};

// TODO: Issue #11 - Create compute shader for memory patterns
// TODO: Issues #14 and #15 - Create GpuTesterConfig and GpuTester struct
