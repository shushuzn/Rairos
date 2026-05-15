//! CLI command handler implementations.
//!
//! Extracted from main.rs for maintainability. Each handler
//! corresponds to one Commands variant from the parent module.

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
use chrono::Utc;
use rairos_core::{Database, Paper};
use rairos_pdf;
use std::collections::HashSet;
use std::path::PathBuf;

use crate::{
    DedupAction,
    parse_status_arg, status_str,
};


// ====================================================================
// Handler implementations
// ====================================================================

// ============================================================================
// Command Handlers
// ============================================================================

pub fn handle_init(db_path: &PathBuf) -> Result<()> {
    if db_path.exists() {
        println!("Database already exists at: {}", db_path.display());
    } else {
        println!("Creating new database: {}", db_path.display());
    }
    let db = Database::open(db_path)?;
    let stats = db.stats()?;
    println!("Database initialized.");
    println!("  Papers: {}", stats.total);
    println!("  Gaps:   {}", stats.gaps);
    Ok(())
}

pub fn handle_add(db: &Database, arxiv_id: &str) -> Result<()> {
    let arxiv_id = arxiv_id.trim();

    // Check if already exists
    if let Ok(Some(_)) = db.get_paper_by_arxiv(arxiv_id) {
        println!("Paper {} already exists in database.", arxiv_id);
        return Ok(());
    }

    add_paper_from_arxiv(db, arxiv_id)
}

pub fn add_paper_from_arxiv(db: &Database, arxiv_id: &str) -> Result<()> {
    // Fetch from arXiv API
    println!("Fetching metadata from arXiv for {}...", arxiv_id);

    let url = format!("https://export.arxiv.org/api/query?id_list={}", arxiv_id);
    let resp = reqwest::blocking::get(&url).context("Failed to connect to arXiv API")?;

    if !resp.status().is_success() {
        anyhow::bail!("arXiv API returned error: {}", resp.status());
    }

    let body = resp.text().context("Failed to read arXiv response")?;

    // arXiv ATOM feed has feed-level <title> and <summary> BEFORE <entry>
    // We need the entry-level fields, so extract entry block first
    let entry_start = body.find("<entry>").unwrap_or(0);
    let entry_end = body.find("</entry>").unwrap_or(body.len());
    let entry = &body[entry_start..entry_end];

    // Parse entry-level fields
    let title = extract_xml_field(entry, "<title>")
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|| format!("arXiv:{}", arxiv_id));
    let abstract_text = extract_xml_field(entry, "<summary>")
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|| "Abstract not available".to_string());

    let authors: Vec<String> = extract_all_xml_fields(entry, "<name>")
        .into_iter()
        .map(|s| s.trim().to_string())
        .collect();

    let published = extract_xml_field(entry, "<published>")
        .and_then(|s| chrono::DateTime::parse_from_rfc3339(&s).ok())
        .map(|dt| dt.with_timezone(&chrono::Utc))
        .unwrap_or_else(chrono::Utc::now);

    let categories: Vec<String> = extract_all_xml_fields(entry, "<category term=")
        .into_iter()
        .filter(|s| s.contains('"'))
        .map(|s| s.split('"').nth(1).unwrap_or(&s).to_string())
        .collect();

    let _primary_category = extract_xml_field(entry, "arxiv:primary_category term=")
        .map(|s| s.split('"').nth(1).unwrap_or(&s).to_string())
        .unwrap_or_else(|| categories.first().cloned().unwrap_or_default());

    println!("  Title: {}", title.chars().take(60).collect::<String>());
    println!(
        "  Authors: {}",
        authors
            .iter()
            .take(3)
            .cloned()
            .collect::<Vec<_>>()
            .join(", ")
    );
    println!("  Published: {}", published.format("%Y-%m-%d"));
    println!(
        "  Categories: {}",
        categories
            .iter()
            .take(5)
            .cloned()
            .collect::<Vec<_>>()
            .join(", ")
    );

    // Build paper with fetched metadata
    let mut paper = Paper::new(Some(arxiv_id.to_string()), title, abstract_text);
    paper.authors = authors;
    paper.categories = categories;
    paper.published = published;

    db.insert_paper(&paper)?;
    println!("\n[OK] Added: {} ({})", paper.id, arxiv_id);
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

