#![allow(dead_code)]
#![allow(
    clippy::type_complexity,
)]
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NormalizedMetric {
    pub raw_value: String,
    pub numeric: Option<f64>,
    pub is_higher_better: bool,
    pub confidence: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkTable {
    pub paper_id: String,
    pub table_id: i64,
    pub caption: String,
    pub page: i32,
    pub headers: Vec<String>,
    pub rows: Vec<Vec<NormalizedMetric>>,
    #[serde(default)]
    pub benchmark_name: String,
    #[serde(default)]
    pub models: Vec<String>,
    #[serde(default)]
    pub metrics: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkMatch {
    pub benchmark_name: String,
    pub metric_name: String,
    pub entries: Vec<(String, f64, String)>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkResult {
    pub paper_ids: Vec<String>,
    pub tables_found: HashMap<String, Vec<BenchmarkTable>>,
    pub matches: Vec<BenchmarkMatch>,
    #[serde(default)]
    pub unmatched: Vec<BenchmarkTable>,
}

const METRIC_KEYWORDS: &[&str] = &[
    "accuracy",
    "bleu",
    "rouge",
    "f1",
    "f-score",
    "f-measure",
    "precision",
    "recall",
    "perplexity",
    "ppl",
    "wer",
    "cer",
    "map",
    "ndcg",
    "auc",
    "mse",
    "mae",
    "rmse",
    "r2",
    "psnr",
    "ssim",
    "iou",
    "miou",
    "top-1",
    "top-5",
    "top1",
    "top5",
    "error",
    "err",
    "loss",
    "latency",
    "throughput",
    "params",
    "flops",
    "macs",
    "gflops",
    "win rate",
    "winrate",
    "elo",
    "score",
    "pass@",
    "humaneval",
    "mbpp",
    "gsm8k",
    "mmlu",
    "hellaswag",
    "arc",
    "task",
    "dataset",
    "model",
    "method",
    "result",
];

const BENCHMARK_NAMES: &[&str] = &[
    "imagenet",
    "cifar",
    "mnist",
    "svhn",
    "coco",
    "pascal voc",
    "pascal",
    "cityscapes",
    "ade20k",
    "squad",
    "glue",
    "superglue",
    "xnli",
    "wmt",
    "multinli",
    "sst",
    "sst-2",
    "cola",
    "mrpc",
    "qnli",
    "rte",
    "wnli",
    "wikitext",
    "ptb",
    "penn treebank",
    "enwik8",
    "text8",
    "librispeech",
    "wsj",
    "tedlium",
    "voxceleb",
    "halalbench",
    "halal",
    "openai",
    "truthfulqa",
    "gsm8k",
    "math",
    "humaneval",
    "mbpp",
    "mmlu",
    "arc-e",
    "arc-c",
    "arc-easy",
    "arc-challenge",
    "hellaswag",
    "piqa",
    "winogrande",
    "boolq",
    "siqa",
    "openbookqa",
    "anli",
    "storycloze",
    "lambada",
    "wikitext-103",
];

fn re_percent() -> &'static Regex {
    static RE: std::sync::LazyLock<Regex> =
        std::sync::LazyLock::new(|| Regex::new(r"^([\d.]+)\s*%$").expect("valid regex"));
    &RE
}

fn re_range() -> &'static Regex {
    static RE: std::sync::LazyLock<Regex> =
        std::sync::LazyLock::new(|| Regex::new(r"^([\d.]+)±").expect("valid regex"));
    &RE
}

fn re_fraction() -> &'static Regex {
    static RE: std::sync::LazyLock<Regex> =
        std::sync::LazyLock::new(|| Regex::new(r"^([\d.]+)/([\d.]+)$").expect("valid regex"));
    &RE
}

fn re_suffix() -> &'static Regex {
    static RE: std::sync::LazyLock<Regex> =
        std::sync::LazyLock::new(|| Regex::new(r"^([\d.]+)([BKMG])$").expect("valid regex"));
    &RE
}

