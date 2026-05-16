//! Benchmark Leaderboard — ranked paper2code implementations.

#![allow(dead_code)]
//!
//! Persists benchmark results keyed by arxiv_id, ranks by combined score:
//!   combined_score = pass_rate × 0.7 + coverage_ratio × 0.3
//!
//! # Closure
//! paper2code pipeline (run_benchmark) → upsert_leaderboard_entry
//! → ranked leaderboard → MCP tool for status/rankings/render_html

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

/// Home directory data root
fn gp_dir() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".ai_research_os")
        .join("evolution")
}

fn leaderboard_file() -> PathBuf {
    gp_dir().join("leaderboard.json")
}

fn cross_domain_file() -> PathBuf {
    gp_dir().join("cross_domain.json")
}

/// Minimum domains a capsule must pass in to be a transfer capsule
const TRANSFER_MIN_DOMAINS: usize = 2;
/// Minimum pass_rate per domain to count toward transfer
const TRANSFER_MIN_PASS_RATE: f64 = 0.5;

/// Weights for combined score
const PASS_RATE_WEIGHT: f64 = 0.7;
const COVERAGE_WEIGHT: f64 = 0.3;

// ─── LeaderboardEntry ────────────────────────────────────────────────────────

/// Difficulty thresholds for stub-rate-based penalty
const STUB_RATE_HIGH: f64 = 0.70;
const STUB_RATE_MEDIUM: f64 = 0.40;
const PENALTY_HIGH: f64 = 0.40;
const PENALTY_MEDIUM: f64 = 0.20;
const PENALTY_LOW: f64 = 0.05;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LeaderboardEntry {
    pub arxiv_id: String,
    pub title: String,
    pub passed: i32,
    pub failed: i32,
    pub skipped: i32,
    pub duration_seconds: f64,
    /// passed / (passed + failed)
    pub pass_rate: f64,
    /// from BenchmarkResult
    pub coverage_ratio: f64,
    /// weighted composite (raw)
    pub combined_score: f64,
    /// combined_score × (1 - difficulty_penalty)
    pub calibrated_score: f64,
    /// 0.0–0.5 based on stub rate
    pub difficulty_penalty: f64,
    /// skipped / total tests
    pub stub_rate: f64,
    pub framework: String,
    pub capsule_id: String,
    /// ISO timestamp
    pub last_updated: String,
    pub numerical_claims_total: i32,
    pub numerical_claims_covered: i32,
    /// e.g. "vision", "nlp", "reasoning" — for cross-domain detection
    pub benchmark_domain: String,
}

impl Default for LeaderboardEntry {
    fn default() -> Self {
        Self {
            arxiv_id: String::new(),
            title: String::new(),
            passed: 0,
            failed: 0,
            skipped: 0,
            duration_seconds: 0.0,
            pass_rate: 0.0,
            coverage_ratio: 0.0,
            combined_score: 0.0,
            calibrated_score: 0.0,
            difficulty_penalty: 0.0,
            stub_rate: 0.0,
            framework: "pytorch".to_string(),
            capsule_id: String::new(),
            last_updated: String::new(),
            numerical_claims_total: 0,
            numerical_claims_covered: 0,
            benchmark_domain: String::new(),
        }
    }
}

impl LeaderboardEntry {
    /// Compute both raw combined_score and calibrated_score with difficulty penalty.
    pub fn compute_score(&mut self) -> f64 {
        let total = self.passed + self.failed + self.skipped;
        self.stub_rate = if total > 0 {
            (self.skipped as f64 / total as f64 * 1000.0).round() / 1000.0
        } else {
            0.0
        };

        // Difficulty penalty based on stub rate
        self.difficulty_penalty = if self.stub_rate >= STUB_RATE_HIGH {
            PENALTY_HIGH
        } else if self.stub_rate >= STUB_RATE_MEDIUM {
            PENALTY_MEDIUM
        } else {
            PENALTY_LOW
        };

        // Raw combined score
        self.combined_score = (self.pass_rate * PASS_RATE_WEIGHT
            + self.coverage_ratio * COVERAGE_WEIGHT * 100.0)
            / 100.0;

        // Calibrated: penalize easy papers
        self.calibrated_score =
            (self.combined_score * (1.0 - self.difficulty_penalty) * 10000.0).round() / 10000.0;

        self.calibrated_score
    }
}