fn extract_all_xml_fields(xml: &str, tag: &str) -> Vec<String> {
    let mut results = Vec::new();
    let mut search_pos = 0;

    while let Some(tag_start) = xml[search_pos..].find(tag) {
        let content_start = tag_start + tag.len();
        let rest = &xml[search_pos..];
        let after_tag = &rest[content_start..];
        if let Some(end_offset) = after_tag.find("</") {
            results.push(after_tag[..end_offset].to_string());
            search_pos += content_start + end_offset + 3; // Skip past </tag>
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

    // Year filter
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

    // Tag filter (paper must have ALL specified tags)
    if !tags.is_empty() {
        papers.retain(|p| {
            let paper_tags: std::collections::HashSet<_> =
                p.categories.iter().map(|s| s.to_lowercase()).collect();
            tags.iter().all(|t| paper_tags.contains(&t.to_lowercase()))
        });
    }

    // Sort
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
        _ => {} // added_at - keep insertion order
    }

    // Apply offset/limit
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

    // Table format
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

pub fn handle_search(
    db: &Database,
    query: &str,
    limit: usize,
    field: &str,
    format: &str,
) -> Result<()> {
    // Use search_papers for real keyword matching in title/abstract
    let papers = db.search_papers(query, limit)?;

    let filtered: Vec<&Paper> = if field == "all" {
        papers.iter().collect()
    } else {
        papers
            .iter()
            .filter(|p| match field {
                "title" => p.title.to_lowercase().contains(&query.to_lowercase()),
                "abstract" => p
                    .abstract_text
                    .to_lowercase()
                    .contains(&query.to_lowercase()),
                "authors" => p
                    .authors
                    .iter()
                    .any(|a| a.to_lowercase().contains(&query.to_lowercase())),
                "categories" => p
                    .categories
                    .iter()
                    .any(|c| c.to_lowercase().contains(&query.to_lowercase())),
                _ => true,
            })
            .collect()
    };

    let papers_vec: Vec<Paper> = filtered.into_iter().cloned().collect();

    if format == "json" {
        let out: Vec<serde_json::Value> = papers_vec
            .iter()
            .map(|p| {
                serde_json::json!({
                    "id": p.id,
                    "arxiv_id": p.arxiv_id,
                    "title": p.title,
                    "authors": p.authors,
                    "published": p.published,
                    "categories": p.categories,
                    "cited_by": p.metadata.cited_by,
                })
            })
            .collect();
        println!("{}", serde_json::to_string_pretty(&out)?);
        return Ok(());
    }

    if papers_vec.is_empty() {
        println!("No papers found for query: {} (field: {})", query, field);
        return Ok(());
    }

    println!(
        "Found {} papers for '{}' (field: {}):",
        papers_vec.len(),
        query,
        field
    );
    println!();
    for (i, paper) in papers_vec.iter().enumerate() {
        println!("{}. {}", i + 1, paper.title);
        if let Some(ref arxiv) = paper.arxiv_id {
            println!("   arXiv: {}", arxiv);
        }
        println!(
            "   {} | cited_by: {}",
            paper.published, paper.metadata.cited_by
        );
        let abstract_preview = if paper.abstract_text.len() > 100 {
            format!("{}...", &paper.abstract_text[..100])
        } else {
            paper.abstract_text.clone()
        };
        println!("   {}", abstract_preview);
        println!();
    }
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

pub fn handle_parse(db: &Database, id: &str) -> Result<()> {
    let paper = if let Ok(p) = db.get_paper(id) {
        p
    } else if let Ok(Some(p)) = db.get_paper_by_arxiv(id) {
        p
    } else {
        anyhow::bail!("Paper not found: {}", id);
    };

    let arxiv_id = paper.arxiv_id.as_deref().unwrap_or(&paper.id);
    println!("Parsing paper: {}", paper.title);
    println!("  arXiv: {}", arxiv_id);

    let pdf_dir = dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".ai_research_os")
        .join("pdfs");
    std::fs::create_dir_all(&pdf_dir).context("Failed to create pdfs directory")?;

    let pdf_path = pdf_dir.join(format!("{}.pdf", arxiv_id));

    if !pdf_path.exists() {
        let pdf_url = format!("https://arxiv.org/pdf/{}.pdf", arxiv_id);
        println!("  Downloading from {} ...", pdf_url);
        let rt = tokio::runtime::Runtime::new().context("Failed to create tokio runtime")?;
        rt.block_on(rairos_pdf::download_pdf(&pdf_url, &pdf_path))
            .context("Failed to download PDF")?;
        println!("  Downloaded to: {}", pdf_path.display());
    } else {
        println!("  Using cached PDF: {}", pdf_path.display());
    }

    println!("  Extracting text ...");
    let text = rairos_pdf::extract_pdf_text(&pdf_path)
        .context("Failed to extract text from PDF")?;

    println!("\n  Text length: {} characters", text.len());

    let preview: String = text.chars().take(500).collect();
    println!("\n--- Preview (first 500 chars) ---\n{}", preview);

    db.update_paper_status(&paper.id, rairos_core::ParseStatus::Done)
        .context("Failed to update paper status")?;
    println!("\n[OK] Parse complete. Status set to 'done'.");
    Ok(())
}

pub fn handle_import(
    db: &Database,
    path: &Option<PathBuf>,
    ids: &[String],
    skip_existing: bool,
) -> Result<()> {
    if let Some(p) = path {
        // JSON file import
        let content = std::fs::read_to_string(p)?;
        let papers: Vec<Paper> = serde_json::from_str(&content)
            .context("Failed to parse JSON — expected array of Paper objects")?;

        let count = papers.len();
        for paper in &papers {
            db.insert_paper(paper)?;
        }
        println!("Imported {} papers from {}", count, p.display());
    } else if !ids.is_empty() {
        // Batch import from arXiv IDs
        let mut added = 0;
        let mut skipped = 0;
        let mut failed = 0;

        for arxiv_id in ids {
            let arxiv_id = arxiv_id.trim();
            if arxiv_id.is_empty() {
                continue;
            }

            // Check if already exists
            if skip_existing {
                if let Ok(Some(_)) = db.get_paper_by_arxiv(arxiv_id) {
                    println!("Skipped (exists): {}", arxiv_id);
                    skipped += 1;
                    continue;
                }
            }

            println!("Fetching {}...", arxiv_id);
            match add_paper_from_arxiv(db, arxiv_id) {
                Ok(_) => {
                    println!("Added: {}", arxiv_id);
                    added += 1;
                }
                Err(e) => {
                    eprintln!("Failed {}: {}", arxiv_id, e);
                    failed += 1;
                }
            }
        }

        println!(
            "\nImport complete: {} added, {} skipped, {} failed",
            added, skipped, failed
        );
    } else {
        println!("Nothing to import. Use positional arXiv IDs or --ids flag, or provide a JSON file path.");
    }
    Ok(())
}

pub fn handle_export(
    db: &Database,
    path: &PathBuf,
    status: Option<String>,
    format: &str,
) -> Result<()> {
    let parse_status = status.as_ref().and_then(|s| parse_status_arg(s));
    let papers = db.list_papers(parse_status, 10000, 0)?;

    if format == "csv" {
        let mut w = csv::Writer::from_path(path)?;
        w.write_record(&[
            "id",
            "arxiv_id",
            "title",
            "authors",
            "published",
            "status",
            "cited_by",
            "categories",
        ])?;
        for p in &papers {
            w.write_record(&[
                &p.id,
                p.arxiv_id.as_deref().unwrap_or(""),
                &p.title,
                &p.authors.join("; "),
                &p.published.format("%Y-%m-%d").to_string(),
                status_str(&p.parse_status),
                &p.metadata.cited_by.to_string(),
                &p.categories.join("; "),
            ])?;
        }
        w.flush()?;
    } else {
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
                    "cited_by": p.metadata.cited_by,
                    "categories": p.categories,
                    "abstract": p.abstract_text,
                })
            })
            .collect();
        std::fs::write(path, serde_json::to_string_pretty(&out)?)?;
    }

    println!("Exported {} papers to {}", papers.len(), path.display());
    Ok(())
}

