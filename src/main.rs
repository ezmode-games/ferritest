mod cpu;
mod error;
#[cfg(feature = "gpu")]
mod gpu;
mod patterns;
mod stats;
mod traits;

#[cfg(feature = "gpu")]
#[allow(dead_code)] // Will be used when --gpu flag is added
fn gpu_available() -> bool {
    true // Placeholder
}

#[cfg(not(feature = "gpu"))]
#[allow(dead_code)] // Will be used when --gpu flag is added
fn gpu_available() -> bool {
    false
}

use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use std::time::{Duration, Instant};

use clap::Parser;
use cpu::{CpuTester, CpuTesterConfig};
use stats::TestStats;

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
}

fn parse_duration(s: &str) -> Option<Duration> {
    if s.to_lowercase() == "infinite" {
        return None;
    }
    humantime::parse_duration(s).ok()
}

fn main() {
    let args = Args::parse();

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
}
