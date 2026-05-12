//! Benchmark Runner — run pytest tests and encode results to Gene Pool.
//!
//!闭环核心:
//! - 运行 pytest 测试
//! - 通过 → encode CapsuleGene (successful implementation pattern)
//! - 失败 → 反馈给 GapAnalyzer, 标记为低质量路径
//! - 成功后 → 触发 InsightEvolution feedback-descent 进化
//!
//! Python original: `research_loop/benchmark_runner.py` (523 lines)

use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Instant;

use rairos_diagnostics::{check_ruff, Diagnostic};

/// Result of a single benchmark run.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkResult {
    pub arxiv_id: String,
    pub test_dir: PathBuf,
    pub passed: u32,
    pub failed: u32,
    pub skipped: u32,
    pub duration_seconds: f64,
    #[serde(default)]
    pub passed_tests: Vec<String>,
    #[serde(default)]
    pub failed_tests: Vec<String>,
    #[serde(default)]
    pub error_message: String,
    #[serde(default)]
    pub ruff_diagnostics: Vec<Diagnostic>,
    /// Paper extracted numerical claims count
    pub numerical_claims_total: u32,
    /// Claims with real assertions (not stubs)
    pub numerical_claims_covered: u32,
    /// covered/total, 0.0~1.0
    pub coverage_ratio: f64,
    #[serde(default)]
    pub covered_claims: Vec<String>,
    #[serde(default)]
    pub uncovered_claims: Vec<String>,
}

/// Configuration for a benchmark run.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkConfig {
    pub arxiv_id: String,
    pub paper_topic: String,
    pub algorithm_description: String,
    pub test_dir: PathBuf,
    pub code_path: PathBuf,
    #[serde(default = "default_code_quality")]
    pub code_quality: f64,
    #[serde(default)]
    pub min_pass_rate: f64,
    /// Cross-paper dedup via structural fingerprint
    #[serde(default)]
    pub algorithm_fingerprint: String,
    /// Raw code string for provenance comment parsing
    #[serde(default)]
    pub generated_code: String,
    /// Resolved paper_section_refs for archetype
    #[serde(default)]
    pub paper_section_refs: Vec<String>,
    /// Minimum coverage to encode (0 = no gate)
    #[serde(default)]
    pub min_coverage_ratio: f64,
    /// Total numerical claims from paper
    #[serde(default)]
    pub numerical_claims_total: u32,
}

fn default_code_quality() -> f64 {
    0.5
}

/// Run pytest on the generated test suite.
pub fn run_benchmark(config: &BenchmarkConfig) -> BenchmarkResult {
    let start = Instant::now();

    let json_report = config.test_dir.join("report.json");

    let python_exe = std::env::var("PYTHON").unwrap_or_else(|_| "python3".to_string());

    let mut cmd = Command::new(&python_exe);
    cmd.arg("-m")
        .arg("pytest")
        .arg(&config.test_dir)
        .arg("-v")
        .arg("--tb=short")
        .arg("--no-header")
        .arg("-q");

    // Prepend code_path parent to PYTHONPATH
    let pythonpath = if let Ok(existing) = std::env::var("PYTHONPATH") {
        format!("{}:{}", config.code_path.parent().unwrap_or(config.code_path.as_path()).display(), existing)
    } else {
        config.code_path.parent().unwrap_or(config.code_path.as_path()).display().to_string()
    };

    let mut env_vars: HashMap<String, String> = std::env::vars().collect();
    env_vars.insert("PYTHONPATH".to_string(), pythonpath);

    let mut result = BenchmarkResult {
        arxiv_id: config.arxiv_id.clone(),
        test_dir: config.test_dir.clone(),
        passed: 0,
        failed: 0,
        skipped: 0,
        duration_seconds: 0.0,
        passed_tests: Vec::new(),
        failed_tests: Vec::new(),
        error_message: String::new(),
        ruff_diagnostics: Vec::new(),
        numerical_claims_total: config.numerical_claims_total,
        numerical_claims_covered: 0,
        coverage_ratio: 0.0,
        covered_claims: Vec::new(),
        uncovered_claims: Vec::new(),
    };

    // Fast lint check — ruff runs synchronously
    let ruff_diagnostics = check_ruff(&config.code_path);
    result.ruff_diagnostics = ruff_diagnostics.clone();
    if !ruff_diagnostics.is_empty() {
        log_diagnostics(&ruff_diagnostics, &config.code_path);
    }

    match cmd
        .env_clear()
        .envs(env_vars)
        .output()
    {
        Ok(output) => {
            result.duration_seconds = start.elapsed().as_secs_f64();
            let stdout = String::from_utf8_lossy(&output.stdout);
            let stderr = String::from_utf8_lossy(&output.stderr);
            let combined = format!("{}{}", stdout, stderr);
            result.error_message = combined.trim().to_string();
            let error_for_parse = result.error_message.clone();
            parse_pytest_output(&mut result, &error_for_parse);

            // Parse JSON report if available
            if json_report.exists() {
                parse_json_report(&mut result, &json_report);
            }

            // Populate Core Claim Coverage fields
            populate_coverage_fields(&mut result, config);
        }
        Err(e) => {
            result.duration_seconds = start.elapsed().as_secs_f64();
            result.error_message = e.to_string();
        }
    }

    result
}

