//! LLM-based experiment table parser with regex fallback.
//!
//! Provides [`ExperimentTableParser`] which parses raw table data into structured
//! JSON. When no LLM client is provided, it falls back to heuristic regex parsing.

use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::LazyLock;
use thiserror::Error;

static RE_TABLE_NUM: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^\s*[\d.]+\s*$").unwrap());
static RE_NUM: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"[\d.]+").unwrap());

/// Metric keyword list for column identification.
const METRIC_KW: &[&str] = &[
    "accuracy",
    "precision",
    "recall",
    "f1",
    "bleu",
    "rouge",
    "ppl",
    "perplexity",
    "auc",
];

/// Dataset keyword list for column identification.
const DATASET_KW: &[&str] = &["dataset", "bench", "task", "corpus"];

/// Model keyword list for column identification.
const MODEL_KW: &[&str] = &["model", "method", "approach", "system"];

/// Keywords indicating the "our method" / proposed method row.
const OUR_KW: &[&str] = &[
    "ours", "our", "proposed", "this", "method", "approach", "system",
];

/// Errors from parsing operations.
#[derive(Error, Debug)]
pub enum ParseError {
    #[error("Table data is empty or has fewer than 2 rows")]
    TooFewRows,
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("Parse error: {0}")]
    Other(String),
}

/// A parsed metric entry.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ParsedMetric {
    pub name: String,
    pub value: f64,
}

/// Best score achieved by "our method".
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ParsedOursBest {
    pub value: f64,
    pub dataset: String,
    pub metric: String,
}

/// A structured table parsed from raw data.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ParsedTable {
    pub caption: String,
    pub metrics: Vec<ParsedMetric>,
    pub datasets: Vec<String>,
    pub models: Vec<String>,
    #[serde(default)]
    pub baselines: HashMap<String, f64>,
    pub ours_best: ParsedOursBest,
}

/// The result of parsing one or more tables.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ParseResult {
    pub tables: Vec<ParsedTable>,
}

/// Experiment table parser with optional LLM backend.
pub struct ExperimentTableParser {
    _llm_client: Option<()>, // Placeholder for optional LLM integration
}

impl ExperimentTableParser {
    /// Creates a new parser. `llm_client` is unused in this pure-Rust port —
    /// the parser always uses the regex fallback.
    #[allow(dead_code)]
    pub fn new(llm_client: Option<()>) -> Self {
        Self {
            _llm_client: llm_client,
        }
    }

    /// Parses raw table data into a structured result.
    ///
    /// Returns `ParseResult` containing a list of parsed tables.
    /// Always uses the regex fallback (LLM integration is out of scope for this port).
    pub fn parse_table_to_struct(
        &self,
        table_data: &[Vec<String>],
        context_title: &str,
    ) -> Result<ParseResult, ParseError> {
        self.regex_parse(table_data, context_title)
    }

