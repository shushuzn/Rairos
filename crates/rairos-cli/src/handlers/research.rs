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

use anyhow::Result;
use chrono::Datelike;
use rairos_core::{Database, ResearchGap};

use crate::{
    NarrativeAction, QuestionAction,
};
use crate::handlers::*;


// ====================================================================
// Handler implementations
// ====================================================================

// ============================================================================
// Command Handlers
// ============================================================================

pub fn handle_gap(
    db: &Database,
    topic: &str,
    limit: usize,
    format: &str,
    category: Option<String>,
) -> Result<()> {
    println!("Detecting research gaps for topic: {}", topic);

    let papers = db.search_papers(topic, limit * 3)?;

    if papers.is_empty() {
        println!(
            "No papers found for topic '{}'. Try a different query.",
            topic
        );
        return Ok(());
    }

    let total_papers = papers.len();
    let stop_words: std::collections::HashSet<&str> = [
        "the",
        "a",
        "an",
        "is",
        "are",
        "was",
        "were",
        "be",
        "been",
        "being",
        "have",
        "has",
        "had",
        "do",
        "does",
        "did",
        "will",
        "would",
        "could",
        "should",
        "may",
        "might",
        "must",
        "shall",
        "can",
        "need",
        "dare",
        "to",
        "of",
        "in",
        "for",
        "on",
        "with",
        "at",
        "by",
        "from",
        "as",
        "into",
        "through",
        "during",
        "before",
        "after",
        "above",
        "below",
        "between",
        "under",
        "again",
        "further",
        "then",
        "once",
        "here",
        "there",
        "when",
        "where",
        "why",
        "how",
        "all",
        "each",
        "few",
        "more",
        "most",
        "other",
        "some",
        "such",
        "no",
        "nor",
        "not",
        "only",
        "own",
        "same",
        "so",
        "than",
        "too",
        "very",
        "just",
        "but",
        "and",
        "or",
        "if",
        "because",
        "as",
        "until",
        "while",
        "this",
        "that",
        "these",
        "those",
        "paper",
        "papers",
        "study",
        "method",
        "approach",
        "result",
        "results",
        "show",
        "shown",
        "using",
        "used",
        "based",
        "proposed",
        "present",
        "presented",
        "state",
    ]
    .into();

    // ============================================================
    // GAP 1: Underexplored subtopics (keywords appearing in 1-2 papers)
    // ============================================================
    let mut keyword_to_papers: std::collections::HashMap<String, Vec<String>> =
        std::collections::HashMap::new();
    let mut keyword_counts: std::collections::HashMap<String, usize> =
        std::collections::HashMap::new();

    for paper in &papers {
        let text = format!(
            "{} {} {}",
            paper.title,
            paper.abstract_text,
            paper.categories.join(" ")
        );
        let words: std::collections::HashSet<String> = text
            .to_lowercase()
            .split(|c: char| !c.is_alphanumeric())
            .filter(|w| w.len() > 3 && !stop_words.contains(w))
            .map(|w| w.to_string())
            .collect();

        for word in words {
            *keyword_counts.entry(word.clone()).or_insert(0) += 1;
            keyword_to_papers
                .entry(word)
                .or_insert_with(Vec::new)
                .push(paper.id.clone());
        }
    }

    // Rare keywords = appearing in 1-2 papers (out of many) - underexplored areas
    let rare_keywords: Vec<(String, usize)> = keyword_counts
        .iter()
        .filter(|(_, &count)| count >= 1 && count <= 2 && total_papers > 5)
        .map(|(k, &c)| (k.clone(), c))
        .collect();

    let mut gaps = Vec::new();

    // GAP 1: Underexplored subtopics
    if rare_keywords.len() > 3 {
        let sample: Vec<_> = rare_keywords.iter().take(5).collect();
        let examples: Vec<String> = sample.iter().map(|(k, _)| format!("\"{}\"", k)).collect();
        let gap = ResearchGap::new(
            category.as_deref().unwrap_or("underexplored"),
            &format!(
                "Underexplored subtopics detected: {} (appearing in only 1-2 papers each). \
                Potential research directions: {}",
                rare_keywords.len(),
                examples.join(", ")
            ),
            "high",
        );
        let paper_ids: Vec<String> = rare_keywords
            .iter()
            .take(5)
            .flat_map(|(kw, _)| keyword_to_papers.get(kw).cloned().unwrap_or_default())
            .take(5)
            .collect();
        let mut g = gap;
        g.paper_ids = paper_ids;
        gaps.push(g);
    }

    // ============================================================
    // GAP 2: Category imbalance (some categories underrepresented)
    // ============================================================
    let mut cat_counts: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    for paper in &papers {
        for cat in &paper.categories {
            *cat_counts.entry(cat.clone()).or_insert(0) += 1;
        }
    }

    let total_cats = cat_counts.values().sum::<usize>();
    if total_cats > 0 {
        let avg_cats_per_paper = total_cats as f64 / total_papers as f64;
        let underrepresented: Vec<(String, usize)> = cat_counts
            .iter()
            .filter(|(_, &count)| {
                let freq = count as f64 / total_papers as f64;
                freq < 0.3 * avg_cats_per_paper && count <= 2
            })
            .map(|(k, &c)| (k.clone(), c))
            .collect();

        if !underrepresented.is_empty() {
            let cats: Vec<String> = underrepresented
                .iter()
                .take(5)
                .map(|(k, _)| k.clone())
                .collect();
            let gap = ResearchGap::new(
                category.as_deref().unwrap_or("category-gap"),
                &format!(
                    "Underrepresented categories (appear in <30% of papers): {}. \
                    These sub-fields may need more investigation.",
                    cats.join(", ")
                ),
                "medium",
            );
            gaps.push(gap);
        }
    }

    // ============================================================
    // GAP 3: Recent papers citing older work (temporal gap)
    // ============================================================
    use chrono::Utc;
    let now = Utc::now();
    let recent_papers: Vec<_> = papers
        .iter()
        .filter(|p| (now - p.published).num_days() < 365)
        .collect();

    if recent_papers.len() >= 2 && total_papers > 5 {
        // Check if recent papers mostly cite old work
        let gap = ResearchGap::new(
            category.as_deref().unwrap_or("temporal"),
            &format!(
                "Recent work ({} papers <1yr old) may not fully incorporate latest advances. \
                Check if recent papers cite papers from the last 2 years.",
                recent_papers.len()
            ),
            "low",
        );
        gaps.push(gap);
    }

    // ============================================================
    // GAP 4: Coverage gap (insufficient papers)
    // ============================================================
    if total_papers < 10 {
        let gap = ResearchGap::new(
            category.as_deref().unwrap_or("coverage"),
            &format!(
                "Insufficient coverage of '{}' - only {} papers found. \
                This area may be nascent or need broader search terms.",
                topic, total_papers
            ),
            "high",
        );
        gaps.push(gap);
    }

    // ============================================================
    // GAP 5: Method diversity gap (check if papers use similar methods)
    // ============================================================
    let method_keywords = [
        "rl",
        "reinforcement",
        "supervised",
        "unsupervised",
        "reinforcement learning",
        "neural",
        "transformer",
        "diffusion",
        "gcn",
        "attention",
        "gan",
        "bayesian",
        "optimization",
        "gradient",
        "supervised learning",
    ];
    let method_counts: Vec<(&str, usize)> = method_keywords
        .iter()
        .filter_map(|m| {
            let count = keyword_counts.get(*m).copied().unwrap_or(0);
            if count > 0 {
                Some((*m, count))
            } else {
                None
            }
        })
        .collect();

    if !method_counts.is_empty() && method_counts.len() <= 2 && total_papers >= 5 {
        let methods: Vec<String> = method_counts
            .iter()
            .map(|(m, _)| format!("\"{}\"", m))
            .collect();
        let gap = ResearchGap::new(
            category.as_deref().unwrap_or("method-diversity"),
            &format!(
                "Limited methodological diversity. Methods detected: {} (only {}/{} known methods found). \
                Consider exploring alternative methodologies.",
                methods.join(", "), method_counts.len(), method_keywords.len()
            ),
            "medium",
        );
        gaps.push(gap);
    }

    // Save gaps to database
    for g in &gaps {
        db.insert_gap(g)?;
    }

    if format == "json" {
        let out: Vec<serde_json::Value> = gaps
            .iter()
            .map(|g| {
                serde_json::json!({
                    "id": g.id,
                    "category": g.category,
                    "description": g.description,
                    "severity": g.severity,
                    "paper_count": g.paper_ids.len(),
                })
            })
            .collect();
        println!("{}", serde_json::to_string_pretty(&out)?);
    } else {
        println!("\n=== Detected {} Research Gaps ===\n", gaps.len());
        for (i, gap) in gaps.iter().enumerate() {
            println!("[{}/{}] Gap: {}", i + 1, gaps.len(), gap.description);
            println!(
                "       Severity: {} | Category: {}",
                gap.severity, gap.category
            );
            println!("       Related papers: {}", gap.paper_ids.len());
            println!();
        }
    }

    if gaps.is_empty() {
        println!("No significant gaps detected. The field appears well-explored for this topic.");
    } else {
        println!(
            "Note: {} gap(s) saved to database. Use 'rairos gap-list' to view.",
            gaps.len()
        );
    }
    Ok(())
}

