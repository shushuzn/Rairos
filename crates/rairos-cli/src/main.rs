//! Rairos CLI — Rust command-line interface
//!
//! Architecture: commands defined via clap derive, handlers in separate module.

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

mod handlers;

// Inlined CLI-only crates
pub mod bold_vault;
pub mod compare;
pub mod generate;
pub mod journal;
pub mod lsp_diagnostics;
pub mod signal;
pub mod story;
pub mod validator;
pub mod climate_ai_monitor;
pub mod labor_displacement_tracker;
pub mod at_risk_scanner;
pub mod ecosystem;
pub mod workspace_snapshot;
pub mod gap_analyzer;
pub mod value_quantifier;
pub mod batch_optimizer;
pub mod gap_detector;
pub mod policy_impact_tracer;
pub mod discover;
pub mod scout;

use anyhow::{Context, Result};
use clap::{CommandFactory, Parser};
use clap_complete::{generate, shells};
use rairos_core::{Database, ParseStatus};
use std::path::PathBuf;


// Re-export handler symbols for dispatch
use handlers::*;

// ============================================================================
// CLI App
// ============================================================================

#[derive(Parser)]
#[command(
    name = "rairos",
    version = "0.1.0",
    about = "Self-Evolving Research OS — manage papers, detect gaps, generate insights"
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    /// Path to the database file
    #[arg(long, global = true, default_value = "rairos.db")]
    db: PathBuf,

    /// Enable verbose logging
    #[arg(short, long, global = true)]
    verbose: bool,
}

// ============================================================================
// Helpers
// ============================================================================

fn load_env() {
    let _ = rairos_cli_shared::load_dotenv();
}

fn open_db(path: &PathBuf) -> Result<Database> {
    if !path.exists() {
        eprintln!(
            "Database not found at {}. Run 'rairos init' first.",
            path.display()
        );
        std::process::exit(1);
    }
    Database::open(path).context("Failed to open database")
}

fn parse_status_arg(s: &str) -> Option<ParseStatus> {
    match s.to_lowercase().as_str() {
        "pending" => Some(ParseStatus::Pending),
        "parsing" => Some(ParseStatus::Parsing),
        "done" => Some(ParseStatus::Done),
        "failed" => Some(ParseStatus::Failed),
        _ => None,
    }
}

fn status_str(status: &ParseStatus) -> &'static str {
    match status {
        ParseStatus::Pending => "pending",
        ParseStatus::Parsing => "parsing",
        ParseStatus::Done => "done",
        ParseStatus::Failed => "failed",
    }
}

// All handler implementations moved to handlers.rs
#[cfg(test)]
mod tests;