// ─── CrossDomainEntry ─────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CrossDomainEntry {
    pub capsule_id: String,
    /// domain → pass_rate
    pub domains: HashMap<String, f64>,
    pub is_transfer_capsule: bool,
    /// domains where it passes well
    pub transfer_domains: Vec<String>,
}

// ─── Leaderboard ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct LeaderboardData {
    version: String,
    updated_at: String,
    entries: Vec<LeaderboardEntry>,
}

#[derive(Debug, Clone)]
pub struct Leaderboard {
    entries: HashMap<String, LeaderboardEntry>,
}

impl Default for Leaderboard {
    fn default() -> Self {
        Self::new()
    }
}

impl Leaderboard {
    pub fn new() -> Self {
        let mut board = Self {
            entries: HashMap::new(),
        };
        board._load();
        board
    }

    fn _load(&mut self) {
        let path = leaderboard_file();
        if !path.exists() {
            return;
        }
        match fs::read_to_string(&path) {
            Ok(text) => match serde_json::from_str::<LeaderboardData>(&text) {
                Ok(data) => {
                    for entry in data.entries {
                        self.entries.insert(entry.arxiv_id.clone(), entry);
                    }
                }
                Err(e) => {
                    eprintln!("[leaderboard] failed to parse {}: {}", path.display(), e);
                }
            },
            Err(e) => {
                eprintln!("[leaderboard] failed to read {}: {}", path.display(), e);
            }
        }
    }

