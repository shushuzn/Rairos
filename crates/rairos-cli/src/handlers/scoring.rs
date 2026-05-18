//! Handlers for scoring commands (merged from impact, momentum, rigor).

use anyhow::Result;

pub fn handle_impact_leaderboard(_limit: usize) -> Result<()> {
    println!("📊 Impact Leaderboard");
    println!("   Use 'rairos rank' to rank papers in your database");
    println!("   Use 'rairos impact <paper_id>' to score a specific paper");
    Ok(())
}

pub fn handle_impact_score(paper_id: &str) -> Result<()> {
    println!("🔬 Impact Score");
    println!("   Paper: {}", paper_id);
    println!("   Use 'rairos rank <paper_id>' to score a specific paper");
    Ok(())
}

pub fn handle_rigor_score(paper_id: &str) -> Result<()> {
    use rairos_rigor::RigorScorer;

    let _scorer = RigorScorer::new();
    println!("🔬 Rigor Score for: {}", paper_id);
    println!("   Use 'rairos replicate <paper_id>' for full replication check");
    Ok(())
}

pub fn handle_momentum_score(tag: &str) -> Result<()> {
    use rairos_scoring_momentum::ResearchMomentum;

    let _scorer = ResearchMomentum::new();
    println!("📈 Scoring Momentum: {}", tag);
    println!("   Use 'rairos momentum leaderboard' for top tags");
    Ok(())
}

pub fn handle_momentum_leaderboard() -> Result<()> {
    use rairos_scoring_momentum::ResearchMomentum;

    let _scorer = ResearchMomentum::new();
    println!("📈 Momentum Leaderboard");
    println!("   Use 'rairos momentum score <tag>' to score a specific tag");
    Ok(())
}
