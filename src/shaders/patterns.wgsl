// WGSL compute shader for memory test pattern generation
// This shader writes test patterns to GPU memory for VRAM testing.

// Pattern ID constants (must match Rust TestPattern enum order)
const PATTERN_WALKING_ONES: u32 = 0u;
const PATTERN_WALKING_ZEROS: u32 = 1u;
const PATTERN_CHECKERBOARD: u32 = 2u;
const PATTERN_INVERSE_CHECKERBOARD: u32 = 3u;
const PATTERN_RANDOM: u32 = 4u;
const PATTERN_ALL_ZEROS: u32 = 5u;
const PATTERN_ALL_ONES: u32 = 6u;
const PATTERN_SEQUENTIAL: u32 = 7u;

// Parameters passed from Rust
struct Params {
    pattern_id: u32,
    seed: u32,
    total_elements: u32,
    _padding: u32,
}

@group(0) @binding(0) var<uniform> params: Params;
@group(0) @binding(1) var<storage, read_write> data: array<u32>;

// XORShift32 PRNG for deterministic random pattern
// Must match Rust implementation for consistency
fn xorshift32(state: u32) -> u32 {
    var x = state;
    x = x ^ (x << 13u);
    x = x ^ (x >> 17u);
    x = x ^ (x << 5u);
    return x;
}

// Generate the expected value for a given index and pattern
fn generate_value(index: u32, pattern_id: u32, seed: u32) -> u32 {
    switch pattern_id {
        case PATTERN_WALKING_ONES: {
            return 1u << (index % 32u);
        }
        case PATTERN_WALKING_ZEROS: {
            return ~(1u << (index % 32u));
        }
        case PATTERN_CHECKERBOARD: {
            return 0xAAAAAAAAu;
        }
        case PATTERN_INVERSE_CHECKERBOARD: {
            return 0x55555555u;
        }
        case PATTERN_RANDOM: {
            // Combine seed and index for unique but deterministic value
            return xorshift32(seed ^ index ^ (index << 16u));
        }
        case PATTERN_ALL_ZEROS: {
            return 0u;
        }
        case PATTERN_ALL_ONES: {
            return 0xFFFFFFFFu;
        }
        case PATTERN_SEQUENTIAL: {
            return index;
        }
        default: {
            return 0u;
        }
    }
}

// Main compute shader entry point
// Workgroup size 256 for broad GPU compatibility
@compute @workgroup_size(256)
fn write_pattern(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let index = global_id.x;

    // Bounds check to avoid out-of-bounds access
    if index >= params.total_elements {
        return;
    }

    data[index] = generate_value(index, params.pattern_id, params.seed);
}