fn re_numeric() -> &'static Regex {
    static RE: std::sync::LazyLock<Regex> =
        std::sync::LazyLock::new(|| Regex::new(r"\d+\.?\d*").expect("valid regex"));
    &RE
}

fn contains_numeric(cell: &str) -> bool {
    let cell = cell.trim();
    if cell.is_empty() {
        return false;
    }
    re_numeric().is_match(cell)
}

fn parse_numeric(value: &str) -> Option<f64> {
    let value = value.trim().replace(" ", "").replace(",", "");

    if value.is_empty() {
        return None;
    }

    if let Some(m) = re_percent().captures(&value) {
        return m.get(1).and_then(|v| v.as_str().parse().ok());
    }

    if let Some(m) = re_range().captures(&value) {
        return m.get(1).and_then(|v| v.as_str().parse().ok());
    }

    if let Some(m) = re_fraction().captures(&value) {
        let num: f64 = m.get(1)?.as_str().parse().ok()?;
        let den: f64 = m.get(2)?.as_str().parse().ok()?;
        if den == 0.0 {
            return None;
        }
        return Some(num / den);
    }

    if let Some(m) = re_suffix().captures(&value.to_uppercase()) {
        let multipliers: HashMap<char, f64> = [('B', 1e9), ('M', 1e6), ('K', 1e3), ('G', 1e9)]
            .into_iter()
            .collect();
        let num: f64 = m.get(1)?.as_str().parse().ok()?;
        let suffix = m.get(2)?.as_str().chars().next()?;
        let mult = multipliers.get(&suffix).copied().unwrap_or(1.0);
        return Some(num * mult);
    }

    if let Ok(v) = value.parse::<f64>() {
        return Some(v);
    }

    None
}

fn is_higher_better(metric_name: &str) -> bool {
    let lower_better: HashSet<&str> = [
        "perplexity",
        "ppl",
        "wer",
        "cer",
        "mse",
        "mae",
        "rmse",
        "loss",
        "error",
        "err",
        "latency",
        "flops",
        "macs",
        "gflops",
        "params",
        "token",
        "time",
        "runtime",
        "cost",
        "top-1 error",
        "top-5 error",
        "word error",
    ]
    .into_iter()
    .collect();

    let name_lower = metric_name.to_lowercase().trim().to_string();
    for lb in &lower_better {
        if name_lower.contains(*lb) {
            return false;
        }
    }
    true
}

fn fuzzy_match_name(name1: &str, name2: &str) -> f64 {
    let n1 = name1.to_lowercase().trim().replace(['-', '_'], " ");
    let n2 = name2.to_lowercase().trim().replace(['-', '_'], " ");

    if n1 == n2 {
        return 1.0;
    }

    if n1.contains(&n2) || n2.contains(&n1) {
        return 0.9;
    }

    let w1: HashSet<&str> = n1.split_whitespace().collect();
    let w2: HashSet<&str> = n2.split_whitespace().collect();

    if w1.is_empty() || w2.is_empty() {
        return 0.0;
    }

    let intersection: HashSet<_> = w1.intersection(&w2).collect();
    let union: HashSet<_> = w1.union(&w2).collect();

    intersection.len() as f64 / union.len() as f64
}

fn guess_benchmark_name(caption: &str, headers: &[String]) -> String {
    let text = caption.to_lowercase();
    for name in BENCHMARK_NAMES {
        if text.contains(&name.to_lowercase()) {
            return name.to_string();
        }
    }

    for h in headers {
        let h_lower = h.to_lowercase();
        for name in BENCHMARK_NAMES {
            if h_lower.contains(&name.to_lowercase()) {
                return name.to_string();
            }
        }
    }

    if caption.is_empty() {
        "Unknown".to_string()
    } else {
        caption.chars().take(80).collect()
    }
}

pub struct BenchmarkComparator;

impl BenchmarkComparator {
    pub fn new() -> Self {
        Self
    }

