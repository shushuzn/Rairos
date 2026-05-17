//! Handlers for achievement and gamification commands.

use anyhow::Result;

pub fn handle_achievements_list() -> Result<()> {
    let system = rairos_achievements::get_achievement_system();
    
    let unlocked = system.get_unlocked_achievements();
    let pending = system.get_pending_achievements();
    
    println!("=== Achievement Progress ===");
    println!("Total Points: {}", system.total_points());
    println!();
    
    if !unlocked.is_empty() {
        println!("Unlocked ({}/{}):", unlocked.len(), system.all_achievements().len());
        for ach in &unlocked {
            println!("  {} {} - {} ({} pts)", 
                ach.icon, ach.name, ach.description, ach.points);
        }
        println!();
    }
    
    if !pending.is_empty() {
        println!("Locked:");
        for ach in &pending {
            println!("  {} {} - {} ({} pts)", 
                ach.icon, ach.name, ach.description, ach.points);
        }
    }
    
    Ok(())
}

pub fn handle_achievements_report() -> Result<()> {
    let system = rairos_achievements::get_achievement_system();
    println!("{}", system.get_progress_report());
    Ok(())
}

pub fn handle_achievements_stats() -> Result<()> {
    let system = rairos_achievements::get_achievement_system();
    let stats = system.user_stats();
    
    println!("=== User Statistics ===");
    println!("Papers processed: {}", stats.papers_processed);
    println!("API calls saved: {}", stats.api_calls_saved);
    println!("Hours saved: {:.1}", stats.hours_saved);
    println!("Searches performed: {}", stats.searches_performed);
    println!("Imports performed: {}", stats.imports_performed);
    println!();
    println!("Total Points: {}", system.total_points());
    
    Ok(())
}

pub fn handle_achievements_unlock(achievement_id: &str) -> Result<()> {
    let mut system = rairos_achievements::get_achievement_system();
    if let Some(ach) = system.unlock_achievement(achievement_id) {
        println!("Unlocked: {} {}!", ach.icon, ach.name);
    } else {
        println!("Achievement '{}' not found.", achievement_id);
    }
    Ok(())
}
