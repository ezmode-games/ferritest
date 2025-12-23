use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use clap::Parser;
use crossbeam::channel;
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use rand::{Rng, SeedableRng};

const BLOCK_SIZE: usize = 64 * 1024 * 1024; // 64 MB per block
const DEFAULT_TOTAL_MB: usize = 1024; // 1 GB default

#[derive(Parser, Debug)]
#[command(author, version, about = "Comprehensive memory stress tester", long_about = None)]
struct Args {
    #[arg(short, long, default_value_t = DEFAULT_TOTAL_MB)]
    memory_mb: usize,

    #[arg(short, long, help = "Duration to run (e.g., '5m', '1h', 'infinite')")]
    duration: Option<String>,

    #[arg(short, long, help = "Number of threads (default: CPU count)")]
    threads: Option<usize>,

    #[arg(short, long, default_value_t = false)]
    verbose: bool,

    #[arg(long, default_value_t = false, help = "Run continuously until error or interrupt")]
    continuous: bool,
}

#[derive(Debug, Clone, Copy)]
enum TestPattern {
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
    fn all_patterns() -> Vec<Self> {
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

    fn name(&self) -> &'static str {
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

    fn fill_block(&self, block: &mut [u64], seed: u64) {
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

    fn verify_block(&self, block: &[u64], seed: u64) -> Result<(), usize> {
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

#[derive(Debug)]
struct MemoryError {
    pattern: TestPattern,
    offset: usize,
    thread_id: usize,
}

struct TestStats {
    bytes_tested: AtomicU64,
    errors_found: AtomicU64,
    tests_completed: AtomicU64,
}

impl TestStats {
    fn new() -> Self {
        Self {
            bytes_tested: AtomicU64::new(0),
            errors_found: AtomicU64::new(0),
            tests_completed: AtomicU64::new(0),
        }
    }

    fn add_bytes(&self, bytes: u64) {
        self.bytes_tested.fetch_add(bytes, Ordering::Relaxed);
    }

    fn add_error(&self) {
        self.errors_found.fetch_add(1, Ordering::Relaxed);
    }

    fn add_test(&self) {
        self.tests_completed.fetch_add(1, Ordering::Relaxed);
    }

    fn get_bytes(&self) -> u64 {
        self.bytes_tested.load(Ordering::Relaxed)
    }

    fn get_errors(&self) -> u64 {
        self.errors_found.load(Ordering::Relaxed)
    }

    fn get_tests(&self) -> u64 {
        self.tests_completed.load(Ordering::Relaxed)
    }
}

fn test_memory_block(
    block: &mut [u64],
    pattern: TestPattern,
    seed: u64,
    thread_id: usize,
    stats: &TestStats,
) -> Option<MemoryError> {
    pattern.fill_block(block, seed);

    stats.add_bytes((block.len() * std::mem::size_of::<u64>()) as u64);

    if let Err(offset) = pattern.verify_block(block, seed) {
        stats.add_error();
        return Some(MemoryError {
            pattern,
            offset,
            thread_id,
        });
    }

    let block_len = block.len();
    let mut rng = rand::thread_rng();
    for _ in 0..1000 {
        let idx = rng.gen_range(0..block_len);
        let _read = block[idx];
    }

    if let Err(offset) = pattern.verify_block(block, seed) {
        stats.add_error();
        return Some(MemoryError {
            pattern,
            offset,
            thread_id,
        });
    }

    stats.add_test();
    None
}

fn worker_thread(
    thread_id: usize,
    blocks_per_thread: usize,
    stats: Arc<TestStats>,
    should_stop: Arc<AtomicBool>,
    error_tx: channel::Sender<MemoryError>,
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

fn parse_duration(s: &str) -> Option<Duration> {
    if s.to_lowercase() == "infinite" {
        return None;
    }
    humantime::parse_duration(s).ok()
}

fn main() {
    let args = Args::parse();

    let num_threads = args.threads.unwrap_or_else(num_cpus::get);
    let total_blocks = (args.memory_mb * 1024 * 1024) / BLOCK_SIZE;
    let blocks_per_thread = (total_blocks + num_threads - 1) / num_threads;
    let actual_memory_mb = (blocks_per_thread * num_threads * BLOCK_SIZE) / (1024 * 1024);

    println!("Memory Stress Test");
    println!("==================");
    println!(
        "Memory to test: {} MB (requested: {} MB)",
        actual_memory_mb, args.memory_mb
    );
    println!("Block size: {} MB", BLOCK_SIZE / (1024 * 1024));
    println!("Threads: {}", num_threads);
    println!("Blocks per thread: {}", blocks_per_thread);
    println!(
        "Mode: {}",
        if args.continuous {
            "Continuous"
        } else {
            "Single pass"
        }
    );

    if let Some(ref duration_str) = args.duration {
        if let Some(duration) = parse_duration(duration_str) {
            println!("Duration: {:?}", duration);
        } else {
            println!("Duration: Infinite");
        }
    }

    println!();

    let stats = Arc::new(TestStats::new());
    let should_stop = Arc::new(AtomicBool::new(false));
    let (error_tx, error_rx) = channel::bounded(10);

    let multi_progress = MultiProgress::new();
    let main_progress = multi_progress.add(ProgressBar::new(100));
    main_progress.set_style(
        ProgressStyle::default_bar()
            .template("{msg}\n{bar:40.cyan/blue} {pos}/{len}")
            .unwrap()
            .progress_chars("=>-"),
    );

    let start_time = Instant::now();
    let timeout = args.duration.as_ref().and_then(|s| parse_duration(s));

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
            let mb_per_sec = (bytes as f64 / (1024.0 * 1024.0)) / elapsed.as_secs_f64();

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

    let mut thread_handles = Vec::new();
    for thread_id in 0..num_threads {
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
                args.continuous,
            );
        });
        thread_handles.push(handle);
    }

    drop(error_tx);

    let error_handler = std::thread::spawn(move || {
        let mut errors = Vec::new();
        while let Ok(error) = error_rx.recv() {
            errors.push(error);
        }
        errors
    });

    for handle in thread_handles {
        handle.join().expect("Thread panicked");
    }

    should_stop.store(true, Ordering::Relaxed);
    stats_thread.join().expect("Stats thread panicked");

    let errors = error_handler.join().expect("Error handler thread panicked");

    println!();
    println!("Test Complete");
    println!("=============");
    println!("Total bytes tested: {} MB", stats.get_bytes() / (1024 * 1024));
    println!("Total tests completed: {}", stats.get_tests());
    println!("Errors found: {}", errors.len());
    println!("Duration: {:.2}s", start_time.elapsed().as_secs_f64());

    if !errors.is_empty() {
        println!();
        println!("MEMORY ERRORS DETECTED:");
        println!("=======================");
        for (i, error) in errors.iter().enumerate() {
            println!(
                "Error {}: Thread {} - Pattern {} - Offset: 0x{:X}",
                i + 1,
                error.thread_id,
                error.pattern.name(),
                error.offset
            );
        }
        std::process::exit(1);
    } else {
        println!();
        println!("SUCCESS: No memory errors detected!");
        std::process::exit(0);
    }
}