    /// Regex-based fallback parser using heuristics.
    fn regex_parse(
        &self,
        table_data: &[Vec<String>],
        title: &str,
    ) -> Result<ParseResult, ParseError> {
        if table_data.is_empty() || table_data.len() < 2 {
            return Err(ParseError::TooFewRows);
        }

        let header: Vec<String> = table_data[0]
            .iter()
            .map(|s| s.trim().to_lowercase())
            .collect();
        let rows = &table_data[1..];

        // Identify metric, dataset, and model columns
        let mut metric_cols: Vec<(usize, String)> = Vec::new();
        let mut dataset_cols: Vec<(usize, String)> = Vec::new();
        let mut model_cols: Vec<(usize, String)> = Vec::new();

        for (i, h) in header.iter().enumerate() {
            if h.is_empty() {
                continue;
            }
            if METRIC_KW.iter().any(|kw| h.contains(kw)) {
                metric_cols.push((i, h.clone()));
            } else if DATASET_KW.iter().any(|kw| h.contains(kw)) {
                dataset_cols.push((i, h.clone()));
            } else if MODEL_KW.iter().any(|kw| h.contains(kw)) {
                model_cols.push((i, h.clone()));
            }
        }

        // If no metric columns found, detect numeric-only columns
        if metric_cols.is_empty() {
            for row in rows.iter().take(3) {
                for (j, cell) in row.iter().enumerate() {
                    if RE_TABLE_NUM.is_match(cell.trim()) {
                        metric_cols.push((j, format!("metric_{}", j)));
                    }
                }
            }
        }

        let mut metrics: Vec<(String, f64, String)> = Vec::new(); // (name, value, model)
        let mut datasets: HashSet<String> = HashSet::new();
        let mut models: HashSet<String> = HashSet::new();
        let mut baselines: HashMap<String, f64> = HashMap::new();
        let mut ours_best_val: f64 = 0.0;
        let mut ours_best_dataset: String = title.to_string();
        let mut ours_best_metric: String = String::new();

        for row in rows {
            let needed_cols: Vec<usize> = metric_cols
                .iter()
                .chain(dataset_cols.iter())
                .chain(model_cols.iter())
                .map(|(i, _)| *i)
                .collect();
            if !needed_cols.is_empty() && row.len() <= *needed_cols.iter().max().unwrap_or(&0) {
                continue;
            }

            // Model name
            let mut model_name = String::new();
            if let Some(&(ci, _)) = model_cols.first() {
                if ci < row.len() {
                    model_name = row[ci].trim().to_string();
                }
            } else if !row.is_empty() {
                model_name = row[0].trim().to_string();
            }

            if !model_name.is_empty() {
                models.insert(model_name.clone());
                let row_text = row.join(" ").to_lowercase();
                let is_our_row = OUR_KW.iter().any(|kw| row_text.contains(kw));

                // Dataset
                for &(di, _) in &dataset_cols {
                    if di < row.len() {
                        let ds = row[di].trim().to_string();
                        if !ds.is_empty() {
                            datasets.insert(ds);
                        }
                    }
                }

                // Metrics
                for &(mi, ref mname) in &metric_cols {
                    if mi < row.len() {
                        let raw = row[mi].trim().to_string();
                        if let Some(caps) = RE_NUM.find(&raw) {
                            if let Ok(val) = caps.as_str().parse::<f64>() {
                                metrics.push((mname.clone(), val, model_name.clone()));
                                if val > ours_best_val {
                                    ours_best_val = val;
                                    ours_best_metric = mname.clone();
                                    if let Some(&(di, _)) = dataset_cols.first() {
                                        if di < row.len() {
                                            ours_best_dataset = row[di].trim().to_string();
                                        }
                                    }
                                }
                                if !is_our_row && !model_name.is_empty() {
                                    baselines.insert(model_name.clone(), val);
                                }
                            }
                        }
                    }
                }
            }
        }

        // Deduplicate metrics by name, keeping highest value
        let mut unique_metrics: HashMap<String, f64> = HashMap::new();
        for (name, val, _) in &metrics {
            unique_metrics
                .entry(name.clone())
                .and_modify(|v| {
                    if *v < *val {
                        *v = *val;
                    }
                })
                .or_insert(*val);
        }

        let parsed_metrics: Vec<ParsedMetric> = unique_metrics
            .into_iter()
            .map(|(name, value)| ParsedMetric { name, value })
            .collect();

        let mut parsed_datasets: Vec<String> = datasets.into_iter().collect();
        parsed_datasets.sort();
        let mut parsed_models: Vec<String> = models.into_iter().collect();
        parsed_models.sort();

        let parsed_tables = vec![ParsedTable {
            caption: title.to_string(),
            metrics: parsed_metrics,
            datasets: parsed_datasets,
            models: parsed_models,
            baselines,
            ours_best: ParsedOursBest {
                value: ours_best_val,
                dataset: ours_best_dataset,
                metric: ours_best_metric,
            },
        }];

        Ok(ParseResult {
            tables: parsed_tables,
        })
    }
}

