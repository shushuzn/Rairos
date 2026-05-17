//! Handlers for game mode and badge commands.

use anyhow::Result;

pub fn handle_badges_list() -> Result<()> {
    let mut manager = rairos_game_mode::BadgeManager::new();
    manager.load_badges();
    manager.check_and_award_badges();
    
    let unlocked = manager.get_unlocked_badges();
    
    println!("=== Research Game Mode ===");
    println!("Unlocked badges: {}", unlocked.len());
    println!();
    
    if !unlocked.is_empty() {
        for badge in &unlocked {
            println!("  {} {} - {} (earned: {})", 
                badge.icon, badge.name, badge.description, 
                badge.earned_at.as_deref().unwrap_or("unknown"));
        }
    } else {
        println!("No badges earned yet. Keep researching!");
    }
    
    Ok(())
}

pub fn handle_badges_award(_badge_id: &str) -> Result<()> {
    let mut manager = rairos_game_mode::BadgeManager::new();
    manager.load_badges();
    manager.check_and_award_badges();
    manager.save_badges();
    println!("Checked and updated badges.");
    Ok(())
}
