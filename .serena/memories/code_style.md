# Code Style and Conventions

## Rust Conventions

### Formatting
- Use `cargo fmt` for all formatting
- Standard Rust formatting (4-space indentation)
- No trailing whitespace

### Naming
- Snake_case for functions and variables: `fill_block`, `test_memory_block`
- PascalCase for types and enums: `TestPattern`, `MemoryError`, `TestStats`
- SCREAMING_SNAKE_CASE for constants: `BLOCK_SIZE`, `DEFAULT_TOTAL_MB`
- Use descriptive, clear names

### Documentation
- Doc comments (`///`) for public APIs where needed
- Inline comments for complex logic
- Minimal comments - code should be self-explanatory

### Code Patterns
- Use `Self` keyword in impl blocks instead of repeating the type name
- Prefer iterators and functional style: `block.iter_mut().enumerate()`
- Use `wrapping_*` methods for intentional overflow: `1u64.wrapping_shl()`
- Use explicit type annotations where helpful for clarity

### Error Handling
- Prefer `Option` and `Result` over panics
- Use `?` operator for error propagation
- Avoid `unwrap()` except in tests or when logically impossible to fail

### Dependencies
- Prefer well-maintained, widely-used crates
- Use derive macros where appropriate (`clap::Parser`, `Debug`, `Clone`, `Copy`)
- Feature flags for optional functionality

### Testing
- Test module at bottom of file with `#[cfg(test)]`
- Comprehensive unit tests for all patterns
- Test names describe what they test: `test_walking_ones_fill_and_verify`
- Test edge cases and error conditions

### Safety
- Zero unsafe code (project goal)
- Use atomic operations for thread-safe statistics
- Lock-free design where possible

## Clippy Rules
- All Clippy warnings are treated as errors in CI (`-D warnings`)
- Fix all Clippy suggestions before committing
