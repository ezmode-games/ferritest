//! GPU/VRAM memory testing implementation.
//!
//! This module provides GPU memory testing using wgpu for cross-platform
//! support (Vulkan, Metal, DX12). Tests VRAM using compute shaders that
//! run the same test patterns as CPU testing.

pub mod buffers;
pub mod device;
pub mod shaders;
pub mod tester;

#[allow(unused_imports)] // Re-exports for Issue #15
pub use buffers::{BufferManager, ErrorInfo, ShaderParams};
#[allow(unused_imports)] // Re-exports for Issue #15
pub use device::{enumerate_gpus, select_gpu, GpuInfo};
#[allow(unused_imports)] // Re-exports for Issue #15
pub use shaders::{ShaderManager, WORKGROUP_SIZE};
#[allow(unused_imports)] // Re-exports for Issue #15
pub use tester::GpuTester;