    pub fn detect_tables(
        &self,
        _paper_id: &str,
        _tables: Vec<(i64, String, i32, Vec<String>, Vec<Vec<String>>)>,
    ) -> Vec<BenchmarkTable> {
        Vec::new()
    }

    pub fn compare(&self, paper_ids: Vec<String>) -> BenchmarkResult {
        BenchmarkResult {
            paper_ids,
            tables_found: HashMap::new(),
            matches: Vec::new(),
            unmatched: Vec::new(),
        }
    }

    pub fn render_leaderboard(&self, result: &BenchmarkResult) -> String {
        let mut lines = Vec::new();
        lines.push(format!("\n{}", "=".repeat(70)));
        lines.push("  Cross-Paper Benchmark Comparison".to_string());
        lines.push(format!("  Papers: {}", result.paper_ids.join(", ")));
        lines.push("=".repeat(70).to_string());

        if result.matches.is_empty() {
            lines.push("\n  No matching benchmarks found across papers.".to_string());
            return lines.join("\n");
        }

        for m in &result.matches {
            lines.push(format!("\n{}", "-".repeat(70)));
            lines.push(format!("  {} → {}", m.benchmark_name, m.metric_name));
            let direction = if is_higher_better(&m.metric_name) {
                "↑ higher is better"
            } else {
                "↓ lower is better"
            };
            lines.push(format!("  ({})", direction));
            lines.push("-".repeat(70));

            lines.push(format!(
                "  {:<6} {:<16} {:<22} {:<10}",
                "Rank", "Paper ID", "Model", "Score"
            ));
            lines.push(format!(
                "  {:<6} {:<16} {:<22} {:<10}",
                "-".repeat(6),
                "-".repeat(16),
                "-".repeat(22),
                "-".repeat(10)
            ));

            for (rank, (pid, val, model)) in m.entries.iter().enumerate().map(|(i, e)| (i + 1, e)) {
                let medal = match rank {
                    1 => "🥇",
                    2 => "🥈",
                    3 => "🥉",
                    _ => &format!("  {}.", rank),
                };
                lines.push(format!(
                    "  {} {:<16} {:<22} {:<10.4}",
                    medal,
                    pid,
                    &model[..model.len().min(20)],
                    val
                ));
            }
        }

        lines.push(format!("\n{}", "=".repeat(70)));
        lines.join("\n")
    }

    pub fn render_text(&self, result: &BenchmarkResult) -> String {
        self.render_leaderboard(result)
    }

    pub fn render_markdown(&self, result: &BenchmarkResult) -> String {
        let mut lines = Vec::new();
        lines.push("# Benchmark Comparison".to_string());
        lines.push(String::new());
        lines.push(format!("**Papers**: {}", result.paper_ids.join(", ")));
        lines.push(String::new());

        for m in &result.matches {
            lines.push(format!("## {} — {}", m.benchmark_name, m.metric_name));
            lines.push(String::new());
            lines.push("| Rank | Paper ID | Model | Score |".to_string());
            lines.push("|------|----------|-------|-------|".to_string());
            for (rank, (pid, val, model)) in m.entries.iter().enumerate().map(|(i, e)| (i + 1, e)) {
                lines.push(format!("| {} | `{}` | {} | {:.4} |", rank, pid, model, val));
            }
            lines.push(String::new());
        }

        lines.join("\n")
    }

    pub fn render_json(&self, result: &BenchmarkResult) -> String {
        let mut output = HashMap::new();
        for m in &result.matches {
            let key = format!("{}/{}", m.benchmark_name, m.metric_name);
            let entries: Vec<_> = m
                .entries
                .iter()
                .enumerate()
                .map(|(rank, (pid, val, model))| {
                    serde_json::json!({
                        "rank": rank + 1,
                        "paper_id": pid,
                        "model": model,
                        "score": val
                    })
                })
                .collect();
            output.insert(key, entries);
        }
        serde_json::to_string_pretty(&output).unwrap_or_default()
    }
}