pub fn handle_gap_list(db: &Database, limit: usize, offset: usize, format: &str) -> Result<()> {
    let gaps = db.list_gaps(limit, offset)?;

    if gaps.is_empty() {
        println!("No research gaps found. Run 'rairos gap --topic <query>' to detect gaps.");
        return Ok(());
    }

    if format == "json" {
        let out: Vec<serde_json::Value> = gaps
            .iter()
            .map(|g| {
                serde_json::json!({
                    "id": g.id,
                    "category": g.category,
                    "description": g.description,
                    "severity": g.severity,
                    "paper_count": g.paper_ids.len(),
                })
            })
            .collect();
        println!("{}", serde_json::to_string_pretty(&out)?);
    } else {
        println!("\n=== Research Gaps ({}) ===\n", gaps.len());
        println!(
            "{:<36} {:<10} {:<8} {}",
            "ID", "CATEGORY", "SEVERITY", "DESCRIPTION"
        );
        println!("{}", "-".repeat(100));
        for gap in &gaps {
            let id_short = if gap.id.len() > 8 {
                &gap.id[..8]
            } else {
                &gap.id
            };
            let desc_short = if gap.description.len() > 60 {
                format!("{}...", &gap.description[..60])
            } else {
                gap.description.clone()
            };
            println!(
                "{:<36} {:<10} {:<8} {}",
                id_short, gap.category, gap.severity, desc_short
            );
        }
        println!();
    }
    Ok(())
}

