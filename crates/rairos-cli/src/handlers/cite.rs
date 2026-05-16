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

use anyhow::Result;
use chrono::Datelike;
use rairos_core::Database;



// ====================================================================
// Handler implementations
// ====================================================================

// ============================================================================
// Command Handlers
// ============================================================================

pub fn handle_citations(
    db: &Database,
    from: Option<&str>,
    to: Option<&str>,
    format: &str,
) -> Result<()> {
    if from.is_none() && to.is_none() {
        eprintln!("Error: must specify --from or --to");
        std::process::exit(1);
    }

    match (from, to) {
        // Bridge mode: --from A --to B
        (Some(f), Some(t)) => {
            let from_title = db.get_paper(f)?.title;
            let to_title = db.get_paper(t)?.title;

            let citations_from = db.get_citations(f)?;
            let citations_to = db.get_citations(t)?;

            let direct = citations_from.references.contains(&t.to_string());
            let citing_to_sources: std::collections::HashSet<String> =
                citations_to.citing.into_iter().collect();
            let via_papers: Vec<&String> = citations_from
                .references
                .iter()
                .filter(|id| citing_to_sources.contains(*id))
                .collect();

            if format == "json" {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&serde_json::json!({
                        "from": f,
                        "from_title": from_title,
                        "to": t,
                        "to_title": to_title,
                        "direct": direct,
                        "via_papers": via_papers,
                    }))?
                );
                return Ok(());
            }

            println!("Citation Bridge — {} ↔ {}", f, t);
            println!("  From: {}", from_title.chars().take(60).collect::<String>());
            println!("  To:   {}", to_title.chars().take(60).collect::<String>());
            if direct {
                println!("  ✅ DIRECT: {} cites {}", f, t);
            }
            if !via_papers.is_empty() {
                println!("  ⚡ INDIRECT ({} connections):", via_papers.len());
                for v in &via_papers {
                    println!("    {} → {} → {}", f, v, t);
                }
            }
            if !direct && via_papers.is_empty() {
                println!("  No citation path found between these papers");
            }
        }
        // Single direction mode
        (Some(pid), None) | (None, Some(pid)) => {
            let direction = if from.is_some() { "from" } else { "to" };
            let paper_result = db.get_paper(pid);
            let title = paper_result
                .as_ref()
                .map(|p| p.title.clone())
                .unwrap_or_else(|_| "?".to_string());

            let citations = db.get_citations(pid)?;

            let ids: Vec<String> = if direction == "from" {
                citations.references
            } else {
                citations.citing
            };

            if format == "json" {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&serde_json::json!({
                        "paper_id": pid,
                        "direction": direction,
                        "count": ids.len(),
                        "citations": ids,
                    }))?
                );
                return Ok(());
            }

            let label = if direction == "from" {
                "References"
            } else {
                "Cited by"
            };

            println!("{} — {}", label, pid);
            println!("  {}", title.chars().take(60).collect::<String>());
            if ids.is_empty() {
                println!("  No citations found");
            } else {
                println!("  Found {} citation(s):", ids.len());
                for cid in &ids {
                    println!("    {}", cid);
                }
            }
        }
        (None, None) => unreachable!(), // already checked above
    }
    Ok(())
}

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

pub fn severity_icon(severity: &str) -> &'static str {
    match severity.to_lowercase().as_str() {
        "high" => "🔴",
        "medium" => "🟡",
        "low" => "🟢",
        _ => "⚪",
    }
}

