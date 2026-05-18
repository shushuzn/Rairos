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

pub fn handle_status(db: &Database, format: &str) -> Result<()> {
    let papers = db.list_papers(None, 10000, 0)?;
    let total = papers.len();

    let mut by_status: std::collections::BTreeMap<String, usize> = std::collections::BTreeMap::new();

    for p in &papers {
        let status = p.parse_status.to_string();
        *by_status.entry(status).or_default() += 1;
    }

    if format == "json" {
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({
                "total_papers": total,
                "by_status": by_status,
            }))?
        );
        return Ok(());
    }

    println!("Total papers: {}", total);
    println!(
        "By status: {}",
        by_status
            .iter()
            .map(|(k, v)| format!("{}={}", k, v))
            .collect::<Vec<_>>()
            .join(", ")
    );
    Ok(())
}