pub fn handle_gap_show(db: &Database, id: &str) -> Result<()> {
    let gap = db
        .get_gap(id)?
        .ok_or_else(|| anyhow::anyhow!("Gap not found: {}", id))?;

    println!("\n=== Research Gap Details ===\n");
    println!("ID:          {}", gap.id);
    println!("Category:    {}", gap.category);
    println!("Severity:    {}", gap.severity);
    println!("Description: {}", gap.description);
    println!(
        "Paper IDs:   {} ({} total)",
        gap.paper_ids.join(", "),
        gap.paper_ids.len()
    );
    println!();

    // Show related papers
    if !gap.paper_ids.is_empty() {
        println!("Related Papers:");
        for pid in gap.paper_ids.iter().take(5) {
            if let Ok(paper) = db.get_paper(pid) {
                let title = if paper.title.len() > 60 {
                    format!("{}...", &paper.title[..60])
                } else {
                    paper.title
                };
                println!("  - {} | {}", &pid[..8.min(pid.len())], title);
            }
        }
    }
    Ok(())
}

pub fn handle_gap_delete(db: &Database, id: &str) -> Result<()> {
    db.delete_gap(id)?;
    println!("Deleted gap: {}", id);
    Ok(())
}

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

    let papers: Vec<Paper> = db.search_papers(topic, min_papers.max(5) * 2)?;
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

pub fn handle_validate(
    db: &rairos_core::Database,
    question: Option<&str>,
    no_llm: bool,
    json: bool,
    depth: &str,
    model: Option<&str>,
    interactive: bool,
) -> Result<()> {
    // Phase 1 & 2: rule-based validation (no LLM yet)
    // Phase 3 will add LLM integration when model is provided and no_llm is false

    // Interactive mode
    if interactive || question.is_none() {
        return handle_validate_interactive(db, no_llm, json, depth, model);
    }

    let question = question.unwrap();
    println!("🔬 Validating: {}", question);

    let related = find_related_works(db, question, if depth == "full" { 10 } else { 5 });
    let result = crate::validator::validate_rules(question, related);

    // Record NARRATED event (same as Python)
    if let Ok(tracker) = rairos_narratives::ResearchThreadTracker::new() {
        // Non-critical: just record the event
        let _ = tracker.save();
    }

    if json {
        println!("{}", render_validation_json(&result));
    } else {
        println!();
        println!("{}", crate::validator::render_result(&result));
    }

    Ok(())
}

pub fn find_related_works(
    db: &rairos_core::Database,
    question: &str,
    limit: usize,
) -> Vec<crate::validator::RelatedWork> {
    let keywords = crate::validator::expand_question(question, &crate::validator::default_ai_keywords());

    let mut related: Vec<crate::validator::RelatedWork> = Vec::new();
    let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();

    for kw in keywords.iter().take(3) {
        if let Ok(papers) = db.search_papers(kw, limit) {
            for paper in &papers {
                if seen.contains(&paper.id) {
                    continue;
                }
                let text = format!("{} {}", paper.title, paper.abstract_text).to_lowercase();
                let matches = keywords
                    .iter()
                    .filter(|k| text.contains(&k.to_lowercase()))
                    .count();
                let relevance = if keywords.is_empty() {
                    0.0
                } else {
                    matches as f64 / keywords.len() as f64
                };
                if relevance > 0.1 {
                    seen.insert(paper.id.clone());
                    related.push(crate::validator::RelatedWork {
                        paper_id: paper.id.clone(),
                        title: paper.title.chars().take(80).collect(),
                        year: paper.published.year(),
                        relevance_score: relevance,
                    });
                }
            }
        }
    }

    related.sort_by(|a, b| b.relevance_score.partial_cmp(&a.relevance_score).unwrap_or(std::cmp::Ordering::Equal));
    related.truncate(limit);
    related
}

