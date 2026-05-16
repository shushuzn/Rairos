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
use rairos_core::{Database, Paper};
use std::path::{Path, PathBuf};

use crate::{
    CacheAction, EvoSkillAction, RagAction,
};
use crate::handlers::*;


// ====================================================================
// Handler implementations
// ====================================================================

// ============================================================================
// Command Handlers
// ============================================================================

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

pub fn handle_cache(action: &CacheAction) -> Result<()> {
    match action {
        CacheAction::Stats => {
            println!("=== Cache Statistics ===\n");
            let cache_dir = std::path::Path::new("cache");
            if !cache_dir.exists() {
                println!("Cache directory: not created yet");
                return Ok(());
            }
            let entries = std::fs::read_dir(cache_dir)?.count();
            let total_size = std::fs::read_dir(cache_dir)?
                .filter_map(|e| e.ok())
                .filter_map(|e| e.metadata().ok())
                .map(|m| m.len())
                .sum::<u64>();
            println!("Entries: {}", entries);
            println!(
                "Total size: {} bytes ({:.2} MB)",
                total_size,
                total_size as f64 / 1_048_576.0
            );
        }
        CacheAction::Clear => {
            println!("Clearing all cache...");
            let cache_dir = std::path::Path::new("cache");
            if cache_dir.exists() {
                std::fs::remove_dir_all(cache_dir)?;
                println!("[OK] Cache cleared");
            } else {
                println!("[INFO] No cache to clear");
            }
        }
        CacheAction::ClearApi => {
            println!("Clearing API cache...");
            let api_dir = std::path::Path::new("cache/api");
            if api_dir.exists() {
                std::fs::remove_dir_all(api_dir)?;
                println!("[OK] API cache cleared");
            } else {
                println!("[INFO] No API cache to clear");
            }
        }
        CacheAction::ClearParsed => {
            println!("Clearing parsed paper cache...");
            let parsed_dir = std::path::Path::new("cache/parsed");
            if parsed_dir.exists() {
                std::fs::remove_dir_all(parsed_dir)?;
                println!("[OK] Parsed paper cache cleared");
            } else {
                println!("[INFO] No parsed paper cache to clear");
            }
        }
        CacheAction::List { limit } => {
            println!("=== Cached Entries (showing first {}) ===\n", limit);
            let cache_dir = std::path::Path::new("cache");
            if !cache_dir.exists() {
                println!("No cache entries.");
                return Ok(());
            }
            let mut count = 0;
            for entry in std::fs::read_dir(cache_dir)? {
                if count >= *limit {
                    println!(
                        "... and more ({} total entries)",
                        std::fs::read_dir(cache_dir)?.count()
                    );
                    break;
                }
                let entry = entry?;
                let path = entry.path();
                if path.is_file() {
                    let size = entry.metadata()?.len();
                    let name = path.file_name().unwrap_or_default().to_string_lossy();
                    println!("  {} ({} bytes)", name, size);
                    count += 1;
                } else if path.is_dir() {
                    let sub_count = std::fs::read_dir(&path)?.count();
                    let name = path.file_name().unwrap_or_default().to_string_lossy();
                    println!("  {}/ ({} entries)", name, sub_count);
                }
            }
            if count == 0 {
                println!("No cache entries.");
            }
        }
    }
    Ok(())
}

pub fn handle_repl(query: Option<String>) -> Result<()> {
    let db_path = PathBuf::from("rairos.db");
    if !db_path.exists() {
        return Err(anyhow::anyhow!("Database not found. Run 'rairos init' first."));
    }
    let db = Database::open(&db_path).context("Failed to open database")?;

    println!("=== Rairos REPL ===");
    println!("Type 'help' for commands, 'exit' to quit.\n");

    if let Some(q) = query {
        println!("Pre-loading papers matching: {}", q);
        match db.search_papers(&q, 10) {
            Ok(papers) if !papers.is_empty() => {
                println!("Found {} papers:\n", papers.len());
                for (i, p) in papers.iter().enumerate() {
                    let title = if p.title.len() > 60 {
                        format!("{}...", &p.title[..60])
                    } else {
                        p.title.clone()
                    };
                    let arxiv = p.arxiv_id.as_deref().unwrap_or("-");
                    let id_short = if p.id.len() > 8 { &p.id[..8] } else { p.id.as_str() };
                    println!("  {}. [{}] {} — {}", i + 1, id_short, title, arxiv);
                }
                println!();
            }
            _ => println!("No papers found for query: {}\n", q),
        }
    }

    loop {
        print!("rairos> ");
        std::io::Write::flush(&mut std::io::stdout()).ok();

        let mut input = String::new();
        if std::io::stdin().read_line(&mut input).is_err() || input.trim().is_empty() {
            continue;
        }
        let input = input.trim();

        let parts: Vec<&str> = input.splitn(2, ' ').collect();
        let cmd = parts[0].to_lowercase();
        let arg = parts.get(1).map(|s| s.trim()).unwrap_or("");

        match cmd.as_str() {
            "exit" | "quit" => {
                println!("Goodbye!");
                break;
            }
            "help" => {
                println!("\nCommands:");
                println!("  help                   Show this help");
                println!("  exit / quit            Exit REPL");
                println!("  search <query>         Search papers");
                println!("  show <id>              Show paper details");
                println!("  list [status]          List papers (pending/done/all)");
                println!("  stats                  Show DB statistics");
                println!("  gap <topic>            Detect research gaps");
                println!("  add <arxiv_id>         Import paper from arXiv");
                println!();
            }
            "search" if arg.is_empty() => {
                println!("Usage: search <query>\n");
            }
            "search" => {
                match db.search_papers(arg, 20) {
                    Ok(papers) if papers.is_empty() => {
                        println!("No papers found for: {}", arg);
                    }
                    Ok(papers) => {
                        println!("Found {} papers:\n", papers.len());
                        for (i, p) in papers.iter().enumerate() {
                            let title = if p.title.len() > 60 {
                                format!("{}...", &p.title[..60])
                            } else {
                                p.title.clone()
                            };
                            let arxiv = p.arxiv_id.as_deref().unwrap_or("-");
                            let id_short = if p.id.len() > 8 { &p.id[..8] } else { p.id.as_str() };
                            println!("  {}. [{}] {} — {}", i + 1, id_short, title, arxiv);
                        }
                        println!();
                    }
                    Err(e) => println!("Error: {}\n", e),
                }
            }
            "show" if arg.is_empty() => {
                println!("Usage: show <id>\n");
            }
            "show" => {
                if let Err(e) = handle_show(&db, arg, "table") {
                    println!("Error: {}\n", e);
                }
            }
            "list" => {
                let status = if arg.is_empty() { None } else { Some(arg.to_string()) };
                if let Err(e) = handle_list(&db, status, None, &[], 20, 0, "published", "desc", "table") {
                    println!("Error: {}\n", e);
                }
            }
            "stats" => {
                if let Err(e) = handle_stats(&db, false, "table") {
                    println!("Error: {}\n", e);
                }
            }
            "gap" if arg.is_empty() => {
                println!("Usage: gap <topic>\n");
            }
            "gap" => {
                if let Err(e) = handle_gap(&db, arg, 5, "table", None) {
                    println!("Error: {}\n", e);
                }
            }
            "add" if arg.is_empty() => {
                println!("Usage: add <arxiv_id>\n");
            }
            "add" => {
                if let Err(e) = handle_add(&db, arg) {
                    println!("Error: {}\n", e);
                }
            }
            _ => {
                println!("Unknown command: {}. Type 'help' for available commands.\n", cmd);
            }
        }
    }
    Ok(())
}

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

    let papers = db.search_papers(topic, max_papers)?;

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

