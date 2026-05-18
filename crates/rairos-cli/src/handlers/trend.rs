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

pub fn handle_trend(db: &Database, topic: &str, range: &str, format: &str) -> Result<()> {
    use chrono::{Duration, Utc};

    // Parse time range
    let days = match range {
        "6m" => 180,
        "1y" => 365,
        "2y" => 730,
        "5y" => 1825,
        "all" => 9999,
        other => {
            println!("Unknown range '{}'. Use: 6m, 1y, 2y, 5y, all", other);
            return Ok(());
        }
    };

    let cutoff = Utc::now() - Duration::days(days);
    println!("=== Research Trends ===");
    println!("Topic: {}", topic);
    println!("Time range: {} (papers from last {} days)", range, days);
    println!();

    let all_papers = db.search_papers(topic, 500)?;
    let papers: Vec<_> = all_papers
        .into_iter()
        .filter(|p| p.published >= cutoff)
        .collect();

    if papers.is_empty() {
        println!(
            "No papers found for topic '{}' in the last {}.",
            topic, range
        );
        return Ok(());
    }

    println!(
        "Found {} papers on '{}' in the specified time range.",
        papers.len(),
        topic
    );
    println!();

    // Group by year
    let mut year_counts: std::collections::HashMap<String, usize> =
        std::collections::HashMap::new();
    for paper in &papers {
        let year = paper.published.format("%Y").to_string();
        *year_counts.entry(year).or_insert(0) += 1;
    }

    let mut years: Vec<_> = year_counts.keys().cloned().collect();
    years.sort();

    println!("Papers per year:");
    for year in &years {
        let count = year_counts[year];
        let bar = "#".repeat(count.min(50));
        println!("  {} | {} {}", year, count, bar);
    }

    println!();
    println!("Trend analysis:");
    if years.len() >= 2 {
        println!("  - {} different years covered", years.len());
        if let (Some(first), Some(last)) = (years.first(), years.last()) {
            let first_count = year_counts[first];
            let last_count = year_counts[last];
            if last_count > first_count {
                println!(
                    "  - Growing trend: {} -> {} papers",
                    first_count, last_count
                );
            } else {
                println!(
                    "  - Stable/declining: {} -> {} papers",
                    first_count, last_count
                );
            }
        }
    } else if years.len() == 1 {
        println!("  - Only one year represented: {}", years[0]);
    }

    // Top categories
    let mut cat_counts: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    for paper in &papers {
        for cat in &paper.categories {
            *cat_counts.entry(cat.clone()).or_insert(0) += 1;
        }
    }
    let mut top_cats: Vec<_> = cat_counts.iter().collect();
    top_cats.sort_by(|a, b| b.1.cmp(a.1));
    println!(
        "  - Top categories: {}",
        top_cats
            .iter()
            .take(5)
            .map(|(c, _)| c.as_str())
            .collect::<Vec<_>>()
            .join(", ")
    );

    if format == "json" {
        let out = serde_json::json!({
            "topic": topic,
            "range": range,
            "days": days,
            "papers_found": papers.len(),
            "year_counts": year_counts,
            "top_categories": top_cats.iter().take(5).map(|(c, n)| (c, *n)).collect::<std::collections::HashMap<_, _>>()
        });
        println!("{}", serde_json::to_string_pretty(&out)?);
    }

    Ok(())
}
