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

use std::sync::LazyLock;
use anyhow::Result;
use chrono::Datelike;
use regex::Regex;
use rairos_core::Database;

static ARXIV_REF_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)\barXiv:\s*(\d+\.\d+\b)").expect("valid regex")
});

static DOI_REF_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)\b10\.\d{4,}/[^\s]+").expect("valid regex")
});

static PMID_REF_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)\bPMID:\s*(\d{6,})\b").expect("valid regex")
});

static ISBN_REF_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)\bISBN(?:-13)?:?\s*([0-9-X]{10,})\b").expect("valid regex")
});

static REFS_SECTION_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)(?:\n|^)[ ]*(?:\d+\.?\s*)?(?:References|Bibliography|Citations)").expect("valid regex")
});

pub fn handle_cite_import(
    db: &Database,
    json_input: Option<&str>,
    dry_run: bool,
    skip_missing: bool,
    extract: bool,
    paper: Option<&str>,
    _dedup: bool,
) -> Result<()> {
    if extract {
        let paper_id = match paper {
            Some(id) => id,
            None => {
                eprintln!("Error: --paper PAPER_ID required with --extract");
                std::process::exit(1);
            }
        };

        if !db.paper_exists(paper_id) {
            eprintln!("Error: paper '{}' not found in DB", paper_id);
            std::process::exit(1);
        }

        let text = match db.get_paper_plain_text(paper_id)? {
            Some(t) if !t.is_empty() => t,
            _ => {
                eprintln!("Error: paper '{}' has no plain_text to extract from", paper_id);
                std::process::exit(1);
            }
        };

        let arxiv_re = &*ARXIV_REF_REGEX;
        let doi_re = &*DOI_REF_REGEX;
        let pmid_re = &*PMID_REF_REGEX;
        let isbn_re = &*ISBN_REF_REGEX;

        let refs_section_re = &*REFS_SECTION_REGEX;
        let refs_text = if let Some(m) = refs_section_re.find(&text) {
            &text[m.start()..]
        } else {
            &text[..]
        };

        let arxiv_ids: Vec<String> = arxiv_re
            .captures_iter(refs_text)
            .map(|c| c[1].to_string())
            .collect();

        let dois: Vec<String> = doi_re
            .find_iter(refs_text)
            .map(|m| m.as_str().to_string())
            .collect();

        let pmids: Vec<String> = pmid_re
            .captures_iter(refs_text)
            .map(|c| c[1].to_string())
            .collect();

        let isbns: Vec<String> = isbn_re
            .captures_iter(refs_text)
            .map(|c| c[1].to_string())
            .collect();

        if !arxiv_ids.is_empty() {
            println!("  arXiv IDs ({}): {}", arxiv_ids.len(), arxiv_ids.join(", "));
        }
        if !dois.is_empty() {
            println!("  DOIs ({}): {}", dois.len(), dois.join(", "));
        }
        if !pmids.is_empty() {
            println!("  PMIDs ({}): {}", pmids.len(), pmids.join(", "));
        }
        if !isbns.is_empty() {
            println!("  ISBNs ({}): {}", isbns.len(), isbns.join(", "));
        }

        if arxiv_ids.is_empty() && dois.is_empty() && pmids.is_empty() && isbns.is_empty() {
            println!("No references found in '{}'", paper_id);
            return Ok(());
        }

        let mut db_ids: Vec<String> = Vec::new();
        for aid in &arxiv_ids {
            let full = format!("arxiv:{}", aid.to_lowercase());
            if db.paper_exists(&full) {
                db_ids.push(full);
            }
        }

        if db_ids.is_empty() {
            println!("No matching papers found in DB for extracted references");
            return Ok(());
        }

        if dry_run {
            println!("\n[dry-run] Would import {} citation edge(s):", db_ids.len());
            for tgt in &db_ids {
                println!("  {} -> {}", paper_id, tgt);
            }
        } else {
            let mut new_count = 0u32;
            for tgt in &db_ids {
                db.insert_citation(paper_id, tgt)?;
                new_count += 1;
            }
            println!("\nImported {} citation edge(s)", new_count);
        }

        return Ok(());
    }

    let raw = match json_input {
        Some(s) => s,
        None => {
            eprintln!("Error: json_input required (JSON string or @filepath)");
            std::process::exit(1);
        }
    };

    let data: serde_json::Value = if let Some(path) = raw.strip_prefix('@') {
        let content = std::fs::read_to_string(path)
            .map_err(|e| anyhow::anyhow!("Error reading {}: {}", path, e))?;
        serde_json::from_str(&content)
            .map_err(|e| anyhow::anyhow!("Error parsing JSON from {}: {}", path, e))?
    } else {
        serde_json::from_str(raw)
            .map_err(|e| anyhow::anyhow!("Error: invalid JSON: {}", e))?
    };

    let items: Vec<&serde_json::Value> = match &data {
        serde_json::Value::Array(arr) => arr.iter().collect(),
        serde_json::Value::Object(_) => vec![&data],
        _ => {
            eprintln!("Error: JSON must be a list of objects or a single object");
            std::process::exit(1);
        }
    };

    let mut total_new = 0u32;
    let mut total_skip_missing = 0u32;
    let mut errors: Vec<String> = Vec::new();

    for (i, item) in items.iter().enumerate() {
        let obj = match item.as_object() {
            Some(o) => o,
            None => {
                errors.push(format!("[{}] item is not an object, skipping", i));
                continue;
            }
        };

        let source = obj
            .get("source")
            .or_else(|| obj.get("source_id"))
            .and_then(|v| v.as_str())
            .unwrap_or("");

        let targets = obj
            .get("targets")
            .or_else(|| obj.get("target_ids"))
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                    .collect::<Vec<String>>()
            })
            .unwrap_or_default();

        if source.is_empty() {
            errors.push(format!("[{}] missing 'source' field, skipping", i));
            continue;
        }

        if targets.is_empty() {
            errors.push(format!("[{}] empty 'targets' for source={}, skipping", i, source));
            continue;
        }

        if !db.paper_exists(source) {
            if skip_missing {
                total_skip_missing += 1;
                if dry_run {
                    println!("  [dry-run] skip (missing): {}", source);
                }
                continue;
            } else {
                errors.push(format!("[{}] source paper '{}' not in DB", i, source));
                continue;
            }
        }

        let mut valid_targets: Vec<String> = Vec::new();
        for tgt in &targets {
            if !db.paper_exists(tgt) {
                if skip_missing {
                    total_skip_missing += 1;
                    if dry_run {
                        println!("  [dry-run] skip (missing): {}", tgt);
                    }
                } else {
                    errors.push(format!("[{}] target paper '{}' not in DB", i, tgt));
                }
                continue;
            }
            valid_targets.push(tgt.clone());
        }

        if valid_targets.is_empty() {
            continue;
        }

        if dry_run {
            for tgt in &valid_targets {
                println!("  [dry-run] add citation: {} -> {}", source, tgt);
            }
            total_new += valid_targets.len() as u32;
        } else {
            for tgt in &valid_targets {
                db.insert_citation(source, tgt)?;
            }
            total_new += valid_targets.len() as u32;
        }
    }

    if !errors.is_empty() {
        println!("  warnings/errors : {}", errors.len());
        for err in errors.iter().take(10) {
            println!("    - {}", err);
        }
        if errors.len() > 10 {
            println!("    ... and {} more", errors.len() - 10);
        }
        std::process::exit(1);
    }

    println!("Import complete.");
    println!("  new citations : {}", total_new);
    if skip_missing {
        println!("  skipped (missing papers): {}", total_skip_missing);
    }

    Ok(())
}