/// Parse pytest stdout/stderr for pass/fail counts.
fn parse_pytest_output(result: &mut BenchmarkResult, output: &str) {
    // Matches: "N passed", "N passed, M failed", "N passed, M failed, K skipped"
    let re_passed = Regex::new(r"(\d+)\s+passed").unwrap();
    if let Some(caps) = re_passed.captures(output) {
        result.passed = caps.get(1).unwrap().as_str().parse().unwrap_or(0);
        // Check for failed
        let re_failed = Regex::new(r",\s*(\d+)\s+failed").unwrap();
        if let Some(fcaps) = re_failed.captures(output) {
            result.failed = fcaps.get(1).unwrap().as_str().parse().unwrap_or(0);
        }
        // Check for skipped
        let re_skipped = Regex::new(r",\s*(\d+)\s+skipped").unwrap();
        if let Some(scaps) = re_skipped.captures(output) {
            result.skipped = scaps.get(1).unwrap().as_str().parse().unwrap_or(0);
        }
    } else {
        // Handle all-skipped or all-failed
        let re_skipped = Regex::new(r"(\d+)\s+skipped").unwrap();
        if let Some(m) = re_skipped.captures(output) {
            result.skipped = m.get(1).unwrap().as_str().parse().unwrap_or(0);
        }
        let re_failed = Regex::new(r"(\d+)\s+failed").unwrap();
        if let Some(m) = re_failed.captures(output) {
            result.failed = m.get(1).unwrap().as_str().parse().unwrap_or(0);
        }
        // Handle collection errors
        let re_error = Regex::new(r"(\d+)\s+error").unwrap();
        if let Some(m) = re_error.captures(output) {
            result.failed = m.get(1).unwrap().as_str().parse().unwrap_or(0);
        }
    }
}

/// Parse pytest-json-report output.
fn parse_json_report(result: &mut BenchmarkResult, report_path: &Path) {
    if let Ok(content) = fs::read_to_string(report_path) {
        if let Ok(data) = serde_json::from_str::<serde_json::Value>(&content) {
            if let Some(summary) = data.get("summary").and_then(|s| s.as_object()) {
                if let Some(p) = summary.get("passed").and_then(|v| v.as_u64()) {
                    result.passed = p as u32;
                }
                if let Some(f) = summary.get("failed").and_then(|v| v.as_u64()) {
                    result.failed = f as u32;
                }
                if let Some(s) = summary.get("skipped").and_then(|v| v.as_u64()) {
                    result.skipped = s as u32;
                }
            }
            if let Some(results) = data.get("results").and_then(|r| r.as_array()) {
                for node in results {
                    if let Some(nodeid) = node.get("nodeid").and_then(|v| v.as_str()) {
                        if let Some(outcome) = node.get("outcome").and_then(|v| v.as_str()) {
                            match outcome {
                                "passed" => result.passed_tests.push(nodeid.to_string()),
                                "failed" => result.failed_tests.push(nodeid.to_string()),
                                _ => {}
                            }
                        }
                    }
                }
            }
        }
    }
}

