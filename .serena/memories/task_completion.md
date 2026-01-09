# Task Completion Checklist

After completing a coding task, run through this checklist before considering the work done:

## Required Steps

### 1. Format Code
```bash
cargo fmt
```
CI will fail if code is not properly formatted.

### 2. Run Clippy
```bash
cargo clippy -- -D warnings
```
All warnings must be resolved. CI treats warnings as errors.

### 3. Run Tests
```bash
cargo test
```
All existing tests must pass. Add new tests for new functionality.

### 4. Build Check
```bash
cargo build
```
Ensure the project compiles without errors.

## Optional Steps

### Release Build (if performance-critical changes)
```bash
cargo build --release
```

### Run the Application (if behavior changed)
```bash
cargo run --release -- -m 512  # Quick test with smaller memory
```

## Before Committing

1. All 4 required steps pass
2. New functionality has corresponding tests
3. No new Clippy warnings introduced
4. Documentation updated if public APIs changed

## CI Pipeline Mirrors These Steps
The GitHub Actions CI runs:
- `cargo check`
- `cargo fmt --all -- --check`
- `cargo clippy -- -D warnings`
- `cargo test` (on Linux, macOS, and Windows)

All must pass for PRs to be merged.
