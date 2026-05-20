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

use crate::InsightAction;

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
            manager.flush(); // O(1) save instead of O(n)
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