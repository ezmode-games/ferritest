//! CPU/RAM memory testing implementation.
//!
//! This module provides multi-threaded CPU memory testing using
//! the test patterns defined in the patterns module.

use crate::patterns::TestPattern;
use crate::stats::TestStats;
use crossbeam::channel;
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use rand::Rng;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

/// Block size for memory testing (64 MB).
pub const BLOCK_SIZE: usize = 64 * 1024 * 1024;

/// Information about a detected memory error during CPU testing.
#[derive(Debug)]
pub struct CpuMemoryError {
    /// The test pattern that detected the error.
    pub pattern: TestPattern,
    /// Byte offset within the block where the error was detected.
    pub offset: usize,
    /// Thread ID that detected the error.
    pub thread_id: usize,
}

/// CPU memory tester configuration.
#[derive(Debug, Clone)]
#[allow(dead_code)] // verbose field used in Issue #6
pub struct CpuTesterConfig {
    /// Amount of memory to test in megabytes.
    pub memory_mb: usize,
    /// Number of threads (defaults to CPU count if None).
    pub threads: Option<usize>,
    /// Run continuously until stopped.
    pub continuous: bool,
    /// Optional timeout duration.
    pub timeout: Option<Duration>,
    /// Enable verbose output.
    pub verbose: bool,
}

impl Default for CpuTesterConfig {
    fn default() -> Self {
        Self {
            memory_mb: 1024,
            threads: None,
            continuous: false,
            timeout: None,
            verbose: false,
        }
    }
}

/// CPU/RAM memory tester.
///
/// Tests system RAM using multiple threads, each testing a portion
/// of memory with various bit patterns.
#[allow(dead_code)] // Methods used in Issue #6 (MemoryTester implementation)
pub struct CpuTester {
    config: CpuTesterConfig,
    num_threads: usize,
}

#[allow(dead_code)] // Methods used in Issue #6 (MemoryTester implementation)
impl CpuTester {
    /// Creates a new CPU tester with the given configuration.
    pub fn new(config: CpuTesterConfig) -> Self {
        let num_threads = config.threads.unwrap_or_else(num_cpus::get);
        Self {
            config,
            num_threads,
        }
    }

    /// Returns the name of this tester.
    pub fn name(&self) -> &'static str {
        "CPU/RAM"
    }

    /// Returns information about the device being tested.
    pub fn device_info(&self) -> String {
        format!(
            "{} threads, {} MB block size",
            self.num_threads,
            BLOCK_SIZE / (1024 * 1024)
        )
    }

    /// Returns the number of threads used for testing.
    pub fn num_threads(&self) -> usize {
        self.num_threads
    }

    /// Runs the CPU memory test.
    ///
    /// Returns a vector of memory errors found, or empty if no errors.
    pub fn run(&self, stats: Arc<TestStats>, should_stop: Arc<AtomicBool>) -> Vec<CpuMemoryError> {
        let total_blocks = (self.config.memory_mb * 1024 * 1024) / BLOCK_SIZE;
        let blocks_per_thread = total_blocks.div_ceil(self.num_threads);
        let actual_memory_mb = (blocks_per_thread * self.num_threads * BLOCK_SIZE) / (1024 * 1024);

        println!("Memory Stress Test");
        println!("==================");
        println!(
            "Memory to test: {} MB (requested: {} MB)",
            actual_memory_mb, self.config.memory_mb
        );
        println!("Block size: {} MB", BLOCK_SIZE / (1024 * 1024));
        println!("Threads: {}", self.num_threads);
        println!("Blocks per thread: {}", blocks_per_thread);
        println!(
            "Mode: {}",
            if self.config.continuous {
                "Continuous"
            } else {
                "Single pass"
            }
        );

        if let Some(timeout) = self.config.timeout {
            println!("Duration: {:?}", timeout);
        }
        println!();

        let (error_tx, error_rx) = channel::bounded(10);
        let multi_progress = MultiProgress::new();

        let start_time = Instant::now();
        let timeout = self.config.timeout;
        let continuous = self.config.continuous;

        // Stats monitoring thread
        let stats_clone = Arc::clone(&stats);
        let should_stop_clone = Arc::clone(&should_stop);
        let stats_thread = std::thread::spawn(move || {
            let stats_progress = ProgressBar::new_spinner();
            stats_progress.set_style(
                ProgressStyle::default_spinner()
                    .template("{spinner:.green} {msg}")
                    .unwrap(),
            );

            while !should_stop_clone.load(Ordering::Relaxed) {
                let elapsed = start_time.elapsed();
                let bytes = stats_clone.get_bytes();
                let errors = stats_clone.get_errors();
                let tests = stats_clone.get_tests();
                let mb_per_sec = if elapsed.as_secs_f64() > 0.0 {
                    (bytes as f64 / (1024.0 * 1024.0)) / elapsed.as_secs_f64()
                } else {
                    0.0
                };

                stats_progress.set_message(format!(
                    "Elapsed: {:.1}s | Tested: {} MB | Speed: {:.2} MB/s | Tests: {} | Errors: {}",
                    elapsed.as_secs_f64(),
                    bytes / (1024 * 1024),
                    mb_per_sec,
                    tests,
                    errors
                ));

                if let Some(timeout_duration) = timeout {
                    if elapsed >= timeout_duration {
                        should_stop_clone.store(true, Ordering::Relaxed);
                        break;
                    }
                }

                std::thread::sleep(Duration::from_millis(100));
            }
            stats_progress.finish_with_message("Statistics reporting complete");
        });

        // Worker threads
        let mut thread_handles = Vec::new();
        for thread_id in 0..self.num_threads {
            let stats = Arc::clone(&stats);
            let should_stop = Arc::clone(&should_stop);
            let error_tx = error_tx.clone();
            let progress = multi_progress.add(ProgressBar::new(
                (blocks_per_thread * TestPattern::all_patterns().len()) as u64,
            ));
            progress.set_style(
                ProgressStyle::default_bar()
                    .template("{msg}\n{bar:40.cyan/blue} {pos}/{len}")
                    .unwrap()
                    .progress_chars("=>-"),
            );

            let handle = std::thread::spawn(move || {
                worker_thread(
                    thread_id,
                    blocks_per_thread,
                    stats,
                    should_stop,
                    error_tx,
                    progress,
                    continuous,
                );
            });
            thread_handles.push(handle);
        }

        drop(error_tx);

        // Error collector thread
        let error_handler = std::thread::spawn(move || {
            let mut errors = Vec::new();
            while let Ok(error) = error_rx.recv() {
                errors.push(error);
            }
            errors
        });

        // Wait for all worker threads
        for handle in thread_handles {
            handle.join().expect("Thread panicked");
        }

        should_stop.store(true, Ordering::Relaxed);
        stats_thread.join().expect("Stats thread panicked");

        error_handler.join().expect("Error handler thread panicked")
    }
}

