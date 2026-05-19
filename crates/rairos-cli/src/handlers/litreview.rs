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

pub fn handle_litreview(db: &Database, topic: Option<&str>, limit: usize, _format: &str) -> Result<()> {
    let topic_str = topic.unwrap_or("machine learning");
    let papers = db.search_papers_smart(topic_str, limit)?;
    let rust_papers: Vec<rairos_litreview_analyzer::Paper> = papers
        .iter()
        .map(|p| rairos_litreview_analyzer::Paper {
            id: Some(p.id.clone()),
            arxiv_id: p.arxiv_id.clone(),
            title: Some(p.title.clone()),
            abstract_text: Some(p.abstract_text.clone()),
            published: Some(p.published.to_rfc3339()),
            score: 0.0,
            categories: p.categories.clone(),
        })
        .collect();

    let analyzer = rairos_litreview_analyzer::LitReviewAnalyzer::new();
    println!("📚 Literature Analysis for: {}", topic_str);
    println!("   Papers analyzed: {}", rust_papers.len());

    let trends = analyzer.analyze_trends(&rust_papers);
    println!("   Trends: {:?}", trends);

    let controversies = analyzer.find_controversies(&rust_papers);
    if !controversies.is_empty() {
        println!("   Controversies:");
        for c in &controversies {
            println!("     • {}", c);
        }
    }

    let problems = analyzer.extract_open_problems(&rust_papers);
    if !problems.is_empty() {
        println!("   Open Problems:");
        for p in &problems {
            println!("     • {}", p);
        }
    }

    Ok(())
}

pub fn handle_report(format: &str) -> Result<()> {
    let report = rairos_evolution_report::generate_evolution_report(7);
    if format == "json" {
        println!("{}", serde_json::to_string_pretty(&report)?);
    } else {
        println!("{}", report.to_markdown());
    }
    Ok(())
}

pub fn handle_research(_db: &Database, action: &str, content: Option<&str>) -> Result<()> {
    match action {
        "list" => {
            let notes = rairos_research_log::get_notes(None, 50);
            if notes.is_empty() {
                println!("No research notes found.");
            } else {
                for note in &notes {
                    println!("[{}] {} — {}", note.paper_id, note.timestamp, &note.note[..note.note.len().min(80)]);
                }
            }
        }
        "add" => {
            let Some(c) = content else {
                eprintln!("Usage: research add --content <note>");
                std::process::exit(1);
            };
            if rairos_research_log::add_note("", c, None) {
                println!("✓ Note added");
            } else {
                eprintln!("Failed to add note");
            }
        }
        _ => eprintln!("Unknown action: {}. Use: list, add", action),
    }
    Ok(())
}

pub fn handle_digest(weeks: usize) -> Result<()> {
    let _digest = rairos_weekly_digest::WeeklyDigest::new();
    println!("📊 Weekly Digest");
    println!("   Period: {} weeks", weeks);
    println!("   (Full digest requires journal/experiment data)");
    Ok(())
}

