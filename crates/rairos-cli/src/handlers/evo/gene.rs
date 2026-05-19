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
use rairos_llm::{Capsule, CapsuleStatus, GenePool, GenePoolDiversityCalculator};


pub fn handle_gene_show(id: &str, format: &str) -> Result<()> {
    let pool = GenePool::load().context("Failed to load gene pool")?;
    if let Some(cap) = pool
        .capsules()
        .iter()
        .find(|c| c.capsule_id == id || c.capsule_id.starts_with(id))
    {
        if format == "json" {
            println!("{}", serde_json::to_string_pretty(cap)?);
            return Ok(());
        }

        println!("=== Gene Details ===\n");
        println!("ID:           {}", cap.capsule_id);
        println!("Gap Type:     {}", cap.action_gap_type);
        println!("Approach:     {}", cap.archetype.approach_summary);
        println!(
            "Status:       {}",
            if cap.archived {
                "archived".to_string()
            } else {
                cap.status.to_string()
            }
        );
        println!("Impact Score: {:.4}", cap.impact_score);
        println!("Success:      {}", cap.success_count);
        println!("Failure:      {}", cap.failure_count);
        println!("Created:      {}", cap.created_at);
        println!("Updated:      {}", cap.updated_at);
        println!("Keywords:     {:?}", cap.trigger_keywords);
        if let Some(ref fp) = cap.archetype.algorithm_fingerprint {
            println!("Fingerprint:  {}", fp);
        }
        if let Some(ref pid) = cap.archetype.source_paper_id {
            println!("Source Paper: {}", pid);
        }
    } else {
        anyhow::bail!("Gene not found: {}", id);
    }
    Ok(())
}

pub fn handle_gene_feedback(id: &str, positive: bool) -> Result<()> {
    let mut pool = GenePool::load().context("Failed to load gene pool")?;
    if let Some(cap) = pool
        .capsules_mut()
        .iter_mut()
        .find(|c| c.capsule_id == id || c.capsule_id.starts_with(id))
    {
        if positive {
            cap.record_success();
            println!("[OK] Recorded positive feedback for {}", id);
        } else {
            cap.record_failure();
            println!("[OK] Recorded negative feedback for {}", id);
        }
        println!("  Success count: {}", cap.success_count);
        println!("  Failure count: {}", cap.failure_count);
        println!("  New impact score: {:.4}", cap.impact_score);
        pool.save().context("Failed to save gene pool")?;
    } else {
        anyhow::bail!("Gene not found: {}", id);
    }
    Ok(())
}

pub fn handle_gene_diversity(format: &str) -> Result<()> {
    let pool = GenePool::load().context("Failed to load gene pool")?;
    let diversity = GenePoolDiversityCalculator::calculate(pool.capsules());

    if format == "json" {
        println!("{}", serde_json::to_string_pretty(&diversity)?);
        return Ok(());
    }

    println!("=== Gene Pool Diversity ===\n");
    println!("Total Capsules:     {}", diversity.capsule_count);
    println!("Shannon Index:      {:.4}", diversity.shannon_index);
    println!("Shannon Normalized:  {:.4}", diversity.shannon_normalized);
    println!("Diversity Score:    {} / 100", diversity.diversity_score);
    println!(
        "Family Coverage:     {:.1}%",
        diversity.family_coverage * 100.0
    );
    println!();

    println!("Family Distribution:");
    let mut families: Vec<_> = diversity.family_counts.iter().collect();
    families.sort_by(|a, b| b.1.cmp(a.1));
    for (fam, count) in families {
        println!("  {:20} {:>4}", fam, count);
    }
    println!();

    if !diversity.underrepresented_families.is_empty() {
        println!(
            "Underrepresented: {:?}",
            diversity.underrepresented_families
        );
    }
    if !diversity.overrepresented_families.is_empty() {
        println!("Overrepresented:  {:?}", diversity.overrepresented_families);
    }
    Ok(())
}

