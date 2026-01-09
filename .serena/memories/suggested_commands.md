# Suggested Commands for Ferritest Development

## Build Commands

```bash
# Debug build
cargo build

# Release build (optimized, LTO enabled)
cargo build --release

# Check compilation without building
cargo check
```

## Test Commands

```bash
# Run all tests
cargo test

# Run tests with output shown
cargo test -- --nocapture

# Run specific test
cargo test test_walking_ones_fill_and_verify

# Run tests matching a pattern
cargo test test_pattern
```

## Linting and Formatting

```bash
# Check formatting (CI uses this)
cargo fmt --all -- --check

# Apply formatting
cargo fmt

# Run Clippy lints (CI fails on warnings)
cargo clippy -- -D warnings

# Run Clippy and auto-fix
cargo clippy --fix
```

## Running the Application

```bash
# Quick 1GB test
cargo run --release

# Test with specific memory size (MB)
cargo run --release -- -m 4096

# Run for specific duration
cargo run --release -- -d 10m

# Continuous mode
cargo run --release -- --continuous

# Verbose output
cargo run --release -- -v

# Specify thread count
cargo run --release -- -t 8
```

## System Commands (macOS/Darwin)

```bash
# List files
ls -la

# Find files
find . -name "*.rs"

# Search in files
grep -r "pattern" src/

# Git operations
git status
git diff
git add -A
git commit -m "message"
```

## CI Pipeline Commands
The CI runs these checks on every PR:
1. `cargo check` - Compilation check
2. `cargo fmt --all -- --check` - Format check
3. `cargo clippy -- -D warnings` - Lint check (warnings are errors)
4. `cargo test` - All tests (on ubuntu, macos, windows)