fn main() -> Result<()> {
    load_env();
    let cli = Cli::parse();

    // Simple logging setup (no tracing-subscriber dependency for now)
    if cli.verbose {
        eprintln!("[DEBUG] Verbose mode enabled");
    }

    match &cli.command {
        Commands::Version => {
            println!("rairos {}", env!("CARGO_PKG_VERSION"));
        }
        Commands::Benchmark { kind, iterations } => {
            handle_benchmark(kind, *iterations)?;
        }
        Commands::Achievements { action, achievement_id } => {
            match action.as_str() {
                "list" => handle_achievements_list()?,
                "report" => handle_achievements_report()?,
                "stats" => handle_achievements_stats()?,
                "unlock" => {
                    if let Some(id) = achievement_id {
                        handle_achievements_unlock(id)?;
                    } else {
                        eprintln!("Error: --achievement-id required for unlock action");
                    }
                }
                _ => unknown_action(action, &["list", "report", "stats", "unlock"]),
            }
        }
        Commands::Badges { action, badge_id } => {
            match action.as_str() {
                "list" => handle_badges_list()?,
                "award" => {
                    if let Some(id) = badge_id {
                        handle_badges_award(id)?;
                    } else {
                        eprintln!("Error: --badge-id required for award action");
                    }
                }
                _ => unknown_action(action, &["list", "award"]),
            }
        }
        Commands::Contradictions { action, limit } => {
            match action.as_str() {
                "list" => handle_contradictions_list(*limit)?,
                "render" => handle_contradictions_render()?,
                _ => unknown_action(action, &["list", "render"]),
            }
        }
        Commands::Trends { topic, years, format } => {
            match format.as_str() {
                "text" => handle_trends_analyze(topic, *years)?,
                "mermaid" => handle_trends_mermaid(topic, *years)?,
                _ => unknown_action(format, &["text", "mermaid"]),
            }
        }
        Commands::Rigor { paper_id } => {
            handle_rigor_score(paper_id)?;
        }
        Commands::Impact { action, paper_id, limit } => {
            match action.as_str() {
                "leaderboard" => handle_impact_leaderboard(*limit)?,
                "score" => {
                    if let Some(id) = paper_id {
                        handle_impact_score(id)?;
                    } else {
                        eprintln!("Error: --paper-id required for score action");
                    }
                }
                _ => unknown_action(action, &["leaderboard", "score"]),
            }
        }
        Commands::Briefing { arxiv_id, list, limit } => {
            if *list {
                handle_briefing_list(*limit)?;
            } else {
                handle_briefing_generate(arxiv_id)?;
            }
        }
        Commands::Paradigm { topic, list, limit: _ } => {
            if *list {
                handle_paradigm_list()?;
            } else {
                handle_paradigm_detect(topic)?;
            }
        }
        Commands::Crossref { paper_id, list, limit: _ } => {
            if *list {
                handle_crossref_list()?;
            } else {
                handle_crossref_analyze(paper_id)?;
            }
        }
        Commands::Momentum { tag, leaderboard } => {
            if *leaderboard {
                handle_momentum_leaderboard()?;
            } else if !tag.is_empty() {
                handle_momentum_score(tag)?;
            } else {
                handle_momentum_leaderboard()?;
            }
        }
        Commands::Crossover { list } => {
            if *list {
                handle_crossover_list()?;
            } else {
                handle_crossover_run()?;
            }
        }
        Commands::Decay { capsule_id, stats: _ } => {
            if !capsule_id.is_empty() {
                handle_decay_status(capsule_id)?;
            } else {
                handle_decay_stats()?;
            }
        }
        Commands::AtRisk { threshold, keep } => {
            if !keep.is_empty() {
                handle_atrisk_keep(&keep)?;
            } else {
                handle_atrisk_list(*threshold)?;
            }
        }
        Commands::Credibility { trendslop } => {
            if *trendslop {
                handle_credibility_trendslop()?;
            } else {
                handle_credibility_score()?;
            }
        }
        Commands::ClaimGraph { stats: _, contradictions } => {
            if *contradictions {
                handle_claimgraph_contradictions()?;
            } else {
                handle_claimgraph_stats()?;
            }
        }
        Commands::Bold => {
            handle_bold_list()?;
        }
        Commands::Profiler { stats } => {
            if *stats {
                handle_profiler_stats()?;
            } else {
                handle_profiler_report()?;
            }
        }
        Commands::CodeGraph { stats, files, search, node, callers, callees, depth } => {
            if *stats {
                handle_codegraph_stats()?;
            } else if *files {
                handle_codegraph_files()?;
            } else if let Some(q) = search {
                handle_codegraph_search(q)?;
            } else if let Some(id) = node {
                handle_codegraph_node(*id)?;
            } else if let Some(id) = callers {
                handle_codegraph_callers(*id, *depth)?;
            } else if let Some(id) = callees {
                handle_codegraph_callees(*id, *depth)?;
            } else {
                handle_codegraph_stats()?;
            }
        }
        Commands::Agent {
            topic,
            max_papers,
            max_time_minutes,
            format,
        } => {
            let db = open_db(&cli.db)?;
            handle_agent(&db, topic, *max_papers, *max_time_minutes, format)?;
        }
        Commands::Analyze {
            kind,
            paper,
            format,
        } => {
            let db = open_db(&cli.db)?;
            handle_analyze(&db, kind, paper.clone(), format)?;
        }
        Commands::Ask {
            question,
            max_papers,
            format,
        } => {
            let db = open_db(&cli.db)?;
            handle_ask(&db, question, *max_papers, format)?;
        }
        Commands::Dedup { action } => {
            let db = open_db(&cli.db)?;
            handle_dedup(&db, action)?;
        }
        Commands::Similar { paper, limit } => {
            let db = open_db(&cli.db)?;
            handle_similar(&db, paper, *limit)?;
        }
        Commands::Compare { papers, aspect } => {
            let db = open_db(&cli.db)?;
            handle_compare(&db, papers, aspect)?;
        }
        Commands::Trend {
            topic,
            range,
            format,
        } => {
            let db = open_db(&cli.db)?;
            handle_trend(&db, topic, range, format)?;
        }
        Commands::Init => {
            handle_init(&cli.db)?;
        }
        Commands::Stats { json, format } => {
            let db = open_db(&cli.db)?;
            handle_stats(&db, *json, format)?;
        }
        Commands::Add { arxiv_id } => {
            let db = open_db(&cli.db)?;
            handle_add(&db, arxiv_id)?;
        }
        Commands::List {
            status,
            year,
            tag,
            limit,
            offset,
            sort,
            order,
            format,
        } => {
            let db = open_db(&cli.db)?;
            handle_list(
                &db,
                status.clone(),
                *year,
                &tag,
                *limit,
                *offset,
                sort,
                order,
                format,
            )?;
        }
        Commands::Show { id, format } => {
            let db = open_db(&cli.db)?;
            handle_show(&db, id, format)?;
        }
        Commands::Search {
            query,
            limit,
            field,
            format,
        } => {
            let db = open_db(&cli.db)?;
            handle_search(&db, query, *limit, field, format)?;
        }
        Commands::Delete { id, force } => {
            let db = open_db(&cli.db)?;
            handle_delete(&db, &id, *force)?;
        }
        Commands::UpdateStatus { id, status } => {
            let db = open_db(&cli.db)?;
            handle_update_status(&db, &id, status)?;
        }
        Commands::Parse { id } => {
            let db = open_db(&cli.db)?;
            handle_parse(&db, id)?;
        }
        Commands::Import {
            path,
            ids,
            skip_existing,
        } => {
            let db = open_db(&cli.db)?;
            handle_import(&db, path, &ids, *skip_existing)?;
        }
        Commands::Export {
            path,
            status,
            format,
        } => {
            let db = open_db(&cli.db)?;
            handle_export(&db, path, status.clone(), format)?;
        }
        Commands::Gap {
            topic,
            limit,
            format,
            category,
        } => {
            let db = open_db(&cli.db)?;
            handle_gap(&db, topic, *limit, format, category.clone())?;
        }
        Commands::GapList {
            limit,
            offset,
            format,
        } => {
            let db = open_db(&cli.db)?;
            handle_gap_list(&db, *limit, *offset, format)?;
        }
        Commands::GapShow { id } => {
            let db = open_db(&cli.db)?;
            handle_gap_show(&db, id)?;
        }
        Commands::GapDelete { id } => {
            let db = open_db(&cli.db)?;
            handle_gap_delete(&db, id)?;
        }
        Commands::GapSuggestCode {
            gap_id,
            crate_name,
            format,
        } => {
            let db = open_db(&cli.db)?;
            handle_gap_suggest_code(&db, gap_id, crate_name.clone(), format)?;
        }
        Commands::Optimize {
            topic,
            crate_name,
            limit,
            format,
        } => {
            let db = open_db(&cli.db)?;
            handle_optimize(&db, topic, crate_name.clone(), *limit, format)?;
        }
        Commands::CodeGeneList {
            crate_name,
            limit,
            format,
        } => {
            let db = open_db(&cli.db)?;
            handle_code_gene_list(&db, crate_name.clone(), *limit, format)?;
        }
        Commands::CodeEvolve {
            crate_name,
            max_crossovers,
            format,
        } => {
            handle_code_evolve(crate_name.clone(), *max_crossovers, format)?;
        }
        Commands::WorkflowStats => {
            let db = open_db(&cli.db)?;
            handle_workflow_stats(&db)?;
        }
        Commands::GapCodeLink { gap_id } => {
            let db = open_db(&cli.db)?;
            handle_gap_code_link(&db, gap_id.clone())?;
        }
        Commands::OptimizePipeline { topic, crate_name, optimizations, evolutions } => {
            let db = open_db(&cli.db)?;
            handle_optimize_pipeline(&db, topic, crate_name.clone(), *optimizations, *evolutions)?;
        }
        Commands::CodeGeneFeedback { id, positive } => {
            handle_code_gene_feedback(id, *positive)?;
        }
        Commands::CodeGeneExport { output, crate_name } => {
            handle_code_gene_export(output, crate_name.clone())?;
        }
        Commands::CodeGeneClean { min_score, min_feedback, min_code_length, dry_run } => {
            handle_code_gene_clean(*min_score, *min_feedback, *min_code_length, *dry_run)?;
        }
        Commands::CodeGeneSyncToIssue { ids, crate_name, min_score } => {
            handle_code_gene_sync_to_issue(ids, crate_name.clone(), *min_score)?;
        }
        Commands::CodeGeneSyncFromIssue { issues, repo } => {
            handle_code_gene_sync_from_issue(issues, repo)?;
        }
        Commands::CodeGeneImplement { issue, repo, execute } => {
            handle_code_gene_implement(*issue, repo, *execute)?;
        }
        Commands::GeneAdd {
            approach,
            gap_type,
            keywords,
            paper_id,
        } => {
            handle_gene_add(approach, gap_type, keywords, paper_id.clone())?;
        }
        Commands::GeneList {
            gap_type,
            status,
            limit,
            format,
        } => {
            handle_gene_list(gap_type.clone(), status.clone(), *limit, format)?;
        }
        Commands::GeneShow { id, format } => {
            handle_gene_show(id, format)?;
        }
        Commands::GeneFeedback { id, positive } => {
            handle_gene_feedback(id, *positive)?;
        }
        Commands::GeneDiversity { format } => {
            handle_gene_diversity(format)?;
        }
        Commands::GeneEvolve {
            max_crossovers,
            format,
        } => {
            handle_gene_evolve(*max_crossovers, format)?;
        }
        Commands::KgStats { format } => {
            handle_kg_stats(format)?;
        }
        Commands::KgRank { limit, format } => {
            handle_kg_rank(*limit, format)?;
        }
        Commands::KgPath { source, target } => {
            handle_kg_path(source, target)?;
        }
        Commands::KgAddPaper { paper_id } => {
            let db = open_db(&cli.db)?;
            handle_kg_add_paper(&db, paper_id)?;
        }
        Commands::KgAddCitation { source, target } => {
            let db = open_db(&cli.db)?;
            handle_kg_add_citation(&db, source, target)?;
        }
        Commands::KgGraph {
            paper_id,
            depth,
            format,
        } => {
            handle_kg_graph(paper_id, *depth, format)?;
        }
        Commands::KgSearch {
            node_type,
            keyword,
            format,
        } => {
            handle_kg_search(node_type.as_deref(), keyword.as_deref(), format)?;
        }
        Commands::KgRebuild { incremental } => {
            let db = open_db(&cli.db)?;
            handle_kg_rebuild(&db, *incremental)?;
        }
        Commands::RateLimitBenchmark { count } => {
            handle_rate_limit_benchmark(*count)?;
        }
        Commands::RateLimitCheck { endpoint } => {
            handle_rate_limit_check(endpoint)?;
        }
        Commands::Daemon {
            port,
            log_level,
            foreground,
        } => {
            let db = open_db(&cli.db)?;
            handle_daemon(&db, *port, log_level, *foreground)?;
        }
        Commands::Subscribe {
            query,
            interval_minutes,
            max_papers,
            auto_add,
        } => {
            let db = open_db(&cli.db)?;
            handle_subscribe(&db, query, *interval_minutes, *max_papers, *auto_add)?;
        }
        Commands::Cache { action } => {
            handle_cache(action)?;
        }
        Commands::Repl { query } => {
            handle_repl(query.clone())?;
        }
        Commands::Setup { guide } => {
            if *guide {
                let wizard = rairos_setup::SetupWizard::new();
                println!("{}", wizard.quick_start_guide());
            } else {
                let mut wizard = rairos_setup::SetupWizard::new();
                let results = wizard.run();
                println!("Setup complete: {}/{} steps done",
                    results.iter().filter(|(_, done)| *done).count(),
                    results.len()
                );
                for (name, done) in &results {
                    println!("  {}: {}", if *done { "✓" } else { "✗" }, name);
                }
            }
        }
        Commands::Radar { action, tags, note_date, format: _format } => {
            let root = dirs::home_dir().map(|h| h.join(".ai_research_os")).unwrap_or_default();
            if action == "show" {
                match rairos_updaters::read_radar(&root) {
                    Ok(state) => println!("{:#?}", state),
                    Err(e) => eprintln!("Failed to read radar: {}", e),
                }
            } else if action == "update" {
                let tag_list: Vec<String> = tags.as_deref()
                    .map(|t| t.split(',').map(|s| s.trim().to_string()).collect())
                    .unwrap_or_default();
                let date = note_date.as_deref().unwrap_or("today");
                match rairos_updaters::update_radar(&root, &tag_list, date) {
                    Ok(_) => println!("Radar updated"),
                    Err(e) => eprintln!("Failed to update radar: {}", e),
                }
            }
        }
        Commands::Timeline { action, year, pnote, title, format: _format } => {
            let root = dirs::home_dir().map(|h| h.join(".ai_research_os")).unwrap_or_default();
            if action == "show" {
                match rairos_updaters::read_timeline(&root) {
                    Ok(state) => {
                        let rendered = rairos_updaters::render_timeline(&state);
                        println!("{}", rendered);
                    }
                    Err(e) => eprintln!("Failed to read timeline: {}", e),
                }
            } else if action == "update" {
                let y = year.as_deref().unwrap_or("2026");
                let p = pnote.as_deref().unwrap_or("");
                let t = title.as_deref().unwrap_or("");
                match rairos_updaters::update_timeline(&root, y, p, t) {
                    Ok(_) => println!("Timeline updated"),
                    Err(e) => eprintln!("Failed to update timeline: {}", e),
                }
            }
        }
        Commands::Doctor { format } => {
            handle_doctor(format)?;
        }
        Commands::StanceAdd {
            topic,
            claim,
            stance,
            reasoning,
        } => {
            handle_stance_add(&topic, &claim, &stance, &reasoning)?;
        }
        Commands::StanceList { topic, tag, format } => {
            handle_stance_list(topic.clone(), tag.clone(), &format)?;
        }
        Commands::StanceShow { id, format } => {
            handle_stance_show(&id, &format)?;
        }
        Commands::MemoryStats { format } => {
            handle_memory_stats(&format)?;
        }
        Commands::Status { format } => {
            let db = open_db(&cli.db)?;
            handle_status(&db, &format)?;
        }
        Commands::Citations { from, to, format } => {
            let db = open_db(&cli.db)?;
            handle_citations(&db, from.as_deref(), to.as_deref(), &format)?;
        }
        Commands::CiteStats { paper, top, format } => {
            let db = open_db(&cli.db)?;
            handle_cite_stats(&db, paper.as_deref(), *top, &format)?;
        }
        Commands::Queue { add, list, pending, dequeue, cancel, clear, format } => {
            let db = open_db(&cli.db)?;
            handle_queue(&db, add.as_deref(), *list, *pending, *dequeue, *cancel, *clear, &format)?;
        }
        Commands::Influence { top, paper, min_cites, format } => {
            let db = open_db(&cli.db)?;
            handle_influence(&db, *top, paper.as_deref(), *min_cites, &format)?;
        }
        Commands::Merge { keep, dry_run, auto, target_id, duplicate_id } => {
            let db = open_db(&cli.db)?;
            handle_merge(&db, keep, *dry_run, *auto, target_id.as_deref(), duplicate_id.as_deref())?;
        }
        Commands::CiteImport { json_input, dry_run, skip_missing, extract, paper, dedup } => {
            let db = open_db(&cli.db)?;
            handle_cite_import(&db, json_input.as_deref(), *dry_run, *skip_missing, *extract, paper.as_deref(), *dedup)?;
        }
        Commands::Signal { keyword } => {
            handle_signal(keyword)?;
        }
        Commands::Story { topic } => {
            let db = open_db(&cli.db)?;
            handle_story(&db, topic.as_deref())?;
        }
        Commands::Argue { thesis } => {
            let db = open_db(&cli.db)?;
            handle_argue(&db, thesis)?;
        }
        Commands::Discover { force } => {
            handle_discover(*force)?;
        }
        Commands::Scout { topic, sources, max_results } => {
            handle_scout(topic.as_deref(), sources, *max_results)?;
        }
        Commands::Journal { action, content, tags, mood } => {
            handle_journal(action, content.as_deref(), tags.as_deref(), mood.as_deref())?;
        }
        Commands::Intel { topic, verbose } => {
            handle_intel(topic, *verbose)?;
        }
        Commands::Litreview { topic, limit, format } => {
            let db = open_db(&cli.db)?;
            handle_litreview(&db, topic.as_deref(), *limit, format)?;
        }
        Commands::Report { format } => {
            handle_report(format)?;
        }
        Commands::Research { action, content } => {
            let db = open_db(&cli.db)?;
            handle_research(&db, action, content.as_deref())?;
        }
        Commands::Digest { weeks } => {
            handle_digest(*weeks)?;
        }
        Commands::Trace { arxiv_id, list, refs, limit } => {
            let db = open_db(&cli.db)?;
            handle_trace(&db, arxiv_id.as_deref(), *list, *refs, *limit)?;
        }
        Commands::Review { action, paper, content } => {
            let db = open_db(&cli.db)?;
            handle_review(&db, action, paper.as_deref(), content.as_deref())?;
        }
        Commands::Replicate { paper_id } => {
            let db = open_db(&cli.db)?;
            handle_replicate(&db, paper_id)?;
        }

        // ── Batch 5 ───────────────────────────────────────────────────────

        Commands::Friction { friction_type, days, json, limit } => {
            handle_friction(friction_type.as_deref(), *days, *json, *limit)?;
        }
        Commands::Experiment { action, name, desc, milestone, tag, id, metrics, metric_name, metric_value, unit, ids, result } => {
            handle_experiment(action, name.as_deref(), desc.as_deref(), milestone.as_deref(), tag.clone(),
                id.as_deref(), metrics.as_deref(), metric_name.as_deref(), *metric_value, unit,
                ids.clone(), result.as_deref())?;
        }
        Commands::Evolution { stats, patterns, feedback, report, sessions, days, clear, export } => {
            handle_evolution(*stats, *patterns, *feedback, *report, *sessions, *days, *clear, *export)?;
        }
        Commands::Dashboard { port, host, no_browser } => {
            handle_dashboard(*port, host, *no_browser)?;
        }
        Commands::CitationChain { paper_id, depth, graphviz, mermaid, influencers, impact, path } => {
            let db = open_db(&cli.db)?;
            handle_citation_chain(&db, paper_id.as_deref(), *depth, *graphviz, *mermaid, *influencers, *impact, path.as_deref())?;
        }
        Commands::Hypothesize { topic, gap, trend, story, no_llm, creative, json, model, top } => {
            handle_hypothesize(topic.as_deref(), gap, trend, story, *no_llm, *creative, *json, model.as_deref(), *top)?;
        }

        // ── Batch 6 ───────────────────────────────────────────────────────

        Commands::CiteGraph { paper, depth, max_nodes, format } => {
            let db = open_db(&cli.db)?;
            handle_cite_graph(&db, paper.as_deref(), *depth, *max_nodes, format)?;
        }
        Commands::CiteFetch { paper_id, dry_run } => {
            handle_cite_fetch(paper_id.as_deref(), *dry_run)?;
        }
        Commands::Lean { file, hypothesis, install, check, json } => {
            handle_lean(file.as_deref(), hypothesis.as_deref(), *install, *check, *json)?;
        }
        Commands::Visual { paper, query, limit, output } => {
            let db = open_db(&cli.db)?;
            handle_visual(&db, paper.as_deref(), query.as_deref(), *limit, output.as_deref())?;
        }
        Commands::Ingest { paper_id, json, no_pdf, source } => {
            handle_ingest(paper_id.as_deref(), *json, *no_pdf, source)?;
        }
        Commands::Session { action, title, topic, days, limit } => {
            handle_session(action, title.as_deref(), topic.as_deref(), *days, *limit)?;
        }
        Commands::Question { action } => {
            handle_question(action)?;
        }
        Commands::Narrative { action } => {
            handle_narrative(action)?;
        }
        Commands::Validate {
            question,
            no_llm,
            json,
            depth,
            model,
            interactive,
        } => {
            let db = open_db(&cli.db)?;
            handle_validate(
                &db,
                question.as_deref(),
                *no_llm,
                *json,
                depth,
                model.as_deref(),
                *interactive,
            )?;
        }

        Commands::Postprocess {
            paper_id,
            root,
            stages,
            skip_llm,
            tags,
        } => {
            let db = open_db(&cli.db)?;
            handle_postprocess(
                &db,
                paper_id,
                root,
                stages,
                *skip_llm,
                tags.as_deref(),
            )?;
        }

        Commands::Path {
            topic,
            level,
            max,
            min_year,
            max_year,
            mermaid,
            interactive,
        } => {
            let db = open_db(&cli.db)?;
            handle_path(
                &db,
                topic.as_deref(),
                level,
                *max,
                *min_year,
                *max_year,
                *mermaid,
                *interactive,
            )?;
        }

        Commands::Slides {
            paper_ids,
            format,
            template,
            num_slides,
            output,
            include_notes,
            lang,
        } => {
            let db = open_db(&cli.db)?;
            handle_slides(&db, paper_ids, format, template, *num_slides, output.as_deref(), *include_notes, lang)?;
        }

        Commands::Roadmap { question, text, json, export_md } => {
            handle_roadmap(question.as_deref(), text.as_deref(), *json, export_md.as_deref())?;
        }

        Commands::Demo { quick, papers, insights } => {
            handle_demo(*quick, *papers, *insights)?;
        }

        Commands::Pipeline {
            topic,
            hypothesis_only,
            top_n,
            min_papers,
            model,
            json,
            no_llm,
            verbose,
        } => {
            let db = open_db(&cli.db)?;
            handle_pipeline(
                &db,
                topic,
                *hypothesis_only,
                *top_n,
                *min_papers,
                model.as_deref(),
                *json,
                *no_llm,
                *verbose,
            )?;
        }

        Commands::Insight { action } => {
            handle_insight(action)?;
        }
        Commands::Jin10 { action } => {
            handle_jin10(action)?;
        }
        Commands::Route { query, json, exec, all } => {
            handle_route(query, *json, *exec, *all)?;
        }
        Commands::EvoSkill { action } => {
            handle_evoskill(action)?;
        }
        Commands::Rag { action } => {
            handle_rag(action)?;
        }
        Commands::Chat {
            question,
            paper,
            concept,
            limit,
            interactive,
            no_cite,
            model,
            verbose,
            stream,
            export,
            format,
        } => {
            handle_chat(
                question.as_deref(),
                paper.as_deref(),
                concept.as_deref(),
                *limit,
                *interactive,
                *no_cite,
                model.as_deref(),
                *verbose,
                *stream,
                export.as_deref(),
                format.as_deref(),
            )?;
        }
        Commands::ChatTui => {
            handle_chat_tui()?;
        }

        // ── Utility commands ──────────────────────────────────────

        Commands::Diagnostics { ruff, pyright, path } => {
            handle_diagnostics(*ruff, *pyright, &path.to_string_lossy())?;
        }

        Commands::Workspace { action } => {
            match action {
                WorkspaceAction::Snapshot { path } => {
                    handle_workspace_snapshot(&path.to_string_lossy())?;
                }
            }
        }

        Commands::Sysinfo => {
            handle_sysinfo()?;
        }

        Commands::Completions { shell } => {
            let mut cmd = Cli::command();
            let name = cmd.get_name().to_string();
            let mut stdout = std::io::stdout();
            match shell.as_str() {
                "bash" => generate(shells::Bash, &mut cmd, name, &mut stdout),
                "zsh" => generate(shells::Zsh, &mut cmd, name, &mut stdout),
                "fish" => generate(shells::Fish, &mut cmd, name, &mut stdout),
                _ => anyhow::bail!("Unsupported shell: {}. Use: bash, zsh, fish", shell),
            }
        }
    }

    Ok(())
}