pub fn handle_gene_evolve(max_crossovers: usize, format: &str) -> Result<()> {
    let pool = GenePool::load().context("Failed to load gene pool")?;
    let active: Vec<&Capsule> = pool.active_capsules();
    
    let mut suggestions = Vec::new();
    for i in 0..active.len() {
        for j in (i + 1)..active.len() {
            if suggestions.len() >= max_crossovers {
                break;
            }
            suggestions.push((
                active[i].action_gap_type.clone(),
                active[i].capsule_id.clone(),
                active[j].capsule_id.clone(),
            ));
        }
        if suggestions.len() >= max_crossovers {
            break;
        }
    }

    if format == "json" {
        let out: Vec<serde_json::Value> = suggestions
            .iter()
            .map(|(gt, id1, id2)| {
                serde_json::json!({
                    "gap_type": gt,
                    "parent_1": id1,
                    "parent_2": id2,
                })
            })
            .collect();
        println!("{}", serde_json::to_string_pretty(&out)?);
        return Ok(());
    }

    println!(
        "=== Evolution Suggestions ({} crossovers) ===\n",
        suggestions.len()
    );
    for (i, (gap_type, id1, id2)) in suggestions.iter().enumerate() {
        println!(
            "{}. {} × {} -> {}",
            i + 1,
            &id1[..8.min(id1.len())],
            &id2[..8.min(id2.len())],
            gap_type
        );
    }
    Ok(())
}

pub fn handle_gene_add(
    approach: &str,
    gap_type: &str,
    keywords: &str,
    paper_id: Option<String>,
) -> Result<()> {
    let keywords: Vec<String> = keywords.split(',').map(|s| s.trim().to_string()).collect();
    let mut capsule = Capsule::new(approach, gap_type, keywords);
    if let Some(pid) = paper_id {
        capsule = capsule.with_paper(&pid);
    }
    let mut pool = GenePool::load().context("Failed to load gene pool")?;
    pool.add_capsule(capsule);
    pool.save().context("Failed to save gene pool")?;

    println!("[OK] Gene added to pool");
    println!(
        "Capsule ID: {}",
        pool.capsules()
            .last()
            .map(|c| c.capsule_id.as_str())
            .unwrap_or("N/A")
    );
    Ok(())
}

pub fn handle_gene_list(
    gap_type: Option<String>,
    status: Option<String>,
    limit: usize,
    format: &str,
) -> Result<()> {
    let pool = GenePool::load().context("Failed to load gene pool")?;
    let all_capsules = pool.capsules();

    let filtered: Vec<&Capsule> = all_capsules
        .iter()
        .filter(|c| {
            if let Some(ref gt) = gap_type {
                if &c.action_gap_type != gt {
                    return false;
                }
            }
            if let Some(ref s) = status {
                let status_match = match s.to_lowercase().as_str() {
                    "active" => c.status == CapsuleStatus::Active && !c.archived,
                    "dormant" => c.status == CapsuleStatus::Dormant,
                    "archived" => c.archived,
                    _ => true,
                };
                if !status_match {
                    return false;
                }
            }
            true
        })
        .take(limit)
        .collect();

    if format == "json" {
        let out: Vec<serde_json::Value> = filtered
            .iter()
            .map(|c| {
                let status = if c.archived {
                    "archived".to_string()
                } else {
                    c.status.to_string()
                };
                serde_json::json!({
                    "capsule_id": c.capsule_id,
                    "gap_type": c.action_gap_type,
                    "approach": c.archetype.approach_summary,
                    "status": status,
                    "impact_score": c.impact_score,
                    "success_count": c.success_count,
                    "failure_count": c.failure_count,
                    "created_at": c.created_at,
                })
            })
            .collect();
        println!("{}", serde_json::to_string_pretty(&out)?);
        return Ok(());
    }

    let count = filtered.len();
    println!("=== Gene Pool ({} capsules) ===\n", count);
    println!(
        "{:<38} {:<15} {:<12} {:>8} {:>8} {:>8}",
        "ID", "GAP_TYPE", "STATUS", "IMPACT", "SUCCESS", "FAILED"
    );
    println!("{}", "-".repeat(95));
    for cap in &filtered {
        let id_short = if cap.capsule_id.len() > 8 {
            &cap.capsule_id[..8]
        } else {
            &cap.capsule_id
        };
        let status_str = if cap.archived {
            "archived".to_string()
        } else {
            cap.status.to_string()
        };
        println!(
            "{:<38} {:<15} {:<12} {:>8.3} {:>8} {:>8}",
            id_short,
            cap.action_gap_type,
            status_str,
            cap.impact_score,
            cap.success_count,
            cap.failure_count
        );
    }
    println!("\n{} capsules shown", count);
    Ok(())
}