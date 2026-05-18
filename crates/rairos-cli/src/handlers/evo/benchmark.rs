#![allow(
    clippy::too_many_arguments,
    clippy::needless_borrow,
    clippy::print_literal,
    clippy::unwrap_or_default,
    clippy::unnecessary_sort_by,
    clippy::format_in_format_args,
    clippy::map_identity,
    clippy::unused_enumerate_index,
    clippy::needless_borrows_for_generic_args,
    clippy::unnecessary_to_owned,
    clippy::manual_range_contains
)]

use anyhow::{Context, Result};
use rairos_core::RateLimiter;
use rairos_memory::ResearchMemory;
use std::time::{Duration, Instant};

pub fn handle_memory_stats(format: &str) -> Result<()> {
    let memory = ResearchMemory::load().context("Failed to load research memory")?;
    let stats = memory.stats();

    if format == "json" {
        println!("{}", serde_json::to_string_pretty(&stats)?);
        return Ok(());
    }

    println!("=== Research Memory Stats ===\n");
    println!("Total Stances:  {}", stats.total_stances);
    println!("Total Anomalies: {}", stats.total_anomalies);
    println!("\nBy Stance:");
    for (stance, count) in &stats.by_stance {
        println!("  {}: {}", stance, count);
    }
    if !stats.by_severity.is_empty() {
        println!("\nBy Severity:");
        for (sev, count) in &stats.by_severity {
            println!("  {}: {}", sev, count);
        }
    }
    Ok(())
}

pub fn handle_rate_limit_benchmark(count: usize) -> Result<()> {
    let limiter = RateLimiter::new();
    let handle = limiter.get_or_create("benchmark");
    handle.reset();

    let start = Instant::now();
    let mut allowed = 0usize;
    let mut waited = 0usize;
    let mut total_wait = Duration::ZERO;

    for _ in 0..count {
        if handle.can() {
            allowed += 1;
        } else {
            waited += 1;
            let wait_start = Instant::now();
            handle.wait_for_slot();
            total_wait += wait_start.elapsed();
        }
    }

    let elapsed = start.elapsed();
    println!("=== Rate Limiter Benchmark ===");
    println!("Total requests:  {}", count);
    println!("Allowed:         {}", allowed);
    println!("Waited:          {}", waited);
    println!("Total wait time: {:.3}s", total_wait.as_secs_f64());
    println!(
        "Throughput:      {:.0} req/s",
        count as f64 / elapsed.as_secs_f64()
    );
    Ok(())
}

pub fn handle_rate_limit_check(endpoint: &str) -> Result<()> {
    let limiter = RateLimiter::new();
    let handle = limiter.get_or_create(endpoint);

    println!("=== Rate Limit Status: {} ===", endpoint);
    println!("Available: {}", handle.can());
    if !handle.can() {
        println!("(wait_for_slot not shown — would block)");
    }
    Ok(())
}