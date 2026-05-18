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

pub fn handle_dedup(db: &Database, action: &DedupAction) -> Result<()> {
    match action {
        DedupAction::Find { threshold } => {
            println!("=== Finding Duplicate Papers ===");
            println!("Similarity threshold: {:.2}", threshold);
            println!();

            let papers = db.list_papers(None, 1000, 0)?;
            println!("Checking {} papers for duplicates...", papers.len());

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
