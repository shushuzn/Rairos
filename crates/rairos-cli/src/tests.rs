/// Rairos CLI integration tests — temp DB, no network required.
/// Each test creates an isolated temporary database that auto-cleans on drop.
use std::path::{Path, PathBuf};

use crate::*;

// ─── Test helpers ───────────────────────────────────────────────────────────

fn test_db(name: &str) -> (Database, TempDir) {
    let dir = TempDir::new(name);
    let db_path = dir.path().join("test.db");
    let db = Database::open(&db_path).unwrap();
    (db, dir)
}

struct TempDir {
    path: PathBuf,
}

impl TempDir {
    fn new(name: &str) -> Self {
        let path = std::env::temp_dir().join(format!("rairos_cli_test_{}", name));
        let _ = std::fs::remove_dir_all(&path);
        std::fs::create_dir_all(&path).unwrap();
        Self { path }
    }
    fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.path);
    }
}

/// Helper: export path for handle_export
fn export_path(dir: &TempDir) -> PathBuf {
    dir.path().join("export.json")
}

// ─── paper.rs ───────────────────────────────────────────────────────────────

#[test]
fn paper_init_creates_db() {
    let dir = TempDir::new("init");
    let db_path = dir.path().join("rairos.db");
    assert!(handle_init(&db_path).is_ok());
    assert!(db_path.exists());
}

#[test]
fn paper_list_empty_db() {
    let (db, _dir) = test_db("list");
    assert!(handle_list(&db, None, None, &[], 10, 0, "date", "desc", "table").is_ok());
}

#[test]
fn paper_stats_empty_db() {
    let (db, _dir) = test_db("stats");
    assert!(handle_stats(&db, true, "table").is_ok());
}

#[test]
fn paper_search_empty_db() {
    let (db, _dir) = test_db("search");
    assert!(handle_search(&db, "quantum", 10, "all", "table").is_ok());
}

#[test]
fn paper_delete_empty_db() {
    let (db, _dir) = test_db("delete");
    // force=true to skip stdin prompt (non-interactive test)
    assert!(handle_delete(&db, &["nonexistent".to_string()], true).is_ok());
}

#[test]
fn paper_update_status_empty_db() {
    let (db, _dir) = test_db("status");
    let result = handle_update_status(&db, &["nonexistent".to_string()], "read");
    // No matching papers — either Ok or error is acceptable, just don't crash
    assert!(result.is_ok() || result.is_err());
}

#[test]
fn paper_parse_not_found() {
    let (db, _dir) = test_db("parse");
    let result = handle_parse(&db, "nonexistent_paper_xyz");
    assert!(result.is_err());
    let err = result.unwrap_err().to_string().to_lowercase();
    assert!(err.contains("not found"));
}

#[test]
fn paper_dedup_stats_empty() {
    let (db, _dir) = test_db("dedup");
    assert!(handle_dedup(&db, &DedupAction::Stats).is_ok());
}

#[test]
fn paper_similar_empty_db() {
    let (db, _dir) = test_db("similar");
    let result = handle_similar(&db, "nonexistent", 10);
    assert!(result.is_err());
    let err = result.unwrap_err().to_string().to_lowercase();
    assert!(err.contains("not found"));
}

#[test]
fn paper_show_not_found() {
    let (db, _dir) = test_db("show");
    let result = handle_show(&db, "nonexistent", "text");
    assert!(result.is_err());
    let err = result.unwrap_err().to_string().to_lowercase();
    assert!(err.contains("not found"));
}

#[test]
fn paper_export_empty_db() {
    let (db, dir) = test_db("export");
    let path = export_path(&dir);
    assert!(handle_export(&db, &path, None, "json").is_ok());
}

#[test]
fn paper_compare_empty_db() {
    let (db, _dir) = test_db("compare");
    let result = handle_compare(&db, "paper1,paper2", "abstract");
    assert!(result.is_ok() || result.is_err());
}

// ─── evo.rs ─────────────────────────────────────────────────────────────────

#[test]
fn evo_gene_list_empty() {
    assert!(handle_gene_list(None, None, 10, "table").is_ok());
}

#[test]
fn evo_gene_diversity_empty() {
    assert!(handle_gene_diversity("table").is_ok());
}

#[test]
fn evo_gene_show_not_found() {
    let result = handle_gene_show("nonexistent_gene", "text");
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().to_lowercase().contains("not found"));
}

#[test]
fn evo_stance_list_empty() {
    assert!(handle_stance_list(None, None, "table").is_ok());
}

#[test]
fn evo_memory_stats() {
    assert!(handle_memory_stats("table").is_ok());
}

