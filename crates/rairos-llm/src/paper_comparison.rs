//! Paper Comparison — compare multiple papers side-by-side by methods/datasets/metrics.
//!
//! Mirrors llm/paper_comparison.py

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ─── Data types ───────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComparisonColumn {
    pub paper_id: String,
    pub title: String,
    pub year: i32,
    pub authors: Vec<String>,
    pub methods: Vec<String>,
    pub datasets: Vec<String>,
    pub metrics: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AspectRow {
    pub aspect: String,
    pub values: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComparisonResult {
    pub columns: Vec<ComparisonColumn>,
    pub aspect_rows: Vec<AspectRow>,
}

// ─── Keyword maps ─────────────────────────────────────────────────────────

const METHOD_KEYWORDS: &[(&str, &str)] = &[
    ("transformer", "Transformer"),
    ("bert", "BERT"),
    ("gpt", "GPT"),
    ("lstm", "LSTM"),
    ("cnn", "CNN"),
    ("gan", "GAN"),
    ("attention", "Attention"),
    ("embedding", "Embedding"),
    ("retrieval", "Retrieval"),
    ("rag", "RAG"),
    ("fine-tun", "Fine-tuning"),
    ("rlhf", "RLHF"),
    ("chain-of-thought", "Chain-of-Thought"),
    ("prompt", "Prompting"),
];

const DATASET_KEYWORDS: &[(&str, &str)] = &[
    ("glue", "GLUE"),
    ("super.glue", "SuperGLUE"),
    ("squad", "SQuAD"),
    ("natural questions", "NQ"),
    ("triviaqa", "TriviaQA"),
    ("mmlu", "MMLU"),
    ("humaneval", "HumanEval"),
    ("mbpp", "MBPP"),
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
];

const METRIC_NAMES: &[(&str, &str)] = &[
    ("accuracy", "Acc"),
    ("precision", "Prec"),
    ("recall", "Rec"),
    ("f1", "F1"),
    ("bleu", "BLEU"),
    ("rouge", "ROUGE"),
    ("perplexity", "PPL"),
    ("latency", "Latency"),
    ("throughput", "Throughput"),
];

// ─── Extraction functions ─────────────────────────────────────────────────

pub fn extract_methods(title: &str, abstract_text: &str, method: &str) -> Vec<String> {
    let text = format!("{} {} {}", title, abstract_text, method).to_lowercase();
    let mut found: Vec<String> = Vec::new();
    for (kw, name) in METHOD_KEYWORDS {
        if text.contains(kw) && !found.contains(&name.to_string()) {
            found.push(name.to_string());
        }
        if found.len() >= 5 {
            break;
        }
    }
    found
}

pub fn extract_datasets(title: &str, abstract_text: &str, dataset: &str) -> Vec<String> {
    let text = format!("{} {} {}", title, abstract_text, dataset).to_lowercase();
    let mut found: Vec<String> = Vec::new();
    for (kw, name) in DATASET_KEYWORDS {
        if text.contains(kw) && !found.contains(&name.to_string()) {
            found.push(name.to_string());
        }
        if found.len() >= 5 {
            break;
        }
    }
    found
}

pub fn extract_metrics(abstract_text: &str, result: &str, metrics: &str) -> HashMap<String, String> {
    let text = format!("{} {} {}", abstract_text, result, metrics).to_lowercase();
    let mut found: HashMap<String, String> = HashMap::new();

    // Keyword presence
    for (kw, name) in METRIC_NAMES {
        if text.contains(kw) {
            found.insert(name.to_string(), "✓".to_string());
        }
    }

    // Numeric value extraction
    let patterns: &[(&str, &str)] = &[
        (r"(\d+\.?\d*)\s*%?\s*(accuracy)", "$1%"),
        (r"(\d+\.?\d*)\s*(bleu)", "$1"),
        (r"(\d+\.?\d*)\s*(f1)", "$1"),
    ];

    for (pattern, _) in patterns {
        if let Ok(re) = regex::Regex::new(pattern) {
            if let Some(caps) = re.captures(&text) {
                let val = caps.get(1).map(|m| m.as_str()).unwrap_or("✓");
                let key = caps.get(2).map(|m| m.as_str().to_string()).unwrap_or_default();
                if !key.is_empty() {
                    found.insert(key, val.to_string());
                }
            }
        }
    }

    found
}

pub fn parse_authors(authors: &[String]) -> Vec<String> {
    authors.iter().take(5).cloned().collect()
}

pub fn add_paper(
    paper_id: &str,
    title: &str,
    year: i32,
    authors: &[String],
    abstract_text: &str,
    method: &str,
    dataset: &str,
) -> ComparisonColumn {
    ComparisonColumn {
        paper_id: paper_id.to_string(),
        title: title.to_string(),
        year,
        authors: parse_authors(authors),
        methods: extract_methods(title, abstract_text, method),
        datasets: extract_datasets(title, abstract_text, dataset),
        metrics: extract_metrics(abstract_text, "", ""),
    }
}

pub fn compare(aspects: &[&str], columns: &[ComparisonColumn]) -> Vec<AspectRow> {
    let default_aspects = if aspects.is_empty() {
        vec!["methods", "datasets", "metrics", "authors"]
    } else {
        aspects.to_vec()
    };

    default_aspects.iter().map(|&aspect| {
        let values: HashMap<String, String> = columns.iter().map(|col| {
            let val = match aspect {
                "methods" => Some(col.methods.join(", ")),
                "datasets" => Some(col.datasets.join(", ")),
                "metrics" => {
                    Some(col.metrics.iter().map(|(k, v)| format!("{}={}", k, v)).collect::<Vec<_>>().join(", "))
                }
                "authors" => {
                    let a = &col.authors;
                    let first = a.first().cloned().unwrap_or_default();
                    let rest = if a.len() > 2 { "+" } else { "" };
                    Some(format!("{}{}", first, rest))
                }
                "year" => Some(if col.year > 0 { col.year.to_string() } else { "-".to_string() }),
                "abstract" => {
                    let abs = if col.title.len() > 100 {
                        format!("{}...", &col.title[..100])
                    } else {
                        col.title.clone()
                    };
                    Some(if abs.is_empty() { "-".to_string() } else { abs })
                }
                _ => Some("-".to_string()),
            };
            (col.paper_id.clone(), val.unwrap_or_else(|| "-".to_string()))
        }).collect();

        AspectRow {
            aspect: aspect[..1].to_uppercase() + &aspect[1..],
            values,
        }
    }).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_methods_bert() {
        let m = extract_methods("BERT Model", "We use BERT for classification.", "");
        assert!(m.contains(&"BERT".to_string()));
    }

    #[test]
    fn test_extract_methods_limit() {
        let m = extract_methods("A B C D E F G H", "transformer bert lstm cnn gan attention", "");
        assert!(m.len() <= 5);
    }

    #[test]
    fn test_extract_datasets_squad() {
        let d = extract_datasets("SQuAD dataset", "We evaluate on SQuAD.", "");
        assert!(d.contains(&"SQuAD".to_string()));
    }

    #[test]
    fn test_extract_datasets_limit() {
        let d = extract_datasets("glue squad mnli qnli cola sst stsb", "", "");
        assert!(d.len() <= 5);
    }

    #[test]
    fn test_extract_metrics_with_values() {
        let m = extract_metrics("We achieved 92.5 accuracy and 0.89 f1", "", "");
        assert!(m.contains_key("Acc") || m.contains_key("F1"));
    }

    #[test]
    fn test_parse_authors_limit() {
        let authors: Vec<String> = (1..=10).map(|i| format!("Author {}", i)).collect();
        let parsed = parse_authors(&authors);
        assert_eq!(parsed.len(), 5);
    }

    #[test]
    fn test_compare_methods_aspect() {
        let cols = vec![
            ComparisonColumn {
                paper_id: "p1".into(), title: "Paper 1".into(), year: 2024,
                authors: vec!["A".into()], methods: vec!["BERT".into()],
                datasets: vec![], metrics: HashMap::new(),
            },
        ];
        let rows = compare(&["methods"], &cols);
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].aspect, "Methods");
        assert!(rows[0].values.get("p1").unwrap().contains("BERT"));
    }

    #[test]
    fn test_compare_empty_columns() {
        let rows = compare(&[], &[]);
        assert!(!rows.is_empty()); // uses defaults
    }
}