    fn _save(&self) {
        let dir = gp_dir();
        if let Err(e) = fs::create_dir_all(&dir) {
            eprintln!(
                "[leaderboard] failed to create dir {}: {}",
                dir.display(),
                e
            );
            return;
        }

        let mut entries: Vec<LeaderboardEntry> = self.entries.values().cloned().collect();
        entries.sort_by(|a, b| {
            b.calibrated_score
                .partial_cmp(&a.calibrated_score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        let data = LeaderboardData {
            version: "1.0".to_string(),
            updated_at: now_iso(),
            entries,
        };

        let path = leaderboard_file();
        match serde_json::to_string_pretty(&data) {
            Ok(json) => match fs::write(&path, json) {
                Ok(_) => {}
                Err(e) => {
                    eprintln!("[leaderboard] failed to write {}: {}", path.display(), e);
                }
            },
            Err(e) => {
                eprintln!("[leaderboard] failed to serialize: {}", e);
            }
        }
    }

    /// Add or update an entry
    pub fn upsert(&mut self, mut entry: LeaderboardEntry) {
        entry.last_updated = now_iso();
        entry.compute_score();
        self.entries.insert(entry.arxiv_id.clone(), entry);
        self._save();
    }

    /// Get a single entry
    pub fn get(&self, arxiv_id: &str) -> Option<&LeaderboardEntry> {
        self.entries.get(arxiv_id)
    }

    /// Top entries sorted by calibrated_score descending
    pub fn rankings(&self, limit: usize) -> Vec<&LeaderboardEntry> {
        let mut entries: Vec<&LeaderboardEntry> = self.entries.values().collect();
        entries.sort_by(|a, b| {
            b.calibrated_score
                .partial_cmp(&a.calibrated_score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        entries.truncate(limit);
        entries
    }

    /// Top entries sorted by pass_rate descending
    pub fn rankings_by_pass_rate(&self, limit: usize) -> Vec<&LeaderboardEntry> {
        let mut entries: Vec<&LeaderboardEntry> = self.entries.values().collect();
        entries.sort_by(|a, b| {
            b.pass_rate
                .partial_cmp(&a.pass_rate)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        entries.truncate(limit);
        entries
    }

    /// Top entries sorted by coverage_ratio descending
    pub fn rankings_by_coverage(&self, limit: usize) -> Vec<&LeaderboardEntry> {
        let mut entries: Vec<&LeaderboardEntry> = self.entries.values().collect();
        entries.sort_by(|a, b| {
            b.coverage_ratio
                .partial_cmp(&a.coverage_ratio)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        entries.truncate(limit);
        entries
    }

    pub fn total_count(&self) -> usize {
        self.entries.len()
    }

    pub fn avg_pass_rate(&self) -> f64 {
        if self.entries.is_empty() {
            return 0.0;
        }
        let sum: f64 = self.entries.values().map(|e| e.pass_rate).sum();
        (sum / self.entries.len() as f64 * 1000.0).round() / 1000.0
    }

    pub fn avg_coverage(&self) -> f64 {
        let with_cov: f64 = self
            .entries
            .values()
            .filter(|e| e.coverage_ratio > 0.0)
            .map(|e| e.coverage_ratio)
            .sum();
        let count = self
            .entries
            .values()
            .filter(|e| e.coverage_ratio > 0.0)
            .count();
        if count == 0 {
            return 0.0;
        }
        (with_cov / count as f64 * 1000.0).round() / 1000.0
    }
}

// ─── Cross-domain detection ──────────────────────────────────────────────────

/// Find capsules that achieve good pass rates across multiple benchmark domains.
/// Returns capsule_id → CrossDomainEntry
pub fn detect_transfer_capsules() -> HashMap<String, CrossDomainEntry> {
    let board = Leaderboard::new();
    let mut capsule_domains: HashMap<String, CrossDomainEntry> = HashMap::new();

    for entry in board.entries.values() {
        if entry.capsule_id.is_empty() || entry.benchmark_domain.is_empty() {
            continue;
        }
        if entry.pass_rate < TRANSFER_MIN_PASS_RATE {
            continue;
        }

        let cde = capsule_domains
            .entry(entry.capsule_id.clone())
            .or_insert_with(|| CrossDomainEntry {
                capsule_id: entry.capsule_id.clone(),
                domains: HashMap::new(),
                is_transfer_capsule: false,
                transfer_domains: Vec::new(),
            });

        cde.domains
            .insert(entry.benchmark_domain.clone(), entry.pass_rate);
    }

    for cde in capsule_domains.values_mut() {
        let qualifying: Vec<&String> = cde
            .domains
            .iter()
            .filter(|(_, &pr)| pr >= TRANSFER_MIN_PASS_RATE)
            .map(|(d, _)| d)
            .collect();
        if qualifying.len() >= TRANSFER_MIN_DOMAINS {
            cde.is_transfer_capsule = true;
            cde.transfer_domains = qualifying.into_iter().cloned().collect();
        }
    }

    capsule_domains
}

/// All transfer capsules as a list of dicts
pub fn get_transfer_capsules() -> Vec<serde_json::Value> {
    let transfer_map = detect_transfer_capsules();
    transfer_map
        .values()
        .filter(|cde| cde.is_transfer_capsule)
        .map(|cde| {
            serde_json::json!({
                "capsule_id": cde.capsule_id,
                "domains": cde.domains,
                "domain_count": cde.domains.len(),
                "is_transfer_capsule": cde.is_transfer_capsule,
                "transfer_domains": cde.transfer_domains,
            })
        })
        .collect()
}

// ─── Upsert from benchmark result ─────────────────────────────────────────────

/// Benchmark result fields we care about (generic to avoid external dependency)
#[derive(Debug, Clone, Default)]
pub struct BenchmarkResult {
    pub passed: i32,
    pub failed: i32,
    pub skipped: i32,
    pub duration_seconds: f64,
    pub coverage_ratio: f64,
    pub numerical_claims_total: i32,
    pub numerical_claims_covered: i32,
}

/// Create or update a leaderboard entry from a BenchmarkResult object.
pub fn upsert_from_benchmark(
    arxiv_id: &str,
    result: &BenchmarkResult,
    paper_title: &str,
    framework: &str,
    capsule_id: &str,
    benchmark_domain: &str,
) -> LeaderboardEntry {
    let total = result.passed + result.failed;
    let pass_rate = if total > 0 {
        result.passed as f64 / total as f64
    } else {
        0.0
    };

    let mut entry = LeaderboardEntry {
        arxiv_id: arxiv_id.to_string(),
        title: if paper_title.is_empty() {
            arxiv_id.to_string()
        } else {
            paper_title.chars().take(100).collect()
        },
        passed: result.passed,
        failed: result.failed,
        skipped: result.skipped,
        duration_seconds: result.duration_seconds,
        pass_rate: (pass_rate * 10000.0).round() / 10000.0,
        coverage_ratio: result.coverage_ratio,
        framework: framework.to_string(),
        capsule_id: capsule_id.to_string(),
        numerical_claims_total: result.numerical_claims_total,
        numerical_claims_covered: result.numerical_claims_covered,
        benchmark_domain: benchmark_domain.to_string(),
        ..Default::default()
    };
    entry.compute_score();

    let mut board = Leaderboard::new();
    board.upsert(entry.clone());
    entry
}

// ─── HTML rendering ───────────────────────────────────────────────────────────

/// Render the leaderboard as an HTML table.
pub fn render_leaderboard_html(sort_by: &str, limit: usize) -> String {
    let board = Leaderboard::new();

    let entries: Vec<&LeaderboardEntry> = match sort_by {
        "pass_rate" => board.rankings_by_pass_rate(limit),
        "coverage" => board.rankings_by_coverage(limit),
        _ => board.rankings(limit),
    };

    let avg_pr = board.avg_pass_rate();
    let avg_cov = board.avg_coverage();

    let mut rows_html = String::new();
    for (idx, e) in entries.iter().enumerate() {
        let rank = idx + 1;
        let score_color = if e.combined_score > 0.7 {
            "#3fb950"
        } else if e.combined_score > 0.4 {
            "#f0883e"
        } else {
            "#8b949e"
        };
        let title_short = if e.title.len() > 50 {
            format!("{}...", &e.title[..50])
        } else {
            e.title.clone()
        };
        rows_html.push_str(&format!(
            "<tr>\
  <td style=\"text-align:center;color:#8b949e\">{}</td>\
  <td><a href=\"https://arxiv.org/abs/{}\" target=\"_blank\" style=\"color:#58a6ff\">{}</a></td>\
  <td style=\"color:#e6edf3\">{}</td>\
  <td style=\"text-align:center;color:#3fb950\">{}</td>\
  <td style=\"text-align:center;color:#f85149\">{}</td>\
  <td style=\"text-align:center;color:#8b949e\">{}</td>\
  <td style=\"text-align:center\">{:.1}%</td>\
  <td style=\"text-align:center\">{:.1}%</td>\
  <td style=\"text-align:center;font-weight:bold;color:{}\">{:.3}</td>\
  <td style=\"text-align:center;color:#8b949e\">{}</td>\
</tr>",
            rank,
            e.arxiv_id,
            e.arxiv_id,
            title_short,
            e.passed,
            e.failed,
            e.skipped,
            e.pass_rate * 100.0,
            e.coverage_ratio * 100.0,
            score_color,
            e.combined_score,
            e.framework
        ));
    }

    format!(
        r#"<!DOCTYPE html>
<html>
<head>
<meta charset="utf-8">
<title>Paper2code Leaderboard</title>
<style>
  body {{ font-family: -apple-system, BlinkMacSystemFont, sans-serif; background: #0d1117; color: #e6edf3; margin: 20px; }}
  h2 {{ color: #58a6ff; }}
  table {{ border-collapse: collapse; width: 100%; max-width: 900px; }}
  th {{ background: #161b22; color: #8b949e; padding: 8px 12px; text-align:left; font-size: 12px; text-transform: uppercase; }}
  td {{ padding: 8px 12px; border-bottom: 1px solid #21262d; font-size: 13px; }}
  tr:hover {{ background: #161b22; }}
  .sort-links {{ margin-bottom: 16px; }}
  .sort-links a {{ color: #58a6ff; margin-right: 16px; text-decoration: none; }}
  .sort-links a.active {{ color: #3fb950; font-weight: bold; }}
  .summary {{ color: #8b949e; font-size: 13px; margin-bottom: 16px; }}
</style>
</head>
<body>
<h2>📊 Paper2code Benchmark Leaderboard</h2>
<div class="summary">
  {} implementations · avg pass rate: {:.1}% · avg coverage: {:.1}%
</div>
<div class="sort-links">
  <a href="?sort=combined" class="{}">Combined Score</a>
  <a href="?sort=pass_rate" class="{}">Pass Rate</a>
  <a href="?sort=coverage" class="{}">Coverage</a>
</div>
<table>
<thead>
<tr>
  <th>#</th><th>arXiv ID</th><th>Title</th>
  <th style="text-align:center">✓ Pass</th>
  <th style="text-align:center">✗ Fail</th>
  <th style="text-align:center">⊘ Skip</th>
  <th style="text-align:center">Pass Rate</th>
  <th style="text-align:center">Coverage</th>
  <th style="text-align:center">Score</th>
  <th style="text-align:center">Framework</th>
</tr>
</thead>
<tbody>
{}
</tbody>
</table>
</body>
</html>"#,
        board.total_count(),
        avg_pr * 100.0,
        avg_cov * 100.0,
        if sort_by == "combined" { "active" } else { "" },
        if sort_by == "pass_rate" { "active" } else { "" },
        if sort_by == "coverage" { "active" } else { "" },
        rows_html
    )
}

// ─── MCP tool dispatcher ──────────────────────────────────────────────────────

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LeaderboardAction {
    pub action: String,
    pub arxiv_id: Option<String>,
    #[serde(default = "default_sort_by")]
    pub sort_by: String,
    #[serde(default = "default_limit")]
    pub limit: usize,
}

fn default_sort_by() -> String {
    "combined".to_string()
}

fn default_limit() -> usize {
    20
}

/// MCP tool dispatcher for Benchmark Leaderboard.
/// Actions: status | rankings | upsert | render | entry | transfer_capsules
pub fn leaderboard_action(
    action: &str,
    arxiv_id: Option<&str>,
    sort_by: &str,
    limit: usize,
) -> serde_json::Value {
    let board = Leaderboard::new();

    match action {
        "status" => serde_json::json!({
            "total_implementations": board.total_count(),
            "avg_pass_rate": board.avg_pass_rate(),
            "avg_coverage_ratio": board.avg_coverage(),
            "file": leaderboard_file().to_string_lossy(),
        }),

        "rankings" => {
            let entries: Vec<&LeaderboardEntry> = match sort_by {
                "pass_rate" => board.rankings_by_pass_rate(limit),
                "coverage" => board.rankings_by_coverage(limit),
                _ => board.rankings(limit),
            };
            serde_json::json!({
                "rankings": entries.iter().enumerate().map(|(idx, e)| {
                    serde_json::json!({
                        "rank": idx + 1,
                        "arxiv_id": e.arxiv_id,
                        "title": e.title,
                        "passed": e.passed,
                        "failed": e.failed,
                        "skipped": e.skipped,
                        "pass_rate": e.pass_rate,
                        "coverage_ratio": e.coverage_ratio,
                        "combined_score": e.combined_score,
                        "calibrated_score": e.calibrated_score,
                        "difficulty_penalty": e.difficulty_penalty,
                        "stub_rate": e.stub_rate,
                        "framework": e.framework,
                        "last_updated": e.last_updated,
                    })
                }).collect::<Vec<_>>(),
                "total": board.total_count(),
                "sort_by": sort_by,
                "note": "rankings sorted by calibrated_score (difficulty-adjusted)",
            })
        }

        "upsert" => {
            if arxiv_id.is_none() {
                return serde_json::json!({"error": "arxiv_id required for upsert"});
            }
            let existing = board.get(arxiv_id.unwrap());
            serde_json::json!({
                "arxiv_id": arxiv_id,
                "existing": existing.cloned(),
                "message": "Use upsert_from_benchmark() after run_benchmark() to auto-populate",
            })
        }

        "render" => {
            let html = render_leaderboard_html(sort_by, limit);
            serde_json::json!({
                "html": html,
                "size_kb": (html.len() as f64 / 1024.0 * 10.0).round() / 10.0,
            })
        }

        "entry" => {
            if arxiv_id.is_none() {
                return serde_json::json!({"error": "arxiv_id required for entry"});
            }
            let e = board.get(arxiv_id.unwrap());
            match e {
                Some(entry) => serde_json::to_value(entry).unwrap_or(serde_json::json!({})),
                None => {
                    serde_json::json!({"error": format!("No entry found for {}", arxiv_id.unwrap())})
                }
            }
        }

        "transfer_capsules" => {
            let capsules = get_transfer_capsules();
            serde_json::json!({
                "transfer_capsules": capsules,
                "total": capsules.len(),
                "note": format!(
                    "capsules with pass_rate >={} in >={} domains",
                    TRANSFER_MIN_PASS_RATE, TRANSFER_MIN_DOMAINS
                ),
            })
        }

        _ => serde_json::json!({
            "error": format!("Unknown action: {}", action)
        }),
    }
}

// ─── Utility ──────────────────────────────────────────────────────────────────

fn now_iso() -> String {
    chrono::Utc::now().format("%Y-%m-%dT%H:%M:%S").to_string()
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_leaderboard_entry_score() {
        let mut entry = LeaderboardEntry {
            arxiv_id: "test".to_string(),
            passed: 7,
            failed: 2,
            skipped: 1,
            duration_seconds: 10.0,
            pass_rate: 7.0 / 9.0,
            coverage_ratio: 0.65,
            ..Default::default()
        };

        let score = entry.compute_score();
        assert!((0.0..=1.0).contains(&score));
        assert!(entry.stub_rate > 0.0);
        assert!(entry.difficulty_penalty > 0.0);
    }

    #[test]
    fn test_leaderboard_rankings() {
        let mut board = Leaderboard::new();
        // Insert some entries
        for i in 0..3 {
            let mut entry = LeaderboardEntry {
                arxiv_id: format!("test_{}", i),
                title: format!("Test Paper {}", i),
                passed: (10 - i * 2),
                failed: 2,
                skipped: 0,
                duration_seconds: 1.0,
                pass_rate: (10 - i * 2) as f64 / 12.0,
                coverage_ratio: 0.5 + i as f64 * 0.1,
                ..Default::default()
            };
            entry.compute_score();
            board.upsert(entry);
        }

        let ranked = board.rankings(10);
        assert!(!ranked.is_empty());
        // First entry should have highest calibrated_score
        if ranked.len() > 1 {
            assert!(ranked[0].calibrated_score >= ranked[1].calibrated_score);
        }
    }

    #[test]
    fn test_render_html() {
        let html = render_leaderboard_html("combined", 10);
        assert!(html.contains("Paper2code Leaderboard"));
        assert!(html.contains("<table>"));
    }
}
