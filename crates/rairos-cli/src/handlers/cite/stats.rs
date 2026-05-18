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

pub fn handle_cite_stats(
    db: &Database,
    paper: Option<&str>,
    top: Option<usize>,
    format: &str,
) -> Result<()> {
    if let Some(paper_id) = paper {
        let citations = db.get_citations(paper_id)?;
        let pap = db.get_paper(paper_id);

        if format == "json" {
            let out = serde_json::json!({
                "paper_id": paper_id,
                "title": pap.ok().map(|p| p.title),
                "citing_count": citations.citing.len(),
                "references_count": citations.references.len(),
            });
            println!("{}", serde_json::to_string_pretty(&out)?);
        } else {
            println!("=== Citation Stats for Paper ===\n");
            println!("Paper ID:   {}", paper_id);
            if let Ok(p) = pap {
                println!("Title:      {}", p.title);
            }
            println!("Cited by:   {} papers", citations.citing.len());
            println!("References: {} papers", citations.references.len());
            if !citations.citing.is_empty() {
                println!("\nCited by:");
                for cid in &citations.citing {
                    println!("  - {}", cid);
                }
            }
            if !citations.references.is_empty() {
                println!("\nReferences:");
                for cid in &citations.references {
                    println!("  - {}", cid);
                }
            }
        }
        return Ok(());
    }

    let stats = db.stats()?;
    let all_papers = db.list_papers(None, 10000, 0)?;

    if format == "json" {
        let out = serde_json::json!({
            "total_papers": stats.total,
            "pending": stats.pending,
            "done": stats.done,
        });
        println!("{}", serde_json::to_string_pretty(&out)?);
        return Ok(());
    }

    println!("=== Citation Statistics ===\n");
    println!("Total papers:  {}", stats.total);
    println!("Pending:       {}", stats.pending);
    println!("Parsed:        {}", stats.done);

    if let Some(n) = top {
        let mut papers_with_cites: Vec<_> = all_papers
            .iter()
            .filter(|p| p.metadata.cited_by > 0 || p.metadata.references > 0)
            .collect();
        papers_with_cites.sort_by(|a, b| b.metadata.cited_by.cmp(&a.metadata.cited_by));
        println!("\nTop {} most-cited papers:", n);
        for p in papers_with_cites.iter().take(n) {
            println!(
                "  [{:4}] {}  {}",
                p.metadata.cited_by,
                p.id,
                p.title.chars().take(60).collect::<String>()
            );
        }
    }

    Ok(())
}