pub fn handle_influence(
    db: &Database,
    top: usize,
    paper: Option<&str>,
    min_cites: usize,
    format: &str,
) -> Result<()> {
    if let Some(paper_id) = paper {
        let pap = db.get_paper(paper_id)?;
        let citations = db.get_citations(paper_id)?;
        let forward = citations.citing.len() as f64;
        let backward = citations.references.len() as f64;

        let year = pap.published.year();
        let age = if year > 2000 && year <= 2026 {
            (2026 - year + 1) as f64
        } else {
            0.0
        };
        let velocity = if age > 0.0 { forward / age } else { 0.0 };

        let impact = if velocity >= 10.0 {
            "\u{1f525} Extremely high velocity (\u{2265}10/y) — field-defining"
        } else if velocity >= 5.0 {
            "\u{1f4c8} High velocity (5-10/y) — very active research"
        } else if velocity >= 1.0 {
            "\u{1f4ca} Moderate velocity (1-5/y) — steady influence"
        } else {
            "\u{1f4c9} Low velocity — emerging or niche"
        };

        println!("=== Paper Influence Profile ===");
        println!("  Paper ID  : {}", paper_id);
        println!("  Title     : {}", pap.title);
        println!("  Published : {}", year);
        if age > 0.0 {
            println!("  Age       : {:.0} years (as of 2026)", age);
        }
        println!();
        println!("  Citations");
        if age > 0.0 {
            println!(
                "    Cited by (forward) : {:.0}  → velocity = {:.0}/{:.0} = {:.2}/y",
                forward, forward, age, velocity
            );
        } else {
            println!("    Cited by (forward) : {:.0}", forward);
        }
        println!("    References (backward): {:.0}", backward);
        println!();
        if age > 0.0 {
            println!("  Impact Assessment");
            println!("    {}", impact);
        }
        return Ok(());
    }

    let all_papers = db.list_papers(None, 100000, 0)?;
    let mut results: Vec<(String, String, i32, f64, f64)> = Vec::new();

    for p in &all_papers {
        if p.metadata.cited_by == 0 && min_cites > 0 {
            continue;
        }
        let forward = p.metadata.cited_by as f64;
        if forward < min_cites as f64 {
            continue;
        }
        let year = p.published.year();
        if year < 2000 || year > 2026 {
            continue;
        }
        let age = (2026 - year + 1) as f64;
        let velocity = forward / age;
        results.push((p.id.clone(), p.title.clone(), year, forward, velocity));
    }

    results.sort_by(|a, b| b.4.partial_cmp(&a.4).unwrap_or(std::cmp::Ordering::Equal));

    if results.is_empty() {
        println!("No papers with sufficient citation data found.");
        return Ok(());
    }

    let top_n: Vec<_> = results.iter().take(top).collect();

    match format {
        "json" => {
            let data: Vec<serde_json::Value> = top_n
                .iter()
                .enumerate()
                .map(|(i, (id, title, year, forward, vel))| {
                    serde_json::json!({
                        "rank": i + 1,
                        "paper_id": id,
                        "title": title,
                        "year": year,
                        "forward_cites": forward,
                        "age_years": (2026 - year + 1) as f64,
                        "velocity": (vel * 100.0).round() / 100.0,
                    })
                })
                .collect();
            println!("{}", serde_json::to_string_pretty(&data)?);
        }
        "csv" => {
            println!("rank,paper_id,title,year,forward_cites,age_years,velocity");
            for (i, (id, title, year, forward, vel)) in top_n.iter().enumerate() {
                let title_esc = title.replace('"', "\"\"");
                println!(
                    "{},\"{}\",\"{}\",{},{},{:.1},{:.2}",
                    i + 1,
                    id,
                    title_esc,
                    year,
                    forward,
                    2026 - year + 1,
                    vel
                );
            }
        }
        _ => {
            println!(
                "{:>4}  {:>8}  {:>5}  {:>3}y  Year  Paper",
                "Rank", "Velocity", "Cites", "Age"
            );
            println!("{}", "-".repeat(50));
            for (i, (_id, title, year, forward, vel)) in top_n.iter().enumerate() {
                let title_short = if title.len() > 50 {
                    format!("{}…", &title[..50])
                } else {
                    title.clone()
                };
                println!(
                    "{:>4}  {:>7.1}/y  {:>5.0}  {:>3.0}   {}  {}",
                    i + 1,
                    vel,
                    forward,
                    2026 - year + 1,
                    year,
                    title_short
                );
            }
            println!();
            println!(
                "Showing {} of {} papers with >= {} citation(s)",
                top_n.len(),
                results.len(),
                min_cites
            );
            println!("Formula: velocity = forward_citations / age_years  (age = 2026 - published + 1)");
        }
    }
    Ok(())
}

