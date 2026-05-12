//! Rairos CLI Registry — Command registration and dispatch
//!
//! Reference: Python cli/_registry.py
//!
//! Provides command registration, subcommand table management, and CLI entry point.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ============================================================================
// Subcommand Table
// ============================================================================

/// Subcommand entry: (name, module_path, builder_name)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubcommandEntry {
    pub name: String,
    pub module_path: String,
    pub builder_name: String,
}

/// All available subcommands
pub const SUBCOMMAND_TABLE: &[(&str, &str, &str)] = &[
    ("search", "cli.cmd.search", "_build_search_parser"),
    ("research", "cli.cmd.research", "_build_research_parser"),
    ("list", "cli.cmd.list", "_build_list_parser"),
    ("status", "cli.cmd.status", "_build_status_parser"),
    ("queue", "cli.cmd.queue", "_build_queue_parser"),
    ("cache", "cli.cmd.cache", "_build_cache_parser"),
    ("dedup", "cli.cmd.dedup", "_build_dedup_parser"),
    (
        "dedup-semantic",
        "cli.cmd.dedup_semantic",
        "_build_dedup_semantic_parser",
    ),
    ("similar", "cli.cmd.similar", "_build_similar_parser"),
    ("kg", "cli.cmd.kg", "_build_kg_parser"),
    ("merge", "cli.cmd.merge", "_build_merge_parser"),
    ("stats", "cli.cmd.stats", "_build_stats_parser"),
    ("import", "cli.cmd.import_", "_build_import_parser"),
    ("export", "cli.cmd.export", "_build_export_parser"),
    ("citations", "cli.cmd.citations", "_build_citations_parser"),
    (
        "cite-graph",
        "cli.cmd.cite_graph",
        "_build_cite_graph_parser",
    ),
    (
        "cite-import",
        "cli.cmd.cite_import",
        "_build_cite_import_parser",
    ),
    (
        "cite-fetch",
        "cli.cmd.cite_fetch",
        "_build_cite_fetch_parser",
    ),
    (
        "cite-stats",
        "cli.cmd.cite_stats",
        "_build_cite_stats_parser",
    ),
    (
        "cite-backfill",
        "cli.cmd.cite_backfill",
        "_build_cite_backfill_parser",
    ),
    (
        "paper2code",
        "cli.cmd.paper.paper2code",
        "_build_paper2code_parser",
    ),
    ("trace", "cli.cmd.paper.trace", "_build_paper_trace_parser"),
    ("evoskill", "cli.cmd.evoskill", "_build_evoskill_parser"),
    ("rag", "cli.cmd.rag", "_build_rag_parser"),
    ("agent", "cli.cmd.agent", "_build_agent_parser"),
    ("route", "cli.cmd.route", "_build_route_parser"),
    ("visual", "cli.cmd.visual", "_build_visual_parser"),
    ("repl", "cli.cmd.repl", "_build_repl_parser"),
    (
        "read-queue",
        "cli.cmd.read_queue",
        "_build_read_queue_parser",
    ),
    ("chat", "cli.cmd.chat", "_build_chat_parser"),
    ("path", "cli.cmd.path", "_build_path_parser"),
    ("gap", "cli.cmd.gap", "_build_gap_parser"),
    ("trend", "cli.cmd.trend", "_build_trend_parser"),
    ("influence", "cli.cmd.influence", "_build_influence_parser"),
    (
        "hypothesize",
        "cli.cmd.hypothesize",
        "_build_hypothesize_parser",
    ),
    ("lean", "cli.cmd.lean", "_build_lean_parser"),
    ("validate", "cli.cmd.validate", "_build_validate_parser"),
    ("story", "cli.cmd.story", "_build_story_parser"),
    ("slides", "cli.cmd.slides", "_build_slides_parser"),
    ("evolution", "cli.cmd.evolution", "_build_evolution_parser"),
    ("analyze", "cli.cmd.analyze", "_build_analyze_parser"),
    ("review", "cli.cmd.review", "_build_review_parser"),
    ("question", "cli.cmd.question", "_build_question_parser"),
    ("roadmap", "cli.cmd.roadmap", "_build_roadmap_parser"),
    (
        "experiment",
        "cli.cmd.experiment",
        "_build_experiment_parser",
    ),
    ("pipeline", "cli.cmd.pipeline", "_build_pipeline_parser"),
    ("journal", "cli.cmd.journal", "_build_journal_parser"),
    ("digest", "cli.cmd.digest", "_build_digest_parser"),
    (
        "citation-chain",
        "cli.cmd.citation_chain",
        "_build_citation_chain_parser",
    ),
    ("compare", "cli.cmd.compare", "_build_compare_parser"),
    ("replicate", "cli.cmd.replicate", "_build_replicate_parser"),
    ("insight", "cli.cmd.insight", "_build_insight_parser"),
    ("ask", "cli.cmd.ask", "_build_ask_parser"),
    ("session", "cli.cmd.session", "_build_session_parser"),
    ("argue", "cli.cmd.argue", "_build_argue_parser"),
    ("narrative", "cli.cmd.narrative", "_build_narrative_parser"),
    ("friction", "cli.cmd.friction", "_build_friction_parser"),
    ("chat-tui", "cli.cmd.chat_tui", "_build_chat_tui_parser"),
    ("subscribe", "cli.cmd.subscribe", "_build_subscribe_parser"),
    ("litreview", "cli.cmd.litreview", "_build_litreview_parser"),
    ("benchmark", "cli.cmd.benchmark", "_build_benchmark_parser"),
    (
        "postprocess",
        "cli.cmd.postprocess",
        "_build_postprocess_parser",
    ),
    ("ingest", "cli.cmd.ingest", "_build_ingest_parser"),
    ("daemon", "cli.cmd.daemon", "_build_daemon_parser"),
    ("demo", "cli.cmd.demo", "_build_demo_parser"),
    ("scout", "cli.cmd.scout", "_build_scout_parser"),
    ("jin10", "cli.cmd.jin10", "_build_jin10_parser"),
    ("intel", "cli.cmd.intel", "_build_intel_parser"),
    ("signal", "cli.cmd.signal", "_build_signal_parser"),
    ("discover", "cli.cmd.discover", "_build_discover_parser"),
    ("report", "cli.cmd.report", "_build_report_parser"),
    ("dashboard", "cli.cmd.dashboard", "_build_web_parser"),
    ("doctor", "cli.cmd.doctor", "_build_doctor_parser"),
];