impl Default for BenchmarkComparator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_numeric_percent() {
        assert_eq!(parse_numeric("92.5%"), Some(92.5));
    }

    #[test]
    fn test_parse_numeric_range() {
        assert_eq!(parse_numeric("92.5±0.3"), Some(92.5));
    }

    #[test]
    fn test_parse_numeric_fraction() {
        assert_eq!(parse_numeric("92.5/100"), Some(0.925));
    }

    #[test]
    fn test_parse_numeric_suffix() {
        assert_eq!(parse_numeric("1.2B"), Some(1.2e9));
        assert_eq!(parse_numeric("350M"), Some(350.0e6));
    }

    #[test]
    fn test_is_higher_better() {
        assert!(!is_higher_better("perplexity"));
        assert!(!is_higher_better("WER"));
        assert!(is_higher_better("accuracy"));
        assert!(is_higher_better("BLEU"));
    }

    #[test]
    fn test_fuzzy_match_name() {
        assert_eq!(fuzzy_match_name("ImageNet", "imagenet"), 1.0);
        assert!((fuzzy_match_name("imagenet accuracy", "imagenet") - 0.9).abs() < 0.01);
        assert!(fuzzy_match_name("abc", "xyz") < 0.5);
    }

    #[test]
    fn test_guess_benchmark_name() {
        assert_eq!(guess_benchmark_name("Results on ImageNet", &[]), "imagenet");
        assert_eq!(guess_benchmark_name("CIFAR-10 accuracy", &[]), "cifar");
    }

    #[test]
    fn test_contains_numeric() {
        assert!(contains_numeric("92.5"));
        assert!(contains_numeric("92.5%"));
        assert!(contains_numeric("abc123"));
        assert!(!contains_numeric("abc"));
    }

    #[test]
    fn test_benchmark_comparator_new() {
        let bc = BenchmarkComparator::new();
        let result = bc.compare(vec!["paper1".to_string()]);
        assert!(result.paper_ids.contains(&"paper1".to_string()));
    }

    #[test]
    fn test_render_leaderboard_empty() {
        let bc = BenchmarkComparator::new();
        let result = BenchmarkResult {
            paper_ids: vec!["paper1".to_string()],
            tables_found: HashMap::new(),
            matches: vec![],
            unmatched: vec![],
        };
        let output = bc.render_leaderboard(&result);
        assert!(output.contains("No matching benchmarks"));
    }
}

// ---------------------------------------------------------------------------
// VeriScale Adversarial Tests (EvolveCoder 2603.12698 + ACE 2605.16299)
// ---------------------------------------------------------------------------

/// An adversarial test case — targets specific failure modes in candidate solutions.
/// ACE paper: tests must be solution-aware (not static) to maintain discriminative
/// power as solvers improve. EvolveCoder: iteratively refine tests against actual
/// solution distribution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdversarialTestCase {
    /// Unique identifier
    pub test_id: String,
    /// Natural-language description of what this test checks
    pub description: String,
    /// The test code itself
    pub test_code: String,
    /// Expected behavior: what a correct solution should produce
    pub expected_output: String,
    /// ACE-style hardness score: fraction of candidate solutions that fail this test.
    /// Higher = more adversarial (fewer solutions pass).
    pub hardness: f64,
    /// How well this test discriminates between good and bad solutions (0-1).
    pub discriminative_power: f64,
    /// Number of times this test was refined
    pub refinement_round: u32,
    /// Execution-only feedback received (errors, crashes, timeouts)
    pub execution_feedback: Vec<String>,
    /// Tags for categorization (boundary, error-path, fuzz, etc.)
    pub tags: Vec<String>,
    /// Whether this test has found a real bug in any solution
    pub bug_detected: bool,
}

