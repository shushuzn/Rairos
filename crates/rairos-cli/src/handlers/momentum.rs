//! Handlers for scoring momentum commands.

use anyhow::Result;

pub fn handle_momentum_score(tag: &str) -> Result<()> {
    use rairos_scoring_momentum::ResearchMomentum;

    let mut scorer = ResearchMomentum::new();
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
