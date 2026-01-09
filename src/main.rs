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

/// GPU selection mode for multi-GPU systems.
#[cfg(feature = "gpu")]
#[derive(Debug, Clone, PartialEq)]
pub enum GpuSelection {
    /// Auto-select best GPU (discrete > integrated).
    Auto,
    /// Select specific GPU by index.
    Index(usize),
    /// Test all available GPUs sequentially.
    All,
}

#[cfg(feature = "gpu")]
impl GpuSelection {
    /// Parse GPU selection from command-line argument.
    pub fn parse(s: Option<&str>) -> Result<Self, String> {
        match s {
            None => Ok(Self::Auto),
            Some("all") => Ok(Self::All),
            Some(n) => n
                .parse::<usize>()
                .map(Self::Index)
                .map_err(|_| format!("Invalid GPU index: '{}'. Use a number or 'all'.", n)),
        }
    }
}

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

    /// Select GPU by index or 'all' for all GPUs (use --list-gpus to see available)
    #[arg(long, value_name = "INDEX|all")]
    gpu_index: Option<String>,

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

/// Find the default GPU index using auto-select logic (discrete > integrated > virtual).
#[cfg(feature = "gpu")]
fn find_default_gpu_index(gpus: &[gpu::GpuInfo]) -> usize {
    use wgpu::DeviceType;

    // Prefer discrete GPU
    if let Some(gpu) = gpus
        .iter()
        .find(|g| g.device_type == DeviceType::DiscreteGpu)
    {
        return gpu.index;
    }

    // Then integrated GPU
    if let Some(gpu) = gpus
        .iter()
        .find(|g| g.device_type == DeviceType::IntegratedGpu)
    {
        return gpu.index;
    }

    // Then virtual GPU
    if let Some(gpu) = gpus
        .iter()
        .find(|g| g.device_type == DeviceType::VirtualGpu)
    {
        return gpu.index;
    }

    // Fall back to first
    0
}