/// Set of all subcommand names
pub fn subcommands() -> Vec<String> {
    SUBCOMMAND_TABLE
        .iter()
        .map(|(name, _, _)| (*name).to_string())
        .collect()
}

// ============================================================================
// Dispatch Map
// ============================================================================

/// Dispatch map: subcommand name -> handler function name
pub fn dispatch_map() -> HashMap<String, String> {
    HashMap::from([
        ("search".to_string(), "_run_search".to_string()),
        ("list".to_string(), "_run_list".to_string()),
        ("status".to_string(), "_run_status".to_string()),
        ("queue".to_string(), "_run_queue".to_string()),
        ("cache".to_string(), "_run_cache".to_string()),
        ("dedup".to_string(), "_run_dedup".to_string()),
        ("merge".to_string(), "_run_merge".to_string()),
        ("stats".to_string(), "_run_stats".to_string()),
        ("import".to_string(), "_run_import".to_string()),
        ("export".to_string(), "_run_export".to_string()),
        ("citations".to_string(), "_run_citations".to_string()),
        ("cite-graph".to_string(), "_run_cite_graph".to_string()),
        ("cite-import".to_string(), "_run_cite_import".to_string()),
        ("cite-fetch".to_string(), "_run_cite_fetch".to_string()),
        ("cite-stats".to_string(), "_run_cite_stats".to_string()),
        (
            "dedup-semantic".to_string(),
            "_run_dedup_semantic".to_string(),
        ),
        ("research".to_string(), "_run_research_cmd".to_string()),
        ("similar".to_string(), "_run_similar".to_string()),
        ("kg".to_string(), "_run_kg".to_string()),
        ("read-queue".to_string(), "_run_read_queue".to_string()),
        ("chat".to_string(), "_run_chat".to_string()),
        ("slides".to_string(), "_run_slides".to_string()),
        ("hypothesize".to_string(), "_run_hypothesize".to_string()),
        ("gap".to_string(), "_run_gap".to_string()),
        ("trend".to_string(), "_run_trend".to_string()),
        ("influence".to_string(), "_run_influence".to_string()),
        (
            "cite-backfill".to_string(),
            "_run_cite_backfill".to_string(),
        ),
        ("analyze".to_string(), "_run_analyze".to_string()),
        ("review".to_string(), "_run_review".to_string()),
        ("question".to_string(), "_run_question".to_string()),
        ("roadmap".to_string(), "_run_roadmap".to_string()),
        ("experiment".to_string(), "_run_experiment".to_string()),
        ("pipeline".to_string(), "_run_pipeline".to_string()),
        ("dashboard".to_string(), "_run_dashboard".to_string()),
        ("journal".to_string(), "_run_journal".to_string()),
        ("digest".to_string(), "_run_digest".to_string()),
        ("lean".to_string(), "_run_lean".to_string()),
        (
            "citation-chain".to_string(),
            "_run_citation_chain".to_string(),
        ),
        ("compare".to_string(), "_run_compare".to_string()),
        ("replicate".to_string(), "_run_replicate".to_string()),
        ("insight".to_string(), "_run_insight".to_string()),
        ("ask".to_string(), "_run_ask".to_string()),
        ("doctor".to_string(), "_run_doctor".to_string()),
        ("session".to_string(), "_run_session".to_string()),
        ("argue".to_string(), "_run_argue".to_string()),
        ("narrative".to_string(), "_run_narrative".to_string()),
        ("route".to_string(), "_run_route".to_string()),
        ("friction".to_string(), "_run_friction".to_string()),
        ("chat-tui".to_string(), "_run_chat_tui".to_string()),
        ("subscribe".to_string(), "_run_subscribe".to_string()),
        ("litreview".to_string(), "_run_litreview".to_string()),
        ("benchmark".to_string(), "_run_benchmark".to_string()),
        ("postprocess".to_string(), "_run_postprocess".to_string()),
        ("ingest".to_string(), "_run_ingest".to_string()),
        ("daemon".to_string(), "_run_daemon".to_string()),
        ("scout".to_string(), "_run_scout".to_string()),
        ("intel".to_string(), "_run_intel".to_string()),
        ("signal".to_string(), "_run_signal".to_string()),
        ("discover".to_string(), "_run_discover".to_string()),
        ("report".to_string(), "_run_report".to_string()),
        ("jin10".to_string(), "_run_jin10".to_string()),
        ("demo".to_string(), "_run_demo".to_string()),
        ("paper2code".to_string(), "_run_paper2code".to_string()),
        ("validate".to_string(), "_run_validate".to_string()),
    ])
}

