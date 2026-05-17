//! Handlers for impact scoring commands.

use anyhow::Result;

pub fn handle_impact_leaderboard(limit: usize) -> Result<()> {
    println!("📊 Impact Leaderboard");
    println!("   Use 'rairos rank' to rank papers in your database");
    println!("   Use 'rairos impact <paper_id>' to score a specific paper");
    Ok(())
}

pub fn handle_impact_score(_paper_id: &str) -> Result<()> {
    println!("🔬 Impact Score");
    println!("   Use 'rairos rank <paper_id>' to score a specific paper");
    Ok(())
}
