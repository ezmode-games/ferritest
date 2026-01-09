//! GPU/VRAM memory testing implementation.
//!
//! This module provides GPU memory testing using wgpu for cross-platform
//! support (Vulkan, Metal, DX12). Tests VRAM using compute shaders that
//! run the same test patterns as CPU testing.

#![allow(unused_imports)] // Re-exports for future use in Issues #10, #15

pub mod device;

pub use device::{enumerate_gpus, select_gpu, GpuInfo};

// TODO: Issue #9 - Create compute shader for memory patterns
// TODO: Issue #10 - Create GpuTesterConfig and GpuTester struct