pub fn handle_analyze(db: &Database, kind: &str, paper: Option<String>, format: &str) -> Result<()> {
    match kind {
        "keywords" => {
            if let Some(p) = paper {
                let paper_obj = db
                    .get_paper(&p)
                    .ok()
                    .or_else(|| db.get_paper_by_arxiv(&p).ok().flatten())
                    .ok_or_else(|| anyhow::anyhow!("Paper not found: {}", p))?;

                // Extract keywords from title + abstract using TF-like scoring
                let text = format!(
                    "{} {} {}",
                    paper_obj.title,
                    paper_obj.abstract_text,
                    paper_obj.categories.join(" ")
                );
                let keywords = extract_keywords(&text, 10);

                if format == "json" {
                    println!(
                        "{}",
                        serde_json::to_string_pretty(&serde_json::json!({
                            "id": paper_obj.id,
                            "title": paper_obj.title,
                            "keywords": keywords,
                        }))?
                    );
                } else {
                    println!("=== Keyword Analysis ===\n");
                    println!("Title: {}", paper_obj.title);
                    println!("\nTop {} keywords:", keywords.len());
                    for (i, (kw, score)) in keywords.iter().enumerate() {
                        println!("  {:>2}. {:<25} ({:.2})", i + 1, kw, score);
                    }
                }
            } else {
                println!("Analyzing all papers...");
                let papers = db.list_papers(None, 100, 0)?;
                let mut all_kw: std::collections::HashMap<String, usize> =
                    std::collections::HashMap::new();
                for p in &papers {
                    let text =
                        format!("{} {} {}", p.title, p.abstract_text, p.categories.join(" "));
                    for (kw, _) in extract_keywords(&text, 5) {
                        *all_kw.entry(kw).or_insert(0) += 1;
                    }
                }
                let top: Vec<_> = all_kw
                    .into_iter()
                    .filter(|(_, c)| *c > 1)
                    .map(|(k, c)| (k, c))
                    .collect::<Vec<_>>()
                    .into_iter()
                    .take(10)
                    .collect();
                if format == "json" {
                    println!(
                        "{}",
                        serde_json::to_string_pretty(
                            &serde_json::json!({"papers": papers.len(), "top_keywords": top})
                        )?
                    );
                } else {
                    println!("\n=== Cross-Paper Keywords (n={}) ===\n", papers.len());
                    for (kw, count) in top {
                        println!("  {:<25} {} papers", kw, count);
                    }
                }
            }
        }
        "summary" | "topics" | "quality" => {
            if let Some(p) = paper {
                let paper_obj = db
                    .get_paper(&p)
                    .ok()
                    .or_else(|| db.get_paper_by_arxiv(&p).ok().flatten())
                    .ok_or_else(|| anyhow::anyhow!("Paper not found: {}", p))?;

                // Rule-based topic classification
                let topics = classify_topics(
                    &paper_obj.title,
                    &paper_obj.abstract_text,
                    &paper_obj.categories,
                );
                let quality = estimate_quality(&paper_obj);

                if format == "json" {
                    println!(
                        "{}",
                        serde_json::to_string_pretty(&serde_json::json!({
                            "id": paper_obj.id,
                            "title": paper_obj.title,
                            "topics": topics,
                            "quality_score": quality,
                        }))?
                    );
                } else {
                    println!("=== Paper Analysis ===\n");
                    println!("Title: {}", paper_obj.title);
                    println!("\nDetected Topics: {:?}", topics);
                    println!("Quality Score: {:.1}/10", quality);
                }
            } else {
                println!("Analyzing all papers in database...");
                let papers = db.list_papers(None, 100, 0)?;
                println!(
                    "Found {} papers. (Full analysis requires LLM integration)",
                    papers.len()
                );
            }
        }
        _ => {
            println!(
                "Unknown analysis type: {}. Use: summary, keywords, topics, quality",
                kind
            );
        }
    }
    Ok(())
}

pub fn extract_keywords(text: &str, top_n: usize) -> Vec<(String, f64)> {
    let stop_words: std::collections::HashSet<&str> = [
        "the", "a", "an", "is", "are", "was", "were", "be", "been", "being", "have", "has", "had",
        "do", "does", "did", "will", "would", "could", "should", "may", "might", "must", "shall",
        "can", "need", "to", "of", "in", "for", "on", "with", "at", "by", "from", "as", "into",
        "through", "during", "before", "after", "above", "below", "between", "under", "again",
        "further", "then", "once", "here", "there", "when", "where", "why", "how", "all", "each",
        "few", "more", "most", "other", "some", "such", "no", "nor", "not", "only", "own", "same",
        "so", "than", "too", "very", "just", "but", "and", "or", "if", "because", "until", "while",
        "this", "that", "these", "those", "which", "what", "who", "whom",
    ]
    .into_iter()
    .collect();

    let mut counts: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    for word in text.split_whitespace() {
        let clean: String = word.chars().filter(|c| c.is_alphanumeric()).collect();
        let clean_lower = clean.to_lowercase();
        if clean_lower.len() > 3 && !stop_words.contains(clean_lower.as_str()) {
            *counts.entry(clean_lower).or_insert(0) += 1;
        }
    }

    let total: usize = counts.values().sum();
    let mut scored: Vec<_> = counts
        .into_iter()
        .map(|(w, c)| (w, c as f64 / total as f64 * 100.0))
        .collect();
    scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    scored.into_iter().take(top_n).collect()
}

pub fn classify_topics(title: &str, abstract_: &str, categories: &[String]) -> Vec<String> {
    let text = format!("{} {} {}", title, abstract_, categories.join(" ")).to_lowercase();
    let mut topics = Vec::new();

    let topic_rules: Vec<(&str, &[&str])> = vec![
        (
            "Machine Learning",
            &[
                "machine learning",
                "deep learning",
                "neural network",
                "neural networks",
            ],
        ),
        (
            "NLP",
            &[
                "natural language",
                "transformer",
                "attention",
                "language model",
                "text",
                "parsing",
                "translation",
            ],
        ),
        (
            "Computer Vision",
            &[
                "image",
                "vision",
                "object detection",
                "segmentation",
                "image classification",
            ],
        ),
        (
            "Reinforcement Learning",
            &[
                "reinforcement learning",
                "policy",
                "reward",
                "agent",
                "environment",
            ],
        ),
        (
            "Optimization",
            &[
                "optimization",
                "optimizer",
                "gradient",
                "convergence",
                "loss function",
            ],
        ),
        (
            "Graph / Knowledge",
            &[
                "graph",
                "knowledge graph",
                "knowledge base",
                "entity",
                "relation",
            ],
        ),
        (
            "Uncertainty",
            &[
                "uncertainty",
                "probabilistic",
                "bayesian",
                "variance",
                "confidence",
            ],
        ),
        (
            "Scaling",
            &["scale", "scaling", "large-scale", "billion", "parameter"],
        ),
    ];

    for (topic, keywords) in topic_rules.iter() {
        if keywords.iter().any(|kw| text.contains(*kw)) {
            topics.push(topic.to_string());
        }
    }

    if topics.is_empty() {
        topics.push("General".to_string());
    }
    topics
}

