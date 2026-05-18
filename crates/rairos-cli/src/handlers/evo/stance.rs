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
use rairos_memory::{ResearchMemory, ResearchStance, StanceType};

pub fn handle_stance_add(topic: &str, claim: &str, stance: &str, reasoning: &str) -> Result<()> {
    let stance_type = match stance.to_lowercase().as_str() {
        "supported" => StanceType::Supported,
        "rejected" => StanceType::Rejected,
        "deferred" => StanceType::Deferred,
        "qualified" => StanceType::Qualified,
        _ => anyhow::bail!(
            "Invalid stance: {}. Use: supported, rejected, deferred, qualified",
            stance
        ),
    };

    let mut memory = ResearchMemory::load().context("Failed to load research memory")?;
    let new_stance = ResearchStance::new(topic, claim, stance_type, reasoning);
    memory.add_stance(new_stance);
    memory.save().context("Failed to save research memory")?;
    println!("[OK] Stance added");
    Ok(())
}

pub fn handle_stance_list(topic: Option<String>, tag: Option<String>, format: &str) -> Result<()> {
    let memory = ResearchMemory::load().context("Failed to load research memory")?;

    let stances: Vec<&ResearchStance> = if let Some(ref t) = topic {
        memory.find_by_topic(t)
    } else if let Some(ref t) = tag {
        memory.find_by_tag(t)
    } else {
        memory.stances().iter().collect()
    };

    if format == "json" {
        let out: Vec<serde_json::Value> = stances
            .iter()
            .map(|s| {
                serde_json::json!({
                    "stance_id": s.stance_id,
                    "topic": s.topic,
                    "claim": s.claim,
                    "stance": s.stance.to_string(),
                    "confidence": s.confidence,
                    "tags": s.tags,
                })
            })
            .collect();
        println!("{}", serde_json::to_string_pretty(&out)?);
        return Ok(());
    }

    println!("=== Research Stances ({} found) ===\n", stances.len());
    println!(
        "{:<38} {:<20} {:<15} {:<10}",
        "ID", "TOPIC", "STANCE", "CONFIDENCE"
    );
    println!("{}", "-".repeat(85));
    for s in stances {
        let id_short = if s.stance_id.len() > 8 {
            &s.stance_id[..8]
        } else {
            &s.stance_id
        };
        println!(
            "{:<38} {:<20} {:<15} {:.2}",
            id_short,
            &s.topic[..20.min(s.topic.len())],
            s.stance,
            s.confidence
        );
    }
    Ok(())
}

pub fn handle_stance_show(id: &str, format: &str) -> Result<()> {
    let memory = ResearchMemory::load().context("Failed to load research memory")?;

    let stance = memory.get_stance(id).or_else(|| {
        memory
            .stances()
            .iter()
            .find(|s| s.stance_id.starts_with(id))
    });

    if let Some(s) = stance {
        if format == "json" {
            println!("{}", serde_json::to_string_pretty(s)?);
            return Ok(());
        }
        println!("=== Stance Details ===\n");
        println!("ID:         {}", s.stance_id);
        println!("Topic:      {}", s.topic);
        println!("Claim:      {}", s.claim);
        println!("Stance:     {}", s.stance);
        println!("Confidence: {:.2}", s.confidence);
        println!("Reasoning: {}", s.reasoning);
        println!("Tags:      {:?}", s.tags);
        println!("Evidence:  {:?}", s.evidence_refs);
        println!("Created:   {}", s.created_at);
        println!("Updated:   {}", s.updated_at);

        let anomalies = memory.get_anomalies_by_stance(&s.stance_id);
        if !anomalies.is_empty() {
            println!("\n=== Anomalies ({} found) ===", anomalies.len());
            for a in anomalies {
                println!(
                    "  - [{}] {} ({})",
                    format!("{:?}", a.severity),
                    a.paper_title,
                    a.anomaly_type
                );
            }
        }
    } else {
        anyhow::bail!("Stance not found: {}", id);
    }
    Ok(())
}