pub fn handle_dedup(db: &Database, action: &DedupAction) -> Result<()> {
    match action {
        DedupAction::Find { threshold } => {
            println!("=== Finding Duplicate Papers ===");
            println!("Similarity threshold: {:.2}", threshold);
            println!();

            let papers = db.list_papers(None, 1000, 0)?;
            println!("Checking {} papers for duplicates...", papers.len());

            // Build Jaccard similarity for all pairs
            let mut dup_groups: Vec<Vec<usize>> = Vec::new();
            let mut used: Vec<bool> = vec![false; papers.len()];

            for i in 0..papers.len() {
                if used[i] {
                    continue;
                }
                let mut group: Vec<usize> = vec![i];
                used[i] = true;

                for j in (i + 1)..papers.len() {
                    if used[j] {
                        continue;
                    }
                    let sim = title_similarity(&papers[i].title, &papers[j].title);
                    if sim >= *threshold as f64 {
                        group.push(j);
                        used[j] = true;
                    }
                }

                if group.len() > 1 {
                    dup_groups.push(group);
                }
            }

            if dup_groups.is_empty() {
                println!("\n[OK] Found 0 duplicate groups");
            } else {
                println!("\n[OK] Found {} duplicate group(s):", dup_groups.len());
                for (gi, group) in dup_groups.iter().enumerate() {
                    println!("\n--- Group {} ({} papers) ---", gi + 1, group.len());
                    for &idx in group {
                        let p = &papers[idx];
                        let arxiv = p.arxiv_id.as_deref().unwrap_or("-");
                        let title_short = if p.title.len() > 70 {
                            format!("{}...", &p.title[..70])
                        } else {
                            p.title.clone()
                        };
                        println!("  [{:>3}] {}  [{}]", idx + 1, title_short, arxiv);
                    }
                }
            }
        }
        DedupAction::Remove { papers: _ids } => {
            println!("=== Removing Duplicates ===");
            // In production would remove selected IDs from database
            println!("(Full removal requires --confirm flag and careful ID selection)");
            println!("To remove, run: rairos dedup find --threshold 0.85, then manually remove");
        }
        DedupAction::Groups => {
            println!("=== Duplicate Groups ===");
            println!("Run 'rairos dedup find --threshold <0.0-1.0>' first to detect duplicates");
        }
        DedupAction::Stats => {
            let (total, with_emb) = db.get_embedding_stats()?;
            let pct = if total > 0 { (with_emb as f64 / total as f64) * 100.0 } else { 0.0 };
            println!("\n  \x1b[36mEmbedding Coverage\x1b[0m");
            println!("  \x1b[36mPapers with embedding:\x1b[0m  \x1b[92m{}\x1b[0m", with_emb);
            println!("  \x1b[36mPapers with text:\x1b[0m      \x1b[91m{}\x1b[0m", total);
            println!("  \x1b[36mCoverage:\x1b[0m              \x1b[93m{:.1}%\x1b[0m", pct);
            println!();
        }
        DedupAction::Semantic { paper, threshold, limit } => {
            let exists = db.paper_exists(paper);
            if !exists {
                eprintln!("Paper '{}' not found", paper);
                return Ok(());
            }
            let sims = db.find_similar(paper, *limit, *threshold)?;
            if sims.is_empty() {
                println!("\n  \x1b[36mSimilar Papers — {}\x1b[0m", paper);
                println!("  \x1b[90mNo similar papers above threshold=\x1b[36m{}\x1b[0m", threshold);
                println!();
                return Ok(());
            }
            println!("\n  \x1b[36mSimilar Papers — \x1b[91m{}\x1b[0m (threshold=\x1b[91m{}\x1b[0m)\x1b[0m", paper, threshold);
            println!("  \x1b[36m{} similar papers found\x1b[0m", sims.len());
            println!();
            println!("  {:<10} {:>12}  {}", "Score", "Paper ID", "Title");
            println!("  {} {} {}", "─".repeat(10), "─".repeat(12), "─".repeat(50));
            for (id, score) in &sims {
                let score_color = if *score >= 0.95 { "\x1b[92m" } else if *score >= 0.85 { "\x1b[93m" } else { "\x1b[91m" };
                let paper = db.get_paper(id)?;
                let title = if paper.title.len() > 47 { format!("{}...", &paper.title[..47]) } else { paper.title.clone() };
                println!("  {}{:.4}\x1b[0m  {:>12}  \x1b[36m{}\x1b[0m", score_color, score, id, title);
            }
            println!();
        }
    }

    Ok(())
}