pub fn handle_cite_graph(db: &Database, paper: Option<&str>, depth: i32, max_nodes: usize, format: &str) -> Result<()> {
    let Some(pid) = paper else {
        eprintln!("Usage: cite-graph --paper <paper_id>");
        return Ok(());
    };

    let papers = db.search_papers_smart(pid, 1)?;
    let root_title = papers.first().map(|p| p.title.as_str()).unwrap_or(pid);

    println!("Citation graph for {} (depth={}):", root_title, depth);

    let mut builder = rairos_citation_chain::CitationChainBuilder::new();
    for p in db.search_papers_smart(pid, 5)? {
        builder.add_paper(p.id.clone(), p.title.clone(), p.published.year(), Vec::new(), Vec::new(), String::new(), 0);
    }
    let chain = builder.build_from_db(pid, depth);

    match format {
        "mermaid" => println!("{}", builder.render_mermaid(&chain)),
        "json" => println!("{}", serde_json::to_string_pretty(&serde_json::json!({
            "nodes": chain.nodes.len(),
            "depth": depth,
            "paper_id": pid,
        }))?),
        _ => println!("{}", builder.render_text(&chain, max_nodes)),
    }

    Ok(())
}

pub fn handle_cite_fetch(paper_id: Option<&str>, dry_run: bool) -> Result<()> {
    let Some(pid) = paper_id else {
        eprintln!("Usage: cite-fetch <paper_id>");
        return Ok(());
    };

    println!("🔍 Fetching metadata for: {}", pid);

    let rt = tokio::runtime::Runtime::new()?;
    let result = rt.block_on(async {
        rairos_parser::fetch_paper(pid).await
    });

    match result {
        Ok(paper) => {
            if dry_run {
                println!("[dry-run] Would import: {} (authors: {}, categories: {:?})",
                    paper.title, paper.authors.len(), paper.categories);
            } else {
                println!("Title: {}", paper.title);
                println!("Authors: {}", paper.authors.join(", "));
                println!("Published: {}", paper.published);
                println!("Categories: {:?}", paper.categories);
                println!("Abstract: {}...", &paper.abstract_text[..200.min(paper.abstract_text.len())]);
            }
        }
        Err(e) => eprintln!("Failed to fetch {}: {}", pid, e),
    }

    Ok(())
}