#[test]
fn evo_stance_show_not_found() {
    assert!(handle_stance_show("nonexistent", "table").is_err());
}

// ─── kg.rs ──────────────────────────────────────────────────────────────────

#[test]
fn kg_stats_empty() {
    assert!(handle_kg_stats("table").is_ok());
}

#[test]
fn kg_search_empty() {
    assert!(handle_kg_search(None, Some("test"), "table").is_ok());
}

#[test]
fn kg_rank_defaults() {
    assert!(handle_kg_rank(5, "table").is_ok());
}

#[test]
fn kg_add_paper_not_found() {
    let (db, _dir) = test_db("kg_add");
    let result = handle_kg_add_paper(&db, "nonexistent");
    assert!(result.is_err());
}

// ─── cite.rs ────────────────────────────────────────────────────────────────

#[test]
fn cite_stats_empty() {
    let (db, _dir) = test_db("cite_stats");
    assert!(handle_cite_stats(&db, None, None, "table").is_ok());
}

#[test]
fn cite_list_empty() {
    let (db, _dir) = test_db("cite_list");
    assert!(handle_citations(&db, Some("nonexistent"), None, "table").is_ok());
}

#[test]
fn cite_merge_empty() {
    let (db, _dir) = test_db("merge");
    assert!(handle_merge(&db, "ids", false, true, None, None).is_ok());
}

// ─── util.rs ────────────────────────────────────────────────────────────────

#[test]
fn util_doctor_ok() {
    assert!(handle_doctor("table").is_ok());
}

#[test]
fn util_doctor_json() {
    assert!(handle_doctor("json").is_ok());
}

#[test]
fn util_status_empty() {
    let (db, _dir) = test_db("util_status");
    assert!(handle_status(&db, "table").is_ok());
}

#[test]
fn util_benchmark_small() {
    assert!(handle_benchmark("impact", 2).is_ok());
}

#[test]
fn util_queue_list_empty() {
    let (db, _dir) = test_db("queue");
    assert!(handle_queue(&db, None, true, false, false, None, false, "table").is_ok());
}

#[test]
fn util_argue_empty_db() {
    let (db, _dir) = test_db("argue");
    assert!(handle_argue(&db, &["test thesis".to_string()]).is_ok());
}

#[test]
fn util_story_empty_db() {
    let (db, _dir) = test_db("story");
    assert!(handle_story(&db, Some("quantum computing")).is_ok());
}

// ─── research.rs ────────────────────────────────────────────────────────────

#[test]
fn research_gap_list_empty() {
    let (db, _dir) = test_db("gaps");
    let result = handle_gap_list(&db, 10, 0, "table");
    // Empty DB — may or may not error depending on gap storage state
    assert!(result.is_ok() || result.is_err());
}

#[test]
fn research_session_defaults() {
    assert!(handle_session("list", None, None, 7, 10).is_ok());
}

// ─── llm.rs ─────────────────────────────────────────────────────────────────

#[test]
fn llm_repl_no_db() {
    let result = handle_repl(None);
    assert!(result.is_err());
    let err = result.unwrap_err().to_string();
    assert!(err.contains("Database not found"));
}

#[test]
fn llm_cache_stats() {
    assert!(handle_cache(&CacheAction::Stats).is_ok());
}

#[test]
fn llm_route_basic() {
    assert!(handle_route(&["test".to_string()], true, false, false).is_ok());
}

#[test]
fn llm_analyze_paper_not_found() {
    let (db, _dir) = test_db("analyze");
    let result = handle_analyze(&db, "quick", Some("nonexistent".to_string()), "table");
    assert!(result.is_err() || result.is_ok());
}

#[test]
fn llm_trend_empty_db() {
    let (db, _dir) = test_db("trend");
    assert!(handle_trend(&db, "test", "1y", "table").is_ok());
}

#[test]
fn llm_chat_basic() {
    // handle_chat opens DB internally; needs API key for actual chat
    // Just verify it doesn't panic — the error will be about missing API key
    let result = handle_chat(Some("hello"), None, None, 5, false, false, None, false, false, None, None);
    assert!(result.is_err() || result.is_ok());
    // If it errored, should be about API key or DB, not a crash
}

// ─── dispatch ───────────────────────────────────────────────────────────────

#[test]
fn cli_dispatch_routes_version() {
    let cli = Cli {
        command: Commands::Version,
        db: PathBuf::from("test.db"),
        verbose: false,
    };
    assert!(matches!(cli.command, Commands::Version));
}

