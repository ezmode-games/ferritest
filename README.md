# ferritest

A comprehensive, multi-threaded memory stress testing tool written in Rust. Designed to detect faulty RAM through various memory test patterns.

The name combines "ferrite" (a nod to [ferrite core memory](https://en.wikipedia.org/wiki/Magnetic-core_memory)) with "test".

## Features

- **8 Test Patterns**: Walking ones, walking zeros, checkerboard, inverse checkerboard, random, all zeros, all ones, and sequential
- **Multi-threaded**: Uses all available CPU cores by default for maximum stress
- **Sequential & Random Access**: Tests both access patterns to catch different types of errors
- **Real-time Progress**: Live statistics showing speed, errors, and progress
- **Flexible Duration**: Run for a specific time, continuously, or single pass
- **Safe Rust**: Zero unsafe code, leveraging Rust's memory safety guarantees

## Installation

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

The optimized binary will be at `target/release/ferritest`.

## Usage

### Basic Usage (1 GB, single pass)
```bash
ferritest
```

### Test 4 GB of RAM
```bash
ferritest -m 4096
```

### Run for 10 minutes
```bash
ferritest -d 10m
```

### Run continuously until error or Ctrl+C
```bash
ferritest --continuous
```

### Use specific number of threads
```bash
ferritest -t 8
```

### Combine options (8 GB, 4 threads, 1 hour)
```bash
ferritest -m 8192 -t 4 -d 1h
```

## Command Line Options

- `-m, --memory-mb <MB>`: Amount of memory to test in megabytes (default: 1024)
- `-d, --duration <DURATION>`: How long to run (e.g., '5m', '1h', 'infinite')
- `-t, --threads <NUM>`: Number of threads to use (default: CPU count)
- `--continuous`: Run continuously until error or interrupt
- `-v, --verbose`: Enable verbose output
- `-h, --help`: Show help message

## Test Patterns

The tool runs 8 different test patterns to catch various types of memory errors:

1. **Walking Ones**: Single bit walks through each position
2. **Walking Zeros**: Inverse of walking ones
3. **Checkerboard**: Alternating 0xAA pattern
4. **Inverse Checkerboard**: Alternating 0x55 pattern
5. **Random Pattern**: Seeded random data for reproducibility
6. **All Zeros**: Test writing and reading zeros
7. **All Ones**: Test writing and reading ones
8. **Sequential**: Sequential number pattern

## How It Works

1. **Allocation**: Each thread allocates multiple 64 MB blocks
2. **Fill**: Writes a test pattern to the entire block
3. **Verify**: Reads back and verifies the pattern
4. **Random Access**: Performs 1000 random reads
5. **Re-verify**: Verifies the pattern again
6. **Repeat**: Continues with next pattern or iteration

## Exit Codes

- `0`: Success, no errors detected
- `1`: Memory errors detected

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

## When to Use This

- After installing new RAM
- Diagnosing system instability or crashes
- Before deploying a critical server
- After overclocking memory
- Periodic hardware health checks

## Limitations

- Requires sufficient free RAM to run
- Cannot test reserved or kernel memory
- Results depend on OS memory allocation
- May not catch all hardware issues (use memtest86+ for bootable testing)

## Performance Notes

- Block size is 64 MB for optimal cache behavior
- Uses `u64` operations for efficient testing
- Multi-threading maximizes memory bandwidth utilization
- Random access patterns stress the memory controller

## Architecture

Built with safe Rust patterns:

- `Arc<AtomicU64>` for lock-free statistics sharing
- `crossbeam::channel` for error reporting
- `indicatif` for progress visualization
- No unsafe code blocks
- Type-safe test pattern implementation

## License

MIT