/// Tests a single memory block with the given pattern.
fn test_memory_block(
    block: &mut [u64],
    pattern: TestPattern,
    seed: u64,
    thread_id: usize,
    stats: &TestStats,
) -> Option<CpuMemoryError> {
    pattern.fill_block(block, seed);

    stats.add_bytes(std::mem::size_of_val(block) as u64);

    if let Err(offset) = pattern.verify_block(block, seed) {
        stats.add_error();
        return Some(CpuMemoryError {
            pattern,
            offset,
            thread_id,
        });
    }

    // Random access test to stress the memory
    let block_len = block.len();
    let mut rng = rand::thread_rng();
    for _ in 0..1000 {
        let idx = rng.gen_range(0..block_len);
        let _read = block[idx];
    }

    if let Err(offset) = pattern.verify_block(block, seed) {
        stats.add_error();
        return Some(CpuMemoryError {
            pattern,
            offset,
            thread_id,
        });
    }

    stats.add_test();
    None
}

/// Worker thread that tests memory blocks.
fn worker_thread(
    thread_id: usize,
    blocks_per_thread: usize,
    stats: Arc<TestStats>,
    should_stop: Arc<AtomicBool>,
    error_tx: channel::Sender<CpuMemoryError>,
    progress: ProgressBar,
    continuous: bool,
) {
    let block_size_u64 = BLOCK_SIZE / std::mem::size_of::<u64>();
    let mut blocks: Vec<Vec<u64>> = (0..blocks_per_thread)
        .map(|_| vec![0u64; block_size_u64])
        .collect();

    progress.set_message(format!("Thread {} initializing", thread_id));

    let mut iteration = 0u64;
    loop {
        if should_stop.load(Ordering::Relaxed) {
            break;
        }

        for pattern in TestPattern::all_patterns() {
            if should_stop.load(Ordering::Relaxed) {
                break;
            }

            progress.set_message(format!(
                "Thread {} - {} (iter {})",
                thread_id,
                pattern.name(),
                iteration
            ));

            for (block_idx, block) in blocks.iter_mut().enumerate() {
                let seed = thread_id as u64 * 1000000 + block_idx as u64 + iteration;

                if let Some(error) = test_memory_block(block, pattern, seed, thread_id, &stats) {
                    if error_tx.send(error).is_err() {
                        break;
                    }
                    should_stop.store(true, Ordering::Relaxed);
                    return;
                }

                progress.inc(1);
            }
        }

        iteration += 1;

        if !continuous {
            break;
        }
    }

    progress.finish_with_message(format!("Thread {} complete", thread_id));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cpu_tester_new() {
        let config = CpuTesterConfig::default();
        let tester = CpuTester::new(config);
        assert_eq!(tester.name(), "CPU/RAM");
        assert!(tester.num_threads() > 0);
    }

    #[test]
    fn test_cpu_tester_config_default() {
        let config = CpuTesterConfig::default();
        assert_eq!(config.memory_mb, 1024);
        assert!(config.threads.is_none());
        assert!(!config.continuous);
        assert!(config.timeout.is_none());
        assert!(!config.verbose);
    }

    #[test]
    fn test_cpu_tester_device_info() {
        let config = CpuTesterConfig {
            threads: Some(4),
            ..Default::default()
        };
        let tester = CpuTester::new(config);
        let info = tester.device_info();
        assert!(info.contains("4 threads"));
        assert!(info.contains("64 MB"));
    }

    #[test]
    fn test_memory_block_no_error() {
        let mut block = vec![0u64; 1024];
        let stats = TestStats::new();

        let result = test_memory_block(&mut block, TestPattern::AllOnes, 0, 0, &stats);

        assert!(result.is_none());
        assert!(stats.get_bytes() > 0);
        assert_eq!(stats.get_errors(), 0);
        assert_eq!(stats.get_tests(), 1);
    }
}