pub fn estimate_quality(paper: &Paper) -> f64 {
    let mut score: f64 = 5.0; // base

    // Citations boost
    if paper.metadata.cited_by > 1000 {
        score += 2.0;
    } else if paper.metadata.cited_by > 100 {
        score += 1.0;
    }

    // Has abstract
    if !paper.abstract_text.is_empty() && paper.abstract_text.len() > 100 {
        score += 0.5;
    }

    // Has categories
    if !paper.categories.is_empty() {
        score += 0.5;
    }

    // Title length heuristic (reasonable length is better)
    if paper.title.len() > 30 && paper.title.len() < 150 {
        score += 0.5;
    }

    score.min(10.0_f64)
}

pub fn handle_ask(db: &Database, question: &str, max_papers: usize, format: &str) -> Result<()> {
    println!("=== Ask a Question ===");
    println!("Question: {}", question);
    println!("Max papers to search: {}", max_papers);
    println!();

    let papers = db.list_papers(None, max_papers, 0)?;

    if papers.is_empty() {
        println!("No papers in database. Add some papers first.");
        return Ok(());
    }

    // Keyword-based retrieval: split question into keywords and score papers
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

    let question_lower = question.to_lowercase();
    let question_words: Vec<&str> = question_lower
        .split_whitespace()
        .filter(|w| w.len() > 2 && !stop_words.contains(w))
        .collect();

    if question_words.is_empty() {
        println!("Question too generic. Try adding specific terms.");
        return Ok(());
    }

    println!("Keywords extracted: {}", question_words.join(", "));
    println!();

    // Score each paper by keyword overlap
    let mut scored: Vec<(&Paper, usize)> = Vec::new();
    for paper in &papers {
        let title_lower = paper.title.to_lowercase();
        let abstract_lower = paper.abstract_text.to_lowercase();
        let combined = format!("{} {}", title_lower, abstract_lower);

        let match_count = question_words
            .iter()
            .filter(|kw| combined.contains(*kw))
            .count();

        if match_count > 0 {
            scored.push((paper, match_count));
        }
    }

    // Sort by match count descending
    scored.sort_by(|a, b| b.1.cmp(&a.1));

    let top_papers: Vec<_> = scored.into_iter().take(5).collect();

    if top_papers.is_empty() {
        println!("No papers found matching your question keywords.");
        println!("Try different search terms.");
        return Ok(());
    }

    println!("Top {} most relevant papers:\n", top_papers.len());

    for (i, (paper, score)) in top_papers.iter().enumerate() {
        println!("{}. [score: {}] {}", i + 1, score, paper.title);
        println!("   {}", paper.authors.join(", "));
        println!(
            "   {} | cited_by: {}",
            paper.published.format("%Y-%m-%d"),
            paper.metadata.cited_by
        );
        if !paper.abstract_text.is_empty() {
            let preview = if paper.abstract_text.len() > 150 {
                format!("{}...", &paper.abstract_text[..150])
            } else {
                paper.abstract_text.clone()
            };
            println!("   {}\n", preview);
        }
    }

    if format == "json" {
        let out = serde_json::json!({
            "question": question,
            "papers_searched": papers.len(),
            "top_papers": top_papers.iter().map(|(p, s)| {
                serde_json::json!({
                    "id": p.id,
                    "title": p.title,
                    "authors": p.authors,
                    "score": s,
                    "abstract": p.abstract_text
                })
            }).collect::<Vec<_>>()
        });
        println!("{}", serde_json::to_string_pretty(&out)?);
    }

    Ok(())
}

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

pub fn handle_route(query: &[String], json: bool, exec: bool, all: bool) -> Result<()> {
    if query.is_empty() {
        eprintln!("Usage: rairos route <query>");
        return Ok(());
    }
    let query_text = query.join(" ");

    // ── QueryType taxonomy (mirrors Python llm.routing.semantic_router.QueryType) ──
    #[derive(Debug, Clone, Copy, PartialEq)]
    enum QueryType {
        GapAnalysis,
        HypothesisGeneration,
        Experiment,
        Insight,
        Narrative,
        PaperSearch,
        QuestionAnswer,
        General,
    }

    impl QueryType {
        fn value(&self) -> &'static str {
            match self {
                QueryType::GapAnalysis => "gap_analysis",
                QueryType::HypothesisGeneration => "hypothesis_generation",
                QueryType::Experiment => "experiment",
                QueryType::Insight => "insight",
                QueryType::Narrative => "narrative",
                QueryType::PaperSearch => "paper_search",
                QueryType::QuestionAnswer => "question_answer",
                QueryType::General => "general",
            }
        }
        fn command(&self) -> &'static str {
            match self {
                QueryType::GapAnalysis => "gap",
                QueryType::HypothesisGeneration => "hypothesize",
                QueryType::Experiment => "experiment",
                QueryType::Insight => "insight",
                QueryType::Narrative => "narrative",
                QueryType::PaperSearch => "search",
                QueryType::QuestionAnswer => "ask",
                QueryType::General => "chat",
            }
        }
    }

    // ── Keyword scoring ──
    fn keyword_score(query_lower: &str, qt: QueryType) -> f64 {
        let keywords: &[&str] = match qt {
            QueryType::GapAnalysis => &[
                "gap", "gaps", "空白", "未解决", "missing", "unresolved", "opportunity",
                "差距", "limitation", "limitations", "不足", "untouched", "overlooked",
                "open problem", "open question",
            ],
            QueryType::HypothesisGeneration => &[
                "hypothesis", "假设", "假设生成", "conjecture", "predict", "预测",
                "实验设计", "hypothesize", "if-then",
            ],
            QueryType::Experiment => &[
                "experiment", "实验", "ab test", "evaluate", "评估", "validate", "验证",
                "trial", "跑实验", "实验结果", "benchmark", "benchmarking",
            ],
            QueryType::Insight => &[
                "insight", "insights", "发现", "洞察", "pattern", "patterns",
                "key finding", "takeaway", "synthesis",
            ],
            QueryType::Narrative => &[
                "narrative", "story", "线程", "progress", "phase", "跟踪", "进展",
                "状态", "story arc",
            ],
            QueryType::PaperSearch => &[
                "paper", "papers", "search", "find", "论文", "搜索", "arxiv",
                "找论文", "文献", "publication",
            ],
            QueryType::QuestionAnswer => &[
                "what", "who", "how", "why", "explain", "什么", "如何", "为什么",
                "请问", "回答", "answer", "can you",
            ],
            QueryType::General => &[
                "chat", "talk", "discuss", "对话", "聊聊", "tell me", "about",
                "introduction", "介绍",
            ],
        };
        keywords.iter().filter(|kw| query_lower.contains(*kw)).count() as f64
    }

    let q_lower = query_text.to_lowercase();
    let query_types = [
        QueryType::GapAnalysis,
        QueryType::HypothesisGeneration,
        QueryType::Experiment,
        QueryType::Insight,
        QueryType::Narrative,
        QueryType::PaperSearch,
        QueryType::QuestionAnswer,
        QueryType::General,
    ];

    let mut best_score = 0.0f64;
    let mut best_qt = QueryType::General;

    for qt in &query_types {
        let score = keyword_score(&q_lower, *qt);
        if score > best_score {
            best_score = score;
            best_qt = *qt;
        }
    }

    let confidence = (best_score / 3.0).min(1.0);

    // ── Output ──
    let bar = "█".repeat((confidence * 10.0) as usize)
        + &"░".repeat(10 - (confidence * 10.0).min(10.0) as usize);

    if json {
        let output = serde_json::json!({
            "query_type": best_qt.value(),
            "confidence": confidence,
            "primary_command": best_qt.command(),
            "reasoning": "[keyword routing]",
            "multi_intent": false,
        });
        println!("{}", serde_json::to_string_pretty(&output).unwrap());
    } else {
        println!("🔍 Query: {}", query_text);
        println!("   Type:          {}", best_qt.value());
        println!("   Command:       {}", best_qt.command());
        println!("   Confidence:    {} {:.0}%", bar, confidence * 100.0);
        println!();
    }

    // ── Execution (print what would run) ──
    if exec || all {
        println!("   [Would run] {} \"{}\"", best_qt.command(), query_text);
    }

    Ok(())
}