pub fn handle_merge(
    db: &Database,
    keep: &str,
    dry_run: bool,
    auto: bool,
    target_id: Option<&str>,
    duplicate_id: Option<&str>,
) -> Result<()> {
    if auto {
        let papers = db.list_papers(None, 100000, 0)?;
        let mut seen: std::collections::HashSet<(String, String)> = std::collections::HashSet::new();
        let mut merged_count = 0u32;
        let mut skipped_count = 0u32;

        for paper in &papers {
            if seen.iter().any(|(a, b)| a == &paper.id || b == &paper.id) {
                continue;
            }
            if paper.title.is_empty() {
                continue;
            }

            let sims = match db.find_similar(&paper.id, 10, 0.95) {
                Ok(s) => s,
                Err(_) => continue,
            };

            for (sim_id, score) in &sims {
                let pair_key = if paper.id < *sim_id {
                    (paper.id.clone(), sim_id.clone())
                } else {
                    (sim_id.clone(), paper.id.clone())
                };
                if seen.contains(&pair_key) {
                    continue;
                }
                seen.insert(pair_key.clone());

                if *score < 0.95 {
                    continue;
                }

                let sim_paper = match db.get_paper(sim_id) {
                    Ok(p) => p,
                    Err(_) => continue,
                };

                let (keep_id, drop_id) = {
                    let (keep_label, _) = pick_keep(
                        &paper.title,
                        &sim_paper.title,
                        &paper.parse_status.to_string(),
                        &sim_paper.parse_status.to_string(),
                        keep,
                    );
                    if keep_label == "A" {
                        (&paper.id, sim_id)
                    } else {
                        (sim_id, &paper.id)
                    }
                };

                if dry_run {
                    println!("Would merge {} into {}", drop_id, keep_id);
                    println!("  keeping : [{}] {}", keep_id, &paper.title[..paper.title.len().min(70)]);
                    println!("  deleting: [{}] {}", drop_id, &sim_paper.title[..sim_paper.title.len().min(70)]);
                    println!("  semantic similarity: {:.3}", score);
                    println!();
                    skipped_count += 1;
                } else {
                    let merged = db.merge_papers(keep_id, &[drop_id])?;
                    if merged {
                        db.log_dedup(keep_id, drop_id, "semantic-auto")?;
                        println!("Merged {} into {} (similarity={:.3})", drop_id, keep_id, score);
                        merged_count += 1;
                    } else {
                        println!("Merge failed for {} -> {}", drop_id, keep_id);
                    }
                }
            }
        }

        if dry_run {
            println!("({} pair(s) would be merged, dry-run)", skipped_count);
        } else {
            println!("Auto-merge complete: {} pair(s) merged", merged_count);
        }
        return Ok(());
    }

    let target_id = match target_id {
        Some(id) => id,
        None => {
            eprintln!("merge requires TARGET_ID and DUPLICATE_ID (or use --auto)");
            std::process::exit(1);
        }
    };
    let duplicate_id = match duplicate_id {
        Some(id) => id,
        None => {
            eprintln!("merge requires TARGET_ID and DUPLICATE_ID (or use --auto)");
            std::process::exit(1);
        }
    };

    let target = match db.get_paper(target_id) {
        Ok(p) => p,
        Err(_) => {
            eprintln!("Target paper {} not found", target_id);
            std::process::exit(1);
        }
    };
    let duplicate = match db.get_paper(duplicate_id) {
        Ok(p) => p,
        Err(_) => {
            eprintln!("Duplicate paper {} not found", duplicate_id);
            std::process::exit(1);
        }
    };

    let sim = db.get_similarity(target_id, duplicate_id).ok().flatten();

    let (keep_id, drop_id, drop_title) = if keep == "semantic" {
        if let Some(s) = sim {
            if s >= 0.8 {
                let (keep_label, _) = pick_keep(
                    &target.title,
                    &duplicate.title,
                    &target.parse_status.to_string(),
                    &duplicate.parse_status.to_string(),
                    keep,
                );
                if keep_label == "A" {
                    (target.id.clone(), duplicate.id.clone(), duplicate.title.clone())
                } else {
                    (duplicate.id.clone(), target.id.clone(), target.title.clone())
                }
            } else {
                eprintln!("Note: low similarity, falling back to 'parsed' (similarity: {:.3})", s);
                let (keep_label, _) = pick_keep(
                    &target.title,
                    &duplicate.title,
                    &target.parse_status.to_string(),
                    &duplicate.parse_status.to_string(),
                    "parsed",
                );
                if keep_label == "A" {
                    (target.id.clone(), duplicate.id.clone(), duplicate.title.clone())
                } else {
                    (duplicate.id.clone(), target.id.clone(), target.title.clone())
                }
            }
        } else {
            eprintln!("Note: no embeddings available, falling back to 'parsed'");
            let (keep_label, _) = pick_keep(
                &target.title,
                &duplicate.title,
                &target.parse_status.to_string(),
                &duplicate.parse_status.to_string(),
                "parsed",
            );
            if keep_label == "A" {
                (target.id.clone(), duplicate.id.clone(), duplicate.title.clone())
            } else {
                (duplicate.id.clone(), target.id.clone(), target.title.clone())
            }
        }
    } else {
        let (keep_label, _) = pick_keep(
            &target.title,
            &duplicate.title,
            &target.parse_status.to_string(),
            &duplicate.parse_status.to_string(),
            keep,
        );
        if keep_label == "A" {
            (target.id.clone(), duplicate.id.clone(), duplicate.title.clone())
        } else {
            (duplicate.id.clone(), target.id.clone(), target.title.clone())
        }
    };

    if dry_run {
        println!("Would merge {} into {} (--keep={})", drop_id, keep_id, keep);
        println!("  keeping : [{}] {}", keep_id, &target.title[..target.title.len().min(70)]);
        println!("  deleting: [{}] {}", drop_id, &drop_title[..drop_title.len().min(70)]);
        if let Some(s) = sim {
            println!("  semantic similarity: {:.3}", s);
        } else {
            println!("  semantic similarity: no embeddings available");
        }
        return Ok(());
    }

    println!("Merging {} into {}", drop_id, keep_id);
    println!("  Keeping: [{}] {}", keep_id, &target.title[..target.title.len().min(70)]);
    println!("  Deleting: [{}] {}", drop_id, &drop_title[..drop_title.len().min(70)]);
    if let Some(s) = sim {
        println!("  Similarity: {:.3}", s);
    } else {
        println!("  semantic similarity: no embeddings available");
    }

    let ok = db.merge_papers(&keep_id, &[&drop_id])?;
    if ok {
        db.log_dedup(&keep_id, &drop_id, keep)?;
        println!("Merged {} into {}", drop_id, keep_id);
    } else {
        eprintln!("Merge failed for {} -> {}", drop_id, keep_id);
        std::process::exit(1);
    }

    Ok(())
}

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

        // Verify the paper exists
        if !db.paper_exists(paper_id) {
            eprintln!("Error: paper '{}' not found in DB", paper_id);
            std::process::exit(1);
        }

        // Get plain_text from the DB
        let text = match db.get_paper_plain_text(paper_id)? {
            Some(t) if !t.is_empty() => t,
            _ => {
                eprintln!("Error: paper '{}' has no plain_text to extract from", paper_id);
                std::process::exit(1);
            }
        };

        // Extract references using regex
        let arxiv_re = regex::Regex::new(r"(?i)\barXiv:\s*(\d+\.\d+\b)").unwrap();
        let doi_re = regex::Regex::new(r"(?i)\b10\.\d{4,}/[^\s]+").unwrap();
        let pmid_re = regex::Regex::new(r"(?i)\bPMID:\s*(\d{6,})\b").unwrap();
        let isbn_re = regex::Regex::new(r"(?i)\bISBN(?:-13)?:?\s*([0-9-X]{10,})\b").unwrap();

        // Find references section
        let refs_section_re = regex::Regex::new(r"(?i)(?:\n|^)[ ]*(?:\d+\.?\s*)?(?:References|Bibliography|Citations)").unwrap();
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

        // Print extracted references
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

        // Look up arXiv IDs in DB and import citations
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
                // insert_citation uses INSERT OR IGNORE — always succeeds
                db.insert_citation(paper_id, tgt)?;
                new_count += 1;
            }
            println!("\nImported {} citation edge(s)", new_count);
        }

        return Ok(());
    }

    // ── JSON input mode ──
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

    // Normalise to array
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

        // Check source exists
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

