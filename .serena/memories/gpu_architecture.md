# GPU VRAM Testing Architecture

## Overview
Extending ferritest to support GPU VRAM testing alongside CPU RAM testing using wgpu for cross-platform compute shaders.

## Target Module Structure
```
src/
  main.rs          # CLI parsing, dispatch (thin)
  patterns.rs      # TestPattern enum (shared)
  stats.rs         # TestStats (shared)
  error.rs         # FerritestError, GpuError
  traits.rs        # MemoryTester trait
  cpu.rs           # CpuTester (current logic)
  gpu/
    mod.rs         # GPU module root
    device.rs      # GPU enumeration, selection
    shaders.rs     # Shader loading, pipelines
    buffers.rs     # Buffer management
    tester.rs      # GpuTester implementation
  shaders/
    patterns.wgsl  # Write patterns
    verify.wgsl    # Verify patterns
```

## Key Abstractions

### MemoryTester Trait
```rust
pub trait MemoryTester: Send + Sync {
    fn name(&self) -> &'static str;
    fn device_info(&self) -> String;
    fn max_testable_memory(&self) -> u64;
    fn run_tests(&mut self, config, stats, should_stop) -> Result<Vec<TestResult>, FerritestError>;
}
```

### CLI Flags
- `--gpu` - Enable GPU VRAM testing
- `--gpu-index <N|all>` - Select GPU or test all
- `--list-gpus` - List available GPUs
- `--gpu-timeout <SECS>` - Per-operation timeout

## Dependencies (feature-gated)
```toml
wgpu = { version = "28.0", optional = true }
pollster = { version = "0.4", optional = true }
bytemuck = { version = "1.14", features = ["derive"], optional = true }
thiserror = "2.0"

[features]
default = ["gpu"]
gpu = ["dep:wgpu", "dep:pollster", "dep:bytemuck"]
```

## Implementation Phases

### Phase 1: Foundation (#1-#4)
Extract patterns, stats, errors to modules. Define MemoryTester trait.

### Phase 2: CPU Refactor (#5-#6)
Move CPU logic to cpu.rs, implement trait for CpuTester.

### Phase 3: GPU Foundation (#7-#10)
Add wgpu dependency, GPU enumeration, error types, CLI args.

### Phase 4: GPU Testing Core (#11-#15)
WGSL shaders, pipelines, buffers, GpuTester implementation.

### Phase 5: Polish (#16-#18)
Progress reporting, multi-GPU, documentation.

## GitHub Issues
18 issues created across 5 milestones. See: https://github.com/ezmode-games/ferritest/issues