impl AdversarialTestCase {
    /// Create a new adversarial test case.
    pub fn new(test_id: &str, description: &str, test_code: &str, expected_output: &str) -> Self {
        Self {
            test_id: test_id.to_string(),
            description: description.to_string(),
            test_code: test_code.to_string(),
            expected_output: expected_output.to_string(),
            hardness: 0.5,
            discriminative_power: 0.5,
            refinement_round: 0,
            execution_feedback: Vec::new(),
            tags: Vec::new(),
            bug_detected: false,
        }
    }

    /// Record that this test found a bug in a solution.
    pub fn mark_bug_found(&mut self) {
        self.bug_detected = true;
        self.discriminative_power = (self.discriminative_power + 0.1).min(1.0);
    }

    /// Record execution feedback.
    pub fn add_feedback(&mut self, feedback: &str) {
        if !self.execution_feedback.contains(&feedback.to_string()) {
            self.execution_feedback.push(feedback.to_string());
        }
    }

    /// Advance to next refinement round and increase hardness slightly.
    pub fn refine(&mut self, new_test_code: &str, new_expected: &str) {
        self.refinement_round += 1;
        self.test_code = new_test_code.to_string();
        self.expected_output = new_expected.to_string();
        self.hardness = (self.hardness + 0.05).min(0.99);
    }
}

/// A solution submission to be evaluated against adversarial tests.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CandidateSolution {
    pub solution_id: String,
    pub code: String,
    pub passed_tests: Vec<String>,
    pub failed_tests: Vec<String>,
    pub execution_errors: Vec<String>,
    pub execution_time_ms: f64,
    pub passed: bool,
}

impl CandidateSolution {
    /// Compute pass rate across a set of adversarial tests.
    pub fn pass_rate(&self, all_tests: &[AdversarialTestCase]) -> f64 {
        if all_tests.is_empty() {
            return 1.0;
        }
        let total = all_tests.len();
        let passed = all_tests.iter()
            .filter(|t| self.passed_tests.contains(&t.test_id))
            .count();
        passed as f64 / total as f64
    }
}

/// Statistics about an adversarial test suite evolution round.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestEvolutionStats {
    pub round: u32,
    pub suite_size: usize,
    pub avg_hardness: f64,
    pub avg_discriminative_power: f64,
    pub tests_that_found_bugs: usize,
    pub solutions_tested: usize,
    pub solutions_passed_all: usize,
    pub solutions_failed_some: usize,
}

/// VeriScale Adversarial Test Suite — manages adversarial test evolution.
/// Based on ACE: alternating between solution generation and adversarial test
/// generation, guided by execution-only feedback.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdversarialTestSuite {
    pub name: String,
    pub tests: Vec<AdversarialTestCase>,
    pub current_round: u32,
}

impl Default for AdversarialTestSuite {
    fn default() -> Self {
        Self::new("default")
    }
}

