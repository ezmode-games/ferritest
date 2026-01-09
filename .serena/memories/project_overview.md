# Ferritest Project Overview

## Purpose
Ferritest was built as a Rust learning project. It's a comprehensive, multi-threaded memory stress testing tool for detecting faulty RAM. Designed for game mod developers to quickly rule out memory issues before spending hours debugging crashes that might just be bad RAM.

The name combines "ferrite" (referencing ferrite core memory) with "test".

## Tech Stack
- **Language**: Rust (Edition 2021)
- **Build System**: Cargo
- **Key Dependencies**:
  - `clap` (4.5) - CLI argument parsing with derive macros
  - `rand` (0.8) - Random number generation for test patterns
  - `crossbeam` (0.8) - Concurrency utilities and channels
  - `indicatif` (0.17) - Progress bars and status display
  - `num_cpus` (1.16) - CPU core detection
  - `humantime` (2.1) - Human-readable time parsing
  - `bytesize` (1.3) - Human-readable byte sizes

## Architecture
Single-file application (`src/main.rs`) containing:
- **Structs**: `Args` (CLI arguments), `MemoryError`, `TestStats`
- **Enum**: `TestPattern` with 8 test patterns (Walking Ones/Zeros, Checkerboard, Random, etc.)
- **Core Functions**: `test_memory_block`, `worker_thread`, `parse_duration`, `main`
- **Test Module**: Comprehensive unit tests for all patterns and functionality

## Key Features
- Multi-threaded testing (uses all CPU cores by default)
- 64 MB block size for optimal cache behavior
- 8 different test patterns for various memory fault types
- Lock-free statistics via `Arc<AtomicU64>`
- Zero unsafe code
- Configurable duration, memory size, and thread count

## Exit Codes
- `0`: No errors detected
- `1`: Memory errors found

## Repository
- GitHub: https://github.com/ezmode-games/ferritest
- License: MIT
- Part of ezmode.games
