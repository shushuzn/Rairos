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
use chrono::Utc;
use rairos_core::{Database, Paper};
use std::collections::HashSet;


pub fn handle_search(
    db: &Database,
    query: &str,
    limit: usize,
    field: &str,
    format: &str,
) -> Result<()> {
    let papers = db.search_papers_smart(query, limit)?;

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

    let mut similarities: Vec<(String, f64, Paper)> = all_papers
        .into_iter()
        .filter(|p| p.id != paper.id)
        .map(|p| {
            let sim = title_similarity(&target_title, &p.title.to_lowercase());
            (p.id.clone(), sim, p)
        })
        .filter(|(_id, sim, _)| *sim > 0.3 && *sim < 1.0)
        .collect();

    similarities.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    similarities.truncate(limit);

    if similarities.is_empty() {
        println!("No similar papers found.");
        return Ok(());
    }

    println!("{} similar papers found:\n", similarities.len());
    for (i, (_id, sim, p)) in similarities.iter().enumerate() {
        println!(
            "{}. {} [{:.2}]",
            i + 1,
            p.title,
            sim
        );
        if let Some(ref arxiv) = p.arxiv_id {
            println!("   arXiv: {}", arxiv);
        }
        println!(
            "   {} | cited_by: {}",
            p.published, p.metadata.cited_by
        );
        println!();
    }

    Ok(())
}

fn title_similarity(a: &str, b: &str) -> f64 {
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

pub fn handle_compare(db: &Database, papers_arg: &str, aspect: &str) -> Result<()> {
    println!("=== Compare Papers ===\n");

    let paper_ids: Vec<&str> = papers_arg.split(',').map(|s| s.trim()).collect();

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