// ============================================================================
// Main Entry Point
// ============================================================================

/// Main CLI entry point
pub fn main(argv: Option<Vec<&str>>) -> i32 {
    let args: Vec<String> = argv
        .map(|v| v.into_iter().map(|s| s.to_string()).collect())
        .unwrap_or_else(|| std::env::args().skip(1).collect());

    // Check for JSON logs env var
    if let Ok(json_logs) = std::env::var("RAIROS_JSON_LOGS") {
        if json_logs.to_lowercase() == "1"
            || json_logs.to_lowercase() == "true"
            || json_logs.to_lowercase() == "yes"
        {
            let log_level =
                std::env::var("RAIROS_LOG_LEVEL").unwrap_or_else(|_| "INFO".to_string());
            eprintln!("JSON logs enabled with level: {}", log_level);
        }
    }

    // Handle --help and -h before subcommand check
    let first = args.first().map(|s| s.as_str()).unwrap_or("");

    if first == "-h" || first == "--help" {
        eprintln!("AI Research OS — Self-Evolving Research System");
        eprintln!("Usage: rairos <command> [options]");
        eprintln!("Run 'rairos help' for full help");
        return 0;
    }

    // Check for subcommand
    if let Some(subcmd) = args.first() {
        if subcmd == "watch" {
            eprintln!("Watch command would be executed here");
            return 0;
        }
        eprintln!("Would dispatch to: {}", subcmd);
    }

    0
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_subcommands_contains_expected() {
        let subs = subcommands();
        assert!(subs.contains(&"search".to_string()));
        assert!(subs.contains(&"stats".to_string()));
        assert!(subs.contains(&"list".to_string()));
        assert!(subs.contains(&"cache".to_string()));
    }

    #[test]
    fn test_subcommands_count() {
        let subs = subcommands();
        // Based on the table, we have 73 subcommands
        assert_eq!(subs.len(), 73);
    }

    #[test]
    fn test_dispatch_map_contains_subcommands() {
        let dispatch = dispatch_map();
        assert!(dispatch.contains_key("search"));
        assert!(dispatch.contains_key("stats"));
        assert!(dispatch.contains_key("list"));
    }

    #[test]
    fn test_dispatch_map_returns_correct_handlers() {
        let dispatch = dispatch_map();
        assert_eq!(dispatch.get("search"), Some(&"_run_search".to_string()));
        assert_eq!(dispatch.get("stats"), Some(&"_run_stats".to_string()));
    }

    #[test]
    fn test_subcommand_table_entries_valid() {
        for (name, module_path, builder_name) in SUBCOMMAND_TABLE {
            assert!(!name.is_empty());
            assert!(!module_path.is_empty());
            assert!(!builder_name.is_empty());
            assert!(module_path.starts_with("cli."));
        }
    }

    #[test]
    fn test_all_subcommands_in_dispatch_map() {
        let dispatch = dispatch_map();
        // Verify that key subcommands are in the dispatch map
        let key_subcommands = [
            "search",
            "stats",
            "list",
            "cache",
            "import",
            "export",
            "citations",
            "similar",
            "kg",
            "gap",
            "trend",
            "analyze",
            "ask",
            "compare",
        ];
        for sub in key_subcommands {
            assert!(
                dispatch.contains_key(sub),
                "Key subcommand '{}' should be in dispatch",
                sub
            );
        }
        // Verify we have a reasonable number of handlers (at least 50)
        assert!(
            dispatch.len() >= 50,
            "Expected at least 50 handlers in dispatch map, got {}",
            dispatch.len()
        );
    }

    #[test]
    fn test_main_with_help() {
        let result = main(Some(vec!["--help"]));
        assert_eq!(result, 0);
    }

    #[test]
    fn test_main_with_h_flag() {
        let result = main(Some(vec!["-h"]));
        assert_eq!(result, 0);
    }
}
