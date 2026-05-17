//! Handlers for at-risk capsule scanning commands.

use anyhow::Result;

pub fn handle_atrisk_list(threshold: u32) -> Result<()> {
    use rairos_at_risk_scanner::get_at_risk_capsules;

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
    use rairos_at_risk_scanner::keep_active;

    let kept = keep_active(capsule_id);
    println!("⚠️  Keep Active: {}", capsule_id);
    println!("   Result: {}", if kept { "Kept active" } else { "Failed" });
    Ok(())
}