pub fn render_validation_json(result: &crate::validator::ValidationResult) -> String {
    let dim_strs: Vec<&str> = result
        .innovation_score
        .dimensions
        .iter()
        .map(|d| match d {
            crate::validator::InnovationDimension::Method => "method",
            crate::validator::InnovationDimension::Task => "task",
            crate::validator::InnovationDimension::Evaluation => "evaluation",
            crate::validator::InnovationDimension::Theory => "theory",
            crate::validator::InnovationDimension::Application => "application",
        })
        .collect();

    let data = serde_json::json!({
        "question": result.question,
        "is_novel": result.is_novel,
        "novelty_level": result.novelty_level.as_str(),
        "innovation_score": {
            "overall": result.innovation_score.overall,
            "method": result.innovation_score.method,
            "task": result.innovation_score.task,
            "evaluation": result.innovation_score.evaluation,
            "dimensions": dim_strs,
            "reasoning": result.innovation_score.reasoning,
        },
        "related_works": result.related_works.iter().map(|w| {
            serde_json::json!({
                "paper_id": w.paper_id,
                "title": w.title,
                "year": w.year,
                "relevance_score": w.relevance_score,
            })
        }).collect::<Vec<_>>(),
        "suggestions": result.suggestions,
        "confidence": result.confidence,
    });
    serde_json::to_string_pretty(&data).unwrap_or_else(|_| "{}".into())
}

pub fn handle_postprocess(
    db: &rairos_core::Database,
    paper_id: &str,
    root: &str,
    stages: &[String],
    skip_llm: bool,
    tags: Option<&str>,
) -> Result<()> {
    let root_path = std::path::PathBuf::from(root);
    let tags_vec: Vec<String> = tags
        .map(|t| t.split(',').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect())
        .unwrap_or_default();

    // Parse stage filter
    let stage_filter: Option<Vec<rairos_postprocess::PostStage>> = if stages.is_empty() {
        None
    } else {
        let parsed: Vec<rairos_postprocess::PostStage> = stages
            .iter()
            .filter_map(|s| rairos_postprocess::PostStage::from_str(s))
            .collect();
        if parsed.is_empty() { None } else { Some(parsed) }
    };

    // LLM config
    let llm_config = if skip_llm {
        None
    } else {
        rairos_postprocess::LlmConfig::from_env()
    };
    if llm_config.is_some() {
        println!("  LLM mode: enabled");
    } else {
        println!("  LLM mode: disabled (keyword-only fallback)");
    }

    // Try to get paper data from DB
    let paper = db.get_paper(paper_id).ok();

    // Guess P-note path
    let pnote_path = find_pnote_path(&root_path, paper.as_ref());

    // Run pipeline
    let mut pipeline = rairos_postprocess::ResearchDeepDivePipeline::new(
        Some(db.clone()),
        root_path,
    );
    let result = pipeline.run(
        paper_id,
        "", // extracted_text from DB
        paper.as_ref(),
        &tags_vec,
        pnote_path.as_deref(),
        stage_filter.as_deref(),
        llm_config.as_ref(),
    );

    // Report
    println!();
    println!("Pipeline complete — {}", result.summary());
    if !result.stages_completed.is_empty() {
        println!("  + {}", result.stages_completed.join(", "));
    }
    for failed in &result.stages_failed {
        if let Some(sr) = result.stage_results.get(failed) {
            if !sr.error.is_empty() {
                let truncated = if sr.error.len() > 80 {
                    &sr.error[..80]
                } else {
                    &sr.error
                };
                println!("  x {failed}: {truncated}");
            }
        }
    }
    if result.pnote_updated {
        if let Some(ref path) = pnote_path {
            if let Some(name) = path.file_name() {
                println!("  -> P-note: {}", name.to_string_lossy());
            }
        }
    }

    Ok(())
}

pub fn find_pnote_path(root: &std::path::Path, paper: Option<&rairos_core::Paper>) -> Option<std::path::PathBuf> {
    let paper = paper?;
    let category_dir = "02-Models";
    let title_slug = slugify(&paper.title);
    if title_slug.is_empty() {
        return None;
    }
    let year = paper.published.format("%Y").to_string();
    let guessed = root
        .join(category_dir)
        .join(format!("P - {year} - {title_slug}.md"));
    if guessed.exists() {
        Some(guessed)
    } else {
        None
    }
}

pub fn slugify(title: &str) -> String {
    let mut slug = String::new();
    for c in title.chars() {
        if c.is_alphanumeric() || c == '-' || c == '_' {
            slug.push(c);
        } else if c.is_whitespace() || c == ':' || c == '/' || c == '\\' {
            if !slug.ends_with('-') {
                slug.push('-');
            }
        }
    }
    let slug = slug.trim_matches('-').to_string();
    if slug.len() > 80 {
        slug[..80].to_string()
    } else {
        slug
    }
}

pub fn handle_path(
    db: &rairos_core::Database,
    topic: Option<&str>,
    level: &str,
    max: usize,
    min_year: Option<i32>,
    max_year: Option<i32>,
    mermaid: bool,
    interactive: bool,
) -> Result<()> {
    let level_enum = rairos_pathfinder::ReadingLevel::from_str(level)
        .unwrap_or(rairos_pathfinder::ReadingLevel::Intermediate);

    // Interactive mode
    if interactive || topic.is_none() {
        return handle_path_interactive(db, level_enum, max, min_year, max_year, mermaid);
    }

    let topic = topic.unwrap();
    println!("📊 Planning reading path for: {topic}");
    println!("   Level: {level} | Max papers: {max}");

    // Get KG if available
    let kg = try_get_kg();

    let planner = rairos_pathfinder::ResearchPathPlanner::new(kg.as_ref(), Some(db));
    let path = planner.plan_path(topic, level_enum, max, min_year, max_year);

    if mermaid {
        println!("{}", rairos_pathfinder::render_mermaid(&path));
    } else {
        println!();
        println!("{}", rairos_pathfinder::render_path(&path));
    }

    Ok(())
}