use std::collections::hash_set::HashSet;

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn parser() -> ExperimentTableParser {
        ExperimentTableParser::new(None)
    }

    #[test]
    fn test_parse_empty_table() {
        let p = parser();
        let result = p.parse_table_to_struct(&[], "Title");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_single_row_table() {
        let p = parser();
        let table = vec![vec!["Model".to_string(), "Accuracy".to_string()]];
        let result = p.parse_table_to_struct(&table, "Title");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_basic_results_table() {
        let p = parser();
        let table = vec![
            vec![
                "Model".to_string(),
                "Accuracy".to_string(),
                "F1".to_string(),
            ],
            vec!["BERT".to_string(), "90.5".to_string(), "88.0".to_string()],
            vec![
                "RoBERTa".to_string(),
                "92.1".to_string(),
                "90.3".to_string(),
            ],
        ];
        let result = p.parse_table_to_struct(&table, "Main Results").unwrap();
        assert_eq!(result.tables.len(), 1);
        let pt = &result.tables[0];
        assert_eq!(pt.caption, "Main Results");
        assert_eq!(pt.models, vec!["BERT", "RoBERTa"]);
        // Metrics are deduplicated by name, keeping highest value
        assert!(pt
            .metrics
            .iter()
            .any(|m| m.name == "accuracy" && (m.value - 92.1).abs() < 0.01));
        assert!(pt
            .metrics
            .iter()
            .any(|m| m.name == "f1" && (m.value - 90.3).abs() < 0.01));
    }

    #[test]
    fn test_parse_table_with_dataset_column() {
        let p = parser();
        let table = vec![
            vec![
                "Dataset".to_string(),
                "Model".to_string(),
                "Accuracy".to_string(),
            ],
            vec!["SQuAD".to_string(), "BERT".to_string(), "88.0".to_string()],
            vec![
                "SQuAD".to_string(),
                "RoBERTa".to_string(),
                "92.0".to_string(),
            ],
            vec!["MNLI".to_string(), "BERT".to_string(), "85.0".to_string()],
            vec![
                "MNLI".to_string(),
                "RoBERTa".to_string(),
                "86.5".to_string(),
            ],
        ];
        let result = p.parse_table_to_struct(&table, "Results").unwrap();
        let pt = &result.tables[0];
        assert_eq!(pt.datasets, vec!["MNLI", "SQuAD"]);
        assert!(pt.models.contains(&"BERT".to_string()));
        assert!(pt.models.contains(&"RoBERTa".to_string()));
    }

    #[test]
    fn test_parse_table_ours_best() {
        let p = parser();
        let table = vec![
            vec!["Model".to_string(), "Accuracy".to_string()],
            vec!["Baseline".to_string(), "85.0".to_string()],
            vec!["Our Method".to_string(), "92.0".to_string()],
        ];
        let result = p.parse_table_to_struct(&table, "Our Results").unwrap();
        let pt = &result.tables[0];
        // Our best should be 92.0
        assert!((pt.ours_best.value - 92.0).abs() < 0.01);
        assert_eq!(pt.ours_best.metric, "accuracy");
    }

    #[test]
    fn test_parse_table_with_metric_in_header() {
        let p = parser();
        let table = vec![
            vec![
                "Method".to_string(),
                "BLEU".to_string(),
                "ROUGE".to_string(),
            ],
            vec![
                "Seq2Seq".to_string(),
                "25.3".to_string(),
                "45.0".to_string(),
            ],
            vec![
                "Transformer".to_string(),
                "30.1".to_string(),
                "50.5".to_string(),
            ],
        ];
        let result = p
            .parse_table_to_struct(&table, "Translation Results")
            .unwrap();
        let pt = &result.tables[0];
        assert!(pt.metrics.iter().any(|m| m.name == "bleu"));
        assert!(pt.metrics.iter().any(|m| m.name == "rouge"));
    }

    #[test]
    fn test_parse_table_baselines() {
        let p = parser();
        let table = vec![
            vec!["Method".to_string(), "Score".to_string()],
            vec!["Random Baseline".to_string(), "50.0".to_string()],
            vec!["SVM".to_string(), "72.3".to_string()],
            vec!["CNN".to_string(), "78.5".to_string()],
        ];
        let result = p.parse_table_to_struct(&table, "Baselines").unwrap();
        let pt = &result.tables[0];
        // None of these are "our" so they should be in baselines
        assert!(pt.baselines.contains_key("Random Baseline"));
        assert!(pt.baselines.contains_key("SVM"));
        assert!(pt.baselines.contains_key("CNN"));
    }

    #[test]
    fn test_parse_table_no_metric_columns() {
        let p = parser();
        let table = vec![
            vec!["Text".to_string(), "Label".to_string()],
            vec!["Hello world".to_string(), "A".to_string()],
            vec!["Foo bar".to_string(), "B".to_string()],
        ];
        let result = p.parse_table_to_struct(&table, "Non-numeric");
        // No numeric columns means no metrics — still valid but empty metrics
        let pt = result.unwrap();
        assert!(pt.tables[0].metrics.is_empty());
    }

    #[test]
    fn test_parse_multiple_tables_context() {
        let p = parser();
        let table = vec![
            vec!["System".to_string(), "Accuracy".to_string()],
            vec!["Model A".to_string(), "89.0".to_string()],
            vec!["Model B".to_string(), "91.5".to_string()],
        ];
        let result = p
            .parse_table_to_struct(&table, "A Great Paper on AI")
            .unwrap();
        assert_eq!(result.tables[0].caption, "A Great Paper on AI");
    }

    #[test]
    fn test_parsed_metric_serde() {
        let m = ParsedMetric {
            name: "accuracy".to_string(),
            value: 92.5,
        };
        let json = serde_json::to_string(&m).unwrap();
        let parsed: ParsedMetric = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.name, "accuracy");
        assert!((parsed.value - 92.5).abs() < 0.01);
    }

    #[test]
    fn test_parsed_ours_best_serde() {
        let ob = ParsedOursBest {
            value: 95.0,
            dataset: "squad".to_string(),
            metric: "exact_match".to_string(),
        };
        let json = serde_json::to_string(&ob).unwrap();
        let parsed: ParsedOursBest = serde_json::from_str(&json).unwrap();
        assert!((parsed.value - 95.0).abs() < 0.01);
        assert_eq!(parsed.dataset, "squad");
    }

    #[test]
    fn test_parsed_table_serde() {
        let pt = ParsedTable {
            caption: "Main".to_string(),
            metrics: vec![ParsedMetric {
                name: "accuracy".to_string(),
                value: 90.0,
            }],
            datasets: vec!["squad".to_string()],
            models: vec!["BERT".to_string()],
            baselines: HashMap::from([("BERT".to_string(), 88.0)]),
            ours_best: ParsedOursBest {
                value: 90.0,
                dataset: "squad".to_string(),
                metric: "accuracy".to_string(),
            },
        };
        let json = serde_json::to_string(&pt).unwrap();
        let parsed: ParsedTable = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.caption, "Main");
        assert_eq!(parsed.datasets, vec!["squad"]);
    }

    #[test]
    fn test_parse_result_serde() {
        let result = ParseResult {
            tables: vec![ParsedTable {
                caption: "Test".to_string(),
                metrics: vec![],
                datasets: vec![],
                models: vec![],
                baselines: HashMap::new(),
                ours_best: ParsedOursBest {
                    value: 0.0,
                    dataset: String::new(),
                    metric: String::new(),
                },
            }],
        };
        let json = serde_json::to_string(&result).unwrap();
        let parsed: ParseResult = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.tables.len(), 1);
    }

    #[test]
    fn test_parse_table_with_decimal_in_metric_name() {
        let p = parser();
        let table = vec![
            vec!["Model".to_string(), "BLEU".to_string()],
            vec!["Transformer".to_string(), "28.5".to_string()],
        ];
        let result = p.parse_table_to_struct(&table, "Translation").unwrap();
        assert!(!result.tables[0].metrics.is_empty());
    }

    #[test]
    fn test_parse_table_row_too_short() {
        let p = parser();
        let table = vec![
            vec![
                "Model".to_string(),
                "Accuracy".to_string(),
                "F1".to_string(),
            ],
            vec!["BERT".to_string()], // only 1 element, can't fill all columns
        ];
        let result = p.parse_table_to_struct(&table, "Test");
        // Should not crash, may produce partial results
        assert!(result.is_ok());
    }

    #[test]
    fn test_metric_value_extraction_from_parenthesized() {
        let p = parser();
        let table = vec![
            vec!["Model".to_string(), "Accuracy".to_string()],
            vec!["BERT".to_string(), "90.5 (std)".to_string()],
        ];
        let result = p.parse_table_to_struct(&table, "With Variance").unwrap();
        let m = result.tables[0]
            .metrics
            .iter()
            .find(|m| m.name == "accuracy")
            .unwrap();
        assert!((m.value - 90.5).abs() < 0.01);
    }
}
