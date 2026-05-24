//! Handlers for vault commands (merged from bold, atrisk, credibility).

use anyhow::Result;

pub fn handle_bold_list() -> Result<()> {
    use crate::bold_vault::get_bold_capsules;

    let capsules = get_bold_capsules();
    println!("🔥 Bold Capsules (High-Risk/High-Reward)");
    if capsules.is_empty() {
        println!("   No bold capsules found");
    } else {
        for c in capsules.iter().take(10) {
            println!("   - {} (novelty: {:.2})", c.capsule_id, c.novelty_score);
        }
        if capsules.len() > 10 {
            println!("   ... and {} more", capsules.len() - 10);
        }
    }
    Ok(())
}

pub fn handle_atrisk_list(threshold: u32) -> Result<()> {
    use crate::at_risk_scanner::get_at_risk_capsules;

    let capsules = get_at_risk_capsules(threshold);
    println!("⚠️  At-Risk Capsules (threshold: {})", threshold);
    if capsules.is_empty() {
        println!("   No at-risk capsules found");
    } else {
        for c in capsules.iter().take(10) {
            println!("   - {} (outcome: {:.2}, streak: {})", c.capsule_id, c.outcome_score, c.low_score_streak);
        }
        if capsules.len() > 10 {
            println!("   ... and {} more", capsules.len() - 10);
        }
    }
    Ok(())
}

pub fn handle_atrisk_keep(capsule_id: &str) -> Result<()> {
    use crate::at_risk_scanner::keep_active;

    let kept = keep_active(capsule_id);
    println!("⚠️  Keep Active: {}", capsule_id);
    println!("   Result: {}", if kept { "Kept active" } else { "Failed" });
    Ok(())
}

pub fn handle_credibility_score() -> Result<()> {
    use crate::credibility::CredibilityScorer;

    let mut scorer = CredibilityScorer::new();
    let results = scorer.compute_credibility(false);

    println!("📊 Credibility Scores");
    println!("   Capsules scored: {}", results.len());
    println!("   Use 'rairos credibility trendslop' for trend-slop detection");
    Ok(())
}

pub fn handle_credibility_trendslop() -> Result<()> {
    use crate::credibility::CredibilityScorer;

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
