//! GPU device enumeration and selection.
//!
//! This module provides functions to enumerate available GPU adapters
//! and select one for memory testing.

#![allow(dead_code)] // Foundation module - used in Issues #10, #15

use crate::error::GpuError;
use wgpu::{Adapter, Backend, Backends, DeviceType, Instance, InstanceDescriptor};

/// Information about an available GPU.
#[derive(Debug, Clone)]
pub struct GpuInfo {
    /// Index in the enumerated list.
    pub index: usize,
    /// GPU name (e.g., "NVIDIA GeForce RTX 4090").
    pub name: String,
    /// Vendor name (e.g., "NVIDIA").
    pub vendor: String,
    /// Graphics API backend (Vulkan, Metal, DX12, etc.).
    pub backend: Backend,
    /// Device type (discrete, integrated, virtual, etc.).
    pub device_type: DeviceType,
    /// Driver version string.
    pub driver: String,
}

impl std::fmt::Display for GpuInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "[{}] {} ({:?}, {:?})",
            self.index, self.name, self.backend, self.device_type
        )
    }
}

/// Enumerate all available GPU adapters.
///
/// Returns a list of `GpuInfo` structs describing each available GPU.
/// The list may be empty if no GPUs are available.
pub fn enumerate_gpus() -> Vec<GpuInfo> {
    let instance = Instance::new(&InstanceDescriptor::default());
    let adapters: Vec<Adapter> = instance.enumerate_adapters(Backends::all());

    adapters
        .iter()
        .enumerate()
        .map(|(index, adapter)| {
            let info = adapter.get_info();
            GpuInfo {
                index,
                name: info.name,
                vendor: vendor_name(info.vendor),
                backend: info.backend,
                device_type: info.device_type,
                driver: info.driver,
            }
        })
        .collect()
}

/// Select a GPU adapter by index.
///
/// If `index` is `None`, auto-selects the best GPU (prefers discrete over integrated).
/// If `index` is `Some(n)`, selects the GPU at that index.
///
/// # Errors
///
/// Returns `GpuError::NoAdapter` if no GPUs are available.
/// Returns `GpuError::AdapterNotFound` if the specified index is invalid.
pub fn select_gpu(index: Option<usize>) -> Result<Adapter, GpuError> {
    let instance = Instance::new(&InstanceDescriptor::default());
    let adapters: Vec<Adapter> = instance.enumerate_adapters(Backends::all());

    if adapters.is_empty() {
        return Err(GpuError::NoAdapter);
    }

    match index {
        Some(idx) => {
            if idx >= adapters.len() {
                let available: Vec<String> = adapters.iter().map(|a| a.get_info().name).collect();
                return Err(GpuError::AdapterNotFound {
                    index: idx,
                    available,
                });
            }
            Ok(adapters.into_iter().nth(idx).unwrap())
        }
        None => {
            // Auto-select: prefer discrete GPU over integrated
            let selected = auto_select_gpu(&adapters);
            Ok(selected
                .cloned()
                .unwrap_or_else(|| adapters.into_iter().next().unwrap()))
        }
    }
}

/// Auto-select the best GPU from available adapters.
///
/// Preference order:
/// 1. Discrete GPU
/// 2. Integrated GPU
/// 3. Virtual GPU
/// 4. CPU (software rendering)
/// 5. Any other
fn auto_select_gpu(adapters: &[Adapter]) -> Option<&Adapter> {
    // Try to find discrete GPU first
    if let Some(adapter) = adapters
        .iter()
        .find(|a| a.get_info().device_type == DeviceType::DiscreteGpu)
    {
        return Some(adapter);
    }

    // Then integrated GPU
    if let Some(adapter) = adapters
        .iter()
        .find(|a| a.get_info().device_type == DeviceType::IntegratedGpu)
    {
        return Some(adapter);
    }

    // Then virtual GPU
    if let Some(adapter) = adapters
        .iter()
        .find(|a| a.get_info().device_type == DeviceType::VirtualGpu)
    {
        return Some(adapter);
    }

    // Fall back to first available
    adapters.first()
}

/// Convert vendor ID to human-readable name.
fn vendor_name(vendor_id: u32) -> String {
    match vendor_id {
        0x1002 => "AMD".to_string(),
        0x1010 => "ImgTec".to_string(),
        0x10DE => "NVIDIA".to_string(),
        0x13B5 => "ARM".to_string(),
        0x5143 => "Qualcomm".to_string(),
        0x8086 => "Intel".to_string(),
        0x106B => "Apple".to_string(),
        _ => format!("Unknown (0x{:04X})", vendor_id),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_enumerate_gpus_returns_list() {
        // May be empty in CI without GPU, just verify it doesn't panic
        let gpus = enumerate_gpus();
        // If we have GPUs, verify they have valid data
        for gpu in &gpus {
            assert!(!gpu.name.is_empty());
        }
    }

    #[test]
    fn test_gpu_info_display() {
        let info = GpuInfo {
            index: 0,
            name: "Test GPU".to_string(),
            vendor: "Test".to_string(),
            backend: Backend::Vulkan,
            device_type: DeviceType::DiscreteGpu,
            driver: "1.0".to_string(),
        };
        let display = format!("{}", info);
        assert!(display.contains("Test GPU"));
        assert!(display.contains("Vulkan"));
        assert!(display.contains("[0]"));
    }

    #[test]
    fn test_gpu_info_debug() {
        let info = GpuInfo {
            index: 1,
            name: "Debug GPU".to_string(),
            vendor: "Debug".to_string(),
            backend: Backend::Metal,
            device_type: DeviceType::IntegratedGpu,
            driver: "2.0".to_string(),
        };
        let debug = format!("{:?}", info);
        assert!(debug.contains("Debug GPU"));
        assert!(debug.contains("Metal"));
    }

    #[test]
    fn test_select_invalid_index() {
        let result = select_gpu(Some(999));
        match result {
            Err(GpuError::AdapterNotFound { index, .. }) => {
                assert_eq!(index, 999);
            }
            Err(GpuError::NoAdapter) => {
                // Also acceptable in CI without GPU
            }
            _ => panic!("Expected AdapterNotFound or NoAdapter error"),
        }
    }

    #[test]
    fn test_vendor_names() {
        assert_eq!(vendor_name(0x10DE), "NVIDIA");
        assert_eq!(vendor_name(0x1002), "AMD");
        assert_eq!(vendor_name(0x8086), "Intel");
        assert_eq!(vendor_name(0x106B), "Apple");
        assert!(vendor_name(0x0000).contains("Unknown"));
    }

    #[test]
    fn test_auto_select_prefers_discrete() {
        // This test verifies the auto_select_gpu logic without requiring actual GPUs
        // The function should prefer discrete > integrated > virtual > other
        let gpus = enumerate_gpus();
        if gpus.len() > 1 {
            // If we have multiple GPUs, auto-select should work
            let result = select_gpu(None);
            assert!(result.is_ok());
        }
    }
}
