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
use rairos_core::{Database, ResearchGap};

pub fn handle_gap(
    db: &Database,
    topic: &str,
    limit: usize,
    format: &str,
    category: Option<String>,
) -> Result<()> {
    println!("Detecting research gaps for topic: {}", topic);

    let papers = db.search_papers_smart(topic, limit * 3)?;

    if papers.is_empty() {
        println!(
            "No papers found for topic '{}'. Try a different query.",
            topic
        );
        return Ok(());
    }

    let total_papers = papers.len();
    let stop_words: std::collections::HashSet<&str> = [
        "the",
        "a",
        "an",
        "is",
        "are",
        "was",
        "were",
        "be",
        "been",
        "being",
        "have",
        "has",
        "had",
        "do",
        "does",
        "did",
        "will",
        "would",
        "could",
        "should",
        "may",
        "might",
        "must",
        "shall",
        "can",
        "need",
        "dare",
        "to",
        "of",
        "in",
        "for",
        "on",
        "with",
        "at",
        "by",
        "from",
        "as",
        "into",
        "through",
        "during",
        "before",
        "after",
        "above",
        "below",
        "between",
        "under",
        "again",
        "further",
        "then",
        "once",
        "here",
        "there",
        "when",
        "where",
        "why",
        "how",
        "all",
        "each",
        "few",
        "more",
        "most",
        "other",
        "some",
        "such",
        "no",
        "nor",
        "not",
        "only",
        "own",
        "same",
        "so",
        "than",
        "too",
        "very",
        "just",
        "but",
        "and",
        "or",
        "if",
        "because",
        "as",
        "until",
        "while",
        "this",
        "that",
        "these",
        "those",
        "paper",
        "papers",
        "study",
        "method",
        "approach",
        "result",
        "results",
        "show",
        "shown",
        "using",
        "used",
        "based",
        "proposed",
        "present",
        "presented",
        "state",
    ]
    .into();

    // ============================================================
    // GAP 1: Underexplored subtopics (keywords appearing in 1-2 papers)
    // ============================================================
    let mut keyword_to_papers: std::collections::HashMap<String, Vec<String>> =
        std::collections::HashMap::new();
    let mut keyword_counts: std::collections::HashMap<String, usize> =
        std::collections::HashMap::new();

    for paper in &papers {
        let text = format!(
            "{} {} {}",
            paper.title,
            paper.abstract_text,
            paper.categories.join(" ")
        );
        let words: std::collections::HashSet<String> = text
            .to_lowercase()
            .split(|c: char| !c.is_alphanumeric())
            .filter(|w| w.len() > 3 && !stop_words.contains(w))
            .map(|w| w.to_string())
            .collect();

        for word in words {
            *keyword_counts.entry(word.clone()).or_insert(0) += 1;
            keyword_to_papers
                .entry(word)
                .or_insert_with(Vec::new)
                .push(paper.id.clone());
        }
    }

    // Rare keywords = appearing in 1-2 papers (out of many) - underexplored areas
    let rare_keywords: Vec<(String, usize)> = keyword_counts
        .iter()
        .filter(|(_, &count)| count >= 1 && count <= 2 && total_papers > 5)
        .map(|(k, &c)| (k.clone(), c))
        .collect();

    let mut gaps = Vec::new();

    // GAP 1: Underexplored subtopics
    if rare_keywords.len() > 3 {
        let sample: Vec<_> = rare_keywords.iter().take(5).collect();
        let examples: Vec<String> = sample.iter().map(|(k, _)| format!("\"{}\"", k)).collect();
        let gap = ResearchGap::new_simple(
            category.as_deref().unwrap_or("underexplored"),
            &format!(
                "Underexplored subtopics detected: {} (appearing in only 1-2 papers each). \
                Potential research directions: {}",
                rare_keywords.len(),
                examples.join(", ")
            ),
            "high",
        );
        let paper_ids: Vec<String> = rare_keywords
            .iter()
            .take(5)
            .flat_map(|(kw, _)| keyword_to_papers.get(kw).cloned().unwrap_or_default())
            .take(5)
            .collect();
        let mut g = gap;
        g.paper_ids = paper_ids;
        gaps.push(g);
    }

    // ============================================================
    // GAP 2: Category imbalance (some categories underrepresented)
    // ============================================================
    let mut cat_counts: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    for paper in &papers {
        for cat in &paper.categories {
            *cat_counts.entry(cat.clone()).or_insert(0) += 1;
        }
    }

    let total_cats = cat_counts.values().sum::<usize>();
    if total_cats > 0 {
        let avg_cats_per_paper = total_cats as f64 / total_papers as f64;
        let underrepresented: Vec<(String, usize)> = cat_counts
            .iter()
            .filter(|(_, &count)| {
                let freq = count as f64 / total_papers as f64;
                freq < 0.3 * avg_cats_per_paper && count <= 2
            })
            .map(|(k, &c)| (k.clone(), c))
            .collect();

        if !underrepresented.is_empty() {
            let cats: Vec<String> = underrepresented
                .iter()
                .take(5)
                .map(|(k, _)| k.clone())
                .collect();
            let gap = ResearchGap::new_simple(
                category.as_deref().unwrap_or("category-gap"),
                &format!(
                    "Underrepresented categories (appear in <30% of papers): {}. \
                    These sub-fields may need more investigation.",
                    cats.join(", ")
                ),
                "medium",
            );
            gaps.push(gap);
        }
    }

    // ============================================================
    // GAP 3: Recent papers citing older work (temporal gap)
    // ============================================================
    use chrono::Utc;
    let now = Utc::now();
    let recent_papers: Vec<_> = papers
        .iter()
        .filter(|p| (now - p.published).num_days() < 365)
        .collect();

    if recent_papers.len() >= 2 && total_papers > 5 {
        // Check if recent papers mostly cite old work
        let gap = ResearchGap::new_simple(
            category.as_deref().unwrap_or("temporal"),
            &format!(
                "Recent work ({} papers <1yr old) may not fully incorporate latest advances. \
                Check if recent papers cite papers from the last 2 years.",
                recent_papers.len()
            ),
            "low",
        );
        gaps.push(gap);
    }

    // ============================================================
    // GAP 4: Coverage gap (insufficient papers)
    // ============================================================
    if total_papers < 10 {
        let gap = ResearchGap::new_simple(
            category.as_deref().unwrap_or("coverage"),
            &format!(
                "Insufficient coverage of '{}' - only {} papers found. \
                This area may be nascent or need broader search terms.",
                topic, total_papers
            ),
            "high",
        );
        gaps.push(gap);
    }

    // ============================================================
    // GAP 5: Method diversity gap (check if papers use similar methods)
    // ============================================================
    let method_keywords = [
        "rl",
        "reinforcement",
        "supervised",
        "unsupervised",
        "reinforcement learning",
        "neural",
        "transformer",
        "diffusion",
        "gcn",
        "attention",
        "gan",
        "bayesian",
        "optimization",
        "gradient",
        "supervised learning",
    ];
    let method_counts: Vec<(&str, usize)> = method_keywords
        .iter()
        .filter_map(|m| {
            let count = keyword_counts.get(*m).copied().unwrap_or(0);
            if count > 0 {
                Some((*m, count))
            } else {
                None
            }
        })
        .collect();

    if !method_counts.is_empty() && method_counts.len() <= 2 && total_papers >= 5 {
        let methods: Vec<String> = method_counts
            .iter()
            .map(|(m, _)| format!("\"{}\"", m))
            .collect();
        let gap = ResearchGap::new_simple(
            category.as_deref().unwrap_or("method-diversity"),
            &format!(
                "Limited methodological diversity. Methods detected: {} (only {}/{} known methods found). \
                Consider exploring alternative methodologies.",
                methods.join(", "), method_counts.len(), method_keywords.len()
            ),
            "medium",
        );
        gaps.push(gap);
    }

    // Save gaps to database
    for g in &gaps {
        db.insert_gap(g)?;
    }

    if format == "json" {
        let out: Vec<serde_json::Value> = gaps
            .iter()
            .map(|g| {
                serde_json::json!({
                    "id": g.id,
                    "category": g.category,
                    "description": g.description,
                    "severity": g.severity,
                    "paper_count": g.paper_ids.len(),
                })
            })
            .collect();
        println!("{}", serde_json::to_string_pretty(&out)?);
    } else {
        println!("\n=== Detected {} Research Gaps ===\n", gaps.len());
        for (i, gap) in gaps.iter().enumerate() {
            println!("[{}/{}] Gap: {}", i + 1, gaps.len(), gap.description);
            println!(
                "       Severity: {} | Category: {}",
                gap.severity, gap.category
            );
            println!("       Related papers: {}", gap.paper_ids.len());
            println!();
        }
    }

    if gaps.is_empty() {
        println!("No significant gaps detected. The field appears well-explored for this topic.");
    } else {
        println!(
            "Note: {} gap(s) saved to database. Use 'rairos gap-list' to view.",
            gaps.len()
        );
    }
    Ok(())
}

