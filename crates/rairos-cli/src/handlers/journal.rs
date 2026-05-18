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

pub fn handle_journal(action: &str, content: Option<&str>, tags: Option<&str>, mood: Option<&str>) -> Result<()> {
    let journal = crate::journal::Journal::new(None);

    match action {
        "add" => {
            let Some(c) = content else {
                eprintln!("Usage: journal add <content>");
                std::process::exit(1);
            };
            let mut entry = crate::journal::JournalEntry::new(c);
            if let Some(t) = tags {
                entry = entry.with_tags(t.split(',').map(|s| s.trim().to_string()).collect());
            }
            if let Some(m) = mood {
                entry = entry.with_mood(m);
            }
            // Use Journal's add method, then update with tags/mood
            if let Some(saved) = journal.add(c) {
                let entry_id = saved.id.clone();
                // Update with tags and mood
                journal.update(&entry_id, None, Some(entry.tags.clone()));
                println!("✓ Entry [{}] added", entry_id);
            } else {
                eprintln!("Failed to add journal entry");
            }
        }
        "list" => {
            let entries = journal.list_entries(20, None, None, None, false, 0);
            if entries.is_empty() {
                println!("No journal entries found.");
            } else {
                for entry in &entries {
                    println!("[{}] {} — {}", entry.id, &entry.created_at[..10], &entry.content[..entry.content.len().min(80)]);
                    if !entry.tags.is_empty() {
                        println!("    tags: {}", entry.tags.join(", "));
                    }
                }
                println!("\n{} entries total", entries.len());
            }
        }
        "stats" => {
            let entries = journal.list_entries(1000, None, None, None, false, 0);
            println!("📊 Journal Statistics");
            println!("   Total entries: {}", entries.len());
        }
        "delete" => {
            let id = content.unwrap_or("");
            if journal.delete(id) {
                println!("✓ Entry [{}] deleted", id);
            } else {
                eprintln!("Entry [{}] not found", id);
            }
        }
        _ => {
            eprintln!("Unknown journal action: {}. Use: add, list, stats, delete", action);
        }
    }
    Ok(())
}
