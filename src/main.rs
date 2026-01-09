mod cpu;
mod error;
#[cfg(feature = "gpu")]
mod gpu;
mod patterns;
mod stats;
mod traits;

use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use std::time::{Duration, Instant};

use clap::Parser;
use cpu::{CpuTester, CpuTesterConfig};
use stats::TestStats;

#[cfg(feature = "gpu")]
use std::sync::atomic::Ordering;

#[cfg(feature = "gpu")]
use gpu::{enumerate_gpus, select_gpu, GpuTester};

#[cfg(feature = "gpu")]
use indicatif::{ProgressBar, ProgressStyle};

#[cfg(feature = "gpu")]
use traits::{MemoryTester, TestConfig};

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

    #[arg(
        long,
        default_value_t = false,
        help = "Run continuously until error or interrupt"
    )]
    continuous: bool,

    /// Test GPU VRAM instead of CPU RAM
    #[arg(long, default_value_t = false)]
    gpu: bool,

    /// Select GPU by index (use --list-gpus to see available)
    #[arg(long)]
    gpu_index: Option<usize>,

    /// List available GPUs and exit
    #[arg(long, default_value_t = false)]
    list_gpus: bool,

    /// Timeout per GPU operation in seconds
    #[arg(long, default_value_t = 30)]
    gpu_timeout: u64,
}

fn parse_duration(s: &str) -> Option<Duration> {
    if s.to_lowercase() == "infinite" {
        return None;
    }
    humantime::parse_duration(s).ok()
}

fn main() {
    let args = Args::parse();

    // Handle --list-gpus early
    if args.list_gpus {
        #[cfg(feature = "gpu")]
        {
            let gpus = gpu::enumerate_gpus();
            if gpus.is_empty() {
                println!("No GPUs found.");
            } else {
                println!("Available GPUs:");
                for gpu_info in &gpus {
                    println!("  {}", gpu_info);
                }
                println!();
                println!("Use --gpu to test VRAM, --gpu-index N to select specific GPU");
            }
        }
        #[cfg(not(feature = "gpu"))]
        {
            println!("GPU support not compiled. Build with: cargo build --features gpu");
        }
        std::process::exit(0);
    }

    // Validate --gpu flag
    if args.gpu {
        #[cfg(not(feature = "gpu"))]
        {
            eprintln!("Error: GPU support not compiled.");
            eprintln!("Build with: cargo build --features gpu");
            std::process::exit(1);
        }
    }

    // Warn if --gpu-index used without --gpu
    if args.gpu_index.is_some() && !args.gpu {
        eprintln!("Warning: --gpu-index has no effect without --gpu flag");
    }

    // Run appropriate tester
    if args.gpu {
        #[cfg(feature = "gpu")]
        {
            run_gpu_test(&args);
        }
    } else {
        run_cpu_test(&args);
    }
}