impl AdversarialTestSuite {
    /// Create a new named adversarial test suite.
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            tests: Vec::new(),
            current_round: 0,
        }
    }

    /// Add a test case to the suite.
    pub fn add_test(&mut self, test: AdversarialTestCase) {
        self.tests.push(test);
    }

    /// Remove tests with very low discriminative power.
    pub fn prune_low_quality(&mut self, min_discriminative: f64) {
        self.tests.retain(|t| t.discriminative_power >= min_discriminative);
    }

    /// Compute Kahneman-Tversky-inspired hardness score.
    /// H = 1 - pass_rate: hard tests = low pass rate.
    pub fn compute_test_hardness(solutions: &[CandidateSolution], test_id: &str) -> f64 {
        if solutions.is_empty() {
            return 0.5;
        }
        let passed = solutions.iter()
            .filter(|s| s.passed_tests.iter().any(|x| x == test_id))
            .count();
        let pass_rate = passed as f64 / solutions.len() as f64;
        (1.0 - pass_rate).clamp(0.0, 1.0)
    }

    /// Update hardness and discriminative power for all tests.
    pub fn update_from_solutions(&mut self, solutions: &[CandidateSolution]) {
        for test in &mut self.tests {
            let new_hardness = Self::compute_test_hardness(solutions, &test.test_id);
            // Use raw hardness for first update, EMA thereafter for stability
            if test.refinement_round == 0 {
                test.hardness = new_hardness;
            } else {
                test.hardness = test.hardness * 0.7 + new_hardness * 0.3;
            }

            let passed_count = solutions.iter()
                .filter(|s| s.passed_tests.contains(&test.test_id))
                .count();
            let failed_count = solutions.iter()
                .filter(|s| s.failed_tests.contains(&test.test_id))
                .count();

            if passed_count > 0 && failed_count > 0 {
                let pass_frac = passed_count as f64 / solutions.len() as f64;
                let fail_frac = failed_count as f64 / solutions.len() as f64;
                let gini = 2.0 * pass_frac * fail_frac;
                test.discriminative_power = test.discriminative_power * 0.7 + gini * 0.3;
            }

            for sol in solutions {
                if sol.execution_errors.iter().any(|e| e.contains(&test.test_id)) {
                    test.mark_bug_found();
                }
            }
        }
    }

    /// Generate an evolved test from an existing test.
    pub fn evolve_test(
        &self,
        source_test: &AdversarialTestCase,
        solution_hints: &[&str],
    ) -> AdversarialTestCase {
        let mutation_desc = if let Some(hint) = solution_hints.first() {
            if hint.contains("timeout") || hint.contains("TIMEOUT") {
                "Add tighter timeout guard"
            } else if hint.contains("overflow") || hint.contains("OVERFLOW") {
                "Add boundary value test"
            } else if hint.contains("null") || hint.contains("None") || hint.contains("undefined") {
                "Add null-handling assertion"
            } else if hint.contains("assertion") || hint.contains("Assertion") {
                "Strengthen assertion logic"
            } else {
                "Generalize test inputs"
            }
        } else {
            "Increase test coverage"
        };

        let evolved_code = format!(
            "// Evolved from test '{}' (round {})\n// Strategy: {}\n{}\n    // TODO: apply specific mutation based on solution_hints",
            source_test.test_id,
            source_test.refinement_round + 1,
            mutation_desc,
            source_test.test_code
        );

        let mut evolved = source_test.clone();
        evolved.test_id = format!("{}_r{}", source_test.test_id, source_test.refinement_round + 1);
        evolved.test_code = evolved_code;
        evolved.refinement_round += 1;
        evolved.hardness = (source_test.hardness + 0.05).min(0.99);
        evolved.execution_feedback.clear();
        evolved.tags.push(mutation_desc.to_lowercase().replace(' ', "_"));
        evolved
    }

    /// Perform one round of adversarial test evolution.
    pub fn evolve_one_round(&mut self, solutions: &[CandidateSolution]) -> TestEvolutionStats {
        self.current_round += 1;
        let mut new_tests = Vec::new();

        self.update_from_solutions(solutions);

        for test in &self.tests {
            if test.hardness < 0.3 || test.hardness > 0.95 {
                let hints: Vec<_> = solutions.iter()
                    .filter(|s| s.failed_tests.contains(&test.test_id))
                    .flat_map(|s| s.execution_errors.iter())
                    .collect();
                let hint_refs: Vec<&str> = hints.iter().map(|s| s.as_str()).collect();
                let evolved = self.evolve_test(test, &hint_refs);
                new_tests.push(evolved);
            }
        }

        let passed_all = solutions.iter().filter(|s| s.passed).count();
        let failed_some = solutions.len() - passed_all;
        let avg_hardness = if self.tests.is_empty() {
            0.0
        } else {
            self.tests.iter().map(|t| t.hardness).sum::<f64>() / self.tests.len() as f64
        };
        let avg_dp = if self.tests.is_empty() {
            0.0
        } else {
            self.tests.iter().map(|t| t.discriminative_power).sum::<f64>() / self.tests.len() as f64
        };

        self.tests.extend(new_tests);

        TestEvolutionStats {
            round: self.current_round,
            suite_size: self.tests.len(),
            avg_hardness,
            avg_discriminative_power: avg_dp,
            tests_that_found_bugs: self.tests.iter().filter(|t| t.bug_detected).count(),
            solutions_tested: solutions.len(),
            solutions_passed_all: passed_all,
            solutions_failed_some: failed_some,
        }
    }

    /// Run ACE-style solver-adversary loop for a fixed number of rounds.
    pub fn solver_adversary_loop(
        &mut self,
        initial_tests: Vec<AdversarialTestCase>,
        solution_generator: impl Fn(u32, &[AdversarialTestCase]) -> Vec<CandidateSolution>,
        rounds: u32,
    ) -> Vec<TestEvolutionStats> {
        self.tests = initial_tests;
        let mut stats_history = Vec::new();

        for round in 0..rounds {
            let solutions = solution_generator(round, &self.tests);
            self.update_from_solutions(&solutions);
            let stats = self.evolve_one_round(&solutions);
            stats_history.push(stats);
        }

        stats_history
    }

    /// Get the hardest (most adversarial) tests.
    pub fn hardest_tests(&self, count: usize) -> Vec<AdversarialTestCase> {
        let mut sorted = self.tests.clone();
        sorted.sort_by(|a, b| b.hardness.partial_cmp(&a.hardness).unwrap_or(std::cmp::Ordering::Equal));
        sorted.into_iter().take(count).collect()
    }

    /// Get the most discriminative tests.
    pub fn most_discriminative(&self, count: usize) -> Vec<AdversarialTestCase> {
        let mut sorted = self.tests.clone();
        sorted.sort_by(|a, b| b.discriminative_power.partial_cmp(&a.discriminative_power).unwrap_or(std::cmp::Ordering::Equal));
        sorted.into_iter().take(count).collect()
    }
}

