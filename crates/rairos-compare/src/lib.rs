//! rairos-compare — Paper Comparison
//!
//! Compare multiple papers side-by-side: methods, datasets, metrics, authors.
//!
//! Ported from `llm/paper_comparison.py`.

use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComparisonColumn {
    pub paper_id: String,
    pub title: String,
    #[serde(default)]
    pub year: i32,
    #[serde(default)]
    pub authors: Vec<String>,
    #[serde(default)]
    pub methods: Vec<String>,
    #[serde(default)]
    pub datasets: Vec<String>,
    #[serde(default)]
    pub metrics: HashMap<String, String>,
    #[serde(default)]
    pub r#abstract: String,
}

impl ComparisonColumn {
    pub fn new(paper_id: &str, title: &str) -> Self {
        Self {
            paper_id: paper_id.to_string(),
            title: title.to_string(),
            year: 0,
            authors: Vec::new(),
            methods: Vec::new(),
            datasets: Vec::new(),
            metrics: HashMap::new(),
            r#abstract: String::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AspectRow {
    pub aspect: String,
    #[serde(flatten)]
    pub values: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ComparisonResult {
    pub columns: Vec<ComparisonColumn>,
    #[serde(default)]
    pub aspect_rows: Vec<AspectRow>,
}

pub struct PaperComparator;

impl PaperComparator {
    pub fn new() -> Self {
        Self
    }

    pub fn add_paper(&self, paper_id: &str, title: &str, year: i32, authors: Vec<String>, r#abstract: &str) -> ComparisonColumn {
        let text = format!("{} {} {}", title.to_lowercase(), r#abstract.to_lowercase(), "").to_lowercase();

        let method_keywords: HashMap<&str, &str> = [
            ("transformer", "Transformer"),
            ("bert", "BERT"),
            ("gpt", "GPT"),
            ("lstm", "LSTM"),
            ("cnn", "CNN"),
            ("gan", "GAN"),
            ("rl", "Reinforcement Learning"),
            ("attention", "Attention"),
            ("embedding", "Embedding"),
            ("retrieval", "Retrieval"),
            ("rag", "RAG"),
            ("fine-tun", "Fine-tuning"),
            ("rlhf", "RLHF"),
            ("chain-of-thought", "Chain-of-Thought"),
            ("prompt", "Prompting"),
        ].into_iter().collect();

        let mut methods = Vec::new();
        for (kw, name) in &method_keywords {
            if text.contains(*kw) && !methods.contains(&name.to_string()) {
                methods.push(name.to_string());
            }
        }
        methods.truncate(5);

        let dataset_keywords: HashMap<&str, &str> = [
            ("glue", "GLUE"),
            ("super.glue", "SuperGLUE"),
            ("squad", "SQuAD"),
            ("natural questions", "NQ"),
            ("triviaqa", "TriviaQA"),
            ("mmlu", "MMLU"),
            ("humaneval", "HumanEval"),
            ("mbpp", "MBPP"),
            ("alpacaeval", "AlpacaEval"),
            ("coqa", "CoQA"),
            ("hotpotqa", "HotpotQA"),
            ("drop", "DROP"),
            ("fever", "FEVER"),
            ("mnli", "MNLI"),
            ("qnli", "QNLI"),
            ("cola", "CoLA"),
            ("sst", "SST"),
            ("stsb", "STS-B"),
            ("qqp", "QQP"),
            ("mrpc", "MRPC"),
        ].into_iter().collect();

        let mut datasets = Vec::new();
        for (kw, name) in &dataset_keywords {
            if text.contains(*kw) && !datasets.contains(&name.to_string()) {
                datasets.push(name.to_string());
            }
        }
        datasets.truncate(5);

        let metric_keywords = [
            "accuracy", "precision", "recall", "f1", "bleu", "rouge",
            "perplexity", "latency", "throughput",
        ];

        let mut metrics = HashMap::new();
        for kw in metric_keywords {
            if text.contains(kw) {
                metrics.insert(kw.to_string(), "✓".to_string());
            }
        }

        let acc_re = Regex::new(r"(\d+\.?\d*)\s*%?\s*(accuracy)").ok();
        if let Some(re) = &acc_re {
            if let Some(cap) = re.captures(&text) {
                if let (Some(val), Some(_name)) = (cap.get(1), cap.get(2)) {
                    metrics.insert("Accuracy".to_string(), format!("{}%", val.as_str()));
                }
            }
        }

        ComparisonColumn {
            paper_id: paper_id.to_string(),
            title: title.to_string(),
            year,
            authors,
            methods,
            datasets,
            metrics,
            r#abstract: r#abstract.to_string(),
        }
    }

    pub fn compare(&self, columns: Vec<ComparisonColumn>, aspects: Vec<String>) -> ComparisonResult {
        let default_aspects = if aspects.is_empty() {
            vec!["methods".to_string(), "datasets".to_string(), "metrics".to_string(), "authors".to_string()]
        } else {
            aspects
        };

        let mut aspect_rows = Vec::new();

        for aspect in &default_aspects {
            let mut row_values = HashMap::new();
            row_values.insert("aspect".to_string(), capitalize(aspect));

            for col in &columns {
                let val = match aspect.as_str() {
                    "methods" => {
                        if col.methods.is_empty() {
                            "-".to_string()
                        } else {
                            col.methods.join(", ")
                        }
                    }
                    "datasets" => {
                        if col.datasets.is_empty() {
                            "-".to_string()
                        } else {
                            col.datasets.join(", ")
                        }
                    }
                    "metrics" => {
                        if col.metrics.is_empty() {
                            "-".to_string()
                        } else {
                            col.metrics.iter()
                                .map(|(k, v)| format!("{}={}", k, v))
                                .collect::<Vec<_>>()
                                .join(", ")
                        }
                    }
                    "authors" => {
                        if col.authors.is_empty() {
                            "-".to_string()
                        } else {
                            let first_two = &col.authors[..col.authors.len().min(2)];
                            let s = first_two.join(", ");
                            if col.authors.len() > 2 {
                                format!("{}...", s)
                            } else {
                                s
                            }
                        }
                    }
                    "year" => {
                        if col.year > 0 {
                            col.year.to_string()
                        } else {
                            "-".to_string()
                        }
                    }
                    "abstract" => {
                        if col.r#abstract.is_empty() {
                            "-".to_string()
                        } else if col.r#abstract.len() > 100 {
                            format!("{}...", &col.r#abstract[..100])
                        } else {
                            col.r#abstract.clone()
                        }
                    }
                    _ => "-".to_string(),
                };
                row_values.insert(col.paper_id.clone(), val);
            }

            aspect_rows.push(AspectRow {
                aspect: aspect.clone(),
                values: row_values,
            });
        }

        ComparisonResult {
            columns,
            aspect_rows,
        }
    }

    pub fn render_text(&self, result: &ComparisonResult) -> String {
        if result.columns.is_empty() {
            return "No papers to compare.".to_string();
        }

        let mut lines = vec![
            "================================================================================".to_string(),
            "📊 Paper Comparison".to_string(),
            "================================================================================".to_string(),
            String::new(),
        ];

        let mut header: Vec<String> = vec!["Aspect".to_string()];
        for col in &result.columns {
            let title = if col.title.len() > 25 {
                format!("{}...", &col.title[..22])
            } else {
                col.title.clone()
            };
            header.push(title);
        }
        lines.push(header.iter().map(|h| format!("{:^25}", h)).collect::<Vec<_>>().join(" | "));
        lines.push("--------------------------------------------------------------------------------".to_string());

        for row in &result.aspect_rows {
            let mut row_str: Vec<String> = vec![format!("{:12}", row.values.get("aspect").cloned().unwrap_or_default())];
            for col in &result.columns {
                let val = row.values.get(&col.paper_id).cloned().unwrap_or_else(|| "-".to_string());
                let val = if val.len() > 25 {
                    format!("{}...", &val[..22])
                } else {
                    val
                };
                row_str.push(format!("{:^25}", val));
            }
            lines.push(row_str.join(" | "));
        }

        lines.push("--------------------------------------------------------------------------------".to_string());
        lines.push(String::new());
        lines.join("\n")
    }

    pub fn render_markdown(&self, result: &ComparisonResult) -> String {
        let mut lines = Vec::new();
        lines.push("# Paper Comparison\n".to_string());

        if result.columns.is_empty() {
            return format!("{}\nNo papers to compare.", lines.join(""));
        }

        let mut header: Vec<String> = vec!["| Aspect |".to_string()];
        for col in &result.columns {
            let title = if col.title.len() > 40 {
                format!(" {} |", &col.title[..37])
            } else {
                format!(" {} |", col.title)
            };
            header.push(title);
        }
        lines.push(header.join(""));
        lines.push("|".to_string() + &"|".repeat(result.columns.len()) + "|");

        for row in &result.aspect_rows {
            let mut cells: Vec<String> = vec![format!("| {}", row.values.get("aspect").cloned().unwrap_or_default())];
            for col in &result.columns {
                let val = row.values.get(&col.paper_id).cloned().unwrap_or_else(|| "-".to_string());
                cells.push(format!(" {} |", val));
            }
            lines.push(cells.join(""));
        }

        lines.join("\n")
    }

    pub fn render_diff(&self, col_a: &ComparisonColumn, col_b: &ComparisonColumn, field: &str) -> String {
        let (a_items, b_items): (Vec<String>, Vec<String>) = match field {
            "methods" => (col_a.methods.clone(), col_b.methods.clone()),
            "datasets" => (col_a.datasets.clone(), col_b.datasets.clone()),
            _ => (Vec::new(), Vec::new()),
        };

        let a_title = &col_a.title;
        let b_title = &col_b.title;

        let mut lines = Vec::new();
        lines.push(format!(
            "=== Diff: {} vs {} ===",
            if a_title.len() > 30 { &a_title[..30] } else { a_title },
            if b_title.len() > 30 { &b_title[..30] } else { b_title }
        ));
        lines.push(format!("--- {} ---", field));

        let mut a_sorted = a_items.clone();
        let mut b_sorted = b_items.clone();
        a_sorted.sort();
        b_sorted.sort();

        let unified = unified_diff(
            &a_sorted,
            &b_sorted,
            Some(&format!("Paper A ({})", field)),
            Some(&format!("Paper B ({})", field)),
        );

        if unified.is_empty() {
            lines.push("(No differences)".to_string());
        } else {
            lines.extend(unified);
        }

        lines.join("\n")
    }
}

impl Default for PaperComparator {
    fn default() -> Self {
        Self::new()
    }
}

fn capitalize(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        None => String::new(),
        Some(c) => c.to_uppercase().collect::<String>() + chars.as_str(),
    }
}

fn unified_diff(a: &[String], b: &[String], from_file: Option<&str>, to_file: Option<&str>) -> Vec<String> {
    let mut result = Vec::new();

    let from_file = from_file.unwrap_or("a");
    let to_file = to_file.unwrap_or("b");

    let max_len = a.len().max(b.len());
    let mut i = 0;

    while i < max_len {
        let a_line = a.get(i);
        let b_line = b.get(i);

        if a_line != b_line {
            if let Some(al) = a_line {
                result.push(format!("--- {}", from_file));
                result.push(format!("+++ {}", al));
            }
            if let Some(bl) = b_line {
                result.push(format!("+++ {}", to_file));
                result.push(format!("--- {}", bl));
            }
        }
        i += 1;
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_comparison_column_new() {
        let col = ComparisonColumn::new("p1", "Attention Is All You Need");
        assert_eq!(col.paper_id, "p1");
        assert_eq!(col.title, "Attention Is All You Need");
    }

    #[test]
    fn test_add_paper_extracts_methods() {
        let comp = PaperComparator::new();
        let col = comp.add_paper("p1", "BERT Pre-training", 2018, vec!["Author".to_string()], "We propose a new transformer method with attention.");
        assert!(!col.methods.is_empty());
        assert!(col.methods.contains(&"Attention".to_string()));
        assert!(col.methods.contains(&"Transformer".to_string()));
    }

    #[test]
    fn test_add_paper_extracts_datasets() {
        let comp = PaperComparator::new();
        let col = comp.add_paper("p1", "BERT", 2018, vec![], "We evaluate on GLUE and MNLI benchmarks.");
        assert!(col.datasets.contains(&"GLUE".to_string()));
        assert!(col.datasets.contains(&"MNLI".to_string()));
    }

    #[test]
    fn test_add_paper_extracts_metrics() {
        let comp = PaperComparator::new();
        let col = comp.add_paper("p1", "Paper", 2020, vec![], "Our method achieves 90% accuracy on the test set.");
        assert!(col.metrics.contains_key("accuracy"));
    }

    #[test]
    fn test_compare_text() {
        let comp = PaperComparator::new();
        let col1 = comp.add_paper("p1", "Paper A", 2020, vec!["Auth1".to_string()], "Transformer method with attention.");
        let col2 = comp.add_paper("p2", "Paper B", 2021, vec!["Auth2".to_string()], "CNN method for images.");
        let result = comp.compare(vec![col1, col2], vec!["methods".to_string()]);

        assert_eq!(result.columns.len(), 2);
        assert_eq!(result.aspect_rows.len(), 1);
        assert_eq!(result.aspect_rows[0].aspect, "methods");
    }

    #[test]
    fn test_render_text_empty() {
        let comp = PaperComparator::new();
        let result = ComparisonResult::default();
        let text = comp.render_text(&result);
        assert_eq!(text, "No papers to compare.");
    }

    #[test]
    fn test_render_text_with_data() {
        let comp = PaperComparator::new();
        let col1 = comp.add_paper("p1", "Paper A", 2020, vec!["Auth1".to_string()], "A method.");
        let col2 = comp.add_paper("p2", "Paper B", 2021, vec!["Auth2".to_string()], "B method.");
        let result = comp.compare(vec![col1, col2], vec!["methods".to_string()]);
        let text = comp.render_text(&result);
        assert!(text.contains("Paper Comparison"));
        assert!(text.contains("Methods"));
    }

    #[test]
    fn test_render_markdown() {
        let comp = PaperComparator::new();
        let col1 = comp.add_paper("p1", "Paper A", 2020, vec![], "A.");
        let col2 = comp.add_paper("p2", "Paper B", 2021, vec![], "B.");
        let result = comp.compare(vec![col1, col2], vec!["year".to_string()]);
        let md = comp.render_markdown(&result);
        assert!(md.contains("# Paper Comparison"));
        assert!(md.contains("Year"));
    }

    #[test]
    fn test_render_diff() {
        let comp = PaperComparator::new();
        let col1 = comp.add_paper("p1", "Paper A", 2020, vec![], "Transformer.");
        let col2 = comp.add_paper("p2", "Paper B", 2021, vec![], "CNN.");
        let diff = comp.render_diff(&col1, &col2, "methods");
        assert!(diff.contains("Diff"));
        assert!(diff.contains("methods"));
    }

    #[test]
    fn test_compare_default_aspects() {
        let comp = PaperComparator::new();
        let col1 = comp.add_paper("p1", "Paper A", 2020, vec!["A".to_string()], "A method.");
        let result = comp.compare(vec![col1], vec![]);
        assert_eq!(result.aspect_rows.len(), 4);
    }

    #[test]
    fn test_compare_authors() {
        let comp = PaperComparator::new();
        let col = comp.add_paper("p1", "Paper", 2020, vec!["A".to_string(), "B".to_string(), "C".to_string()], "");
        let result = comp.compare(vec![col], vec!["authors".to_string()]);
        let val = result.aspect_rows[0].values.get("p1").unwrap();
        assert!(val.contains("A"));
        assert!(val.contains("..."));
    }
}