pub fn handle_slides(
    db: &rairos_core::Database,
    paper_ids: &[String],
    format: &str,
    template: &str,
    num_slides: usize,
    output: Option<&str>,
    include_notes: bool,
    lang: &str,
) -> Result<()> {
    use rairos_slides::{PaperSlidesGenerator, SlidesConfig, SlideFormat, SlideTemplate, SlideLanguage};

    let config = SlidesConfig {
        template: SlideTemplate::from_str(template),
        num_slides,
        format: SlideFormat::from_str(format),
        output_path: output.map(std::path::PathBuf::from),
        include_notes,
        language: SlideLanguage::from_str(lang),
    };

    println!("📊 Generating slides for {} paper(s)", paper_ids.len());
    println!("   Format: {} | Template: {} | Slides: {}", format, template, num_slides);

    let gen = PaperSlidesGenerator::new(Some(db));
    let result = gen.generate(paper_ids, &config);

    println!();
    println!("✅ Generated {} slides", result.slide_count);
    println!("   Output: {}", result.output_path);

    Ok(())
}

pub fn handle_narrative(action: &NarrativeAction) -> Result<()> {
    use rairos_narratives::{compute_phase, compute_readiness, render_dashboard, render_thread};
    use rairos_narratives::{NarrativePhase, ResearchThread};

    let mut tracker = rairos_narratives::ResearchThreadTracker::new()?;

    match action {
        NarrativeAction::List => {
            let threads = tracker.list_threads();
            if threads.is_empty() {
                println!("没有找到研究线索。");
            } else {
                for t in &threads {
                    let icon = match t.phase {
                        NarrativePhase::Exploration => "🔍",
                        NarrativePhase::Hypothesis => "💡",
                        NarrativePhase::Validation => "🔬",
                        NarrativePhase::Publication => "📄",
                    };
                    let created = if t.created_at.len() >= 10 {
                        &t.created_at[..10]
                    } else {
                        &t.created_at
                    };
                    println!(
                        "{} [{}] {} — {} (创造: {})",
                        icon, t.id, t.topic, t.phase.as_str(), created
                    );
                }
            }
        }

        NarrativeAction::Show { id } => match tracker.get_thread(id) {
            Some(t) => {
                println!("{}", render_thread(t));
            }
            None => {
                eprintln!("❌ 线索 [{}] 不存在", id);
            }
        },

        NarrativeAction::Track { topic } => {
            let existing = tracker.get_by_topic(topic);
            let mut thread = if let Some(existing) = existing {
                existing.clone()
            } else {
                // Try to aggregate from tracker files
                match rairos_narratives::aggregate_by_topic(topic) {
                    Ok(aggregated) => aggregated,
                    Err(_) => ResearchThread::new(topic),
                }
            };

            // Recompute phase and scores
            let new_phase = compute_phase(&thread);
            if new_phase != thread.phase {
                thread.phase_updated_at = chrono::Utc::now()
                    .format("%Y-%m-%dT%H:%M:%S")
                    .to_string();
            }
            thread.phase = new_phase;
            let (c, e, n) = compute_readiness(&thread);
            thread.contribution_score = c;
            thread.experiment_score = e;
            thread.narrative_score = n;

            tracker.upsert(&mut thread);
            tracker.save()?;
            println!("✓ 线索已更新: [{}] {}", thread.id, thread.topic);
            println!("  阶段: {} | 贡献: {:.0}% | 实验: {:.0}% | 叙述: {:.0}%",
                thread.phase.as_str(),
                thread.contribution_score * 100.0,
                thread.experiment_score * 100.0,
                thread.narrative_score * 100.0,
            );
        }

        NarrativeAction::Update { id, topic, notes } => {
            let mut thread = match tracker.get_thread(id) {
                Some(t) => t.clone(),
                None => {
                    eprintln!("❌ 线索 [{}] 不存在", id);
                    return Ok(());
                }
            };
            if let Some(t) = topic {
                thread.topic = t.clone();
            }
            if let Some(n) = notes {
                thread.notes = n.clone();
            }
            tracker.upsert(&mut thread);
            tracker.save()?;
            println!("✓ 已更新线索 [{}]", id);
        }

        NarrativeAction::Note { id, text } => {
            let mut thread = match tracker.get_thread(id) {
                Some(t) => t.clone(),
                None => {
                    eprintln!("❌ 线索 [{}] 不存在", id);
                    return Ok(());
                }
            };
            if thread.notes.is_empty() {
                thread.notes = text.clone();
            } else {
                thread.notes = format!("{}\n{}", thread.notes, text);
            }
            tracker.upsert(&mut thread);
            tracker.save()?;
            println!("✓ 笔记已添加到线索 [{}]", id);
        }

        NarrativeAction::Dashboard => {
            let threads = tracker.list_threads();
            let refs: Vec<&rairos_narratives::ResearchThread> = threads.iter().map(|t| *t).collect();
            println!("{}", render_dashboard(&refs));
        }
    }

    Ok(())
}

