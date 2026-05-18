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

pub fn handle_friction(friction_type: Option<&str>, days: usize, json: bool, limit: usize) -> Result<()> {
    let tracker = rairos_friction::FrictionTracker::new(None);
    let ftype = friction_type.and_then(|s| s.parse::<rairos_friction::FrictionType>().ok());
    let summary = tracker.get_summary(days as i32);
    let events = tracker.get_events(ftype, days as i32, limit);

    if json {
        use std::collections::HashMap;
        let mut output = serde_json::Map::new();
        output.insert("total_events".into(), serde_json::json!(summary.total_events));
        output.insert("abandon_rate".into(), serde_json::json!(summary.abandon_rate));
        let by_type: HashMap<_, _> = summary.by_type.into_iter().collect();
        output.insert("by_type".into(), serde_json::json!(by_type));
        output.insert("events".into(), serde_json::json!(&events));
        println!("{}", serde_json::to_string_pretty(&output)?);
        return Ok(());
    }

    println!();
    println!("  Research Friction Report");
    println!("  Last {} days", days);
    println!();

    if summary.total_events == 0 {
        println!("No friction events recorded yet.");
        return Ok(());
    }

    println!("Total events: {}", summary.total_events);
    println!("  Abandon rate: {:.1}%", summary.abandon_rate * 100.0);
    println!();

    if !summary.by_type.is_empty() {
        println!("By Type:");
        let mut by_type: Vec<_> = summary.by_type.into_iter().collect();
        by_type.sort_by(|a, b| b.1.cmp(&a.1));
        for (t, count) in &by_type {
            let bar = "█".repeat((*count as usize).min(30));
            println!("  {:<12} {} {}", t, bar, count);
        }
        println!();
    }

    if !summary.top_commands.is_empty() {
        println!("Top Friction Commands:");
        for (cmd, count) in &summary.top_commands {
            println!("  {:<20} {} events", cmd, count);
        }
        println!();
    }

    if !events.is_empty() {
        println!("Recent Events (last {}):", events.len().min(limit));
        for e in events.iter().take(limit) {
            let ts = if e.timestamp.len() >= 10 { &e.timestamp[..10] } else { &e.timestamp };
            let status = if e.abandoned { " [ABANDONED]" } else { "" };
            let note_preview = if e.error.len() > 40 { &e.error[..40] } else { &e.error };
            println!("  {}  {:<12} {:<15} {}{}", ts, e.friction_type, e.command, note_preview, status);
        }
    }

    println!();
    Ok(())
}

pub fn handle_hypothesize(
    topic: Option<&str>,
    gap: &str,
    _trend: &str,
    _story: &str,
    no_llm: bool,
    creative: bool,
    json: bool,
    _model: Option<&str>,
    top: usize,
) -> Result<()> {
    let gen = rairos_research::hypothesis_generator::HypothesisGenerator::new();
    let topic_str = topic.unwrap_or("machine learning");

    if no_llm {
        // Template-only generation (sync)
        let result = gen.generate(topic_str, gap, creative);
        if json {
            println!("{}", serde_json::to_string_pretty(&result)?);
        } else {
            println!("Topic: {}", result.topic);
            println!("Summary: {}", result.summary);
            for (i, h) in result.hypotheses.iter().enumerate() {
                println!("  {}. {} (score: {:.2})", i + 1, h.title, h.novelty_score);
                println!("     {}", h.core_statement);
                if let Some(risk) = &h.risk {
                    println!("     Risk: technical={}, hypothesis={}", risk.technical, risk.hypothesis);
                }
            }
        }
    } else {
        println!("🧬 Generating hypotheses for: {}", topic_str);
        println!("    (LLM-enhanced generation not wired in Rust CLI yet — using template mode)");
        let result = gen.generate(topic_str, gap, creative);
        if json {
            println!("{}", serde_json::to_string_pretty(&result)?);
        } else {
            println!("Topic: {}", result.topic);
            println!("Summary: {}", result.summary);
            for (i, h) in result.hypotheses.iter().take(top).enumerate() {
                println!("  {}. {} (novelty: {:.2}, feasibility: {:.2})", i + 1, h.title, h.novelty_score, h.feasibility_score);
                println!("     {}", h.core_statement);
                let exp_design = &h.experiment_design;
                println!("     Baseline: {}", exp_design.baseline);
            }
        }
    }

    Ok(())
}

