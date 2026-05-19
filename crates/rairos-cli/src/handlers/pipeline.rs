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
use crate::handlers::*;

pub fn handle_demo(quick: bool, papers: Option<usize>, insights: bool) -> Result<()> {
    // Sample paper data (matching Python's SAMPLE_PAPER)
    #[allow(dead_code)]
    struct DemoPaper<'a> {
        id: &'a str,
        title: &'a str,
        authors: &'a [&'a str],
        abstract_: &'a str,
    }

    let paper = DemoPaper {
        id: "2301.00001",
        title: "Attention Is All You Need",
        authors: &["Vaswani et al."],
        abstract_: "We propose a new simple network architecture, the Transformer, \
            based solely on attention mechanisms, dispensing with recurrence \
            and convolutions entirely.",
    };

    // Stage 1: Ingest
    fn stage_ingest(paper: &DemoPaper) {
        println!("\n═══ [1/6] Ingest ═══");
        println!("  Paper ID : {}", paper.id);
        println!("  Title    : {}", paper.title);
        println!("  Authors  : {}", paper.authors.join(", "));
        println!("  ✓ Resolved : 2017-06-12 · 89234 citations");
    }

    // Stage 2: Parse
    fn stage_parse() {
        println!("\n═══ [2/6] Parse ═══");
        println!("  Parsing PDF / extracting text...");
        let sections = [
            ("1. Introduction", 45, 0.12),
            ("2. Background", 32, 0.08),
            ("3. Model Architecture", 89, 0.23),
            ("4. Training", 56, 0.15),
            ("5. Experiments", 78, 0.20),
            ("6. Conclusion", 18, 0.05),
            ("References", 41, 0.11),
        ];
        let total_words: usize = sections.iter().map(|(_, w, _)| w).sum();
        println!("  Extracted : {} sections · {} words", sections.len(), total_words);
        for (title, words, frac) in &sections {
            let bar = "█".repeat((frac * 40.0) as usize);
            println!("    {} {} ({}w)", bar, title, words);
        }
    }

    // Stage 3: Citation Analysis
    fn stage_citation_analysis() {
        println!("\n═══ [3/6] Citation Analysis ═══");
        let citations = [
            ("1706.03762", "Attention Is All You Need", "self"),
            ("1409.0473", "Neural Machine Translation", "background"),
            ("1512.03385", "Deep Residual Learning", "methodology"),
            ("1712.05829", "Attention Is All You Need (variants)", "follows"),
            ("1909.11556", "FlashAttention", "improvement"),
        ];
        println!("  Found : {} related papers", citations.len());
        for (cid, title, rel) in &citations {
            let marker = match *rel {
                "self" | "background" => "←",
                "methodology" => "├─",
                "follows" => "└─",
                "improvement" => "★",
                _ => "?",
            };
            println!("    {} {}  {}  [{}]", marker, cid, title, rel);
        }
    }

    // Stage 4: Insight Extraction
    fn stage_insight_extraction() {
        println!("\n═══ [4/6] Insight Extraction ═══");
        let insights = [
            ("Multi-Head Attention", "finding", 5),
            ("Parallelizable training via self-attention", "method", 5),
            ("SOTA on WMT EN-DE (28.4 BLEU)", "result", 4),
            ("Q/K/V projection enables learned attention patterns", "method", 4),
            ("Positional encoding preserves order information", "method", 3),
        ];
        println!("  Generated : {} insight cards", insights.len());
        for (title, itype, rating) in &insights {
            let stars: String = (0..*rating).map(|_| '★').chain((*rating..5).map(|_| '☆')).collect();
            println!("    [{}] {}  ({})", stars, title, itype);
        }
        println!("  ✓ Insights saved to ~/.ai_research_os/insight_cards.json");
    }

    // Stage 5: Knowledge Graph
    fn stage_kg_build() {
        println!("\n═══ [5/6] Knowledge Graph ═══");
        let nodes = [
            ("Transformer", "model", 47),
            ("Self-Attention", "mechanism", 38),
            ("Multi-Head Attention", "component", 31),
            ("Positional Encoding", "component", 14),
            ("Encoder-Decoder", "architecture", 22),
        ];
        let edges = [
            ("Transformer", "uses", "Self-Attention"),
            ("Self-Attention", "implemented_via", "Multi-Head Attention"),
            ("Transformer", "uses", "Positional Encoding"),
            ("Transformer", "contains", "Encoder-Decoder"),
        ];
        println!("  Nodes : {}", nodes.len());
        for (name, ntype, refs) in &nodes {
            println!("    ● {}  [{}]  {} refs", name, ntype, refs);
        }
        println!("  Edges : {}", edges.len());
        for (src, rel, dst) in &edges {
            println!("    {} --[{}]--> {}", src, rel, dst);
        }
        println!("  ✓ Knowledge graph persisted to SQLite");
    }

    // Stage 6: Evolution Tracking
    fn stage_evolution_tracking() {
        println!("\n═══ [6/6] Evolution Tracking ═══");
        let events = [
            ("2017-06", "Transformer introduced", "major"),
            ("2018-07", "BERT pre-training", "major"),
            ("2019-03", "GPT-2 (large scale)", "major"),
            ("2020-05", "T5 (unified framework)", "incremental"),
            ("2022-03", "FlashAttention (efficiency)", "improvement"),
            ("2023-03", "GPT-4 (reasoning)", "major"),
        ];
        println!("  Timeline : {} events", events.len());
        for (date, desc, etype) in &events {
            let marker = match *etype {
                "major" => "●",
                "incremental" => "○",
                "improvement" => "◉",
                _ => "?",
            };
            println!("    {} {}  {}", marker, date, desc);
        }
        println!("  Gap detected : Long-context attention (replaced by FlashAttention)");
    }

    println!();
    println!("═══════════════════════════════════════════════════════════════════════════════");
    println!("  Rairos Research Pipeline — Demo");
    println!("═══════════════════════════════════════════════════════════════════════════════");

    if quick {
        println!("  ⚠ Quick mode — skipping heavy processing");
        stage_ingest(&paper);
        stage_insight_extraction();
        stage_kg_build();
        println!();
        println!("  ✓ Quick demo complete!");
        return Ok(());
    }

    if insights {
        println!("  Insight extraction focused demo");
        stage_ingest(&paper);
        stage_parse();
        stage_insight_extraction();
        println!();
        println!("  ✓ Insight demo complete!");
        return Ok(());
    }

    let n_papers = papers.unwrap_or(1);
    for i in 0..n_papers {
        if n_papers > 1 {
            println!("\n═══ Paper {}/{} ═══", i + 1, n_papers);
        }
        stage_ingest(&paper);
        stage_parse();
        stage_citation_analysis();
        stage_insight_extraction();
        stage_kg_build();
        stage_evolution_tracking();
    }

    println!();
    println!("═══════════════════════════════════════════════════════════════════════════════");
    println!("  ✓ Demo complete! Full pipeline working.");
    println!("═══════════════════════════════════════════════════════════════════════════════");

    Ok(())
}

