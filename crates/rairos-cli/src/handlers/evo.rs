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
use rairos_core::RateLimiter;
use rairos_llm::{Capsule, CapsuleStatus, GenePool, GenePoolDiversityCalculator};
use rairos_memory::{ResearchMemory, ResearchStance, StanceType};
use std::time::{Duration, Instant};

use crate::InsightAction;


// ====================================================================
// Handler implementations
// ====================================================================

// ============================================================================
// Command Handlers
// ============================================================================

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
    let gaps = vec!["capability", "improvement", "reasoning"];
    let mut suggestions = Vec::new();
    for gap_type in &gaps {
        let pairs = pool.suggest_crossover(gap_type, max_crossovers / gaps.len());
        for (id1, id2) in pairs {
            suggestions.push((gap_type.to_string(), id1, id2));
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

pub fn handle_memory_stats(format: &str) -> Result<()> {
    let memory = ResearchMemory::load().context("Failed to load research memory")?;
    let stats = memory.stats();

    if format == "json" {
        println!("{}", serde_json::to_string_pretty(&stats)?);
        return Ok(());
    }

    println!("=== Research Memory Stats ===\n");
    println!("Total Stances:  {}", stats.total_stances);
    println!("Total Anomalies: {}", stats.total_anomalies);
    println!("\nBy Stance:");
    for (stance, count) in &stats.by_stance {
        println!("  {}: {}", stance, count);
    }
    if !stats.by_severity.is_empty() {
        println!("\nBy Severity:");
        for (sev, count) in &stats.by_severity {
            println!("  {}: {}", sev, count);
        }
    }
    Ok(())
}

pub fn handle_rate_limit_benchmark(count: usize) -> Result<()> {
    let limiter = RateLimiter::new();
    let handle = limiter.get_or_create("benchmark");
    handle.reset();

    let start = Instant::now();
    let mut allowed = 0usize;
    let mut waited = 0usize;
    let mut total_wait = Duration::ZERO;

    for _ in 0..count {
        if handle.can() {
            allowed += 1;
        } else {
            waited += 1;
            let wait_start = Instant::now();
            handle.wait_for_slot();
            total_wait += wait_start.elapsed();
        }
    }

    let elapsed = start.elapsed();
    println!("=== Rate Limiter Benchmark ===");
    println!("Total requests:  {}", count);
    println!("Allowed:         {}", allowed);
    println!("Waited:          {}", waited);
    println!("Total wait time: {:.3}s", total_wait.as_secs_f64());
    println!(
        "Throughput:      {:.0} req/s",
        count as f64 / elapsed.as_secs_f64()
    );
    Ok(())
}

pub fn handle_rate_limit_check(endpoint: &str) -> Result<()> {
    let limiter = RateLimiter::new();
    let handle = limiter.get_or_create(endpoint);

    println!("=== Rate Limit Status: {} ===", endpoint);
    println!("Available: {}", handle.can());
    if !handle.can() {
        println!("(wait_for_slot not shown — would block)");
    }
    Ok(())
}

pub fn handle_insight(action: &InsightAction) -> Result<()> {
    use rairos_llm::insight::cards::InsightManager;

    let manager = InsightManager::new(None);

    match action {
        InsightAction::Add {
            content,
            r#type,
            tags,
            paper,
            collection,
        } => {
            let tag_list: Option<Vec<String>> = tags
                .as_ref()
                .map(|t| t.split(',').map(|s| s.trim().to_string()).collect());
            let card = manager.add_card(
                paper.as_deref().unwrap_or(""),
                "",
                content,
                r#type,
                tag_list,
                "",
                "",
            );
            println!("  ✓ Created insight card [{}]: {}", card.card_id, &card.content[..card.content.len().min(60)]);
            if let Some(cid) = collection {
                let _ = manager.add_to_collection(cid, &card.card_id);
                println!("     Added to collection [{}]", cid);
            }
        }

        InsightAction::List { limit } => {
            let cards = manager.search_cards(None, None, None, None);
            if cards.is_empty() {
                println!("  No insight cards found.");
                return Ok(());
            }
            let shown = cards.iter().take(*limit);
            println!("  Insight cards ({} shown / {} total):", shown.clone().count(), cards.len());
            for card in shown {
                let rating = if card.times_rated > 0 {
                    format!("{}★", card.quality_rating)
                } else {
                    "-".to_string()
                };
                let tags_str = if card.tags.is_empty() {
                    String::new()
                } else {
                    format!(" [{}]", card.tags.join(", "))
                };
                println!("  [{}] {} {}{}", card.card_id, rating, card.content, tags_str);
                println!("       Type: {} | Paper: {} | Created: {}", card.insight_type, card.paper_id, card.created_at);
            }
        }

        InsightAction::Search { query, r#type } => {
            let cards = manager.search_cards(Some(query), None, r#type.as_deref(), None);
            if cards.is_empty() {
                println!("  No matching insight cards found.");
                return Ok(());
            }
            println!("  Found {} card(s):", cards.len());
            for card in &cards {
                let rating = if card.times_rated > 0 {
                    format!("{}★", card.quality_rating)
                } else {
                    "-".to_string()
                };
                println!("  [{}] {} | {}", card.card_id, rating, card.content);
            }
        }

        InsightAction::TagCloud => {
            let cloud = manager.get_tag_cloud();
            if cloud.is_empty() {
                println!("  No tags found.");
                return Ok(());
            }
            let mut tags: Vec<(&String, &i32)> = cloud.iter().collect();
            tags.sort_by(|a, b| b.1.cmp(a.1));
            println!("  Tag Cloud:");
            for (tag, count) in &tags {
                let bar = "█".repeat(**count as usize);
                println!("    {} {} ({})", bar, tag, count);
            }
        }

        InsightAction::Rate { card, stars } => {
            let s: i32 = (*stars).clamp(1, 5);
            let ok = manager.rate_card(card, s);
            if ok {
                println!("  ✓ Rated card [{}] with {}★", card, stars);
            } else {
                println!("  ✗ Card [{}] not found.", card);
            }
        }

        InsightAction::Like { card } => {
            let ok = manager.like_card(card);
            if ok {
                println!("  ✓ Liked card [{}]", card);
            } else {
                println!("  ✗ Card [{}] not found.", card);
            }
        }

        InsightAction::Dislike { card } => {
            let ok = manager.dislike_card(card);
            if ok {
                println!("  ✓ Disliked card [{}]", card);
            } else {
                println!("  ✗ Card [{}] not found.", card);
            }
        }

        InsightAction::Top { min_rating, limit } => {
            let cards = manager.get_high_quality_cards(*min_rating, 1);
            if cards.is_empty() {
                println!("  No high-quality cards found (min rating: {}).", min_rating);
                return Ok(());
            }
            let shown = cards.iter().take(*limit);
            println!("  Top insight cards (min {}★, showing {}):", min_rating, shown.clone().count());
            for card in shown {
                println!("  [{:.4}] [{}] {}", card.usefulness_score, card.card_id, card.content);
            }
        }

        InsightAction::Bottom { max_rating, limit } => {
            let cards = manager.get_low_quality_cards(*max_rating, 0);
            if cards.is_empty() {
                println!("  No low-quality cards found (max rating: {}).", max_rating);
                return Ok(());
            }
            let shown = cards.iter().take(*limit);
            println!("  Bottom insight cards (max {}★, showing {}):", max_rating, shown.clone().count());
            for card in shown {
                println!("  [{:.4}] [{}] {}", card.usefulness_score, card.card_id, card.content);
            }
        }
    }

    Ok(())
}

pub fn handle_signal(keyword: &str) -> Result<()> {
    let report = crate::signal::signal(keyword);
    println!("{}", crate::signal::render_signal(&report));
    Ok(())
}

pub fn handle_experiment(
    action: &str,
    name: Option<&str>,
    desc: Option<&str>,
    milestone: Option<&str>,
    tag: Vec<String>,
    id: Option<&str>,
    metrics: Option<&str>,
    metric_name: Option<&str>,
    metric_value: Option<f64>,
    unit: &str,
    ids: Vec<String>,
    result: Option<&str>,
) -> Result<()> {
    let tracker = rairos_experiment_tracker::ExperimentTracker::new(None);

    match action {
        "list" => {
            let exps = tracker.list_experiments(None, milestone, None);
            for e in &exps {
                println!("[{}] {} — {}", e.id, e.name, e.status);
            }
        }
        "run" => {
            let n = name.unwrap_or("unnamed");
            let e = tracker.run(n, desc.unwrap_or(""), milestone.unwrap_or(""), "", None, if tag.is_empty() { None } else { Some(tag.clone()) });
            println!("⚡ Started experiment [{}]: {}", e.id, e.name);
        }
        "get" => {
            let Some(eid) = id else {
                eprintln!("Usage: experiment get --id <id>");
                return Ok(());
            };
            match tracker.get(eid) {
                Some(e) => {
                    println!("Experiment: {}", e.name);
                    println!("ID: {}", e.id);
                    println!("Status: {}", e.status);
                    println!("Created: {}", e.created_at);
                    if !e.roadmap_milestone.is_empty() {
                        println!("Milestone: {}", e.roadmap_milestone);
                    }
                }
                None => eprintln!("Experiment [{}] not found", eid),
            }
        }
        "complete" => {
            let Some(eid) = id else {
                eprintln!("Usage: experiment complete --id <id>");
                return Ok(());
            };
            let results: Option<std::collections::HashMap<String, serde_json::Value>> = metrics.and_then(|m| serde_json::from_str(m).ok());
            match tracker.complete(eid, results) {
                Some(e) => println!("✓ Completed [{}]: {}", e.id, e.name),
                None => eprintln!("Experiment [{}] not found", eid),
            }
        }
        "metric" => {
            let Some(eid) = id else {
                eprintln!("Usage: experiment metric --id <id> --metric-name <name> --metric-value <value>");
                return Ok(());
            };
            let Some(mn) = metric_name else {
                eprintln!("Missing --metric-name");
                return Ok(());
            };
            let mv = metric_value.unwrap_or(0.0);
            match tracker.add_metric(eid, mn, mv, unit) {
                Some(e) => println!("✓ Added metric {}{} to [{}]", mn, if unit.is_empty() { format!("={}", mv) } else { format!("={}{}", mv, unit) }, e.id),
                None => eprintln!("Experiment [{}] not found", eid),
            }
        }
        "compare" => {
            let comp = tracker.compare(&ids, None);
            println!("Comparison (JSON):");
            println!("{}", serde_json::to_string_pretty(&comp)?);
        }
        "delete" => {
            let Some(eid) = id else {
                eprintln!("Usage: experiment delete --id <id>");
                return Ok(());
            };
            if tracker.delete(eid) {
                println!("✓ Deleted [{}]", eid);
            } else {
                eprintln!("Experiment [{}] not found", eid);
            }
        }
        "simulate" => {
            let Some(eid) = id else {
                eprintln!("Usage: experiment simulate --id <id> --result success|fail");
                return Ok(());
            };
            let e = tracker.get(eid);
            match e {
                Some(e_ref) => {
                    if e_ref.status != "running" {
                        eprintln!("Experiment [{}] is not running (status: {})", eid, e_ref.status);
                        return Ok(());
                    }
                    match result {
                        Some("success") => {
                            let _ = tracker.complete(eid, {
                        let mut m = std::collections::HashMap::new();
                        m.insert("simulated".to_string(), serde_json::json!(true));
                        m.insert("outcome".to_string(), serde_json::json!("success"));
                        Some(m)
                    });
                            println!("✅ Simulated success for [{}]: {}", eid, e_ref.name);
                        }
                        Some("fail") => {
                            let _ = tracker.fail(eid, "simulated failure");
                            println!("❌ Simulated failure for [{}]: {}", eid, e_ref.name);
                        }
                        _ => eprintln!("Result must be 'success' or 'fail'"),
                    }
                    println!("  → VALIDATED/REJECTED event written to evolution tracker");
                }
                None => eprintln!("Experiment [{}] not found", eid),
            }
        }
        _ => eprintln!("Unknown action: {}. Use: list, run, get, complete, metric, compare, delete, simulate", action),
    }
    Ok(())
}

pub fn handle_evolution(
    show_stats: bool,
    show_patterns: bool,
    show_feedback: bool,
    show_report: bool,
    show_sessions: bool,
    _days: usize,
    clear: bool,
    export: bool,
) -> Result<()> {
    let evo = rairos_evolution::get_evolution_memory();

    if clear {
        println!("Clear not implemented in Rust CLI — use Python: rairos evolution --clear");
        return Ok(());
    }

    if export {
        let stats = evo.get_stats();
        println!("{}", serde_json::to_string_pretty(&stats)?);
        return Ok(());
    }

    if show_report {
        let stats = evo.get_stats();
        println!();
        println!("  Evolution Report");
        println!();
        for (key, value) in &stats {
            println!("  {}: {}", key, value);
        }
        println!();
        return Ok(());
    }

    if show_stats {
        let stats = evo.get_stats();
        println!("Evolution Statistics:");
        for (key, value) in &stats {
            println!("  {}: {}", key, value);
        }
        return Ok(());
    }

    if show_patterns {
        let patterns = evo.get_all_patterns();
        println!("Learned Patterns ({}):", patterns.len());
        for p in &patterns {
            println!("  - {} (effectiveness: {})", p.name, p.effectiveness);
        }
        return Ok(());
    }

    if show_feedback {
        println!("Recent feedback: (check Python CLI for details)");
        return Ok(());
    }

    if show_sessions {
        println!("Research sessions: (check Python CLI for details)");
        return Ok(());
    }

    // Default: show dashboard summary
    let stats = evo.get_stats();
    println!();
    println!("  Evolution Dashboard");
    println!();
    for (key, value) in &stats {
        println!("  {}: {}", key, value);
    }
    println!();
    println!("  Tips: --stats, --patterns, --feedback, --report, --export");
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

