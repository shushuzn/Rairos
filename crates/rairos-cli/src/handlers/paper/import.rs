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

use super::crud::add_paper_from_arxiv;

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
        let content = std::fs::read_to_string(p)?;
        let papers: Vec<Paper> = serde_json::from_str(&content)
            .context("Failed to parse JSON — expected array of Paper objects")?;

        let count = papers.len();
        for paper in &papers {
            db.insert_paper(paper)?;
        }
        println!("Imported {} papers from {}", count, p.display());
    } else if !ids.is_empty() {
        let mut added = 0;
        let mut skipped = 0;
        let mut failed = 0;

        for arxiv_id in ids {
            let arxiv_id = arxiv_id.trim();
            if arxiv_id.is_empty() {
                continue;
            }

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