pub fn handle_lean(file: Option<&str>, hypothesis: Option<&str>, install: bool, check: bool, json: bool) -> Result<()> {
    if install {
        println!("{}", rairos_lean_verifier::get_lean_install_instructions());
        return Ok(());
    }

    if check {
        let (status, msg) = rairos_lean_verifier::check_lean_installed();
        let msg_str = msg.as_deref().unwrap_or("");
        if json {
            println!("{}", serde_json::json!({
                "installed": matches!(status, rairos_lean_verifier::LeanInstallStatus::Available),
                "message": msg_str
            }));
        } else {
            match status {
                rairos_lean_verifier::LeanInstallStatus::Available => println!("✅ Lean 4 is available"),
                _ => println!("❌ Lean 4 not found: {}", msg_str),
            }
        }
        return Ok(());
    }

    if let Some(h) = hypothesis {
        let (code, _name) = rairos_lean_verifier::translate_hypothesis_to_lean("cli", h, "hypothesis");
        println!("Lean code:\n{}", code);
        return Ok(());
    }

    if let Some(f) = file {
        let content = std::fs::read_to_string(f).unwrap_or_default();
        let result = rairos_lean_verifier::verify_lean_code(&content, "file", f);
        if json {
            println!("{}", rairos_lean_verifier::render_result_json(&result));
        } else {
            println!("{}", rairos_lean_verifier::render_result(&result));
        }
        return Ok(());
    }

    println!("Usage: lean [--check | --install | --hypothesis <text> | <file>]");
    Ok(())
}

pub fn handle_visual(db: &Database, paper: Option<&str>, query: Option<&str>, limit: usize, output: Option<&str>) -> Result<()> {
    if let Some(pid) = paper {
        println!("📊 Generating D3 citation visualization for: {}", pid);

        let graph = rairos_viz::D3ForceGraph::new(Some(db.clone()));
        let d3graph = graph.to_json(Some(vec![pid.to_string()]), None, limit)?;
        let json_str = d3graph.to_json()?;

        if let Some(out) = output {
            std::fs::write(out, &json_str)?;
            println!("✅ Written to {}", out);
        } else {
            println!("{}", json_str);
        }
        return Ok(());
    }

    if let Some(q) = query {
        println!("📊 Searching papers for: {}", q);
        let papers = db.search_papers(q, limit)?;
        println!("Found {} papers", papers.len());
        return Ok(());
    }

    println!("Usage: visual --paper <id> [--output <path>] | visual --query <q>");
    Ok(())
}

pub fn handle_session(action: &str, title: Option<&str>, topic: Option<&str>, days: usize, limit: usize) -> Result<()> {
    let mut tracker = rairos_research_session::ResearchSessionTracker::new(None);

    match action {
        "start" => {
            let session = tracker.start_session(title);
            println!("📚 Session started: {}", session.title);
            println!("   ID: {}", session.id);
            if let Some(t) = topic {
                println!("   Topic: {}", t);
            }
        }
        "list" => {
            let sessions = tracker.get_recent_sessions(days as i64, limit);
            if sessions.is_empty() {
                println!("No sessions found.");
            } else {
                println!("{}", tracker.render_sessions_list(&sessions));
            }
        }
        "current" => {
            match tracker.get_current_session() {
                Some(s) => println!("Current session: {} (ID: {})", s.title, s.id),
                None => println!("No current session."),
            }
        }
        "end" => {
            match tracker.end_session() {
                Some(s) => println!("Ended session: {} (ID: {})", s.title, s.id),
                None => println!("No active session to end."),
            }
        }
        _ => {
            match tracker.get_current_session() {
                Some(s) => println!("Current session: {} (ID: {})", s.title, s.id),
                None => println!("No current session. Use 'session start' to begin one."),
            }
        }
    }

    Ok(())
}