pub fn handle_citation_chain(
    db: &Database,
    paper_id: Option<&str>,
    depth: i32,
    graphviz: bool,
    mermaid: bool,
    influencers: bool,
    impact: bool,
    path: Option<&str>,
) -> Result<()> {
    let mut builder = rairos_citation_chain::CitationChainBuilder::new();

    if influencers || impact {
        let Some(pid) = paper_id else {
            eprintln!("Usage: citation-chain <paper_id> --influencers|--impact");
            return Ok(());
        };

        if influencers {
            println!("Finding influences for: {}", pid);
            if let Ok(papers) = db.search_papers(pid, 1) {
                if let Some(p) = papers.first() {
                builder.add_paper(pid.to_string(), p.title.clone(), p.published.year(), Vec::new(), Vec::new(), String::new(), 0);
            }
        }
        println!("Influencers: (requires citations data in DB)");
    }

    if impact {
        println!("Finding impact for: {}", pid);
        println!("Impact: (requires citations data in DB)");
        }

        return Ok(());
    }

    let Some(pid) = paper_id else {
        eprintln!("Usage: citation-chain <paper_id> [options]");
        return Ok(());
    };

    if let Ok(papers) = db.search_papers(pid, 5) {
        for p in &papers {
            builder.add_paper(p.id.clone(), p.title.clone(), p.published.year(), Vec::new(), Vec::new(), String::new(), 0);
        }
    }

    let chain = builder.build_from_db(pid, depth);

    if graphviz {
        println!("{}", builder.render_graphviz(&chain));
    } else if mermaid {
        println!("{}", builder.render_mermaid(&chain));
    } else {
        println!("{}", builder.render_text(&chain, 20));
    }

    if let Some(_target) = path {
        println!("Path finding requires citation graph data in DB.");
    }

    Ok(())
}

