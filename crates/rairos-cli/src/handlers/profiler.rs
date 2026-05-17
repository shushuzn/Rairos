//! Handlers for profiler commands.

use anyhow::Result;

pub fn handle_profiler_report() -> Result<()> {
    use rairos_profiler::{get_profiler, PerformanceProfiler};

    let profiler = get_profiler();
    let report = profiler.get_report();

    println!("📊 Performance Profiler Report");
    println!("{}", report);
    Ok(())
}

pub fn handle_profiler_stats() -> Result<()> {
    use rairos_profiler::{get_profiler, PerformanceProfiler};

    let profiler = get_profiler();
    let stats = profiler.get_stats_dict();

    println!("📊 Profiler Statistics");
    println!("{}", serde_json::to_string_pretty(&stats)?);
    Ok(())
}
