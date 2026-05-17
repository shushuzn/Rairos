//! Handlers for gene pool decay commands.

use anyhow::Result;

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