fn main() {
    let args = Args::parse();

    // Handle --list-gpus early
    if args.list_gpus {
        #[cfg(feature = "gpu")]
        {
            let gpus = enumerate_gpus();
            if gpus.is_empty() {
                println!("No GPUs found.");
            } else {
                // Find the default GPU index (auto-select logic)
                let default_index = find_default_gpu_index(&gpus);

                println!("Available GPUs:");
                for gpu_info in &gpus {
                    let marker = if gpu_info.index == default_index {
                        " [DEFAULT]"
                    } else {
                        ""
                    };
                    println!(
                        "  [{}] {} ({:?}, {:?}){}",
                        gpu_info.index, gpu_info.name, gpu_info.backend, gpu_info.device_type, marker
                    );
                }
                println!();
                println!("Use:");
                println!("  --gpu                    Test default GPU [{}]", default_index);
                println!("  --gpu --gpu-index N      Test specific GPU");
                println!("  --gpu --gpu-index all    Test all GPUs sequentially");
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
    // Parse GPU selection
    let selection = match GpuSelection::parse(args.gpu_index.as_deref()) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Error: {}", e);
            let gpus = enumerate_gpus();
            if !gpus.is_empty() {
                eprintln!("\nAvailable GPUs:");
                for gpu in &gpus {
                    eprintln!("  [{}] {}", gpu.index, gpu.name);
                }
            }
            std::process::exit(1);
        }
    };

    // Handle "all" GPUs mode
    if selection == GpuSelection::All {
        run_all_gpus_test(args);
        return;
    }

    // Convert selection to index for select_gpu
    let gpu_index_opt = match &selection {
        GpuSelection::Auto => None,
        GpuSelection::Index(i) => Some(*i),
        GpuSelection::All => unreachable!(),
    };

    // Select GPU
    let adapter = match select_gpu(gpu_index_opt) {
        Ok(adapter) => adapter,
        Err(e) => {
            eprintln!("Error selecting GPU: {}", e);
            std::process::exit(1);
        }
    };

    // Get GPU info
    let gpus = enumerate_gpus();
    let gpu_index = match &selection {
        GpuSelection::Auto => find_default_gpu_index(&gpus),
        GpuSelection::Index(i) => *i,
        GpuSelection::All => unreachable!(),
    };
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

/// Run tests on all available GPUs sequentially.
#[cfg(feature = "gpu")]
fn run_all_gpus_test(args: &Args) {
    let gpus = enumerate_gpus();

    if gpus.is_empty() {
        eprintln!("No GPUs found.");
        std::process::exit(1);
    }

    println!("Testing {} GPU(s) sequentially...", gpus.len());
    println!();

    let mut all_passed = true;
    let mut gpu_results: Vec<(String, bool, u64)> = Vec::new();
    let overall_start = Instant::now();

    for gpu_info in &gpus {
        println!("=== GPU {}: {} ===", gpu_info.index, gpu_info.name);

        let adapter = match select_gpu(Some(gpu_info.index)) {
            Ok(a) => a,
            Err(e) => {
                eprintln!("Error selecting GPU {}: {}", gpu_info.index, e);
                gpu_results.push((gpu_info.name.clone(), false, 0));
                all_passed = false;
                println!();
                continue;
            }
        };

        let mut tester = match GpuTester::new(
            adapter,
            gpu_info.clone(),
            args.memory_mb,
            args.gpu_timeout,
            args.verbose,
        ) {
            Ok(t) => t,
            Err(e) => {
                eprintln!("Error creating tester for GPU {}: {}", gpu_info.index, e);
                gpu_results.push((gpu_info.name.clone(), false, 0));
                all_passed = false;
                println!();
                continue;
            }
        };

        let config = TestConfig {
            memory_mb: args.memory_mb,
            patterns: patterns::TestPattern::all_patterns(),
            continuous: false, // Single pass per GPU in all mode
            timeout: args.duration.as_ref().and_then(|s| parse_duration(s)),
            threads: None,
            verbose: args.verbose,
        };

        let stats = Arc::new(TestStats::new());
        let should_stop = Arc::new(AtomicBool::new(false));
        let gpu_start = Instant::now();

        let results = tester.run_tests(&config, Arc::clone(&stats), should_stop);

        let total_errors: u64 = match &results {
            Ok(r) => r.iter().map(|r| r.errors_found).sum(),
            Err(_) => 0,
        };

        let duration = gpu_start.elapsed().as_secs_f64();

        match results {
            Ok(_) if total_errors == 0 => {
                println!(
                    "PASSED: {} MB tested, {:.1}s",
                    stats.get_bytes() / (1024 * 1024),
                    duration
                );
                gpu_results.push((gpu_info.name.clone(), true, 0));
            }
            Ok(_) => {
                println!(
                    "FAILED: {} errors, {} MB tested, {:.1}s",
                    total_errors,
                    stats.get_bytes() / (1024 * 1024),
                    duration
                );
                gpu_results.push((gpu_info.name.clone(), false, total_errors));
                all_passed = false;
            }
            Err(e) => {
                eprintln!("ERROR: {}", e);
                gpu_results.push((gpu_info.name.clone(), false, 0));
                all_passed = false;
            }
        }
        println!();
    }

    // Print summary
    println!("====================");
    println!("Multi-GPU Test Summary");
    println!("====================");
    for (name, passed, errors) in &gpu_results {
        let status = if *passed {
            "PASSED"
        } else if *errors > 0 {
            "FAILED"
        } else {
            "ERROR"
        };
        println!("  {} - {}", name, status);
    }
    println!();
    println!(
        "Total duration: {:.1}s",
        overall_start.elapsed().as_secs_f64()
    );

    if all_passed {
        println!();
        println!("SUCCESS: All GPUs passed!");
        std::process::exit(0);
    } else {
        println!();
        println!("FAILURE: Some GPUs failed!");
        std::process::exit(1);
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
        assert_eq!(args.gpu_index, Some("1".to_string()));
    }

    #[test]
    fn test_parse_gpu_index_all() {
        let args = Args::parse_from(["ferritest", "--gpu", "--gpu-index", "all"]);
        assert!(args.gpu);
        assert_eq!(args.gpu_index, Some("all".to_string()));
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

    #[cfg(feature = "gpu")]
    #[test]
    fn test_gpu_selection_parse_auto() {
        assert!(matches!(GpuSelection::parse(None), Ok(GpuSelection::Auto)));
    }

    #[cfg(feature = "gpu")]
    #[test]
    fn test_gpu_selection_parse_index() {
        assert!(matches!(
            GpuSelection::parse(Some("0")),
            Ok(GpuSelection::Index(0))
        ));
        assert!(matches!(
            GpuSelection::parse(Some("2")),
            Ok(GpuSelection::Index(2))
        ));
    }

    #[cfg(feature = "gpu")]
    #[test]
    fn test_gpu_selection_parse_all() {
        assert!(matches!(
            GpuSelection::parse(Some("all")),
            Ok(GpuSelection::All)
        ));
    }

    #[cfg(feature = "gpu")]
    #[test]
    fn test_gpu_selection_parse_invalid() {
        assert!(GpuSelection::parse(Some("foo")).is_err());
        assert!(GpuSelection::parse(Some("")).is_err());
        assert!(GpuSelection::parse(Some("-1")).is_err());
    }
}
