//! Handlers for bold vault commands.

use anyhow::Result;

pub fn handle_bold_list() -> Result<()> {
    use rairos_bold_vault::get_bold_capsules;

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