/// Populate Core Claim Coverage fields from generated test files.
///
/// A numerical claim is "covered" if the test executes a real assertion
/// (not a skip). Skips indicate the model couldn't be evaluated.
fn populate_coverage_fields(result: &mut BenchmarkResult, config: &BenchmarkConfig) {
    let skip_pattern = Regex::new(r"pytest\.skip\(").unwrap();

    let mut covered: Vec<String> = Vec::new();
    let mut uncovered: Vec<String> = Vec::new();

    if let Ok(entries) = fs::read_dir(&result.test_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.file_name().is_some_and(|n| n == "test_claims.py") {
                if let Ok(content) = fs::read_to_string(&path) {
                    let re_func = Regex::new(r"def (test_numerical_claim_\d+.*?):").unwrap();
                    for caps in re_func.captures_iter(&content) {
                        let func_name = caps.get(1).unwrap().as_str().to_string();
                        let func_start = caps.get(0).unwrap().end();
                        let next_def = content[func_start..].find("\ndef ");
                        let func_body = if let Some(nd) = next_def {
                            &content[func_start..func_start + nd]
                        } else {
                            &content[func_start..]
                        };

                        if skip_pattern.is_match(func_body) {
                            uncovered.push(func_name);
                        } else {
                            covered.push(func_name);
                        }
                    }
                }
            }
        }
    }

    result.numerical_claims_covered = covered.len() as u32;
    result.covered_claims = covered;
    result.uncovered_claims = uncovered;
    let total = config.numerical_claims_total;
    result.coverage_ratio = if total > 0 {
        result.numerical_claims_covered as f64 / total as f64
    } else {
        0.0
    };
}

/// Extract keywords from text (simple stopword-based).
pub fn extract_keywords(text: &str) -> Vec<String> {
    let stopwords: std::collections::HashSet<&str> = [
        "the", "a", "an", "and", "or", "but", "in", "on", "at", "to", "for",
        "of", "with", "by", "from", "is", "are", "was", "were", "be", "been",
        "being", "have", "has", "had", "do", "does", "did", "will", "would",
        "could", "should", "may", "might", "can", "this", "that", "these",
        "those", "it", "its", "we", "our", "you", "your", "i", "my",
    ]
    .iter()
    .cloned()
    .collect();

    let re_word = Regex::new(r"[a-zA-Z]{3,}").unwrap();
    let mut seen: std::collections::HashSet<&str> = std::collections::HashSet::new();
    let mut result: Vec<String> = Vec::new();

    for m in re_word.find_iter(&text.to_lowercase()) {
        let w = m.as_str();
        if !stopwords.contains(w) && seen.insert(w) {
            result.push(w.to_string());
        }
    }

    result.truncate(20);
    result
}

/// Return ISO timestamp.
pub fn timestamp() -> String {
    chrono::Utc::now().format("%Y-%m-%dT%H:%M:%S").to_string()
}

/// Run tests and return the subprocess result (for CLI use).
pub fn run_tests_locally(
    test_dir: &Path,
    verbose: bool,
) -> std::process::Command {
    let python_exe = std::env::var("PYTHON").unwrap_or_else(|_| "python3".to_string());
    let mut cmd = Command::new(&python_exe);
    cmd.arg("-m")
        .arg("pytest")
        .arg(test_dir)
        .arg(if verbose { "-v" } else { "-q" })
        .arg("--tb=short");
    cmd
}

/// Print ruff diagnostics to stderr for visibility.
fn log_diagnostics(diagnostics: &[Diagnostic], code_path: &Path) {
    let code_path_str = code_path.file_name()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_default();
    eprintln!("\n[ruff] {} issue(s) in {}:", diagnostics.len(), code_path_str);
    for d in diagnostics {
        let loc = format!("{}:{}:{}", d.file.display(), d.line, d.column);
        eprintln!("  [{}] {} {}: {}", d.severity.to_uppercase(), loc, d.code, d.message);
    }
    eprintln!("  (running pytest anyway...)");
}