/// Handle `roadmap` — generate research roadmap from a question.
fn handle_roadmap(
    question: Option<&str>,
    text: Option<&str>,
    json: bool,
    export_md: Option<&str>,
) -> Result<()> {
    use rairos_questions::QuestionTracker;
    use rairos_roadmap::RoadmapGenerator;

    // Determine question text
    let (question_text, question_id) = if let Some(qid) = question {
        let tracker = QuestionTracker::new()?;
        let q = tracker
            .get(qid)
            .ok_or_else(|| anyhow::anyhow!("问题 [{}] 不存在", qid))?;
        (q.question.clone(), q.id.clone())
    } else if let Some(t) = text {
        (t.to_string(), String::new())
    } else {
        anyhow::bail!("请提供 --question <id> 或 --text <问题>");
    };

    println!("📋 生成研究路线图...");

    let gen = RoadmapGenerator::new();
    let roadmap = gen.generate(&question_text, &question_id, None, "");

    if json {
        println!("{}", gen.render_json(&roadmap));
    } else if let Some(path) = export_md {
        std::fs::write(path, gen.render_markdown(&roadmap))
            .context("写入文件失败")?;
        println!("✓ 导出到 {}", path);
    } else {
        println!();
        println!("{}", gen.render_text(&roadmap));
    }

    Ok(())
}

/// Print unknown action error.
fn unknown_action(action: &str, valid: &[&str]) {
    eprintln!("Unknown action: {}. Use: {}", action, valid.join(", "));
}