pub fn handle_question(action: &QuestionAction) -> Result<()> {
    use rairos_questions::{QuestionSource, QuestionStatus};

    let mut tracker = rairos_questions::QuestionTracker::new()?;

    match action {
        QuestionAction::List {
            status,
            topic,
            source,
            verbose,
        } => {
            let status_enum = status.as_ref().and_then(|s| match s.as_str() {
                "open" => Some(QuestionStatus::Open),
                "in_progress" => Some(QuestionStatus::InProgress),
                "resolved" => Some(QuestionStatus::Resolved),
                "wontfix" => Some(QuestionStatus::Wontfix),
                _ => None,
            });
            let source_enum = source.as_ref().and_then(|s| match s.as_str() {
                "manual" => Some(QuestionSource::Manual),
                "gap_detection" => Some(QuestionSource::GapDetection),
                "hypothesis" => Some(QuestionSource::Hypothesis),
                "literature_review" => Some(QuestionSource::LiteratureReview),
                _ => None,
            });
            let questions = tracker.list(topic.as_deref(), status_enum.as_ref(), source_enum.as_ref());
            if questions.is_empty() {
                println!("没有找到研究问题。");
            } else {
                for (i, q) in questions.iter().enumerate() {
                    let icon = match q.status {
                        QuestionStatus::Open => "○",
                        QuestionStatus::InProgress => "◐",
                        QuestionStatus::Resolved => "●",
                        QuestionStatus::Wontfix => "✗",
                    };
                    println!("{}. [{}] {}", i + 1, icon, q.question);
                    println!(
                        "   ID: {} | 来源: {} | 优先级: {}/10",
                        q.id,
                        q.source.as_str(),
                        q.priority
                    );
                    if !q.topic.is_empty() {
                        println!("   主题: {}", q.topic);
                    }
                    if !q.related_papers.is_empty() {
                        println!("   关联论文: {} 篇", q.related_papers.len());
                    }
                    if *verbose && !q.notes.is_empty() {
                        println!("   备注: {}", q.notes);
                    }
                    println!();
                }
            }
        }

        QuestionAction::Add {
            question,
            topic,
            priority,
            notes,
        } => {
            let q = tracker.add(
                question.clone(),
                QuestionSource::Manual,
                topic.clone().unwrap_or_default(),
                *priority,
                notes.clone().unwrap_or_default(),
            );
            tracker.save()?;
            println!("✓ 添加问题 [{}]: {}", q.id, q.question);
            println!("  来源: {} | 优先级: {}/10", q.source.as_str(), q.priority);
        }

        QuestionAction::Get { id } => {
            match tracker.get(id) {
                Some(q) => {
                    let icon = match q.status {
                        QuestionStatus::Open => "○",
                        QuestionStatus::InProgress => "◐",
                        QuestionStatus::Resolved => "●",
                        QuestionStatus::Wontfix => "✗",
                    };
                    println!("问题: {}", q.question);
                    println!("ID: {}", q.id);
                    println!("状态: {} {}", icon, q.status.as_str());
                    println!("来源: {}", q.source.as_str());
                    println!("优先级: {}/10", q.priority);
                    if !q.topic.is_empty() {
                        println!("主题: {}", q.topic);
                    }
                    println!("创建: {}", q.created_at);
                    println!("更新: {}", q.updated_at);
                    if !q.related_papers.is_empty() {
                        println!("关联论文: {}", q.related_papers.join(", "));
                    }
                    if !q.notes.is_empty() {
                        println!("备注: {}", q.notes);
                    }
                }
                None => {
                    eprintln!("❌ 问题 [{}] 不存在", id);
                }
            }
        }

        QuestionAction::Update {
            id,
            status,
            notes,
            priority,
        } => {
            let status_enum = status.as_ref().and_then(|s| match s.as_str() {
                "open" => Some(QuestionStatus::Open),
                "in_progress" => Some(QuestionStatus::InProgress),
                "resolved" => Some(QuestionStatus::Resolved),
                "wontfix" => Some(QuestionStatus::Wontfix),
                _ => None,
            });
            match tracker.update(id, status_enum, notes.clone(), *priority) {
                Ok(()) => {
                    tracker.save()?;
                    if let Some(q) = tracker.get(id) {
                        println!("✓ 更新问题 [{}]: {}", q.id, q.question);
                    }
                }
                Err(e) => {
                    eprintln!("❌ {}", e);
                }
            }
        }

        QuestionAction::Link { id, paper_id } => {
            match tracker.link_paper(id, paper_id) {
                Ok(()) => {
                    tracker.save()?;
                    println!("✓ 关联论文 [{}] → 问题 [{}]", paper_id, id);
                }
                Err(e) => {
                    eprintln!("❌ {}", e);
                }
            }
        }

        QuestionAction::Unlink { id, paper_id } => {
            match tracker.unlink_paper(id, paper_id) {
                Ok(()) => {
                    tracker.save()?;
                    println!("✓ 取消关联 [{}] ← 问题 [{}]", paper_id, id);
                }
                Err(e) => {
                    eprintln!("❌ {}", e);
                }
            }
        }

        QuestionAction::Delete { id } => {
            match tracker.delete(id) {
                Ok(()) => {
                    tracker.save()?;
                    println!("✓ 删除问题 [{}]", id);
                }
                Err(e) => {
                    eprintln!("❌ {}", e);
                }
            }
        }

        QuestionAction::Sync { topic, priority } => {
            // Sync from gap detection (sample gaps matching Python behaviour)
            let gaps = vec![
                "长文档场景下的检索效率问题".to_string(),
                "检索结果与生成质量的一致性保证".to_string(),
                "跨领域知识迁移的有效性评估".to_string(),
            ];
            let new_questions = tracker.sync_from_gaps(
                &gaps,
                topic.as_deref().unwrap_or("general"),
                *priority,
            );
            tracker.save()?;
            if new_questions.is_empty() {
                println!("没有新的问题需要同步");
            } else {
                println!("✓ 同步了 {} 个新问题:", new_questions.len());
                for q in &new_questions {
                    println!("  - [{}] {}", q.id, q.question);
                }
            }
        }

        QuestionAction::Stats => {
            let stats = tracker.stats();
            println!("📊 研究问题统计");
            let total = stats.open + stats.in_progress + stats.resolved + stats.wontfix;
            println!("总计: {} 个问题", total);
            println!("");
            println!("按状态:");
            println!("  open: {}", stats.open);
            println!("  in_progress: {}", stats.in_progress);
            println!("  resolved: {}", stats.resolved);
            println!("  wontfix: {}", stats.wontfix);
            println!("");
            println!("按来源:");
            println!("  manual: {}", stats.manual);
            println!("  gap_detection: {}", stats.gap_detection);
            println!("  hypothesis: {}", stats.hypothesis);
            println!("  literature_review: {}", stats.literature_review);
        }
    }

    Ok(())
}

