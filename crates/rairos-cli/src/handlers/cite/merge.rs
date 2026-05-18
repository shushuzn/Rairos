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
use rairos_core::Database;

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
