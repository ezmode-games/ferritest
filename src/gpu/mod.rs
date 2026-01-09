//! GPU/VRAM memory testing implementation.
//!
//! This module provides GPU memory testing using wgpu for cross-platform
//! support (Vulkan, Metal, DX12). Tests VRAM using compute shaders that
//! run the same test patterns as CPU testing.

pub mod buffers;
pub mod device;

#[allow(unused_imports)] // Re-exports for Issues #14, #15
pub use buffers::{BufferManager, ErrorInfo, ShaderParams};
#[allow(unused_imports)] // Re-exports for Issues #14, #15
pub use device::{enumerate_gpus, select_gpu, GpuInfo};