pub fn handle_cite_graph(db: &Database, paper: Option<&str>, depth: i32, max_nodes: usize, format: &str) -> Result<()> {
    let Some(pid) = paper else {
        eprintln!("Usage: cite-graph --paper <paper_id>");
        return Ok(());
    };

    let papers = db.search_papers(pid, 1)?;
    let root_title = papers.first().map(|p| p.title.as_str()).unwrap_or(pid);

    println!("Citation graph for {} (depth={}):", root_title, depth);

    let mut builder = rairos_citation_chain::CitationChainBuilder::new();
    for p in db.search_papers(pid, 5)? {
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

pub fn pick_keep<'a>(
    _title_a: &'a str,
    _title_b: &'a str,
    status_a: &'a str,
    status_b: &'a str,
    strategy: &'a str,
) -> (&'static str, &'static str) {
    match strategy {
        "newer" => ("A", "B"),
        "older" => ("B", "A"),
        "parsed" | "semantic" => {
            fn rank(s: &str) -> u8 {
                match s {
                    "done" => 4,
                    "parsing" => 3,
                    "pending" => 2,
                    "failed" => 1,
                    _ => 0,
                }
            }
            if rank(status_a) >= rank(status_b) {
                ("A", "B")
            } else {
                ("B", "A")
            }
        }
        _ => ("A", "B"),
    }
}