#[cfg(test)]
mod adversarial_tests {
    use super::*;

    fn make_test(id: &str, hardness: f64) -> AdversarialTestCase {
        AdversarialTestCase {
            test_id: id.to_string(),
            description: format!("Test {}", id),
            test_code: format!("fn test_{}() {{}}", id),
            expected_output: "ok".to_string(),
            hardness,
            discriminative_power: 0.5,
            refinement_round: 0,
            execution_feedback: Vec::new(),
            tags: vec!["test".to_string()],
            bug_detected: false,
        }
    }

    fn make_solution(id: &str, passed: Vec<&str>, failed: Vec<&str>) -> CandidateSolution {
        let is_passed = failed.is_empty();
        CandidateSolution {
            solution_id: id.to_string(),
            code: format!("// solution {}", id),
            passed_tests: passed.into_iter().map(String::from).collect(),
            failed_tests: failed.into_iter().map(String::from).collect(),
            execution_errors: Vec::new(),
            execution_time_ms: 100.0,
            passed: is_passed,
        }
    }

    #[test]
    fn test_compute_test_hardness() {
        let solutions = vec![
            make_solution("s1", vec!["t1"], vec![]),
            make_solution("s2", vec!["t1"], vec![]),
            make_solution("s3", vec![], vec!["t1"]),
            make_solution("s4", vec![], vec!["t1"]),
        ];
        let h = AdversarialTestSuite::compute_test_hardness(&solutions, "t1");
        assert!((h - 0.5).abs() < 0.01);
    }

    #[test]
    fn test_compute_test_hardness_all_pass() {
        let solutions = vec![
            make_solution("s1", vec!["t1"], vec![]),
            make_solution("s2", vec!["t1"], vec![]),
        ];
        let h = AdversarialTestSuite::compute_test_hardness(&solutions, "t1");
        assert_eq!(h, 0.0);
    }

    #[test]
    fn test_compute_test_hardness_all_fail() {
        let solutions = vec![
            make_solution("s1", vec![], vec!["t1"]),
            make_solution("s2", vec![], vec!["t1"]),
        ];
        let h = AdversarialTestSuite::compute_test_hardness(&solutions, "t1");
        assert_eq!(h, 1.0);
    }

