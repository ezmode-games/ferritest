//! GPU/VRAM memory testing implementation.
//!
//! This module provides GPU memory testing using wgpu for cross-platform
//! support (Vulkan, Metal, DX12). Tests VRAM using compute shaders that
//! run the same test patterns as CPU testing.
//!
//! # Architecture
//!
//! - [`device`]: GPU enumeration and selection
//! - [`shaders`]: WGSL shader loading and pipeline creation
//! - [`buffers`]: GPU buffer management for test data and errors
//! - [`tester`]: Main testing orchestration via [`GpuTester`]
//!
//! # Usage
//!
//! ```rust,ignore
//! use ferritest::gpu::{enumerate_gpus, select_gpu, GpuTester};
//!
//! // List available GPUs
//! let gpus = enumerate_gpus();
//! for gpu in &gpus {
//!     println!("{}", gpu);
//! }
//!
//! // Select and create tester
//! let adapter = select_gpu(None)?; // Auto-select best GPU
//! let gpu_info = gpus[0].clone();
//! let mut tester = GpuTester::new(adapter, gpu_info, 1024, 30, false)?;
//!
//! // Run tests
//! let results = tester.run_tests(&config, stats, should_stop)?;
//! ```
//!
//! # Platform Support
//!
//! | Platform | Backend |
//! |----------|---------|
//! | Windows  | DirectX 12, Vulkan |
//! | macOS    | Metal |
//! | Linux    | Vulkan |

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
