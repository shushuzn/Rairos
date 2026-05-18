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
use rairos_core::{Database, Paper};
use crate::handlers::*;

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
