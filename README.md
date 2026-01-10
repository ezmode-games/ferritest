# ferritest

A quick memory sanity check for modders. Before you spend hours debugging a crash, rule out bad RAM in under a minute.

The name combines "ferrite" (a nod to [ferrite core memory](https://en.wikipedia.org/wiki/Magnetic-core_memory)) with "test".

## Why This Exists

Modding toolchains are complex. When something crashes, it could be:
- A bad mod
- A load order conflict
- An outdated plugin
- ...or just faulty RAM silently corrupting data

ferritest lets you quickly rule out memory as the culprit. Run it before diving into mod conflict debugging. If it passes, your RAM is fine and you can focus on the actual problem.

## Quick Start

Download the latest release for your platform from [Releases](https://github.com/ezmode-games/ferritest/releases).

```bash
# Quick 1GB test (under a minute)
ferritest

# More thorough 4GB test
ferritest -m 4096

# Let it run while you grab coffee
ferritest -m 8192 -d 10m
```

If you see `SUCCESS: No memory errors detected!` - your RAM is fine. Move on to debugging your mods.

## Installation

### Download Binary

Grab the latest release for your platform:
- **Windows**: `ferritest-windows-x64.zip`
- **macOS (Intel)**: `ferritest-macos-x64.tar.gz`
- **macOS (Apple Silicon)**: `ferritest-macos-arm64.tar.gz`
- **Linux**: `ferritest-linux-x64.tar.gz`

### From crates.io

```bash
cargo install ferritest
```

### From source

```bash
git clone https://github.com/ezmode-games/ferritest.git
cd ferritest
cargo build --release
```

### Building Without GPU Support

GPU support is enabled by default. To build a smaller binary without GPU support:

```bash
cargo build --release --no-default-features
```

## Usage Examples

```bash
# Basic test (1 GB, single pass)
ferritest

# Test 4 GB of RAM
ferritest -m 4096

# Run for 10 minutes
ferritest -d 10m

# Run continuously until error or Ctrl+C
ferritest --continuous

# Use specific number of threads
ferritest -t 8

# Thorough overnight test
ferritest -m 16384 --continuous
```

## GPU Memory Testing

ferritest can also test your GPU's video memory (VRAM) using compute shaders with the same patterns.

### GPU Quick Start

```bash
# List available GPUs
ferritest --list-gpus

# Test default GPU (1GB VRAM)
ferritest --gpu

# Test 4GB of VRAM
ferritest --gpu -m 4096

# Test specific GPU by index
ferritest --gpu --gpu-index 1

# Test all GPUs sequentially
ferritest --gpu --gpu-index all
```

### Platform Support

| Platform | GPU Backend |
|----------|-------------|
| Windows  | DirectX 12, Vulkan |
| macOS    | Metal |
| Linux    | Vulkan |

### GPU Limitations

- Cannot test 100% of VRAM (driver/OS reserves memory)
- Requires compatible GPU with Vulkan/Metal/DX12 support
- GPU tests may be slower than CPU due to readback latency

## Command Line Options

### General Options

| Option | Description |
|--------|-------------|
| `-m, --memory-mb <MB>` | Amount of memory to test (default: 1024) |
| `-d, --duration <TIME>` | How long to run (e.g., '5m', '1h', 'infinite') |
| `-t, --threads <NUM>` | Number of threads (default: all CPU cores) |
| `--continuous` | Run until error or Ctrl+C |
| `-v, --verbose` | Verbose output |
| `-h, --help` | Show help |

### GPU Options

| Option | Description |
|--------|-------------|
| `--gpu` | Enable GPU VRAM testing instead of CPU RAM |
| `--gpu-index <N\|all>` | Select GPU by index or test all GPUs |
| `--list-gpus` | Show available GPUs and exit |
| `--gpu-timeout <SECS>` | Per-operation timeout (default: 30) |

## What It Tests

8 different patterns catch various types of memory errors:

| Pattern | What It Catches |
|---------|-----------------|
| Walking Ones | Stuck-at-zero faults |
| Walking Zeros | Stuck-at-one faults |
| Checkerboard | Adjacent cell interference |
| Inverse Checkerboard | Coupling faults |
| Random | General data retention |
| All Zeros | Basic write/read |
| All Ones | Basic write/read |
| Sequential | Address line faults |

## Example Output

```
Memory Stress Test
==================
Memory to test: 1024 MB (requested: 1024 MB)
Block size: 64 MB
Threads: 16
Blocks per thread: 1
Mode: Single pass

Elapsed: 45.2s | Tested: 8192 MB | Speed: 181.23 MB/s | Tests: 128 | Errors: 0

Test Complete
=============
Total bytes tested: 8192 MB
Total tests completed: 128
Errors found: 0
Duration: 45.23s

SUCCESS: No memory errors detected!
```

## Exit Codes

- `0`: No errors detected
- `1`: Memory errors found

Useful for scripting: `ferritest && echo "RAM OK" || echo "RAM BAD"`

## Limitations

- Tests user-space memory only (not kernel/reserved)
- For thorough hardware testing, use [memtest86+](https://www.memtest.org/) (bootable)
- Results depend on OS memory allocation

## Technical Details

### CPU Testing
- Multi-threaded (uses all CPU cores by default)
- 64 MB block size for optimal cache behavior
- Lock-free statistics via `Arc<AtomicU64>`
- Zero unsafe code

### GPU Testing
- Cross-platform via wgpu (Vulkan/Metal/DX12)
- WGSL compute shaders for pattern generation and verification
- Atomic error counting on GPU
- Staging buffer for error readback

## License

MIT

## Part of ezmode.games

Built for the modding community. Check out our other tools at [ezmode.games](https://ezmode.games).