pub fn title_similarity(a: &str, b: &str) -> f64 {
    let words_a: std::collections::HashSet<&str> = a
        .split_whitespace()
        .map(|w| w.trim_matches(|c: char| !c.is_alphanumeric()))
        .filter(|w| !w.is_empty())
        .collect();
    let words_b: std::collections::HashSet<&str> = b
        .split_whitespace()
        .map(|w| w.trim_matches(|c: char| !c.is_alphanumeric()))
        .filter(|w| !w.is_empty())
        .collect();

    if words_a.is_empty() && words_b.is_empty() {
        return 0.0;
    }

    let intersection = words_a.intersection(&words_b).count();
    let union = words_a.union(&words_b).count();
    if union == 0 {
        0.0
    } else {
        intersection as f64 / union as f64
    }
}

pub fn handle_similar(db: &Database, paper_id: &str, limit: usize) -> Result<()> {
    let paper = db
        .get_paper(paper_id)
        .ok()
        .or_else(|| db.get_paper_by_arxiv(paper_id).ok().flatten())
        .ok_or_else(|| anyhow::anyhow!("Paper not found: {}", paper_id))?;

    println!("=== Similar Papers ===\n");
    println!("Finding papers similar to:");
    println!("  {} ({})", paper.title, paper.published.format("%Y-%m-%d"));
    println!();

    let all_papers = db.list_papers(None, 1000, 0)?;
    let target_title = paper.title.to_lowercase();
    let target_abstract = paper.abstract_text.to_lowercase();
    let target_text = format!("{} {}", target_title, target_abstract);

    // Compute similarity for each paper using word overlap
    let stop_words: std::collections::HashSet<&str> = [
        "the", "a", "an", "is", "are", "was", "were", "be", "been", "being", "have", "has", "had",
        "do", "does", "did", "will", "would", "could", "should", "may", "might", "must", "shall",
        "can", "need", "to", "of", "in", "for", "on", "with", "at", "by", "from", "as", "into",
        "through", "during", "before", "after", "above", "below", "between", "under", "again",
        "further", "then", "once", "here", "there", "when", "where", "why", "how", "all", "each",
        "few", "more", "most", "other", "some", "such", "no", "nor", "not", "only", "own", "same",
        "so", "than", "too", "very", "just", "but", "and", "or", "if", "because", "until", "while",
        "this", "that", "these", "those", "what", "which", "who", "whom",
    ]
    .into();

    let target_words: std::collections::HashSet<String> = target_text
        .split(|c: char| !c.is_alphanumeric())
        .filter(|w| w.len() > 3 && !stop_words.contains(w))
        .map(|w| w.to_lowercase())
        .collect();

    if target_words.is_empty() {
        println!("No similar papers found (target paper has no extractable content).");
        return Ok(());
    }

    // Score all papers by similarity
    let mut scored: Vec<(&Paper, f64)> = Vec::new();
    for p in &all_papers {
        if p.id == paper.id {
            continue;
        }
        let p_text = format!(
            "{} {}",
            p.title.to_lowercase(),
            p.abstract_text.to_lowercase()
        );
        let p_words: std::collections::HashSet<String> = p_text
            .split(|c: char| !c.is_alphanumeric())
            .filter(|w| w.len() > 3 && !stop_words.contains(w))
            .map(|w| w.to_lowercase())
            .collect();

        if p_words.is_empty() {
            continue;
        }

        // Jaccard similarity
        let intersection: std::collections::HashSet<_> =
            target_words.intersection(&p_words).collect();
        let union: std::collections::HashSet<_> = target_words.union(&p_words).collect();
        let sim = if union.is_empty() {
            0.0
        } else {
            intersection.len() as f64 / union.len() as f64
        };

        if sim > 0.05 {
            scored.push((p, sim));
        }
    }

    // Sort by similarity descending
    scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

    let top: Vec<_> = scored.into_iter().take(limit).collect();
    if top.is_empty() {
        println!("No similar papers found in database.");
    } else {
        println!("Similar papers ({} found):\n", top.len());
        println!("{:<6} {:<8} {}", "SIM", "YEAR", "TITLE");
        println!("{}", "-".repeat(80));
        for (p, sim) in &top {
            let year = p.published.format("%Y");
            let title = if p.title.len() > 60 {
                format!("{}...", &p.title[..60])
            } else {
                p.title.clone()
            };
            println!("{:<6.3} {:<8} {}", sim, year, title);
        }
    }

    Ok(())
}

