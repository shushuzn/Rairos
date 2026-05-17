//! Handlers for crossover/evolution commands.

use anyhow::Result;

pub fn handle_crossover_run() -> Result<()> {
    use rairos_crossover::run_evolution;

    println!("🧬 Crossover Evolution");
    let result = run_evolution(5, 10);
    if result.contains_key("error") {
        if let Some(err) = result.get("error") {
            println!("   Error: {}", err);
        }
    } else {
        println!("   Evolution completed successfully");
    }
    Ok(())
}

pub fn handle_crossover_list() -> Result<()> {
    use rairos_crossover::get_top_candidates;

    let candidates = get_top_candidates(10);
    println!("🧬 Top Crossover Candidates");
    if candidates.is_empty() {
        println!("   No active capsules found");
    } else {
        for (i, c) in candidates.iter().take(10).enumerate() {
            println!("   {}. {} (fitness: {:.3})", i + 1, c.capsule_id, c.fitness);
        }
    }
    Ok(())
}