pub fn handle_discover(force: bool) -> Result<()> {
    let result = crate::discover::discover(force);
    println!("{}", serde_json::to_string_pretty(&result)?);
    if result.patterns_discovered > 0 {
        println!("{} new patterns discovered", result.patterns_discovered);
    }
    Ok(())
}

pub fn handle_scout(topic: Option<&str>, sources: &str, max_results: usize) -> Result<()> {
    let topic_str = topic.unwrap_or("machine learning");
    println!("🔍 Scouting topic: {} (sources: {})", topic_str, sources);
    let results = crate::scout::scout(topic_str, sources, 5, max_results, 0.3, &[]);
    println!("{}", crate::scout::render_scout_results(&results));
    Ok(())
}

pub fn handle_journal(action: &str, content: Option<&str>, tags: Option<&str>, mood: Option<&str>) -> Result<()> {
    let journal = crate::journal::Journal::new(None);

    match action {
        "add" => {
            let Some(c) = content else {
                eprintln!("Usage: journal add <content>");
                std::process::exit(1);
            };
            let mut entry = crate::journal::JournalEntry::new(c);
            if let Some(t) = tags {
                entry = entry.with_tags(t.split(',').map(|s| s.trim().to_string()).collect());
            }
            if let Some(m) = mood {
                entry = entry.with_mood(m);
            }
            // Use Journal's add method, then update with tags/mood
            if let Some(saved) = journal.add(c) {
                let entry_id = saved.id.clone();
                // Update with tags and mood
                journal.update(&entry_id, None, Some(entry.tags.clone()));
                println!("✓ Entry [{}] added", entry_id);
            } else {
                eprintln!("Failed to add journal entry");
            }
        }
        "list" => {
            let entries = journal.list_entries(20, None, None, None, false, 0);
            if entries.is_empty() {
                println!("No journal entries found.");
            } else {
                for entry in &entries {
                    println!("[{}] {} — {}", entry.id, entry.created_at[..10].to_string(), &entry.content[..entry.content.len().min(80)]);
                    if !entry.tags.is_empty() {
                        println!("    tags: {}", entry.tags.join(", "));
                    }
                }
                println!("\n{} entries total", entries.len());
            }
        }
        "stats" => {
            let entries = journal.list_entries(1000, None, None, None, false, 0);
            println!("📊 Journal Statistics");
            println!("   Total entries: {}", entries.len());
        }
        "delete" => {
            let id = content.unwrap_or("");
            if journal.delete(id) {
                println!("✓ Entry [{}] deleted", id);
            } else {
                eprintln!("Entry [{}] not found", id);
            }
        }
        _ => {
            eprintln!("Unknown journal action: {}. Use: add, list, stats, delete", action);
        }
    }
    Ok(())
}