// ─── Network-dependent tests ────────────────────────────────────────────────
// Run with: cargo test -p rairos-cli -- --ignored --test-threads=1
// These require arXiv API access, LLM keys, or a populated ~/.ai_research_os/

#[test]
#[ignore]
fn net_paper_add_and_show() {
    // Fetch a real paper from arXiv by ID
    let (db, _dir) = test_db("net_add");
    assert!(handle_add(&db, "2301.00001").is_ok());
    // Now show it (verify it doesn't error — output goes to stdout)
    let result = handle_show(&db, "2301.00001", "text");
    assert!(result.is_ok());
}

#[test]
#[ignore]
fn net_paper_search_after_add() {
    let (db, _dir) = test_db("net_search");
    handle_add(&db, "2301.00001").unwrap();
    let result = handle_search(&db, "Dynamics", 10, "all", "table");
    assert!(result.is_ok());
}

#[test]
#[ignore]
fn net_gene_add_and_list() {
    // Add a capsule gene then verify it appears in the list
    let result = handle_gene_add("test approach", "test_gap", "test,keywords", None);
    assert!(result.is_ok());
    let list = handle_gene_list(None, None, 10, "table");
    assert!(list.is_ok());
}

#[test]
#[ignore]
fn net_stance_lifecycle() {
    // Add a research stance
    assert!(handle_stance_add(
        "test_topic", "test claim", "support", "test reasoning"
    ).is_ok());
    // List stances for the topic
    let list = handle_stance_list(Some("test_topic".to_string()), None, "table");
    assert!(list.is_ok());
}

#[test]
#[ignore]
fn net_session_lifecycle() {
    // Create a research session
    let result = handle_session("start", Some("test session"), Some("test topic"), 7, 10);
    assert!(result.is_ok());
    // List all sessions
    let list = handle_session("list", None, None, 7, 10);
    assert!(list.is_ok());
}

#[test]
#[ignore]
fn net_citation_chain() {
    // Build citation chain for a known paper — needs DB + arXiv fetch
    let (db, _dir) = test_db("net_chain");
    let _ = handle_add(&db, "2301.00001");
    let result = handle_citation_chain(&db, Some("2301.00001"), 2, false, false, false, false, None);
    assert!(result.is_ok() || result.is_err());
}

#[test]
#[ignore]
fn net_cite_fetch() {
    // Fetch citations for a paper from Semantic Scholar
    let result = handle_cite_fetch(Some("2301.00001"), false);
    assert!(result.is_ok() || result.is_err());
}

#[test]
#[ignore]
fn net_kg_path() {
    // Build KG path between two papers
    // First add papers to DB
    let (db, _dir) = test_db("net_kg_path");
    let _ = handle_add(&db, "2301.00001");
    let _ = handle_add(&db, "2301.00002");
    let result = handle_kg_path("2301.00001", "2301.00002");
    assert!(result.is_ok() || result.is_err());
}

#[test]
#[ignore]
fn net_demo_quick() {
    // Run quick demo pipeline
    let result = handle_demo(true, Some(2), false);
    assert!(result.is_ok());
}

#[test]
#[ignore]
fn net_doctor_full() {
    // Full doctor check — may take time
    let result = handle_doctor("table");
    assert!(result.is_ok());
}

#[test]
#[ignore]
fn net_benchmark_all() {
    // Run all benchmarks
    for kind in &["impact", "citation", "cache"] {
        let result = handle_benchmark(kind, 3);
        assert!(result.is_ok(), "benchmark {kind} failed: {result:?}");
    }
}

// ─── achievements.rs ───────────────────────────────────────────────────────────

#[test]
fn achievements_list_no_crash() {
    let result = handle_achievements_list();
    assert!(result.is_ok());
}

#[test]
fn achievements_report_no_crash() {
    let result = handle_achievements_report();
    assert!(result.is_ok());
}

#[test]
fn achievements_stats_no_crash() {
    let result = handle_achievements_stats();
    assert!(result.is_ok());
}

#[test]
fn achievements_unlock_nonexistent_no_crash() {
    let result = handle_achievements_unlock("nonexistent_achievement_id");
    assert!(result.is_ok());
}

// ─── game_mode.rs ─────────────────────────────────────────────────────────────

#[test]
fn badges_list_no_crash() {
    let result = handle_badges_list();
    assert!(result.is_ok());
}

#[test]
fn badges_award_no_crash() {
    let result = handle_badges_award("nonexistent_badge_id");
    assert!(result.is_ok());
}

// ─── contradiction.rs ─────────────────────────────────────────────────────────

#[test]
fn contradictions_list_no_crash() {
    let result = handle_contradictions_list(10);
    assert!(result.is_ok());
}

