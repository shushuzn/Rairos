//! Handlers for credibility scoring commands.

use anyhow::Result;

pub fn handle_credibility_score() -> Result<()> {
    use rairos_credibility::CredibilityScorer;

    let mut scorer = CredibilityScorer::new();
    let results = scorer.compute_credibility(false);

    println!("📊 Credibility Scores");
    println!("   Capsules scored: {}", results.len());
    println!("   Use 'rairos credibility trendslop' for trend-slop detection");
    Ok(())
}

pub fn handle_credibility_trendslop() -> Result<()> {
    use rairos_credibility::CredibilityScorer;

    let mut scorer = CredibilityScorer::new();
    let capsules = scorer.get_trendslop_capsules();

    println!("📉 Trend-Slop Capsules");
    if capsules.is_empty() {
        println!("   No trend-slop detected");
    } else {
        for c in capsules.iter().take(10) {
            println!("   - {}", c.capsule_id);
        }
    }
    Ok(())
}