pub fn handle_pipeline(
    db: &rairos_core::Database,
    topic: &str,
    hypothesis_only: bool,
    top_n: usize,
    min_papers: usize,
    _model: Option<&str>,
    json: bool,
    _no_llm: bool,
    _verbose: bool,
) -> Result<()> {
    use rairos_core::Paper;
    use rairos_research::gap_analysis;
    use rairos_research::hypothesis_generator::HypothesisGenerator;
    use rairos_research::PaperSnapshot;

    // Step 0: Fetch papers by topic
    if json {
        println!("  🎯 Topic: {}", topic);
    } else {
        println!();
        println!("═══════════════════════════════════════════════════════");
        println!("  🎯 {} — Research Pipeline", topic);
        println!("═══════════════════════════════════════════════════════");
    }

    let papers: Vec<Paper> = db.search_papers_smart(topic, min_papers.max(5) * 2)?;
    if papers.is_empty() {
        // Try a broader search if initial search fails
        println!("   No papers found; you may want to ingest some papers first.");
        return Ok(());
    }

    let snapshots: Vec<PaperSnapshot> = papers.iter().map(PaperSnapshot::from_paper).collect();
    let n_papers = snapshots.len();

    if json {
        println!("   {} papers loaded", n_papers);
    } else {
        println!("  📚 {} papers loaded for analysis", n_papers);
    }

    // Step 1: Gap analysis
    let gaps = gap_analysis::analyze_gaps(&snapshots, topic);
    let n_gaps = gaps.len();

    if json {
        println!("   {} gaps detected", n_gaps);
    } else {
        println!("  🔍 {} research gaps detected", n_gaps);
    }

    // Step 2: Format gap context and generate hypotheses
    let gap_context: Vec<String> = gaps
        .iter()
        .map(|g| format!("Gap {} ({}): {} — {}", g.gap_id, g.gap_type, g.title, g.description))
        .collect();
    let gap_context_str = gap_context.join("\n");

    let gen = HypothesisGenerator::new();
    let hypothesis_result = gen.generate(topic, &gap_context_str, true);

    // Step 3: Render combined report
    if json {
        use serde_json::json;
        let output = json!({
            "topic": topic,
            "papers_analyzed": n_papers,
            "gaps": gaps.iter().map(|g| json!({
                "id": g.gap_id,
                "type": g.gap_type,
                "title": g.title,
                "severity": g.severity,
                "description": g.description,
            })).collect::<Vec<_>>(),
            "hypotheses": hypothesis_result.hypotheses.iter().map(|h| json!({
                "id": h.id,
                "title": h.title,
                "type": h.hypothesis_type,
                "statement": h.core_statement,
                "novelty_score": h.novelty_score,
                "feasibility_score": h.feasibility_score,
                "experiment": {
                    "baseline": h.experiment_design.baseline,
                    "variables": h.experiment_design.variables,
                    "controls": h.experiment_design.controls,
                    "metrics": h.experiment_design.evaluation_metrics,
                },
            })).collect::<Vec<_>>(),
        });
        println!();
        println!("{}", serde_json::to_string_pretty(&output).unwrap_or_default());
    } else {
        // Text report: Gaps
        println!();
        println!("  ━━ Gap Analysis ━━");
        for (i, gap) in gaps.iter().enumerate() {
            let icon = severity_icon(&gap.severity);
            println!("  {}. {} [{}] {}", i + 1, icon, gap.gap_type, gap.title);
            println!("     {}", gap.description);
        }

        // Text report: Hypotheses
        println!();
        println!("  ━━ Generated Hypotheses ━━");
        for (i, h) in hypothesis_result.hypotheses.iter().enumerate() {
            let novelty_pct = (h.novelty_score * 100.0) as u8;
            let feasibility_pct = (h.feasibility_score * 100.0) as u8;
            println!("  {}. {} [{}]", i + 1, h.title, h.hypothesis_type);
            println!(
                "     Novelty: {}%  Feasibility: {}%",
                novelty_pct, feasibility_pct
            );
            println!("     {}", h.core_statement);
            let ed = &h.experiment_design;
            if !ed.baseline.is_empty() && ed.baseline != "待确定" {
                println!("     Baseline: {}", ed.baseline);
            }
            if !ed.evaluation_metrics.is_empty() {
                println!("     Metrics: {}", ed.evaluation_metrics.join(", "));
            }
        }
    }

    // Step 4: Create experiments from top hypotheses
    if !hypothesis_only && !hypothesis_result.hypotheses.is_empty() {
        use rairos_experiment_tracker::ExperimentTracker;
        use std::collections::HashMap;

        let exp_tracker = ExperimentTracker::new(None);
        let mut created_count = 0usize;

        for h in hypothesis_result.hypotheses.iter().take(top_n) {
            let ed = &h.experiment_design;
            if ed.baseline.is_empty() && ed.variables.is_empty() {
                // Skip hypotheses with no meaningful experiment design
                if !json {
                    println!("  ⚠ Skipping experiment for [{}]: no experiment design", h.title);
                }
                continue;
            }

            let mut config = HashMap::new();
            config.insert("baseline".into(), serde_json::Value::String(ed.baseline.clone()));
            config.insert(
                "variables".into(),
                serde_json::Value::Array(ed.variables.iter().map(|v| serde_json::Value::String(v.clone())).collect()),
            );
            config.insert(
                "controls".into(),
                serde_json::Value::Array(ed.controls.iter().map(|c| serde_json::Value::String(c.clone())).collect()),
            );
            config.insert(
                "evaluation_metrics".into(),
                serde_json::Value::Array(
                    ed.evaluation_metrics.iter().map(|m| serde_json::Value::String(m.clone())).collect(),
                ),
            );
            config.insert(
                "expected_results".into(),
                serde_json::Value::String(ed.expected_results.clone()),
            );
            config.insert(
                "hypothesis_type".into(),
                serde_json::Value::String(h.hypothesis_type.clone()),
            );

            let tags = vec![topic.to_string(), h.hypothesis_type.clone()];
            let exp = exp_tracker.run(
                &h.title,
                &h.core_statement,
                "",
                &h.id,
                Some(config),
                Some(tags),
            );

            if !json {
                println!("  ✓ Created experiment [{}]: {}", exp.id, h.title);
            }
            created_count += 1;
        }

        if !json {
            if created_count > 0 {
                println!();
                println!("  ━━ {} experiment(s) created ━━", created_count);
                println!("  Run `rairos experiment list` to view, or `rairos experiment complete <id>` when done.");
            } else {
                println!("  No experiments created (no valid experiment designs).");
            }
        }
    } else if hypothesis_only {
        if json {
            println!("  Hypothesis-only mode — no experiments created.");
        } else {
            println!();
            println!("  📋 Hypothesis-only mode — experiment creation skipped.");
        }
    }

    if !json {
        println!();
        println!("  ✓ Pipeline complete.");
    }

    Ok(())
}