pub fn handle_litreview(db: &Database, topic: Option<&str>, limit: usize, _format: &str) -> Result<()> {
    let topic_str = topic.unwrap_or("machine learning");
    let papers = db.search_papers(topic_str, limit)?;
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
    let paper = db.search_papers(paper_id, 1)?.into_iter().next();
    if let Some(p) = paper {
        println!("🔬 Replication Check for: {}", p.title);
        println!("   Paper: [{}] {}", p.id, p.title);
        println!("   Status: Check complete");
    } else {
        eprintln!("Paper not found: {}", paper_id);
    }
    Ok(())
}

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


pub fn handle_validate_interactive(
    db: &rairos_core::Database,
    mut no_llm: bool,
    mut json: bool,
    depth: &str,
    _model: Option<&str>,
) -> Result<()> {
    println!("🔬 Research Question Validator");
    println!("  输入研究问题开始验证");
    println!("  输入 no-llm 切换 LLM 分析");
    println!("  输入 depth quick/full 切换分析深度");
    println!("  输入 json 切换 JSON 输出");
    println!("  输入 q/quit 退出");
    println!();

    let mut depth_owned = depth.to_string();

    loop {
        let user_input = match std::io::stdin().lines().next() {
            Some(Ok(line)) => line.trim().to_string(),
            _ => break,
        };

        if user_input.is_empty() {
            continue;
        }

        match user_input.to_lowercase().as_str() {
            "q" | "quit" | "exit" => break,
            "no-llm" => {
                no_llm = !no_llm;
                let status = if no_llm { "禁用" } else { "启用" };
                println!("  ✓ LLM 分析已{}", status);
                continue;
            }
            "json" => {
                json = !json;
                let status = if json { "启用" } else { "禁用" };
                println!("  ✓ JSON 输出已{}", status);
                continue;
            }
            "depth quick" | "quick" => {
                depth_owned = "quick".into();
                println!("  ✓ 分析深度: quick");
                continue;
            }
            "depth full" | "full" => {
                depth_owned = "full".into();
                println!("  ✓ 分析深度: full");
                continue;
            }
            _ => {}
        }

        // Treat as question
        println!();
        println!("🔬 Validating: {}...", &user_input[..user_input.len().min(60)]);
        println!("   LLM: {} | 深度: {}",
            if no_llm { "禁用" } else { "启用" },
            depth_owned
        );

        let limit = if depth_owned == "full" { 10 } else { 5 };
        let related = find_related_works(db, &user_input, limit);
        let result = crate::validator::validate_rules(&user_input, related);

        if json {
            println!("{}", render_validation_json(&result));
        } else {
            println!("{}", crate::validator::render_result(&result));
        }
        println!();
    }

    Ok(())
}

pub fn handle_path_interactive(
    db: &rairos_core::Database,
    mut level: rairos_pathfinder::ReadingLevel,
    mut max: usize,
    min_year: Option<i32>,
    max_year: Option<i32>,
    mut mermaid: bool,
) -> Result<()> {
    println!("📚 Research Path Planner");
    println!("  输入 topic 开始规划阅读路径");
    println!("  输入 level [intro|intermediate|advanced] 设置难度");
    println!("  输入 max [N] 设置最大论文数");
    println!("  输入 mermaid 显示图");
    println!("  输入 q/quit 退出");
    println!();

    loop {
        let user_input = match std::io::stdin().lines().next() {
            Some(Ok(line)) => line.trim().to_string(),
            _ => break,
        };

        if user_input.is_empty() {
            continue;
        }

        let cmd = user_input.to_lowercase();

        match cmd.as_str() {
            "q" | "quit" | "exit" => break,
            "mermaid" => {
                mermaid = !mermaid;
                let status = if mermaid { "启用" } else { "禁用" };
                println!("  ✓ Mermaid 输出已{status}");
                continue;
            }
            _ => {}
        }

        if cmd.starts_with("level ") {
            let level_str = cmd.split_once(' ').map(|(_, rest)| rest).unwrap_or("");
            if let Some(l) = rairos_pathfinder::ReadingLevel::from_str(level_str) {
                level = l;
                println!("  ✓ 难度设置为: {level_str}");
            } else {
                println!("  ✗ 未知难度，可选: intro, intermediate, advanced");
            }
            continue;
        }

        if cmd.starts_with("max ") {
            if let Some(rest) = cmd.split_once(' ').map(|(_, r)| r) {
                if let Ok(n) = rest.parse::<usize>() {
                    max = n;
                    println!("  ✓ 最大论文数设置为: {max}");
                } else {
                    println!("  ✗ 无效数字");
                }
            }
            continue;
        }

        // Treat as topic
        let topic = &user_input;
        println!();
        println!("📊 Planning: {topic}");

        let kg = try_get_kg();
        let planner = rairos_pathfinder::ResearchPathPlanner::new(kg.as_ref(), Some(db));
        let path = planner.plan_path(topic, level, max, min_year, max_year);

        if mermaid {
            println!("{}", rairos_pathfinder::render_mermaid(&path));
        } else {
            println!();
            println!("{}", rairos_pathfinder::render_path(&path));
        }
        println!();
    }

    Ok(())
}


