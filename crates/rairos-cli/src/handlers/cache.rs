#![allow(
    clippy::too_many_arguments,
    clippy::needless_borrow,
    clippy::print_literal,
    clippy::unwrap_or_default,
    clippy::unnecessary_sort_by,
    clippy::format_in_format_args,
    clippy::map_identity,
    clippy::unused_enumerate_index,
    clippy::needless_borrows_for_generic_args,
    clippy::unnecessary_to_owned,
    clippy::manual_range_contains
)]

use anyhow::Result;
use crate::CacheAction;

pub fn handle_cache(action: &CacheAction) -> Result<()> {
    match action {
        CacheAction::Stats => {
            println!("=== Cache Statistics ===\n");
            let cache_dir = std::path::Path::new("cache");
            if !cache_dir.exists() {
                println!("Cache directory: not created yet");
                return Ok(());
            }
            let entries = std::fs::read_dir(cache_dir)?.count();
            let total_size = std::fs::read_dir(cache_dir)?
                .filter_map(|e| e.ok())
                .filter_map(|e| e.metadata().ok())
                .map(|m| m.len())
                .sum::<u64>();
            println!("Entries: {}", entries);
            println!(
                "Total size: {} bytes ({:.2} MB)",
                total_size,
                total_size as f64 / 1_048_576.0
            );
        }
        CacheAction::Clear => {
            println!("Clearing all cache...");
            let cache_dir = std::path::Path::new("cache");
            if cache_dir.exists() {
                std::fs::remove_dir_all(cache_dir)?;
                println!("[OK] Cache cleared");
            } else {
                println!("[INFO] No cache to clear");
            }
        }
        CacheAction::ClearApi => {
            println!("Clearing API cache...");
            let api_dir = std::path::Path::new("cache/api");
            if api_dir.exists() {
                std::fs::remove_dir_all(api_dir)?;
                println!("[OK] API cache cleared");
            } else {
                println!("[INFO] No API cache to clear");
            }
        }
        CacheAction::ClearParsed => {
            println!("Clearing parsed paper cache...");
            let parsed_dir = std::path::Path::new("cache/parsed");
            if parsed_dir.exists() {
                std::fs::remove_dir_all(parsed_dir)?;
                println!("[OK] Parsed paper cache cleared");
            } else {
                println!("[INFO] No parsed paper cache to clear");
            }
        }
        CacheAction::List { limit } => {
            println!("=== Cached Entries (showing first {}) ===\n", limit);
            let cache_dir = std::path::Path::new("cache");
            if !cache_dir.exists() {
                println!("No cache entries.");
                return Ok(());
            }
            let mut count = 0;
            for entry in std::fs::read_dir(cache_dir)? {
                if count >= *limit {
                    println!(
                        "... and more ({} total entries)",
                        std::fs::read_dir(cache_dir)?.count()
                    );
                    break;
                }
                let entry = entry?;
                let path = entry.path();
                if path.is_file() {
                    let size = entry.metadata()?.len();
                    let name = path.file_name().unwrap_or_default().to_string_lossy();
                    println!("  {} ({} bytes)", name, size);
                    count += 1;
                } else if path.is_dir() {
                    let sub_count = std::fs::read_dir(&path)?.count();
                    let name = path.file_name().unwrap_or_default().to_string_lossy();
                    println!("  {}/ ({} entries)", name, sub_count);
                }
            }
            if count == 0 {
                println!("No cache entries.");
            }
        }
    }
    Ok(())
}