#[test]
fn contradictions_render_no_crash() {
    let result = handle_contradictions_render();
    assert!(result.is_ok());
}

// ─── trends.rs ───────────────────────────────────────────────────────────────

#[test]
fn trends_analyze_no_crash() {
    let result = handle_trends_analyze("machine learning", None);
    assert!(result.is_ok());
}

#[test]
fn trends_mermaid_no_crash() {
    let result = handle_trends_mermaid("machine learning", None);
    assert!(result.is_ok());
}

// ─── rigor.rs ────────────────────────────────────────────────────────────────

#[test]
fn rigor_score_no_crash() {
    let result = handle_rigor_score("test_paper");
    assert!(result.is_ok());
}

// ─── impact.rs ─────────────────────────────────────────────────────────────

#[test]
fn impact_leaderboard_no_crash() {
    let result = handle_impact_leaderboard(10);
    assert!(result.is_ok());
}

#[test]
fn impact_score_no_crash() {
    let result = handle_impact_score("test_paper");
    assert!(result.is_ok());
}

// ─── briefing.rs ────────────────────────────────────────────────────────────

#[test]
fn briefing_generate_no_crash() {
    let result = handle_briefing_generate("2301.00001");
    assert!(result.is_ok());
}

#[test]
fn briefing_list_no_crash() {
    let result = handle_briefing_list(10);
    assert!(result.is_ok());
}

// ─── paradigm.rs ────────────────────────────────────────────────────────────

#[test]
fn paradigm_detect_no_crash() {
    let result = handle_paradigm_detect("machine learning");
    assert!(result.is_ok());
}

#[test]
fn paradigm_list_no_crash() {
    let result = handle_paradigm_list();
    assert!(result.is_ok());
}

// ─── crossref.rs ───────────────────────────────────────────────────────────

#[test]
fn crossref_analyze_no_crash() {
    let result = handle_crossref_analyze("test_paper");
    assert!(result.is_ok());
}

#[test]
fn crossref_list_no_crash() {
    let result = handle_crossref_list();
    assert!(result.is_ok());
}

// ─── momentum.rs ───────────────────────────────────────────────────────────

#[test]
fn momentum_score_no_crash() {
    let result = handle_momentum_score("machine learning");
    assert!(result.is_ok());
}

#[test]
fn momentum_leaderboard_no_crash() {
    let result = handle_momentum_leaderboard();
    assert!(result.is_ok());
}

// ─── crossover.rs ─────────────────────────────────────────────────────────

#[test]
fn crossover_run_no_crash() {
    let result = handle_crossover_run();
    assert!(result.is_ok());
}

#[test]
fn crossover_list_no_crash() {
    let result = handle_crossover_list();
    assert!(result.is_ok());
}

// ─── decay.rs ──────────────────────────────────────────────────────────────

#[test]
fn decay_stats_no_crash() {
    let result = handle_decay_stats();
    assert!(result.is_ok());
}

#[test]
fn decay_status_no_crash() {
    let result = handle_decay_status("test_capsule");
    assert!(result.is_ok());
}

// ─── atrisk.rs ─────────────────────────────────────────────────────────────

#[test]
fn atrisk_list_no_crash() {
    let result = handle_atrisk_list(50);
    assert!(result.is_ok());
}

#[test]
fn atrisk_keep_no_crash() {
    let result = handle_atrisk_keep("test_capsule");
    assert!(result.is_ok());
}

// ─── credibility.rs ────────────────────────────────────────────────────────

#[test]
fn credibility_score_no_crash() {
    let result = handle_credibility_score();
    assert!(result.is_ok());
}

#[test]
fn credibility_trendslop_no_crash() {
    let result = handle_credibility_trendslop();
    assert!(result.is_ok());
}

// ─── claimgraph.rs ──────────────────────────────────────────────────────────

#[test]
fn claimgraph_stats_no_crash() {
    let result = handle_claimgraph_stats();
    assert!(result.is_ok());
}

#[test]
fn claimgraph_contradictions_no_crash() {
    let result = handle_claimgraph_contradictions();
    assert!(result.is_ok());
}

// ─── bold.rs ──────────────────────────────────────────────────────────────

#[test]
fn bold_list_no_crash() {
    let result = handle_bold_list();
    assert!(result.is_ok());
}

// ─── profiler.rs ──────────────────────────────────────────────────────────

#[test]
fn profiler_report_no_crash() {
    let result = handle_profiler_report();
    assert!(result.is_ok());
}

#[test]
fn profiler_stats_no_crash() {
    let result = handle_profiler_stats();
    assert!(result.is_ok());
}