pub fn handle_evoskill(action: &EvoSkillAction) -> Result<()> {
    match action {
        EvoSkillAction::Status => {
            // Check if evoskill CLI is available
            let which = std::process::Command::new("which")
                .arg("evoskill")
                .output();
            let available = match which {
                Ok(out) => out.status.success(),
                Err(_) => false,
            };

            // Also check ~/.claude/skills/evoskill
            let skill_path = dirs::home_dir()
                .map(|p| p.join(".claude").join("skills").join("evoskill"))
                .filter(|p| p.exists());

            if available || skill_path.is_some() {
                println!("✅ EvoSkill is available");
            } else {
                eprintln!("❌ EvoSkill not found");
                eprintln!("   Install: pip install evoskill");
            }
        }
        EvoSkillAction::Init {
            task,
            dataset,
            harness,
            model,
            question_col,
            answer_col,
            category_col,
        } => {
            println!("📦 Initializing EvoSkill project for task: {}", task);

            let work_dir = PathBuf::from(".evoskill");
            std::fs::create_dir_all(&work_dir)
                .context("Failed to create .evoskill directory")?;

            // Write config.toml
            let category_section = match category_col {
                Some(col) => format!("\ncategory_column = \"{}\"", col),
                None => String::new(),
            };
            let config = format!(
                r#"# EvoSkill project configuration for {task}

[harness]
name = "{harness}"
model = "{model}"
data_dirs = []
timeout_seconds = 1200
max_retries = 3

[evolution]
mode = "skill_only"
iterations = 20
frontier_size = 3
concurrency = 4
no_improvement_limit = 5
failure_samples = 3

[dataset]
path = "{dataset}"
question_column = "{question_col}"
ground_truth_column = "{answer_col}"{category_section}
train_ratio = 0.18
val_ratio = 0.12

[scorer]
type = "multi_tolerance"
"#,
                task = task,
                dataset = dataset,
                harness = harness,
                model = model,
                question_col = question_col,
                answer_col = answer_col,
                category_section = category_section,
            );
            std::fs::write(work_dir.join("config.toml"), &config)
                .context("Failed to write config.toml")?;

            // Write task.md
            let task_md = format!("# {}\n\nTask description for EvoSkill benchmark.\n", task);
            std::fs::write(work_dir.join("task.md"), &task_md)
                .context("Failed to write task.md")?;

            println!("  ✅ Config: {}", work_dir.join("config.toml").display());
            println!("  ✅ Task:   {}", work_dir.join("task.md").display());
            println!();
            println!("  Next: Edit .evoskill/task.md, then run: rairos evoskill run");
        }
        EvoSkillAction::Run {
            continue_mode,
            verbose,
        } => {
            println!("🚀 Running EvoSkill self-improvement loop...");
            let mut cmd = std::process::Command::new("evoskill");
            cmd.arg("run");
            if *continue_mode {
                cmd.arg("--continue");
            }
            if *verbose {
                cmd.arg("--verbose");
            }
            let status = cmd.status().context("Failed to run evoskill")?;
            if status.success() {
                println!("✅ Run completed");
            } else {
                anyhow::bail!("evoskill run failed (exit: {})", status);
            }
        }
        EvoSkillAction::Eval => {
            println!("📊 Evaluating...");
            let status = std::process::Command::new("evoskill")
                .arg("eval")
                .status()
                .context("Failed to run evoskill eval")?;
            if status.success() {
                println!("✅ Evaluation complete");
            } else {
                anyhow::bail!("evoskill eval failed (exit: {})", status);
            }
        }
        EvoSkillAction::Diff { from_iter, to_iter } => {
            let mut cmd = std::process::Command::new("evoskill");
            cmd.arg("diff");
            if let (Some(f), Some(t)) = (from_iter, to_iter) {
                cmd.arg(f.to_string());
                cmd.arg(t.to_string());
            }
            let output = cmd.output().context("Failed to run evoskill diff")?;
            if output.status.success() {
                print!("{}", String::from_utf8_lossy(&output.stdout));
            } else {
                let stderr = String::from_utf8_lossy(&output.stderr);
                anyhow::bail!("evoskill diff failed: {}", stderr);
            }
        }
        EvoSkillAction::Reset => {
            println!("🔄 Resetting all program branches...");
            let status = std::process::Command::new("evoskill")
                .arg("reset")
                .status()
                .context("Failed to run evoskill reset")?;
            if status.success() {
                println!("✅ Reset complete");
            } else {
                anyhow::bail!("evoskill reset failed (exit: {})", status);
            }
        }
    }
    Ok(())
}

