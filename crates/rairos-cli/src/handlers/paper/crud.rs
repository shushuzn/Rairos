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

use crate::{
    parse_status_arg, status_str,
};

pub fn handle_add(db: &Database, arxiv_id: &str) -> Result<()> {
    let arxiv_id = arxiv_id.trim();

    if let Ok(Some(_)) = db.get_paper_by_arxiv(arxiv_id) {
        println!("Paper {} already exists in database.", arxiv_id);
        return Ok(());
    }

    add_paper_from_arxiv(db, arxiv_id)
}

pub fn add_paper_from_arxiv(db: &Database, arxiv_id: &str) -> Result<()> {
    println!("Fetching metadata from arXiv for {}...", arxiv_id);

    let rt = tokio::runtime::Runtime::new().context("Failed to create tokio runtime")?;
    let paper = rt
        .block_on(rairos_parser::fetch_arxiv(arxiv_id))
        .with_context(|| format!("Failed to fetch arXiv paper: {}", arxiv_id))?;

    println!("  Title: {}", paper.title.chars().take(60).collect::<String>());
    println!(
        "  Authors: {}",
        paper.authors
            .iter()
            .take(3)
            .cloned()
            .collect::<Vec<_>>()
            .join(", ")
    );
    println!("  Published: {}", paper.published.format("%Y-%m-%d"));
    println!(
        "  Categories: {}",
        paper.categories
            .iter()
            .take(5)
            .cloned()
            .collect::<Vec<_>>()
            .join(", ")
    );

    db.insert_paper(&paper)?;
    println!(
        "\n[OK] Added: {} ({})",
        paper.id,
        paper.arxiv_id.as_deref().unwrap_or(arxiv_id)
    );
    Ok(())
}

pub fn extract_xml_field(xml: &str, tag: &str) -> Option<String> {
    if let Some(start) = xml.find(tag) {
        let content_start = start + tag.len();
        if let Some(end) = xml[content_start..].find("</") {
            return Some(xml[content_start..content_start + end].to_string());
        }
    }
    None
}

#[allow(dead_code)]
fn extract_all_xml_fields(xml: &str, tag: &str) -> Vec<String> {
    let mut results = Vec::new();
    let mut search_pos = 0;

    while let Some(tag_start) = xml[search_pos..].find(tag) {
        let content_start = tag_start + tag.len();
        let rest = &xml[search_pos..];
        let after_tag = &rest[content_start..];
        if let Some(end_offset) = after_tag.find("</") {
            results.push(after_tag[..end_offset].to_string());
            search_pos += content_start + end_offset + 3;
        } else {
            break;
        }
    }
    results
}

pub fn handle_list(
    db: &Database,
    status: Option<String>,
    year: Option<i32>,
    tags: &[String],
    limit: usize,
    offset: usize,
    sort: &str,
    order: &str,
    format: &str,
) -> Result<()> {
    let parse_status = status.as_ref().and_then(|s| parse_status_arg(s));
    let mut papers = db.list_papers(parse_status, 10000, 0)?;

    if let Some(y) = year {
        papers.retain(|p| {
            p.published
                .format("%Y")
                .to_string()
                .parse::<i32>()
                .unwrap_or(0)
                == y
        });
    }

    if !tags.is_empty() {
        let tags_lower: std::collections::HashSet<_> =
            tags.iter().map(|t| t.to_lowercase()).collect();
        papers.retain(|p| {
            let paper_tags: std::collections::HashSet<_> =
                p.categories.iter().map(|s| s.to_lowercase()).collect();
            tags_lower.iter().all(|t| paper_tags.contains(t))
        });
    }

    let reverse = order == "desc";
    match sort {
        "published" => papers.sort_by(|a, b| {
            if reverse {
                b.published.cmp(&a.published)
            } else {
                a.published.cmp(&b.published)
            }
        }),
        "title" => papers.sort_by(|a, b| {
            if reverse {
                b.title.cmp(&a.title)
            } else {
                a.title.cmp(&b.title)
            }
        }),
        "status" => papers.sort_by(|a, b| {
            if reverse {
                status_str(&b.parse_status).cmp(&status_str(&a.parse_status))
            } else {
                status_str(&a.parse_status).cmp(&status_str(&b.parse_status))
            }
        }),
        _ => {}
    }

    let total = papers.len();
    papers = papers.into_iter().skip(offset).take(limit).collect();

    if format == "json" {
        let out: Vec<serde_json::Value> = papers
            .iter()
            .map(|p| {
                serde_json::json!({
                    "id": p.id,
                    "arxiv_id": p.arxiv_id,
                    "title": p.title,
                    "authors": p.authors,
                    "published": p.published,
                    "status": status_str(&p.parse_status),
                    "categories": p.categories,
                })
            })
            .collect();
        println!("{}", serde_json::to_string_pretty(&out)?);
        return Ok(());
    }

    println!(
        "Showing {}/{} papers (sort: {} {}, offset: {})",
        papers.len(),
        total,
        sort,
        order,
        offset
    );
    println!();
    println!("{:<38} {:<10} {:<12} {}", "ID", "STATUS", "ARXIV", "TITLE");
    println!("{}", "-".repeat(120));
    for paper in &papers {
        let id_short = if paper.id.len() > 8 {
            &paper.id[..8]
        } else {
            &paper.id
        };
        let arxiv = paper.arxiv_id.as_deref().unwrap_or("-");
        let title = if paper.title.len() > 50 {
            &paper.title[..50]
        } else {
            &paper.title
        };
        println!(
            "{:<38} {:<10} {:<12} {}",
            id_short,
            status_str(&paper.parse_status),
            arxiv,
            title
        );
    }
    println!("\n{} papers shown.", papers.len());
    Ok(())
}

