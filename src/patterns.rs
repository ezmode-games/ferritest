//! Test pattern definitions for memory testing.
//!
//! This module contains the `TestPattern` enum and its implementation,
//! which defines various bit patterns used to test memory integrity.

use rand::{Rng, SeedableRng};

/// Memory test patterns for detecting different types of memory faults.
///
/// Each pattern is designed to stress memory in different ways:
/// - Walking patterns detect stuck-at faults and coupling faults
/// - Checkerboard patterns detect address decoder faults
/// - Random patterns provide broad coverage
/// - Solid patterns (all zeros/ones) detect stuck bits
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TestPattern {
    WalkingOnes,
    WalkingZeros,
    Checkerboard,
    InverseCheckerboard,
    RandomPattern,
    AllZeros,
    AllOnes,
    Sequential,
}

impl TestPattern {
    /// Returns all available test patterns.
    pub fn all_patterns() -> Vec<Self> {
        vec![
            Self::WalkingOnes,
            Self::WalkingZeros,
            Self::Checkerboard,
            Self::InverseCheckerboard,
            Self::RandomPattern,
            Self::AllZeros,
            Self::AllOnes,
            Self::Sequential,
        ]
    }

    /// Returns the human-readable name of this pattern.
    pub fn name(&self) -> &'static str {
        match self {
            Self::WalkingOnes => "Walking Ones",
            Self::WalkingZeros => "Walking Zeros",
            Self::Checkerboard => "Checkerboard",
            Self::InverseCheckerboard => "Inverse Checkerboard",
            Self::RandomPattern => "Random Pattern",
            Self::AllZeros => "All Zeros",
            Self::AllOnes => "All Ones",
            Self::Sequential => "Sequential",
        }
    }

    /// Returns the numeric pattern ID for GPU shaders.
    ///
    /// These IDs must match the constants in the WGSL shader files.
    #[cfg(feature = "gpu")]
    pub fn pattern_id(&self) -> u32 {
        match self {
            Self::WalkingOnes => 0,
            Self::WalkingZeros => 1,
            Self::Checkerboard => 2,
            Self::InverseCheckerboard => 3,
            Self::RandomPattern => 4,
            Self::AllZeros => 5,
            Self::AllOnes => 6,
            Self::Sequential => 7,
        }
    }

    /// Fills a memory block with this pattern.
    ///
    /// # Arguments
    /// * `block` - The memory block to fill (as u64 slice)
    /// * `seed` - Seed for random pattern generation
    pub fn fill_block(&self, block: &mut [u64], seed: u64) {
        match self {
            Self::WalkingOnes => {
                for (i, val) in block.iter_mut().enumerate() {
                    *val = 1u64.wrapping_shl((i % 64) as u32);
                }
            }
            Self::WalkingZeros => {
                for (i, val) in block.iter_mut().enumerate() {
                    *val = !1u64.wrapping_shl((i % 64) as u32);
                }
            }
            Self::Checkerboard => {
                for val in block.iter_mut() {
                    *val = 0xAAAAAAAAAAAAAAAA;
                }
            }
            Self::InverseCheckerboard => {
                for val in block.iter_mut() {
                    *val = 0x5555555555555555;
                }
            }
            Self::RandomPattern => {
                let mut rng = rand::rngs::StdRng::seed_from_u64(seed);
                for val in block.iter_mut() {
                    *val = rng.gen();
                }
            }
            Self::AllZeros => {
                for val in block.iter_mut() {
                    *val = 0;
                }
            }
            Self::AllOnes => {
                for val in block.iter_mut() {
                    *val = u64::MAX;
                }
            }
            Self::Sequential => {
                for (i, val) in block.iter_mut().enumerate() {
                    *val = i as u64;
                }
            }
        }
    }

    /// Verifies a memory block contains the expected pattern.
    ///
    /// # Arguments
    /// * `block` - The memory block to verify (as u64 slice)
    /// * `seed` - Seed used for random pattern generation
    ///
    /// # Returns
    /// * `Ok(())` if verification passes
    /// * `Err(index)` with the index of the first mismatch
    pub fn verify_block(&self, block: &[u64], seed: u64) -> Result<(), usize> {
        match self {
            Self::WalkingOnes => {
                for (i, &val) in block.iter().enumerate() {
                    let expected = 1u64.wrapping_shl((i % 64) as u32);
                    if val != expected {
                        return Err(i);
                    }
                }
            }
            Self::WalkingZeros => {
                for (i, &val) in block.iter().enumerate() {
                    let expected = !1u64.wrapping_shl((i % 64) as u32);
                    if val != expected {
                        return Err(i);
                    }
                }
            }
            Self::Checkerboard => {
                for (i, &val) in block.iter().enumerate() {
                    if val != 0xAAAAAAAAAAAAAAAA {
                        return Err(i);
                    }
                }
            }
            Self::InverseCheckerboard => {
                for (i, &val) in block.iter().enumerate() {
                    if val != 0x5555555555555555 {
                        return Err(i);
                    }
                }
            }
            Self::RandomPattern => {
                let mut rng = rand::rngs::StdRng::seed_from_u64(seed);
                for (i, &val) in block.iter().enumerate() {
                    let expected: u64 = rng.gen();
                    if val != expected {
                        return Err(i);
                    }
                }
            }
            Self::AllZeros => {
                for (i, &val) in block.iter().enumerate() {
                    if val != 0 {
                        return Err(i);
                    }
                }
            }
            Self::AllOnes => {
                for (i, &val) in block.iter().enumerate() {
                    if val != u64::MAX {
                        return Err(i);
                    }
                }
            }
            Self::Sequential => {
                for (i, &val) in block.iter().enumerate() {
                    if val != i as u64 {
                        return Err(i);
                    }
                }
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const TEST_BLOCK_SIZE: usize = 1024;

    #[test]
    fn test_all_patterns_count() {
        assert_eq!(TestPattern::all_patterns().len(), 8);
    }

    #[test]
    fn test_pattern_names() {
        assert_eq!(TestPattern::WalkingOnes.name(), "Walking Ones");
        assert_eq!(TestPattern::WalkingZeros.name(), "Walking Zeros");
        assert_eq!(TestPattern::Checkerboard.name(), "Checkerboard");
        assert_eq!(
            TestPattern::InverseCheckerboard.name(),
            "Inverse Checkerboard"
        );
        assert_eq!(TestPattern::RandomPattern.name(), "Random Pattern");
        assert_eq!(TestPattern::AllZeros.name(), "All Zeros");
        assert_eq!(TestPattern::AllOnes.name(), "All Ones");
        assert_eq!(TestPattern::Sequential.name(), "Sequential");
    }

    #[test]
    fn test_walking_ones_fill_and_verify() {
        let mut block = vec![0u64; TEST_BLOCK_SIZE];
        TestPattern::WalkingOnes.fill_block(&mut block, 0);

        // Verify pattern correctness
        for (i, &val) in block.iter().enumerate() {
            let expected = 1u64.wrapping_shl((i % 64) as u32);
            assert_eq!(val, expected, "Mismatch at index {}", i);
        }

        // Verify using verify_block
        assert!(TestPattern::WalkingOnes.verify_block(&block, 0).is_ok());
    }

    #[test]
    fn test_walking_zeros_fill_and_verify() {
        let mut block = vec![0u64; TEST_BLOCK_SIZE];
        TestPattern::WalkingZeros.fill_block(&mut block, 0);

        for (i, &val) in block.iter().enumerate() {
            let expected = !1u64.wrapping_shl((i % 64) as u32);
            assert_eq!(val, expected, "Mismatch at index {}", i);
        }

        assert!(TestPattern::WalkingZeros.verify_block(&block, 0).is_ok());
    }

    #[test]
    fn test_checkerboard_fill_and_verify() {
        let mut block = vec![0u64; TEST_BLOCK_SIZE];
        TestPattern::Checkerboard.fill_block(&mut block, 0);

        for &val in &block {
            assert_eq!(val, 0xAAAAAAAAAAAAAAAA);
        }

        assert!(TestPattern::Checkerboard.verify_block(&block, 0).is_ok());
    }

    #[test]
    fn test_inverse_checkerboard_fill_and_verify() {
        let mut block = vec![0u64; TEST_BLOCK_SIZE];
        TestPattern::InverseCheckerboard.fill_block(&mut block, 0);

        for &val in &block {
            assert_eq!(val, 0x5555555555555555);
        }

        assert!(TestPattern::InverseCheckerboard
            .verify_block(&block, 0)
            .is_ok());
    }

    #[test]
    fn test_all_zeros_fill_and_verify() {
        let mut block = vec![0xFFu64; TEST_BLOCK_SIZE];
        TestPattern::AllZeros.fill_block(&mut block, 0);

        for &val in &block {
            assert_eq!(val, 0);
        }

        assert!(TestPattern::AllZeros.verify_block(&block, 0).is_ok());
    }

    #[test]
    fn test_all_ones_fill_and_verify() {
        let mut block = vec![0u64; TEST_BLOCK_SIZE];
        TestPattern::AllOnes.fill_block(&mut block, 0);

        for &val in &block {
            assert_eq!(val, u64::MAX);
        }

        assert!(TestPattern::AllOnes.verify_block(&block, 0).is_ok());
    }

    #[test]
    fn test_sequential_fill_and_verify() {
        let mut block = vec![0u64; TEST_BLOCK_SIZE];
        TestPattern::Sequential.fill_block(&mut block, 0);

        for (i, &val) in block.iter().enumerate() {
            assert_eq!(val, i as u64);
        }

        assert!(TestPattern::Sequential.verify_block(&block, 0).is_ok());
    }

    #[test]
    fn test_random_pattern_deterministic() {
        let mut block1 = vec![0u64; TEST_BLOCK_SIZE];
        let mut block2 = vec![0u64; TEST_BLOCK_SIZE];

        TestPattern::RandomPattern.fill_block(&mut block1, 42);
        TestPattern::RandomPattern.fill_block(&mut block2, 42);

        assert_eq!(block1, block2, "Same seed should produce same pattern");
        assert!(TestPattern::RandomPattern.verify_block(&block1, 42).is_ok());
    }

    #[test]
    fn test_random_pattern_different_seeds() {
        let mut block1 = vec![0u64; TEST_BLOCK_SIZE];
        let mut block2 = vec![0u64; TEST_BLOCK_SIZE];

        TestPattern::RandomPattern.fill_block(&mut block1, 1);
        TestPattern::RandomPattern.fill_block(&mut block2, 2);

        assert_ne!(
            block1, block2,
            "Different seeds should produce different patterns"
        );
    }

    #[test]
    fn test_verify_detects_corruption() {
        let mut block = vec![0u64; TEST_BLOCK_SIZE];
        TestPattern::AllZeros.fill_block(&mut block, 0);

        // Corrupt one byte
        block[100] = 0xFF;

        let result = TestPattern::AllZeros.verify_block(&block, 0);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), 100);
    }

    #[test]
    fn test_all_patterns_fill_and_verify() {
        for pattern in TestPattern::all_patterns() {
            let mut block = vec![0u64; TEST_BLOCK_SIZE];
            let seed = 12345u64;

            pattern.fill_block(&mut block, seed);
            let result = pattern.verify_block(&block, seed);

            assert!(
                result.is_ok(),
                "Pattern {:?} failed verification",
                pattern.name()
            );
        }
    }

    #[test]
    fn test_checkerboard_inverse_relationship() {
        let mut checker = vec![0u64; TEST_BLOCK_SIZE];
        let mut inverse = vec![0u64; TEST_BLOCK_SIZE];

        TestPattern::Checkerboard.fill_block(&mut checker, 0);
        TestPattern::InverseCheckerboard.fill_block(&mut inverse, 0);

        for (c, i) in checker.iter().zip(inverse.iter()) {
            assert_eq!(
                *c ^ *i,
                u64::MAX,
                "Checkerboard and inverse should XOR to all 1s"
            );
        }
    }
}