/// Human-readable summary of benchmark result.
pub fn summarize_result(result: &BenchmarkResult) -> String {
    let total = result.passed + result.failed;
    let pass_rate = if total > 0 {
        result.passed as f64 / total as f64
    } else {
        0.0
    };

    let mut lines = vec![
        format!("arXiv: {}", result.arxiv_id),
        format!(
            "Tests: {} passed, {} failed, {} skipped",
            result.passed, result.failed, result.skipped
        ),
        format!("Duration: {:.2}s", result.duration_seconds),
        format!("Pass rate: {:.1}%", pass_rate * 100.0),
    ];

    if !result.error_message.is_empty() && result.failed > 0 {
        lines.push(format!(
            "\nError (first 300 chars):\n{}",
            &result.error_message[..result.error_message.len().min(300)]
        ));
    }

    lines.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_pytest_output_passed() {
        let mut result = BenchmarkResult {
            arxiv_id: "test".to_string(),
            test_dir: PathBuf::from("/tmp"),
            passed: 0,
            failed: 0,
            skipped: 0,
            duration_seconds: 0.0,
            passed_tests: Vec::new(),
            failed_tests: Vec::new(),
            error_message: String::new(),
            ruff_diagnostics: Vec::new(),
            numerical_claims_total: 0,
            numerical_claims_covered: 0,
            coverage_ratio: 0.0,
            covered_claims: Vec::new(),
            uncovered_claims: Vec::new(),
        };

        parse_pytest_output(&mut result, "5 passed, 2 failed, 1 skipped in 3.45s");
        assert_eq!(result.passed, 5);
        assert_eq!(result.failed, 2);
        assert_eq!(result.skipped, 1);
    }

    #[test]
    fn test_parse_pytest_output_all_passed() {
        let mut result = BenchmarkResult {
            arxiv_id: "test".to_string(),
            test_dir: PathBuf::from("/tmp"),
            passed: 0,
            failed: 0,
            skipped: 0,
            duration_seconds: 0.0,
            passed_tests: Vec::new(),
            failed_tests: Vec::new(),
            error_message: String::new(),
            ruff_diagnostics: Vec::new(),
            numerical_claims_total: 0,
            numerical_claims_covered: 0,
            coverage_ratio: 0.0,
            covered_claims: Vec::new(),
            uncovered_claims: Vec::new(),
        };

        parse_pytest_output(&mut result, "10 passed in 5.00s");
        assert_eq!(result.passed, 10);
        assert_eq!(result.failed, 0);
    }

    #[test]
    fn test_extract_keywords() {
        let text = "The transformer architecture uses self-attention mechanisms for sequence modeling";
        let keywords = extract_keywords(text);
        // Should not contain stopwords, should contain "transformer", "attention", etc.
        assert!(!keywords.contains(&"the".to_string()));
        assert!(!keywords.contains(&"for".to_string()));
        assert!(keywords.iter().any(|k| k.contains("transformer") || k.contains("attention")));
    }

    #[test]
    fn test_summarize_result() {
        let result = BenchmarkResult {
            arxiv_id: "2301.00001".to_string(),
            test_dir: PathBuf::from("/tmp"),
            passed: 8,
            failed: 2,
            skipped: 1,
            duration_seconds: 5.5,
            passed_tests: Vec::new(),
            failed_tests: Vec::new(),
            error_message: String::new(),
            ruff_diagnostics: Vec::new(),
            numerical_claims_total: 10,
            numerical_claims_covered: 8,
            coverage_ratio: 0.8,
            covered_claims: Vec::new(),
            uncovered_claims: Vec::new(),
        };

        let summary = summarize_result(&result);
        assert!(summary.contains("2301.00001"));
        assert!(summary.contains("8 passed"));
        assert!(summary.contains("2 failed"));
        assert!(summary.contains("80.0%"));
    }
}