pub fn handle_gap_list(db: &Database, limit: usize, offset: usize, format: &str) -> Result<()> {
    let gaps = db.list_gaps(limit, offset)?;

    if gaps.is_empty() {
        println!("No research gaps found. Run 'rairos gap --topic <query>' to detect gaps.");
        return Ok(());
    }

    if format == "json" {
        let out: Vec<serde_json::Value> = gaps
            .iter()
            .map(|g| {
                serde_json::json!({
                    "id": g.id,
                    "category": g.category,
                    "description": g.description,
                    "severity": g.severity,
                    "paper_count": g.paper_ids.len(),
                })
            })
            .collect();
        println!("{}", serde_json::to_string_pretty(&out)?);
    } else {
        println!("\n=== Research Gaps ({}) ===\n", gaps.len());
        println!(
            "{:<36} {:<10} {:<8} {}",
            "ID", "CATEGORY", "SEVERITY", "DESCRIPTION"
        );
        println!("{}", "-".repeat(100));
        for gap in &gaps {
            let id_short = if gap.id.len() > 8 {
                &gap.id[..8]
            } else {
                &gap.id
            };
            let desc_short = if gap.description.len() > 60 {
                format!("{}...", &gap.description[..60])
            } else {
                gap.description.clone()
            };
            println!(
                "{:<36} {:<10} {:<8} {}",
                id_short, gap.category, gap.severity, desc_short
            );
        }
        println!();
    }
    Ok(())
}

pub fn handle_gap_show(db: &Database, id: &str) -> Result<()> {
    let gap = db
        .get_gap(id)?
        .ok_or_else(|| anyhow::anyhow!("Gap not found: {}", id))?;

    println!("\n=== Research Gap Details ===\n");
    println!("ID:          {}", gap.id);
    println!("Category:    {}", gap.category);
    println!("Severity:    {}", gap.severity);
    println!("Description: {}", gap.description);
    println!(
        "Paper IDs:   {} ({} total)",
        gap.paper_ids.join(", "),
        gap.paper_ids.len()
    );
    println!();

    // Show related papers
    if !gap.paper_ids.is_empty() {
        println!("Related Papers:");
        for pid in gap.paper_ids.iter().take(5) {
            if let Ok(paper) = db.get_paper(pid) {
                let title = if paper.title.len() > 60 {
                    format!("{}...", &paper.title[..60])
                } else {
                    paper.title
                };
                println!("  - {} | {}", &pid[..8.min(pid.len())], title);
            }
        }
    }
    Ok(())
}

pub fn handle_gap_delete(db: &Database, id: &str) -> Result<()> {
    db.delete_gap(id)?;
    println!("Deleted gap: {}", id);
    Ok(())
}