pub fn handle_trace(db: &Database, arxiv_id: Option<&str>, list: bool, show_refs: bool, limit: usize) -> Result<()> {
    if list || arxiv_id.is_none() {
        let traces = db.list_paper_code_traces(limit as i64)?;
        if traces.is_empty() {
            println!("ℹ️  No traces found.");
            return Ok(());
        }

        println!("✅ Recent paper-code traces ({}):", traces.len());
        println!();
        for t in &traces {
            let _title = &t.paper_id;
            let coverage = if t.total_code_lines > 0 {
                format!("{}/{}", t.tagged_lines, t.total_code_lines)
            } else {
                "N/A".to_string()
            };
            let pr = t.benchmark_pass_rate.map(|r| format!("{:.0}%", r * 100.0)).unwrap_or_else(|| "—".to_string());
            println!(
                "  [\x1b[36m{}\x1b[0m]\n    module={}  framework={}\n    coverage={} lines  pass_rate={}  created={}",
                t.paper_id, t.module_name, t.framework, coverage, pr, &t.created_at[..10.min(t.created_at.len())]
            );
            if show_refs && !t.paper_section_refs.is_empty() {
                for ref_item in t.paper_section_refs.iter().take(5) {
                    let source_ref = ref_item.get("source_ref").and_then(|v| v.as_str()).unwrap_or("");
                    let code_range = ref_item.get("code_range").and_then(|v| v.as_str()).unwrap_or("");
                    let paper_text = ref_item.get("paper_text").and_then(|v| v.as_str()).unwrap_or("");
                    let text_short: String = paper_text.chars().take(60).collect();
                    println!("    {} → line {}: {}", source_ref, code_range, text_short);
                }
            }
            println!();
        }
        return Ok(());
    }

    let pid = arxiv_id.unwrap();
    let traces = db.get_paper_code_trace(pid)?;
    if traces.is_empty() {
        eprintln!("❌ No traces found for paper {}.", pid);
        return Ok(());
    }

    println!("✅ Traces for \x1b[36m{}\x1b[0m ({}):", pid, traces.len());
    println!();
    for (i, t) in traces.iter().enumerate() {
        let coverage = if t.total_code_lines > 0 {
            format!("{}/{}", t.tagged_lines, t.total_code_lines)
        } else {
            "N/A".to_string()
        };
        let pr = t.benchmark_pass_rate.map(|r| format!("{:.0}%", r * 100.0)).unwrap_or_else(|| "—".to_string());

        println!(
            "Trace #{}  module={}  framework={}\n  code_path: {}\n  coverage: {} lines tagged\n  pass_rate: {}  |  untagged ranges: {}  |  unreferenced: {}\n  created: {}",
            i + 1, t.module_name, t.framework, t.code_path, coverage, pr,
            t.untagged_ranges.len(), t.unreferenced_sources.len(), t.created_at
        );

        if show_refs && !t.paper_section_refs.is_empty() {
            println!("  Provenance refs ({}):", t.paper_section_refs.len());
            for ref_item in &t.paper_section_refs {
                let text = ref_item.get("paper_text").and_then(|v| v.as_str()).unwrap_or("");
                let text_short: String = text.chars().take(55).collect();
                let rng = ref_item.get("code_range").and_then(|v| v.as_str()).unwrap_or("");
                let rng_str = if rng.is_empty() { "?".to_string() } else { format!("L{}", rng) };
                let source_ref = ref_item.get("source_ref").and_then(|v| v.as_str()).unwrap_or("");
                println!("    {} → {}: {}", source_ref, rng_str, text_short);
            }
        } else if show_refs {
            println!("  No provenance refs (code may not have # source: comments)");
        }
        println!();
    }

    // Summary stats
    let total_lines: i64 = traces.iter().map(|t| t.total_code_lines).sum();
    let total_tagged: i64 = traces.iter().map(|t| t.tagged_lines).sum();
    if total_lines > 0 {
        let avg_cov = (total_tagged as f64 / total_lines as f64) * 100.0;
        println!("ℹ️  Summary: {}/{} lines traced ({:.1}%) across {} trace(s)", total_tagged, total_lines, avg_cov, traces.len());
    }

    Ok(())
}

pub fn handle_review(db: &Database, action: &str, paper: Option<&str>, _content: Option<&str>) -> Result<()> {
    match action {
        "list" => {
            let papers = db.search_papers("", 20)?;
            println!("📚 Papers available for review:");
            for p in &papers {
                let title_preview = if p.title.len() > 60 {
                    format!("{}...", &p.title[..57])
                } else {
                    p.title.clone()
                };
                println!("  [{}] {}", p.id, title_preview);
            }
        }
        "add" => {
            let Some(pid) = paper else {
                eprintln!("Usage: review add --paper <paper_id>");
                std::process::exit(1);
            };
            println!("📝 Review mode for paper [{}]", pid);
            println!("(Full review generation requires LLM integration)");
        }
        _ => eprintln!("Unknown action: {}. Use: list, add", action),
    }
    Ok(())
}

pub fn handle_replicate(db: &Database, paper_id: &str) -> Result<()> {
    let checker = rairos_replication::ReplicationChecker::new();
    let papers = db.search_papers_smart(paper_id, 1)?;

    if let Some(paper) = papers.into_iter().next() {
        let full_text = db.get_paper_plain_text(&paper.id)?.unwrap_or_default();
        let report = checker.check_paper(&paper.id, &paper.title, &paper.abstract_text, &full_text);

        println!("🔬 Replication Check for: {}", paper.title);
        println!("   Paper: [{}]", paper.id);

        if report.links.is_empty() {
            println!("   Code links: None found");
        } else {
            println!("   Code links found: {}", report.links.len());
            if let Some(link) = &report.primary_link {
                println!("   Primary: {} ({})", link.url, link.platform);
            }
        }

        println!("   Difficulty: {}", report.difficulty);

        if !report.notes.is_empty() {
            println!("   Notes:");
            for note in &report.notes {
                println!("     - {}", note);
            }
        }

        if !report.reproducibility_issues.is_empty() {
            println!("   Issues:");
            for issue in &report.reproducibility_issues {
                println!("     ⚠ {}", issue);
            }
        }
    } else {
        eprintln!("Paper not found: {}", paper_id);
    }
    Ok(())
}
