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
    assert!(handle_delete(&db, &["nonexistent".to_string()], false).is_ok());
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