pub fn handle_show(db: &Database, id: &str, format: &str) -> Result<()> {
    let paper = if let Ok(p) = db.get_paper(id) {
        p
    } else if let Ok(Some(p)) = db.get_paper_by_arxiv(id) {
        p
    } else {
        anyhow::bail!("Paper not found: {}", id);
    };

    if format == "json" {
        let out = serde_json::json!({
            "id": paper.id,
            "arxiv_id": paper.arxiv_id,
            "title": paper.title,
            "authors": paper.authors,
            "published": paper.published,
            "status": status_str(&paper.parse_status),
            "abstract": paper.abstract_text,
            "categories": paper.categories,
            "cited_by": paper.metadata.cited_by,
            "references": paper.metadata.references,
        });
        println!("{}", serde_json::to_string_pretty(&out)?);
        return Ok(());
    }

    println!("=== Paper Details ===");
    println!("ID:          {}", paper.id);
    println!("arXiv:       {:?}", paper.arxiv_id);
    println!("Title:       {}", paper.title);
    println!(
        "Authors:     {:?}",
        paper.authors.iter().take(5).collect::<Vec<_>>()
    );
    if paper.authors.len() > 5 {
        println!("             ... and {} more", paper.authors.len() - 5);
    }
    println!("Published:   {}", paper.published);
    println!("Status:      {}", status_str(&paper.parse_status));
    println!("Categories:  {:?}", paper.categories);
    println!("Cited by:    {}", paper.metadata.cited_by);
    println!("References:  {}", paper.metadata.references);
    println!();
    let abstract_preview = if paper.abstract_text.len() > 300 {
        format!("{}...", &paper.abstract_text[..300])
    } else {
        paper.abstract_text.clone()
    };
    println!("Abstract:\n{}", abstract_preview);
    Ok(())
}

pub fn handle_delete(db: &Database, ids: &[String], force: bool) -> Result<()> {
    if ids.len() > 1 && !force {
        print!("Delete {} papers? [y/N] ", ids.len());
        std::io::Write::flush(&mut std::io::stdout()).ok();
        let mut confirm = String::new();
        if std::io::stdin().read_line(&mut confirm).is_err()
            || !confirm.trim().eq_ignore_ascii_case("y")
        {
            println!("Cancelled.");
            return Ok(());
        }
    } else if ids.len() == 1 && !force {
        print!("Delete paper '{}'? [y/N] ", ids[0]);
        std::io::Write::flush(&mut std::io::stdout()).ok();
        let mut confirm = String::new();
        if std::io::stdin().read_line(&mut confirm).is_err()
            || !confirm.trim().eq_ignore_ascii_case("y")
        {
            println!("Cancelled.");
            return Ok(());
        }
    }

    let mut deleted = 0;
    let mut failed = 0;
    for id in ids {
        match db.delete_paper(id) {
            Ok(_) => {
                println!("Deleted: {}", id);
                deleted += 1;
            }
            Err(e) => {
                eprintln!("Failed: {} ({})", id, e);
                failed += 1;
            }
        }
    }
    println!("\nDelete complete: {} deleted, {} failed", deleted, failed);
    Ok(())
}

pub fn handle_update_status(db: &Database, ids: &[String], status: &str) -> Result<()> {
    let parse_status = parse_status_arg(status).ok_or_else(|| {
        anyhow::anyhow!(
            "Invalid status '{}'. Use: pending, parsing, done, failed",
            status
        )
    })?;

    let mut updated = 0;
    let mut failed = 0;
    for id in ids {
        match db.update_paper_status(id, parse_status) {
            Ok(_) => {
                println!("Updated: {} -> {}", id, status);
                updated += 1;
            }
            Err(e) => {
                eprintln!("Failed: {} ({})", id, e);
                failed += 1;
            }
        }
    }
    println!(
        "\nStatus update complete: {} updated, {} failed",
        updated, failed
    );
    Ok(())
}

pub fn handle_stats(db: &Database, json: bool, format: &str) -> Result<()> {
    let stats = db.stats()?;

    if json || format == "json" {
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({
                "total_papers": stats.total,
                "pending": stats.pending,
                "done": stats.done,
                "research_gaps": stats.gaps,
            }))?
        );
        return Ok(());
    }

    println!("=== Rairos Database Statistics ===");
    println!("Total papers:  {}", stats.total);
    println!("  Pending:     {}", stats.pending);
    println!("  Done:        {}", stats.done);
    println!("Research gaps: {}", stats.gaps);
    Ok(())
}