    #[test]
    fn test_update_from_solutions() {
        let mut suite = AdversarialTestSuite::new("test");
        suite.add_test(make_test("t1", 0.5));
        suite.add_test(make_test("t2", 0.5));

        let solutions = vec![
            make_solution("s1", vec!["t1"], vec!["t2"]),
            make_solution("s2", vec!["t1"], vec!["t2"]),
        ];
        suite.update_from_solutions(&solutions);

        let t1 = suite.tests.iter().find(|t| t.test_id == "t1").unwrap();
        let t2 = suite.tests.iter().find(|t| t.test_id == "t2").unwrap();
        assert!((t1.hardness - 0.0).abs() < 0.01);
        assert!((t2.hardness - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_evolve_test_easy_becomes_harder() {
        let suite = AdversarialTestSuite::new("test");
        let source = make_test("t1", 0.1);
        let evolved = suite.evolve_test(&source, &["timeout after 100ms"]);
        assert_eq!(evolved.refinement_round, 1);
        assert!(evolved.hardness > source.hardness);
        assert!(evolved.test_code.contains("timeout"));
    }

    #[test]
    fn test_solver_adversary_loop() {
        let mut suite = AdversarialTestSuite::new("test");

        let stats = suite.solver_adversary_loop(
            vec![make_test("t1", 0.5)],
            |round, _tests| {
                vec![
                    make_solution(&format!("s{}_1", round), vec!["t1"], vec![]),
                    make_solution(&format!("s{}_2", round), vec![], vec!["t1"]),
                ]
            },
            3,
        );

        assert_eq!(stats.len(), 3);
        assert!(stats[0].solutions_tested >= 0);
    }

    #[test]
    fn test_hardest_tests() {
        let mut suite = AdversarialTestSuite::new("test");
        suite.add_test(make_test("easy", 0.2));
        suite.add_test(make_test("medium", 0.5));
        suite.add_test(make_test("hard", 0.9));

        let hardest = suite.hardest_tests(2);
        assert_eq!(hardest.len(), 2);
        assert_eq!(hardest[0].test_id, "hard");
        assert_eq!(hardest[1].test_id, "medium");
    }

    #[test]
    fn test_most_discriminative() {
        let mut suite = AdversarialTestSuite::new("test");
        suite.add_test(make_test("t1", 0.5));
        suite.tests[0].discriminative_power = 0.3;
        suite.add_test(make_test("t2", 0.5));
        suite.tests[1].discriminative_power = 0.9;

        let most = suite.most_discriminative(1);
        assert_eq!(most[0].test_id, "t2");
    }

    #[test]
    fn test_prune_low_quality() {
        let mut suite = AdversarialTestSuite::new("test");
        suite.add_test(make_test("good", 0.8));
        suite.tests[0].discriminative_power = 0.7;
        suite.add_test(make_test("bad", 0.3));
        suite.tests[1].discriminative_power = 0.1;

        suite.prune_low_quality(0.5);
        assert_eq!(suite.tests.len(), 1);
        assert_eq!(suite.tests[0].test_id, "good");
    }

    #[test]
    fn test_adversarial_test_case_mark_bug() {
        let mut test = make_test("t1", 0.5);
        assert!(!test.bug_detected);
        test.mark_bug_found();
        assert!(test.bug_detected);
    }

    #[test]
    fn test_candidate_solution_pass_rate() {
        let solutions = vec![
            make_solution("s1", vec!["t1", "t2"], vec![]),
            make_solution("s2", vec!["t1"], vec!["t2"]),
        ];
        let tests = vec![make_test("t1", 0.5), make_test("t2", 0.5)];
        // s1 passed 2/2 tests
        let rate = solutions[0].pass_rate(&tests);
        assert!((rate - 1.0).abs() < 0.01);
        // s2 passed 1/2 tests
        let rate2 = solutions[1].pass_rate(&tests);
        assert!((rate2 - 0.5).abs() < 0.01);
    }
}
