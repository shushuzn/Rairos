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
    "accuracy", "bleu", "rouge", "f1", "f-score", "f-measure", "precision", "recall",
    "perplexity", "ppl", "wer", "cer", "map", "ndcg", "auc", "mse", "mae", "rmse", "r2",
    "psnr", "ssim", "iou", "miou", "top-1", "top-5", "top1", "top5", "error", "err",
    "loss", "latency", "throughput", "params", "flops", "macs", "gflops", "win rate",
    "winrate", "elo", "score", "pass@", "humaneval", "mbpp", "gsm8k", "mmlu", "hellaswag",
    "arc", "task", "dataset", "model", "method", "result",
];

const BENCHMARK_NAMES: &[&str] = &[
    "imagenet", "cifar", "mnist", "svhn", "coco", "pascal voc", "pascal", "cityscapes",
    "ade20k", "squad", "glue", "superglue", "xnli", "wmt", "multinli", "sst", "sst-2",
    "cola", "mrpc", "qnli", "rte", "wnli", "wikitext", "ptb", "penn treebank", "enwik8",
    "text8", "librispeech", "wsj", "tedlium", "voxceleb", "halalbench", "halal", "openai",
    "truthfulqa", "gsm8k", "math", "humaneval", "mbpp", "mmlu", "arc-e", "arc-c",
    "arc-easy", "arc-challenge", "hellaswag", "piqa", "winogrande", "boolq", "siqa",
    "openbookqa", "anli", "storycloze", "lambada", "wikitext-103",
];

fn re_percent() -> &'static Regex {
    static RE: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r"^([\d.]+)\s*%$").unwrap()
    });
    &RE
}

fn re_range() -> &'static Regex {
    static RE: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r"^([\d.]+)±").unwrap()
    });
    &RE
}

fn re_fraction() -> &'static Regex {
    static RE: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r"^([\d.]+)/([\d.]+)$").unwrap()
    });
    &RE
}

fn re_suffix() -> &'static Regex {
    static RE: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r"^([\d.]+)([BKMG])$").unwrap()
    });
    &RE
}

fn contains_numeric(cell: &str) -> bool {
    let cell = cell.trim();
    if cell.is_empty() {
        return false;
    }
    regex::Regex::new(r"\d+\.?\d*")
        .map(|re| re.is_match(cell))
        .unwrap_or(false)
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
        let multipliers: HashMap<char, f64> = [('B', 1e9), ('M', 1e6), ('K', 1e3), ('G', 1e9)].into_iter().collect();
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
        "perplexity", "ppl", "wer", "cer", "mse", "mae", "rmse", "loss", "error", "err",
        "latency", "flops", "macs", "gflops", "params", "token", "time", "runtime", "cost",
        "top-1 error", "top-5 error", "word error",
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
    let n1 = name1
        .to_lowercase()
        .trim()
        .replace(['-', '_'], " ");
    let n2 = name2
        .to_lowercase()
        .trim()
        .replace(['-', '_'], " ");

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

    pub fn detect_tables(&self, _paper_id: &str, _tables: Vec<(i64, String, i32, Vec<String>, Vec<Vec<String>>)>) -> Vec<BenchmarkTable> {
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

            lines.push(format!("  {:<6} {:<16} {:<22} {:<10}", "Rank", "Paper ID", "Model", "Score"));
            lines.push(format!("  {:<6} {:<16} {:<22} {:<10}", "-".repeat(6), "-".repeat(16), "-".repeat(22), "-".repeat(10)));

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