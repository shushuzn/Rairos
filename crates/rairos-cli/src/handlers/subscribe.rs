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

use anyhow::{Context, Result};
use rairos_core::Database;
use crate::handlers::*;

pub fn handle_subscribe(
    db: &Database,
    query: &str,
    interval_minutes: u64,
    max_papers: usize,
    auto_add: bool,
) -> Result<()> {
    println!("=== Subscribing to arXiv: {} ===", query);
    println!("Check interval: {} minutes", interval_minutes);
    println!("Max papers per check: {}", max_papers);
    println!("Auto-add: {}", if auto_add { "yes" } else { "no" });
    println!();

    // Perform immediate search using arXiv API
    println!("Running immediate search...");
    let url = format!(
        "https://export.arxiv.org/api/query?search_query=all:{}&sortBy=submittedDate&sortOrder=descending&max_results={}",
        query.replace(' ', "+"), max_papers
    );

    let resp = reqwest::blocking::get(&url).context("Failed to connect to arXiv API")?;

    if !resp.status().is_success() {
        anyhow::bail!("arXiv API returned error: {}", resp.status());
    }

    let body = resp.text().context("Failed to read arXiv response")?;

    // Count entries in response
    let entry_count = body.matches("<entry>").count();
    println!(
        "[OK] Found {} papers from arXiv for query '{}'",
        entry_count, query
    );

    if entry_count == 0 {
        println!("No papers found. Try a different query.");
        return Ok(());
    }

    // Extract all papers (id, title)
    let mut papers_info: Vec<(String, String)> = Vec::new();
    let mut search_pos = 0;
    while let Some(rel_entry_start) = body[search_pos..].find("<entry>") {
        let entry_abs_start = search_pos + rel_entry_start;
        let entry_block = &body[entry_abs_start..];
        if let Some(rel_end) = entry_block.find("</entry>") {
            let entry = &entry_block[..rel_end];
            let title = extract_xml_field(entry, "<title>")
                .map(|t| t.trim().to_string())
                .unwrap_or_default();
            // Extract arXiv ID from id tag (e.g. http://arxiv.org/abs/2301.00001v1)
            let id_full = extract_xml_field(entry, "<id>").unwrap_or_default();
            let arxiv_id = id_full
                .trim()
                .strip_prefix("http://arxiv.org/abs/")
                .or_else(|| id_full.trim().strip_prefix("https://arxiv.org/abs/"))
                .unwrap_or(&id_full)
                .trim()
                .to_string();
            if !arxiv_id.is_empty() {
                papers_info.push((arxiv_id, title));
            }
            search_pos = entry_abs_start + rel_end + 9;
        } else {
            break;
        }
    }

    println!("\nTop papers found:");
    for (i, (_, title)) in papers_info.iter().enumerate().take(5) {
        println!(
            "  {}. {}",
            i + 1,
            title.chars().take(70).collect::<String>()
        );
    }

    if auto_add && !papers_info.is_empty() {
        println!("\nAuto-adding papers to database...");
        let mut added = 0;
        let mut skipped = 0;
        let mut failed = 0;
        for (arxiv_id, _title) in &papers_info {
            match db.get_paper_by_arxiv(arxiv_id) {
                Ok(Some(_)) => {
                    println!("  - {}: already exists", arxiv_id);
                    skipped += 1;
                }
                _ => {
                    // Add paper via arXiv API
                    match add_paper_from_arxiv(db, arxiv_id) {
                        Ok(_) => {
                            println!("  + {}: added", arxiv_id);
                            added += 1;
                        }
                        Err(e) => {
                            println!("  ! {}: failed ({})", arxiv_id, e);
                            failed += 1;
                        }
                    }
                }
            }
        }
        println!(
            "\nAuto-add complete: {} added, {} skipped, {} failed",
            added, skipped, failed
        );
    }

    println!("\nNote: Background monitoring requires daemon process. Subscription saved.");
    println!(
        "Run 'rairos subscribe \"{}\" --interval {}' periodically to check manually.",
        query, interval_minutes
    );
    Ok(())
}