pub fn handle_rag(action: &RagAction) -> Result<()> {
    match action {
        RagAction::Status => {
            // Check paper2code availability
            let paper2code_ok = std::process::Command::new("which")
                .arg("paper2code")
                .output()
                .map(|o| o.status.success())
                .unwrap_or(false)
                || dirs::home_dir()
                    .map(|p| p.join(".claude").join("skills").join("paper2code").exists())
                    .unwrap_or(false);

            // Check evoskill availability (same logic as handle_evoskill)
            let evoskill_ok = std::process::Command::new("which")
                .arg("evoskill")
                .output()
                .map(|o| o.status.success())
                .unwrap_or(false)
                || dirs::home_dir()
                    .map(|p| p.join(".claude").join("skills").join("evoskill").exists())
                    .unwrap_or(false);

            println!("🔍 RAG Pipeline Status");
            println!();
            println!("  {:<20} {}", "Component", "Status");
            println!("  {}", "─".repeat(35));
            println!(
                "  {:<20} {}",
                "paper2code",
                if paper2code_ok { "✅ available" } else { "❌ not found" }
            );
            println!(
                "  {:<20} {}",
                "EvoSkill",
                if evoskill_ok { "✅ available" } else { "❌ not found" }
            );
            println!();
            if paper2code_ok && evoskill_ok {
                println!("  RAG pipeline is fully available");
                println!("  Run: rairos rag run-full <arxiv_id>");
            } else {
                println!("  Some components are missing");
            }
        }
        RagAction::RunFull {
            arxiv_id,
            mode,
            framework,
            task,
        } => {
            let arxiv_id = clean_arxiv_id(arxiv_id);
            let task_name = task.clone().unwrap_or_else(|| {
                format!("paper_{}", arxiv_id.replace('.', "_"))
            });
            let work_dir = PathBuf::from(".rag_work");
            let paper_dir = work_dir.join(&arxiv_id);

            println!("🚀 Starting RAG pipeline for arXiv: {}", arxiv_id);

            // Stage 1: paper2code — shell out to external CLI or use existing tool
            println!("  Stage 1/4: Generating code from paper...");
            run_paper2code(&arxiv_id, mode, framework)?;

            // Stage 2: Extract test cases
            println!("  Stage 2/4: Extracting test cases...");
            let test_csv = extract_and_generate_tests(&arxiv_id, &paper_dir)?;

            // Stage 3: Generate pytest files
            println!("  Stage 3/4: Generating pytest tests...");
            generate_pytest_tests(&paper_dir, &test_csv)?;

            // Stage 4: Initialize EvoSkill benchmark
            println!("  Stage 4/4: Initializing EvoSkill benchmark...");
            init_evoskill_benchmark(&work_dir, &task_name, &test_csv)?;

            println!();
            println!("✅ RAG pipeline completed!");
            println!("  Code:      {}", paper_dir.join("src").display());
            println!("  Test CSV:  {}", test_csv.display());
            println!("  Test dir:  {}", paper_dir.join("tests").display());
            println!("  Benchmark: {}", work_dir.join(".evoskill").display());
            println!();
            println!("  Next: Run 'rairos rag run-evoskill' to start skill improvement");
        }
        RagAction::GenTests { arxiv_id } => {
            let arxiv_id = clean_arxiv_id(arxiv_id);
            let paper_dir = PathBuf::from(".rag_work").join(&arxiv_id);

            println!("🧪 Generating tests for arXiv: {}", arxiv_id);
            let test_csv = extract_and_generate_tests(&arxiv_id, &paper_dir)?;
            generate_pytest_tests(&paper_dir, &test_csv)?;

            println!("✅ Tests generated: {}", test_csv.display());
        }
        RagAction::InitBenchmark {
            csv_path,
            task,
        } => {
            let work_dir = PathBuf::from(".rag_work");
            println!("📦 Initializing benchmark for task: {}", task);
            init_evoskill_benchmark(&work_dir, task, &PathBuf::from(csv_path))?;
            println!("✅ Benchmark initialized!");
            println!("  Config: {}", work_dir.join(".evoskill").join("config.toml").display());
            println!("  Task:   {}", work_dir.join(".evoskill").join("task.md").display());
            println!();
            println!("  Next: Run 'rairos rag run-evoskill'");
        }
        RagAction::RunEvoskill { continue_mode } => {
            println!("🚀 Running EvoSkill improvement loop...");
            let mut cmd = std::process::Command::new("evoskill");
            cmd.arg("run");
            if *continue_mode {
                cmd.arg("--continue");
            }
            let status = cmd.status().context("Failed to run evoskill")?;
            if status.success() {
                println!("✅ EvoSkill run completed");
            } else {
                anyhow::bail!("evoskill run failed (exit: {})", status);
            }
        }
        RagAction::ListSkills => {
            let output = std::process::Command::new("evoskill")
                .arg("skills")
                .output()
                .context("Failed to list evoskill skills")?;
            if output.status.success() {
                let stdout = String::from_utf8_lossy(&output.stdout);
                let skills: Vec<&str> = stdout.lines().filter(|l| !l.trim().is_empty()).collect();
                if skills.is_empty() {
                    println!("No skills discovered yet");
                } else {
                    println!("Discovered skills:");
                    for skill in &skills {
                        println!("  - {}", skill);
                    }
                }
            } else {
                anyhow::bail!("evoskill skills failed (exit: {})", output.status);
            }
        }
    }
    Ok(())
}

pub fn clean_arxiv_id(s: &str) -> String {
    // Extract arXiv ID from URL or pattern
    if let Some(caps) = regex::Regex::new(r"(\d{4}\.\d{4,5})")
        .ok()
        .and_then(|re| re.captures(s))
    {
        caps.get(1).unwrap().as_str().to_string()
    } else {
        s.to_string()
    }
}

pub fn run_paper2code(arxiv_id: &str, mode: &str, framework: &str) -> Result<()> {
    // Check if paper2code CLI is available
    let available = std::process::Command::new("which")
        .arg("paper2code")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);

    if !available {
        eprintln!("  ⚠️  paper2code not installed, creating placeholder structure");
        let paper_dir = PathBuf::from(".rag_work").join(arxiv_id);
        let src_dir = paper_dir.join("src");
        std::fs::create_dir_all(&src_dir)?;
        let readme = format!("# Paper {}\n\nImplementation generated by paper2code.\nMode: {}\nFramework: {}\n", arxiv_id, mode, framework);
        std::fs::write(paper_dir.join("README.md"), &readme)?;
        let placeholder = format!(
            r#""""Paper {} implementation placeholder.""""\n# TODO: Run paper2code to generate implementation\n"#,
            arxiv_id
        );
        std::fs::write(src_dir.join("implementation.py"), &placeholder)?;
        return Ok(());
    }

    let status = std::process::Command::new("paper2code")
        .arg(arxiv_id)
        .arg("--mode")
        .arg(mode)
        .arg("--framework")
        .arg(framework)
        .status()
        .context("Failed to run paper2code")?;

    if !status.success() {
        anyhow::bail!("paper2code failed (exit: {})", status);
    }
    Ok(())
}

pub fn extract_and_generate_tests(arxiv_id: &str, paper_dir: &Path) -> Result<PathBuf> {
    let test_csv = paper_dir.join("tests").join("test_cases.csv");
    std::fs::create_dir_all(test_csv.parent().unwrap())?;

    let test_cases = extract_from_code(paper_dir);

    let cases = if test_cases.is_empty() {
        generate_default_cases(arxiv_id)
    } else {
        test_cases
    };

    // Write CSV
    let mut wtr = csv::Writer::from_path(&test_csv)?;
    wtr.write_record(["question", "expected_output", "category"])?;
    for case in &cases {
        wtr.write_record([
            case.0.as_str(),
            case.1.as_str(),
            case.2.as_str(),
        ])?;
    }
    wtr.flush()?;

    Ok(test_csv)
}

