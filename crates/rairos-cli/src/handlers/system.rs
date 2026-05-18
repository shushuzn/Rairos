//! Handlers for system commands (merged from profiler, decay).

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

pub fn handle_decay_stats() -> Result<()> {
    use rairos_gene_pool_decay::get_resurrection_queue;

    let queue = get_resurrection_queue();
    println!("📉 Gene Pool Decay Statistics");
    println!("   Resurrectable capsules: {}", queue.len());
    println!("   Use 'rairos decay status' for detailed view");
    Ok(())
}

pub fn handle_decay_status(capsule_id: &str) -> Result<()> {
    use rairos_gene_pool_decay::get_resurrection_queue;

    let queue = get_resurrection_queue();
    println!("📉 Decay Status: {}", capsule_id);
    if queue.contains_key(capsule_id) {
        println!("   Status: In resurrection queue");
    } else {
        println!("   Status: Not in resurrection queue");
    }
    Ok(())
}