fn run_cpu_test(args: &Args) {
    // Create CPU tester configuration
    let config = CpuTesterConfig {
        memory_mb: args.memory_mb,
        threads: args.threads,
        continuous: args.continuous,
        timeout: args.duration.as_ref().and_then(|s| parse_duration(s)),
        verbose: args.verbose,
    };

    let tester = CpuTester::new(config);
    let stats = Arc::new(TestStats::new());
    let should_stop = Arc::new(AtomicBool::new(false));

    let start_time = Instant::now();

    // Run the test
    let errors = tester.run(Arc::clone(&stats), Arc::clone(&should_stop));

    // Print results
    println!();
    println!("Test Complete");
    println!("=============");
    println!(
        "Total bytes tested: {} MB",
        stats.get_bytes() / (1024 * 1024)
    );
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

#[cfg(feature = "gpu")]
fn run_gpu_test(args: &Args) {
    // Select GPU
    let adapter = match select_gpu(args.gpu_index) {
        Ok(adapter) => adapter,
        Err(e) => {
            eprintln!("Error selecting GPU: {}", e);
            std::process::exit(1);
        }
    };

    // Get GPU info
    let gpus = enumerate_gpus();
    let gpu_index = args.gpu_index.unwrap_or(0);
    let gpu_info = if gpu_index < gpus.len() {
        gpus[gpu_index].clone()
    } else {
        eprintln!("GPU index {} not found", gpu_index);
        std::process::exit(1);
    };

    // Create GPU tester
    let mut tester = match GpuTester::new(
        adapter,
        gpu_info.clone(),
        args.memory_mb,
        args.gpu_timeout,
        args.verbose,
    ) {
        Ok(tester) => tester,
        Err(e) => {
            eprintln!("Error creating GPU tester: {}", e);
            std::process::exit(1);
        }
    };

    // Print header
    println!("GPU Memory Stress Test");
    println!("======================");
    println!("GPU: {} ({:?})", gpu_info.name, gpu_info.backend);
    println!("VRAM to test: {} MB", args.memory_mb);
    println!(
        "Mode: {}",
        if args.continuous {
            "Continuous"
        } else {
            "Single pass"
        }
    );
    if let Some(ref duration_str) = args.duration {
        println!("Duration: {}", duration_str);
    }
    println!();

    // Create test config
    let config = TestConfig {
        memory_mb: args.memory_mb,
        patterns: patterns::TestPattern::all_patterns(),
        continuous: args.continuous,
        timeout: args.duration.as_ref().and_then(|s| parse_duration(s)),
        threads: None,
        verbose: args.verbose,
    };

    let stats = Arc::new(TestStats::new());
    let should_stop = Arc::new(AtomicBool::new(false));
    let start_time = Instant::now();

    // Create progress bar
    let pb = ProgressBar::new(config.patterns.len() as u64);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("{spinner:.green} [{bar:40.cyan/blue}] {pos}/{len} patterns | {msg}")
            .unwrap()
            .progress_chars("=> "),
    );

    // Start progress update thread
    let stats_clone = Arc::clone(&stats);
    let should_stop_clone = Arc::clone(&should_stop);
    let pb_clone = pb.clone();
    let progress_handle = std::thread::spawn(move || {
        while !should_stop_clone.load(Ordering::Relaxed) {
            let bytes = stats_clone.get_bytes();
            let errors = stats_clone.get_errors();
            let elapsed = start_time.elapsed();

            pb_clone.set_message(format!(
                "{} MB tested | {} errors | {:.1}s",
                bytes / (1024 * 1024),
                errors,
                elapsed.as_secs_f64()
            ));

            std::thread::sleep(Duration::from_millis(100));
        }
    });

    // Run GPU tests
    let results = tester.run_tests(&config, Arc::clone(&stats), Arc::clone(&should_stop));

    // Signal progress thread to stop
    should_stop.store(true, Ordering::Relaxed);
    progress_handle.join().expect("Progress thread panicked");

    // Update progress bar with final count
    pb.set_position(config.patterns.len() as u64);
    pb.finish_with_message("Complete");

    // Print results
    let total_errors: u64 = match &results {
        Ok(r) => r.iter().map(|r| r.errors_found).sum(),
        Err(_) => 0,
    };

    println!();
    println!("Test Complete");
    println!("=============");
    println!(
        "Total bytes tested: {} MB",
        stats.get_bytes() / (1024 * 1024)
    );
    println!("Total tests completed: {}", stats.get_tests());
    println!("Errors found: {}", total_errors);
    println!("Duration: {:.2}s", start_time.elapsed().as_secs_f64());

    match results {
        Ok(_) if total_errors == 0 => {
            println!();
            println!("SUCCESS: No GPU memory errors detected!");
            std::process::exit(0);
        }
        Ok(_) => {
            println!();
            println!("GPU MEMORY ERRORS DETECTED!");
            std::process::exit(1);
        }
        Err(e) => {
            eprintln!();
            eprintln!("GPU test error: {}", e);
            std::process::exit(1);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_duration_minutes() {
        let duration = parse_duration("5m");
        assert!(duration.is_some());
        assert_eq!(duration.unwrap(), Duration::from_secs(300));
    }

    #[test]
    fn test_parse_duration_hours() {
        let duration = parse_duration("2h");
        assert!(duration.is_some());
        assert_eq!(duration.unwrap(), Duration::from_secs(7200));
    }

    #[test]
    fn test_parse_duration_seconds() {
        let duration = parse_duration("30s");
        assert!(duration.is_some());
        assert_eq!(duration.unwrap(), Duration::from_secs(30));
    }

    #[test]
    fn test_parse_duration_infinite() {
        assert!(parse_duration("infinite").is_none());
        assert!(parse_duration("INFINITE").is_none());
        assert!(parse_duration("Infinite").is_none());
    }

    #[test]
    fn test_parse_duration_invalid() {
        assert!(parse_duration("invalid").is_none());
        assert!(parse_duration("").is_none());
    }

    #[test]
    fn test_parse_gpu_flag() {
        let args = Args::parse_from(["ferritest", "--gpu"]);
        assert!(args.gpu);
    }

    #[test]
    fn test_parse_gpu_index() {
        let args = Args::parse_from(["ferritest", "--gpu", "--gpu-index", "1"]);
        assert!(args.gpu);
        assert_eq!(args.gpu_index, Some(1));
    }

    #[test]
    fn test_parse_list_gpus() {
        let args = Args::parse_from(["ferritest", "--list-gpus"]);
        assert!(args.list_gpus);
    }

    #[test]
    fn test_default_gpu_timeout() {
        let args = Args::parse_from(["ferritest"]);
        assert_eq!(args.gpu_timeout, 30);
    }

    #[test]
    fn test_custom_gpu_timeout() {
        let args = Args::parse_from(["ferritest", "--gpu-timeout", "60"]);
        assert_eq!(args.gpu_timeout, 60);
    }
}