pub fn handle_compare(db: &Database, papers_arg: &str, aspect: &str) -> Result<()> {
    println!("=== Compare Papers ===\n");

    let paper_ids: Vec<&str> = papers_arg.split(',').map(|s| s.trim()).collect();

    // Fetch all papers
    let mut papers: Vec<Paper> = Vec::new();
    for pid in &paper_ids {
        if let Ok(paper) = db.get_paper(pid) {
            papers.push(paper);
        } else if let Ok(Some(paper)) = db.get_paper_by_arxiv(pid) {
            papers.push(paper);
        } else {
            eprintln!("Warning: Paper '{}' not found, skipping", pid);
        }
    }

    if papers.is_empty() {
        println!("No valid papers found. Add papers first with `rairos-cli add <arxiv_id>`");
        return Ok(());
    }

    println!("Comparing {} papers:\n", papers.len());
    for (i, p) in papers.iter().enumerate() {
        println!(
            "  {}. {} ({})",
            i + 1,
            p.title,
            p.published.format("%Y-%m-%d")
        );
    }
    println!();

    match aspect {
        "overview" => {
            // Summary table: title, authors, year, categories, citations, references
            println!(
                "{:<6} {:<50} {:<8} {:>6} {:>10} {:>10}",
                "#", "Title", "Year", "Authors", "Cited_by", "Refs"
            );
            println!("{}", "-".repeat(96));
            for (i, p) in papers.iter().enumerate() {
                let title = if p.title.len() > 48 {
                    format!("{}...", &p.title[..48])
                } else {
                    p.title.clone()
                };
                let year = p.published.format("%Y").to_string();
                let author_count = p.authors.len();
                let cited = p.metadata.cited_by;
                let refs = p.metadata.references;
                println!(
                    "{:<6} {:<50} {:<8} {:>6} {:>10} {:>10}",
                    i + 1,
                    title,
                    year,
                    author_count,
                    cited,
                    refs
                );
            }
        }
        "citations" => {
            // Compare citation counts
            println!("Citation Comparison:\n");
            println!("{:<50} {:>12} {:>12}", "Paper", "Cited By", "References");
            println!("{}", "-".repeat(76));
            let mut by_cited: Vec<_> = papers.iter().enumerate().collect();
            by_cited.sort_by(|a, b| b.1.metadata.cited_by.cmp(&a.1.metadata.cited_by));
            for (_rank, (_, p)) in by_cited.iter().enumerate() {
                println!(
                    "{:<50} {:>12} {:>12}",
                    if p.title.len() > 48 {
                        format!("{}...", &p.title[..48])
                    } else {
                        p.title.clone()
                    },
                    p.metadata.cited_by,
                    p.metadata.references
                );
            }
            if papers.len() > 1 {
                let max_cited = papers.iter().map(|p| p.metadata.cited_by).max().unwrap();
                let min_cited = papers.iter().map(|p| p.metadata.cited_by).min().unwrap();
                if max_cited > 0 {
                    println!(
                        "\nCitation spread: {} (max) / {} (min) = {:.1}x",
                        max_cited,
                        min_cited,
                        max_cited as f64 / min_cited as f64
                    );
                }
            }
        }
        "authors" => {
            // Compare author overlap
            println!("Author Comparison:\n");
            for (i, p) in papers.iter().enumerate() {
                println!(
                    "Paper {}: {} author(s) - {}",
                    i + 1,
                    p.authors.len(),
                    p.authors.join(", ")
                );
            }
            if papers.len() > 1 {
                // Find author overlap between all pairs
                println!("\nAuthor Overlap:");
                for i in 0..papers.len() {
                    for j in (i + 1)..papers.len() {
                        let set_i: HashSet<_> =
                            papers[i].authors.iter().map(|a| a.to_lowercase()).collect();
                        let set_j: HashSet<_> =
                            papers[j].authors.iter().map(|a| a.to_lowercase()).collect();
                        let intersection: HashSet<_> = set_i.intersection(&set_j).collect();
                        let union: HashSet<_> = set_i.union(&set_j).collect();
                        let jaccard = if union.is_empty() {
                            0.0
                        } else {
                            intersection.len() as f64 / union.len() as f64
                        };
                        println!(
                            "  Paper {} vs Paper {}: {} shared author(s) (Jaccard: {:.2})",
                            i + 1,
                            j + 1,
                            intersection.len(),
                            jaccard
                        );
                    }
                }
            }
        }
        "topics" | "categories" => {
            // Compare categories
            println!("Category Comparison:\n");
            for (i, p) in papers.iter().enumerate() {
                println!(
                    "Paper {}: {} categories - {}",
                    i + 1,
                    p.categories.len(),
                    p.categories.join(", ")
                );
            }
            if papers.len() > 1 {
                println!("\nCategory Overlap:");
                for i in 0..papers.len() {
                    for j in (i + 1)..papers.len() {
                        let set_i: HashSet<_> = papers[i].categories.iter().collect();
                        let set_j: HashSet<_> = papers[j].categories.iter().collect();
                        let intersection: HashSet<_> = set_i.intersection(&set_j).collect();
                        let union: HashSet<_> = set_i.union(&set_j).collect();
                        let jaccard = if union.is_empty() {
                            0.0
                        } else {
                            intersection.len() as f64 / union.len() as f64
                        };
                        println!(
                            "  Paper {} vs Paper {}: {} shared category/ies (Jaccard: {:.2})",
                            i + 1,
                            j + 1,
                            intersection.len(),
                            jaccard
                        );
                    }
                }
            }
        }
        "timeline" => {
            // Compare publication dates
            println!("Timeline Comparison (newest first):\n");
            let mut sorted: Vec<_> = papers.iter().enumerate().collect();
            sorted.sort_by(|a, b| b.1.published.cmp(&a.1.published));
            println!("{:<50} {:>12} {:>12}", "Paper", "Published", "Age (days)");
            println!("{}", "-".repeat(76));
            let now = Utc::now();
            for (_, p) in sorted.iter() {
                let age = (now - p.published).num_days();
                println!(
                    "{:<50} {:>12} {:>12}",
                    if p.title.len() > 48 {
                        format!("{}...", &p.title[..48])
                    } else {
                        p.title.clone()
                    },
                    p.published.format("%Y-%m-%d"),
                    age
                );
            }
        }
        "abstract" => {
            println!("Abstract Comparison (keyword overlap):\n");
            for (i, p) in papers.iter().enumerate() {
                let words: HashSet<String> = p
                    .abstract_text
                    .to_lowercase()
                    .split(|c: char| !c.is_alphanumeric())
                    .filter(|w| w.len() > 4)
                    .map(|s| s.to_string())
                    .collect();
                println!("Paper {}: {} unique words in abstract", i + 1, words.len());
            }
            if papers.len() > 1 {
                println!("\nAbstract Keyword Overlap:");
                for i in 0..papers.len() {
                    for j in (i + 1)..papers.len() {
                        let words_i: HashSet<String> = papers[i]
                            .abstract_text
                            .to_lowercase()
                            .split(|c: char| !c.is_alphanumeric())
                            .filter(|w| w.len() > 4)
                            .map(|s| s.to_string())
                            .collect();
                        let words_j: HashSet<String> = papers[j]
                            .abstract_text
                            .to_lowercase()
                            .split(|c: char| !c.is_alphanumeric())
                            .filter(|w| w.len() > 4)
                            .map(|s| s.to_string())
                            .collect();
                        let intersection: HashSet<_> = words_i.intersection(&words_j).collect();
                        let union: HashSet<_> = words_i.union(&words_j).collect();
                        let jaccard = if union.is_empty() {
                            0.0
                        } else {
                            intersection.len() as f64 / union.len() as f64
                        };
                        println!(
                            "  Paper {} vs Paper {}: {} shared words (Jaccard: {:.3})",
                            i + 1,
                            j + 1,
                            intersection.len(),
                            jaccard
                        );
                    }
                }
            }
        }
        _ => {
            println!("Unknown aspect: '{}'. Available aspects:", aspect);
            println!("  overview     - Summary table with all metadata");
            println!("  citations    - Citation count comparison");
            println!("  authors      - Author count and overlap");
            println!("  topics       - Category comparison");
            println!("  timeline     - Publication date comparison");
            println!("  abstract     - Keyword overlap in abstracts");
        }
    }

    Ok(())
}

pub fn handle_ingest(paper_id: Option<&str>, json: bool, no_pdf: bool, source: &str) -> Result<()> {
    let Some(pid) = paper_id else {
        eprintln!("Usage: ingest <paper_id>");
        return Ok(());
    };

    println!("📥 Ingesting: {} (source: {}, no_pdf: {})", pid, source, no_pdf);

    let rt = tokio::runtime::Runtime::new()?;
    let result = rt.block_on(async {
        rairos_parser::fetch_paper(pid).await
    });

    match result {
        Ok(paper) => {
            if json {
                println!("{}", serde_json::to_string_pretty(&paper)?);
            } else {
                println!("Title: {}", paper.title);
                println!("ID: {}", paper.id);
                println!("Authors: {}", paper.authors.len());
                println!("Published: {}", paper.published);
                println!("Categories: {:?}", paper.categories);
                println!("Abstract: {}...", &paper.abstract_text[..200.min(paper.abstract_text.len())]);
            }
        }
        Err(e) => eprintln!("Failed to fetch {}: {}", pid, e),
    }

    Ok(())
}

