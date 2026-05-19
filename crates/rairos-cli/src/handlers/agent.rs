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
use rairos_core::Database;

pub fn handle_agent(
    db: &Database,
    topic: &str,
    max_papers: usize,
    _max_time_minutes: u64,
    format: &str,
) -> Result<()> {
    println!("=== Rairos Research Agent ===");
    println!("Topic: {}", topic);
    println!("Max papers: {}", max_papers);
    println!();

    let papers = db.search_papers_smart(topic, max_papers)?;

    if papers.is_empty() {
        println!("No papers found for topic '{}'.", topic);
        return Ok(());
    }

    println!("Found {} papers. Starting research loop...\n", papers.len());
    println!("(Full autonomous research loop requires LLM integration)");
    println!();

    println!("Research Plan:");
    println!(
        "  1. Analyze {} papers for key themes and methodologies",
        papers.len()
    );
    println!("  2. Identify research gaps and opportunities");
    println!("  3. Generate hypotheses for further investigation");
    println!();

    if format == "json" {
        let out = serde_json::json!({
            "topic": topic,
            "papers_found": papers.len(),
            "status": "planned"
        });
        println!("{}", serde_json::to_string_pretty(&out)?);
    } else {
        println!("[OK] Agent initialized for topic: {}", topic);
        println!("Run 'rairos analyze --kind=summary --paper=<id>' to analyze individual papers.");
    }

    Ok(())
}