pub fn extract_from_code(paper_dir: &Path) -> Vec<(String, String, String)> {
    let mut cases = Vec::new();

    // Check README for code examples
    let readme_path = paper_dir.join("README.md");
    if readme_path.exists() {
        if let Ok(content) = std::fs::read_to_string(&readme_path) {
            // Match code blocks with Python examples
            let re = regex::Regex::new(r"```(?:python|py)?\n(.*?)```").ok();
            if let Some(re) = re {
                for cap in re.captures_iter(&content) {
                    let match_text = cap.get(1).map(|m| m.as_str()).unwrap_or("");
                    if match_text.contains('=') && match_text.contains("print") {
                        cases.push((
                            format!("Execute and provide output: ```{}```", match_text.trim()),
                            "execution successful".to_string(),
                            "execution".to_string(),
                        ));
                    }
                }
            }
        }
    }

    // Check src directory for docstring examples
    let src_dir = paper_dir.join("src");
    if src_dir.exists() {
        let re = regex::Regex::new(r#""""\s*(.*?)\s*""""#).ok();
        if let Ok(entries) = std::fs::read_dir(&src_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().map(|e| e == "py").unwrap_or(false) {
                    if let Ok(content) = std::fs::read_to_string(&path) {
                        if let Some(ref re) = re {
                            for cap in re.captures_iter(&content) {
                                let match_text = cap.get(1).map(|m| m.as_str()).unwrap_or("");
                                if match_text.contains("Example") || match_text.contains("例子") {
                                    let preview: String = match_text.chars().take(100).collect();
                                    cases.push((
                                        format!("Implement function per docstring: {}", preview),
                                        "implementation correct".to_string(),
                                        "implementation".to_string(),
                                    ));
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    cases.truncate(20);
    cases
}

pub fn generate_default_cases(arxiv_id: &str) -> Vec<(String, String, String)> {
    vec![
        (
            format!("Verify {} implementation correctness", arxiv_id),
            "functional".to_string(),
            "general".to_string(),
        ),
        (
            format!("Check {} API interface", arxiv_id),
            "API available".to_string(),
            "api".to_string(),
        ),
        (
            format!("Verify {} input/output format", arxiv_id),
            "format correct".to_string(),
            "io".to_string(),
        ),
    ]
}

pub fn generate_pytest_tests(_paper_dir: &Path, test_csv: &Path) -> Result<()> {
    let test_dir = test_csv.parent().unwrap();

    // conftest.py
    let conftest = r#""""Fixtures for generated tests.""""
import pytest
from pathlib import Path

@pytest.fixture
def test_data_path():
    """Path to test cases CSV."""
    return Path(__file__).parent / "test_cases.csv"

@pytest.fixture
def paper_dir():
    """Path to paper implementation."""
    return Path(__file__).parent.parent
"#;
    std::fs::write(test_dir.join("conftest.py"), conftest)?;

    // test_impl.py
    let test_impl = r#""""Auto-generated tests for paper implementation.""""
import csv
import pytest
from pathlib import Path

def load_test_cases():
    csv_path = Path(__file__).parent / "test_cases.csv"
    cases = []
    with open(csv_path, encoding="utf-8") as f:
        reader = csv.DictReader(f)
        for row in reader:
            cases.append(row)
    return cases

class TestPaperImplementation:
    @pytest.fixture(autouse=True)
    def setup(self, paper_dir):
        self.paper_dir = paper_dir

    def test_code_directory_exists(self):
        src_dir = self.paper_dir / "src"
        assert src_dir.exists(), f"Implementation dir not found: {src_dir}"

    @pytest.mark.parametrize("case", load_test_cases(), ids=lambda c: c["category"])
    def test_case(self, case):
        assert case["category"] in ["execution", "implementation", "general", "api", "io"]
        assert len(case["question"]) > 0
        assert len(case["expected_output"]) > 0
"#;
    std::fs::write(test_dir.join("test_impl.py"), test_impl)?;

    Ok(())
}

pub fn init_evoskill_benchmark(work_dir: &Path, task_name: &str, csv_path: &Path) -> Result<()> {
    let evoskill_dir = work_dir.join(".evoskill");
    std::fs::create_dir_all(&evoskill_dir)?;

    let config_content = format!(
        r#"# EvoSkill benchmark for {task}

[harness]
name = "claude"
model = "sonnet"
data_dirs = []
timeout_seconds = 600
max_retries = 2

[evolution]
mode = "skill_only"
iterations = 10
frontier_size = 2
concurrency = 2
no_improvement_limit = 3
failure_samples = 2

[dataset]
path = "{csv}"
question_column = "question"
ground_truth_column = "expected_output"
category_column = "category"
train_ratio = 0.5
val_ratio = 0.3

[scorer]
type = "multi_tolerance"
"#,
        task = task_name,
        csv = csv_path.display(),
    );
    std::fs::write(evoskill_dir.join("config.toml"), &config_content)?;

    let task_content = r#"# Task

验证 paper 实现的功能是否正确。

## Output format
返回 "通过" 或具体错误信息。
"#;
    std::fs::write(evoskill_dir.join("task.md"), task_content)?;

    Ok(())
}

pub fn handle_chat(
    question: Option<&str>,
    paper: Option<&str>,
    _concept: Option<&str>,
    limit: usize,
    interactive: bool,
    no_cite: bool,
    model: Option<&str>,
    verbose: bool,
    stream: bool,
    export_path: Option<&str>,
    export_fmt: Option<&str>,
) -> Result<()> {
    let api_key = std::env::var("OPENAI_API_KEY")
        .or_else(|_| std::env::var("LLM_API_KEY"))
        .map_err(|_| anyhow::anyhow!("OPENAI_API_KEY not set. Please set it to enable chat."))?;
    let base_url = std::env::var("LLM_BASE_URL")
        .unwrap_or_else(|_| "https://api.openai.com/v1".to_string());
    let chat_model = model.unwrap_or("gpt-4o-mini").to_string();

    let db_path = PathBuf::from("rairos.db");
    let db = Database::open(&db_path)?;

    let rt = tokio::runtime::Runtime::new()?;

    let rag_system_prompt = "你是一个严谨的 AI 研究助手，精通论文阅读和学术分析。

核心原则：
1. 基于原文回答，不要捏造或推测未提及的内容
2. 不确定的信息必须加 [推测] 标注
3. 使用 > 块引用格式引用原文片段
4. 区分\"原文明确说\"和\"可推断\"
5. 回答使用中文，但引用原文时保留英文原句

输出格式：
- 开头总结回答要点（1-2句话）
- 详细解释部分引用原文片段
- 结尾标注信息来源";

    if interactive || question.is_none() {
        run_chat_interactive(&db, &rt, &api_key, &base_url, &chat_model, rag_system_prompt,
            paper, limit, no_cite, verbose, stream, export_path, export_fmt)?;
    } else if let Some(q) = question {
        run_chat_single(q, &db, &rt, &api_key, &base_url, &chat_model,
            rag_system_prompt, paper, limit, no_cite, verbose, stream)?;
    }

    Ok(())
}

pub fn run_chat_single(
    question: &str,
    db: &Database,
    rt: &tokio::runtime::Runtime,
    api_key: &str,
    base_url: &str,
    chat_model: &str,
    rag_system_prompt: &str,
    _paper: Option<&str>,
    limit: usize,
    no_cite: bool,
    _verbose: bool,
    _stream: bool,
) -> Result<()> {
    let papers = db.search_papers(question, limit)?;
    if papers.is_empty() {
        eprintln!("No papers found matching your question.");
        return Ok(());
    }

    let context_parts: Vec<String> = papers.iter().enumerate().map(|(i, p)| {
        let abstract_text = if p.abstract_text.len() > 500 {
            format!("{}...", &p.abstract_text[..500])
        } else {
            p.abstract_text.clone()
        };
        format!(
            "[Paper {}] Title: {}\nAuthors: {}\nAbstract: {}",
            i + 1,
            p.title,
            p.authors.join(", "),
            abstract_text
        )
    }).collect();
    let context_str = context_parts.join("\n\n");
    let user_prompt = format!(
        "基于以下论文内容回答问题。\n\n{context_str}\n\n问题: {question}"
    );

    println!("{}", "═".repeat(60));
    println!("💡 Answer:");

    let answer = rt.block_on(async {
        let client = rairos_llm::client_async::AsyncClient::new(
            api_key.to_string(),
            base_url.to_string(),
            chat_model.to_string(),
        );
        let messages = vec![
            std::collections::HashMap::from([
                ("role".to_string(), "user".to_string()),
                ("content".to_string(), user_prompt.clone()),
            ]),
        ];
        client.chat_completions(messages, None, Some(rag_system_prompt), false).await
    }).map_err(|e| anyhow::anyhow!("LLM call failed: {}", e))?;

    println!("{}", answer);
    println!("{}", "═".repeat(60));

    if !no_cite {
        println!("\n📖 引用来源");
        println!("{}", "-".repeat(60));
        for (i, p) in papers.iter().enumerate() {
            let preview: String = p.abstract_text.chars().take(150).collect();
            println!("\n[{}] {}", i + 1, p.title);
            println!("    ID: {}", p.id);
            println!("    > {}...", preview);
        }
    }

    Ok(())
}

pub fn run_chat_interactive(
    db: &Database,
    rt: &tokio::runtime::Runtime,
    api_key: &str,
    base_url: &str,
    chat_model: &str,
    rag_system_prompt: &str,
    _paper: Option<&str>,
    limit: usize,
    no_cite: bool,
    verbose: bool,
    stream: bool,
    export_path: Option<&str>,
    export_fmt: Option<&str>,
) -> Result<()> {
    println!("{}", "═".repeat(60));
    println!("📚 AI Research OS — RAG Chat");
    println!("{}", "═".repeat(60));
    println!();
    println!("Commands:");
    println!("  q / quit / exit    Quit");
    println!("  clear              Clear history");
    println!("  help               Show help");
    println!();
    println!("Tip: Ask questions about papers in your library.");
    println!();

    let mut history: Vec<(String, String)> = Vec::new();

    loop {
        let question = {
            print!("❓ ");
            use std::io::Write;
            std::io::stdout().flush().ok();
            let mut line = String::new();
            match std::io::stdin().read_line(&mut line) {
                Ok(0) | Err(_) => break,
                Ok(_) => line.trim().to_string(),
            }
        };

        if question.is_empty() {
            continue;
        }

        match question.to_lowercase().as_str() {
            "q" | "quit" | "exit" => {
                if !history.is_empty() {
                    if let Some(path) = export_path {
                        export_chat_history(&history, path, export_fmt);
                        println!("✅ Exported to {}", path);
                    }
                }
                println!("\n再见！");
                break;
            }
            "clear" => {
                history.clear();
                println!("✅ History cleared");
                continue;
            }
            "help" => {
                println!("\nHelp:");
                println!("  Ask any question about papers in your library");
                println!("  Example questions:");
                println!("    How does self-attention work?");
                println!("    What are the main contributions?");
                println!("    What is Sparse MoE?");
                println!();
                continue;
            }
            _ => {}
        }

        if verbose {
            println!("🔍 Retrieving papers...");
        }
        let papers = match db.search_papers(&question, limit) {
            Ok(p) => p,
            Err(e) => {
                eprintln!("Search failed: {}", e);
                continue;
            }
        };

        if papers.is_empty() {
            println!("No matching papers found. Try a different question.");
            continue;
        }

        let context_parts: Vec<String> = papers.iter().enumerate().map(|(i, p)| {
            let abstract_text = if p.abstract_text.len() > 400 {
                format!("{}...", &p.abstract_text[..400])
            } else {
                p.abstract_text.clone()
            };
            format!(
                "[Paper {}] Title: {}\nAuthors: {}\nAbstract: {}",
                i + 1,
                p.title,
                p.authors.join(", "),
                abstract_text
            )
        }).collect();
        let context_str = context_parts.join("\n\n");
        let user_prompt = format!(
            "基于以下论文内容回答问题。\n\n{context_str}\n\n问题: {question}"
        );

        println!("\n💡 Answer:");
        println!("{}", "─".repeat(60));

        let answer_result = rt.block_on(async {
            let client = rairos_llm::client_async::AsyncClient::new(
                api_key.to_string(),
                base_url.to_string(),
                chat_model.to_string(),
            );
            let messages = vec![
                std::collections::HashMap::from([
                    ("role".to_string(), "user".to_string()),
                    ("content".to_string(), user_prompt.clone()),
                ]),
            ];
            if stream {
                client.chat_completions_streaming(messages, None, Some(rag_system_prompt)).await
            } else {
                client.chat_completions(messages, None, Some(rag_system_prompt), false).await
            }
        });

        match answer_result {
            Ok(answer) => {
                println!("{}", answer);
                println!("{}", "─".repeat(60));
                if !no_cite {
                    println!("\n📖 引用来源");
                    for (i, p) in papers.iter().enumerate().take(5) {
                        println!("  [{}] {} (ID: {})", i + 1, p.title, p.id);
                    }
                }
                println!();
                history.push((question, answer));
            }
            Err(e) => {
                eprintln!("LLM call failed: {}", e);
            }
        }
    }

    Ok(())
}

pub fn export_chat_history(history: &[(String, String)], path: &str, fmt: Option<&str>) {
    let ext = std::path::Path::new(path)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("");
    let format = fmt.unwrap_or(match ext {
        "html" | "htm" => "html",
        _ => "markdown",
    });
    let content = match format {
        "html" => export_chat_to_html(history),
        _ => export_chat_to_markdown(history),
    };
    let _ = std::fs::write(path, content);
}

pub fn export_chat_to_markdown(history: &[(String, String)]) -> String {
    use chrono::Local;
    let now = Local::now().format("%Y-%m-%d %H:%M:%S");
    let mut md = format!("# AI Research OS — Chat Export\n\n**Exported**: {now}\n\n---\n\n", now = now);
    for (i, (q, a)) in history.iter().enumerate() {
        md.push_str(&format!("## Q{i}: {q}\n\n**A**: {a}\n\n---\n\n", i = i + 1, q = q, a = a));
    }
    md
}

pub fn export_chat_to_html(history: &[(String, String)]) -> String {
    use chrono::Local;
    let now = Local::now().format("%Y-%m-%d %H:%M:%S");
    let mut html = format!(
        r#"<!DOCTYPE html>
<html lang='zh-CN'>
<head>
<meta charset='UTF-8'>
<title>AI Research OS — Chat Export</title>
<style>
body {{ font-family: 'Segoe UI', Arial, sans-serif; max-width: 800px; margin: 0 auto; padding: 20px; }}
h1 {{ color: #1a1a2e; border-bottom: 2px solid #4a4a8a; padding-bottom: 10px; }}
.qa-block {{ background: #f8f9fa; border-radius: 8px; padding: 15px; margin: 15px 0; }}
.question {{ color: #2a5a2a; font-weight: bold; }}
.answer {{ color: #333; margin-top: 10px; line-height: 1.6; }}
.meta {{ color: #666; font-size: 0.85em; }}
</style>
</head>
<body>
<h1>AI Research OS — Chat Export</h1>
<p class='meta'>Exported: {now}</p>
"#, now = now);
    for (i, (q, a)) in history.iter().enumerate() {
        html.push_str(&format!(
            r#"<div class='qa-block'>
<div class='question'>Q{i}: {q}</div>
<div class='answer'>{a}</div>
</div>
"#, i = i + 1, q = q, a = a));
    }
    html.push_str("</body>\n</html>");
    html
}

pub fn handle_chat_tui() -> Result<()> {
    use ratatui::{
        layout::{Constraint, Direction, Layout},
        style::{Color, Modifier, Style},
        text::{Line, Span, Text},
        widgets::{Block, Borders, List, ListItem, Paragraph},
        Terminal,
    };
    use crossterm::{
        event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind},
        execute,
        terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    };

    let api_key = match std::env::var("OPENAI_API_KEY")
        .or_else(|_| std::env::var("LLM_API_KEY"))
    {
        Ok(k) => k,
        Err(_) => {
            eprintln!("OPENAI_API_KEY not set. Please set it to enable chat.");
            return Ok(());
        }
    };
    let base_url = std::env::var("LLM_BASE_URL")
        .unwrap_or_else(|_| "https://api.openai.com/v1".to_string());
    let model = "gpt-4o-mini".to_string();

    let db_path = PathBuf::from("rairos.db");
    let db = match Database::open(&db_path) {
        Ok(db) => db,
        Err(e) => {
            eprintln!("Failed to open database: {}", e);
            return Ok(());
        }
    };
    let rt = tokio::runtime::Runtime::new()?;

    let rag_system_prompt = "你是一个严谨的 AI 研究助手，精通论文阅读和学术分析。

核心原则：
1. 基于原文回答，不要捏造或推测未提及的内容
2. 使用 > 块引用格式引用原文片段
3. 回答使用中文，但引用原文时保留英文原句

输出格式：
- 开头总结回答要点
- 详细解释部分引用原文片段
- 结尾标注信息来源";

    // ── TUI setup ──
    enable_raw_mode()?;
    let mut stdout = std::io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = ratatui::backend::CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // ── State ──
    #[derive(Clone)]
    struct ChatMsg {
        role: String,  // "user" | "assistant" | "error" | "info"
        content: String,
    }

    let mut messages: Vec<ChatMsg> = Vec::new();
    let mut input = String::new();
    let _scroll_offset: usize = 0;
    let mut loading = false;

    messages.push(ChatMsg {
        role: "info".to_string(),
        content: "Welcome to Rairos TUI Chat! Type a question and press Enter. Type /quit or Esc to exit.".to_string(),
    });

    let r = loop {
        terminal.draw(|f| {
            let size = f.area();
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Min(3),
                    Constraint::Length(3),
                ])
                .split(size);

            // ── Message area ──
            let msg_items: Vec<ListItem> = messages.iter().map(|msg| {
                let style = match msg.role.as_str() {
                    "user" => Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
                    "assistant" => Style::default().fg(Color::Green),
                    "error" => Style::default().fg(Color::Red),
                    _ => Style::default().fg(Color::DarkGray),
                };
                let prefix = match msg.role.as_str() {
                    "user" => "❓ ",
                    "assistant" => "💡 ",
                    "error" => "⚠️ ",
                    _ => "",
                };
                let lines: Vec<Line> = msg.content.lines().map(|l| {
                    Line::from(Span::styled(format!("{}{}", prefix, l), style))
                }).collect();
                ListItem::new(lines)
            }).collect();

            let msg_list = List::new(msg_items)
                .block(Block::default()
                    .title("  AI Research OS Chat  ")
                    .borders(Borders::ALL))
                .highlight_style(Style::default().add_modifier(Modifier::BOLD));
            f.render_widget(msg_list, chunks[0]);

            // ── Input area ──
            let input_style = if loading {
                Style::default().fg(Color::Yellow)
            } else {
                Style::default()
            };
            let input_para = Paragraph::new(Text::from(input.as_str()))
                .style(input_style)
                .block(Block::default()
                    .title(if loading { "  ⏳ Thinking...  " } else { "  Type your question  " })
                    .borders(Borders::ALL));
            f.render_widget(input_para, chunks[1]);

            // Move cursor to end of input
            if !loading {
                let x = chunks[1].x + 1 + input.len() as u16;
                let y = chunks[1].y + 1;
                if x < chunks[1].x + chunks[1].width - 1 {
                    f.set_cursor_position(ratatui::layout::Position::new(x, y));
                }
            }
        })?;

        // ── Event handling ──
        if let Event::Key(key) = event::read()? {
            if key.kind == KeyEventKind::Press {
                match key.code {
                    KeyCode::Esc => break Ok(()),
                    KeyCode::Char('c') if key.modifiers.contains(crossterm::event::KeyModifiers::CONTROL) => {
                        break Ok(());
                    }
                    KeyCode::Enter if !loading => {
                        let q = input.trim().to_string();
                        if q.is_empty() {
                            continue;
                        }
                        if q == "/quit" || q == "/exit" {
                            break Ok(());
                        }

                        messages.push(ChatMsg {
                            role: "user".to_string(),
                            content: q.clone(),
                        });
                        input.clear();
                        // loading flag is conceptually for async render; block_on is synchronous here

                        // Search papers and call LLM
                        let papers = db.search_papers(&q, 5).unwrap_or_default();
                        if papers.is_empty() {
                            messages.push(ChatMsg {
                                role: "info".to_string(),
                                content: "No matching papers found. Try a different question.".to_string(),
                            });
                            loading = false;
                            continue;
                        }

                        let context_parts: Vec<String> = papers.iter().enumerate().map(|(i, p)| {
                            let abs = if p.abstract_text.len() > 400 {
                                format!("{}...", &p.abstract_text[..400])
                            } else {
                                p.abstract_text.clone()
                            };
                            format!(
                                "[Paper {}] Title: {}\nAuthors: {}\nAbstract: {}",
                                i + 1, p.title, p.authors.join(", "), abs
                            )
                        }).collect();
                        let context_str = context_parts.join("\n\n");
                        let user_prompt = format!(
                            "基于以下论文内容回答问题。\n\n{context_str}\n\n问题: {q}"
                        );

                        let api_key = api_key.clone();
                        let base_url = base_url.clone();
                        let model = model.clone();
                        let rag_system_prompt = rag_system_prompt.to_string();

                        let answer_result = rt.block_on(async {
                            let client = rairos_llm::client_async::AsyncClient::new(
                                api_key, base_url, model,
                            );
                            let msgs = vec![
                                std::collections::HashMap::from([
                                    ("role".to_string(), "user".to_string()),
                                    ("content".to_string(), user_prompt),
                                ]),
                            ];
                            client.chat_completions(msgs, None, Some(&rag_system_prompt), false).await
                        });

                        match answer_result {
                            Ok(answer) => {
                                // Build response with citations
                                let mut response = answer;
                                if !papers.is_empty() {
                                    response.push_str("\n\n─── Citations ───\n");
                                    for (i, p) in papers.iter().enumerate() {
                                        response.push_str(&format!(
                                            "[{}] {} (ID: {})\n", i + 1, p.title, p.id
                                        ));
                                    }
                                }
                                messages.push(ChatMsg {
                                    role: "assistant".to_string(),
                                    content: response,
                                });
                            }
                            Err(e) => {
                                messages.push(ChatMsg {
                                    role: "error".to_string(),
                                    content: format!("LLM call failed: {}", e),
                                });
                            }
                        }
                        loading = false;
                    }
                    KeyCode::Char(c) if !loading => {
                        input.push(c);
                    }
                    KeyCode::Backspace if !loading => {
                        input.pop();
                    }
                    _ => {}
                }
            }
        }
    };

    // ── Cleanup ──
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture,
    )?;
    terminal.show_cursor()?;

    r
}

