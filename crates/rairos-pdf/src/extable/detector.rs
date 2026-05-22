//! PDF table detection using PyMuPDF patterns adapted to Rust.
//!
//! This module provides [`TableDetector`] which detects and extracts tables
//! from PDF pages. Detection is heuristic-based using keyword matching and
//! numeric pattern analysis.

use regex::Regex;
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::collections::BTreeMap;
use std::path::Path;

/// Metric keywords that indicate an experiment results table.
const METRIC_KEYWORDS: &[&str] = &[
    "accuracy",
    "precision",
    "recall",
    "f1",
    "bleu",
    "rouge",
    "perplexity",
    "loss",
    "auc",
    "map",
    "ndcg",
    "mrr",
    "cer",
    "wer",
    "beam",
    "latency",
    "throughput",
    "param",
    "bpc",
    "bits_per_char",
    "ppl",
    "glue",
    "super gl",
    "squad",
    "arc",
    "hella",
    "lambada",
];

/// Dataset keywords that indicate an experiment results table.
const DATASET_KEYWORDS: &[&str] = &[
    "squad",
    "glue",
    "coco",
    "imagenet",
    "mnist",
    "cifar",
    "wikitext",
    "openwebtext",
    "bookcorpus",
    "arxiv",
    "pubmed",
    "custom",
    "sst",
    "sst-2",
    "qqp",
    "mnli",
    "qnli",
    "rte",
    "cola",
];

/// Represents a detected table on a PDF page.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DetectedTable {
    /// Bounding box as (x0, y0, x1, y1)
    pub bbox: (f64, f64, f64, f64),
    /// 2D table data (rows of cell strings)
    pub data: Vec<Vec<String>>,
    /// Whether this appears to be an experiment results table
    pub is_experiment: bool,
    /// Page number
    pub page: usize,
}

/// Result of table extraction from a page.
/// In pure Rust (no PyMuPDF), this represents structured data that would come
/// from PDF parsing. A full implementation would integrate with a PDF library.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TableExtractionResult {
    pub tables: Vec<DetectedTable>,
    pub page: usize,
}

/// TableDetector detects and extracts tables from PDF pages.
///
/// Note: This is a pure-Rust implementation using regex/heuristic patterns.
/// For full PDF extraction, integrate with libraries like `lopdf` or `pdf-extract`.
#[derive(Debug)]
pub struct TableDetector {
    _has_fitz: bool,
}

/// A wrapper for f64 that implements Ord by rounding to 1 decimal place.
#[derive(Debug, Clone, Copy)]
struct RoundKey(f64);

impl PartialEq for RoundKey {
    fn eq(&self, other: &Self) -> bool {
        self.0.round() == other.0.round()
    }
}

impl Eq for RoundKey {}

impl PartialOrd for RoundKey {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for RoundKey {
    fn cmp(&self, other: &Self) -> Ordering {
        self.0
            .partial_cmp(&other.0)
            .unwrap_or(Ordering::Equal)
            .then_with(|| {
                self.0
                    .round()
                    .partial_cmp(&other.0.round())
                    .unwrap_or(Ordering::Equal)
            })
    }
}

impl TableDetector {
    /// Creates a new TableDetector.
    pub fn new() -> Self {
        Self { _has_fitz: false }
    }

    /// Detects tables on a given page.
    ///
    /// Since we don't have PyMuPDF bindings, this method accepts pre-extracted
    /// text blocks and performs heuristic detection on them.
    ///
    /// In a full integration, this would be called with data from a PDF parser.
    pub fn detect_tables_from_blocks(
        &self,
        blocks: &[PdfTextBlock],
        page_num: usize,
    ) -> Vec<DetectedTable> {
        let mut tables = Vec::new();

        for block in blocks {
            if block.block_type != 0 {
                continue;
            }

            let table_data = self.extract_table_from_block(block);
            if table_data.len() < 2 {
                continue;
            }

            let is_exp = self.is_experiment_table(&table_data);
            tables.push(DetectedTable {
                bbox: block.bbox,
                data: table_data,
                is_experiment: is_exp,
                page: page_num,
            });
        }

        tables
    }

    /// Extracts table rows from a PDF text block.
    fn extract_table_from_block(&self, block: &PdfTextBlock) -> Vec<Vec<String>> {
        let lines = &block.lines;
        if lines.is_empty() {
            return Vec::new();
        }

        // Group lines by rounded y coordinate (row) using BTreeMap with RoundKey
        let mut rows_data: BTreeMap<RoundKey, Vec<(f64, String)>> = BTreeMap::new();

        for line in lines {
            let y0 = line.bbox.1;
            let row_key = RoundKey(y0);
            for span in &line.spans {
                let x0 = span.bbox.0;
                let text = span.text.trim().to_string();
                if !text.is_empty() {
                    rows_data.entry(row_key).or_default();
                    let key = RoundKey(y0);
                    if let Some(v) = rows_data.get_mut(&key) {
                        v.push((x0, text));
                    }
                }
            }
        }

        // Sort each row by x coordinate and flatten
        let mut table = Vec::new();
        for row_key in rows_data.keys().cloned().collect::<Vec<_>>() {
            let mut cells = rows_data.remove(&row_key).unwrap();
            cells.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(Ordering::Equal));
            let row: Vec<String> = cells.into_iter().map(|c| c.1).collect();
            table.push(row);
        }

        table
    }

    /// Determines if a table is an experiment results table using heuristics.
    fn is_experiment_table(&self, table_data: &[Vec<String>]) -> bool {
        if table_data.len() < 2 {
            return false;
        }

        let header = table_data[0].join(" ").to_lowercase();
        let body: String = table_data[1..]
            .iter()
            .map(|r| r.join(" "))
            .collect::<Vec<_>>()
            .join(" ")
            .to_lowercase();

        let score_count = METRIC_KEYWORDS
            .iter()
            .filter(|kw| header.contains(*kw) || body.contains(*kw))
            .count();

        let dataset_count = DATASET_KEYWORDS
            .iter()
            .filter(|kw| body.contains(*kw))
            .count();

        let numeric_re = Regex::new(r"\d+\.?\d*").expect("valid regex");
        let numeric_cells: usize = table_data[1..]
            .iter()
            .flatten()
            .filter(|cell| numeric_re.is_match(cell))
            .count();

        (score_count >= 1 && numeric_cells >= 4) || (dataset_count >= 2 && numeric_cells >= 6)
    }

    /// Extracts all experiment tables from a PDF path.
    ///
    /// This is a stub that returns an empty vec. Full implementation would
    /// require a PDF parsing library integration.
    pub fn extract_all_tables(&self, _pdf_path: &Path, _max_pages: usize) -> Vec<DetectedTable> {
        Vec::new()
    }
}

impl Default for TableDetector {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// PDF structure types (would come from a PDF parsing library)
// ============================================================================

/// A text block from a PDF page (mirrors PyMuPDF "dict" block structure).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PdfTextBlock {
    /// Block type: 0 = text, 1 = image, etc.
    pub block_type: i32,
    /// Bounding box (x0, y0, x1, y1)
    pub bbox: (f64, f64, f64, f64),
    /// Lines of text in this block
    pub lines: Vec<PdfTextLine>,
}

/// A line of text in a PDF block.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PdfTextLine {
    /// Bounding box (x0, y0, x1, y1)
    pub bbox: (f64, f64, f64, f64),
    /// Spans (runs) of text in this line
    pub spans: Vec<PdfTextSpan>,
}

/// A span of uniformly-styled text.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PdfTextSpan {
    /// Bounding box (x0, y0, x1, y1)
    pub bbox: (f64, f64, f64, f64),
    /// The actual text content
    pub text: String,
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn make_span(x: f64, y: f64, text: &str) -> PdfTextSpan {
        PdfTextSpan {
            bbox: (x, y, x + 50.0, y + 10.0),
            text: text.to_string(),
        }
    }

    fn make_line(y: f64, spans: Vec<PdfTextSpan>) -> PdfTextLine {
        PdfTextLine {
            bbox: (0.0, y, 500.0, y + 10.0),
            spans,
        }
    }

    fn make_block(bbox: (f64, f64, f64, f64), lines: Vec<PdfTextLine>) -> PdfTextBlock {
        PdfTextBlock {
            block_type: 0,
            bbox,
            lines,
        }
    }

    #[test]
    fn test_detect_tables_from_blocks_empty() {
        let detector = TableDetector::new();
        let blocks: [PdfTextBlock; 0] = [];
        let tables = detector.detect_tables_from_blocks(&blocks, 0);
        assert!(tables.is_empty());
    }

    #[test]
    fn test_detect_tables_from_blocks_non_text_block() {
        let detector = TableDetector::new();
        let blocks = [PdfTextBlock {
            block_type: 1, // image block
            bbox: (0.0, 0.0, 100.0, 100.0),
            lines: vec![],
        }];
        let tables = detector.detect_tables_from_blocks(&blocks, 0);
        assert!(tables.is_empty());
    }

    #[test]
    fn test_extract_table_from_block_single_row() {
        let detector = TableDetector::new();
        let block = make_block(
            (0.0, 0.0, 500.0, 30.0),
            vec![make_line(
                10.0,
                vec![
                    make_span(0.0, 10.0, "Model"),
                    make_span(100.0, 10.0, "Accuracy"),
                ],
            )],
        );
        let tables = detector.detect_tables_from_blocks(&[block], 0);
        // Single row is filtered out (< 2 rows)
        assert!(tables.is_empty());
    }

    #[test]
    fn test_extract_table_from_block_multiple_rows() {
        let detector = TableDetector::new();
        let block = make_block(
            (0.0, 0.0, 500.0, 60.0),
            vec![
                make_line(
                    10.0,
                    vec![
                        make_span(0.0, 10.0, "Model"),
                        make_span(100.0, 10.0, "Accuracy"),
                    ],
                ),
                make_line(
                    20.0,
                    vec![make_span(0.0, 20.0, "BERT"), make_span(100.0, 20.0, "90.5")],
                ),
                make_line(
                    30.0,
                    vec![
                        make_span(0.0, 30.0, "RoBERTa"),
                        make_span(100.0, 30.0, "92.1"),
                    ],
                ),
            ],
        );
        let tables = detector.detect_tables_from_blocks(&[block], 0);
        assert_eq!(tables.len(), 1);
        let table = &tables[0];
        assert_eq!(table.data.len(), 3);
        assert_eq!(table.data[0], ["Model", "Accuracy"]);
        assert_eq!(table.data[1], ["BERT", "90.5"]);
        assert_eq!(table.data[2], ["RoBERTa", "92.1"]);
    }

    #[test]
    fn test_is_experiment_table_empty() {
        let detector = TableDetector::new();
        assert!(!detector.is_experiment_table(&[]));
    }

    #[test]
    fn test_is_experiment_table_single_row() {
        let detector = TableDetector::new();
        assert!(!detector.is_experiment_table(&[vec!["Model".to_string(), "Accuracy".to_string()]]));
    }

    #[test]
    fn test_is_experiment_table_with_metrics_and_numbers() {
        let detector = TableDetector::new();
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
        // Has "accuracy" and "f1" keywords, and 4+ numeric cells
        assert!(detector.is_experiment_table(&table));
    }

    #[test]
    fn test_is_experiment_table_with_datasets() {
        let detector = TableDetector::new();
        let table = vec![
            vec![
                "Method".to_string(),
                "SQuAD".to_string(),
                "MNLI".to_string(),
            ],
            vec![
                "Our Method".to_string(),
                "92.1".to_string(),
                "86.5".to_string(),
            ],
            vec![
                "Baseline".to_string(),
                "88.0".to_string(),
                "82.1".to_string(),
            ],
            vec!["BERT".to_string(), "85.5".to_string(), "80.3".to_string()],
        ];
        // Has "squad", "mnli" dataset keywords, and 6+ numeric cells
        assert!(detector.is_experiment_table(&table));
    }

    #[test]
    fn test_is_experiment_table_not_enough_numbers() {
        let detector = TableDetector::new();
        let table = vec![
            vec!["Model".to_string(), "Description".to_string()],
            vec!["BERT".to_string(), "Large".to_string()],
        ];
        // Has no metric keywords, no dataset keywords, only 0 numeric cells
        assert!(!detector.is_experiment_table(&table));
    }

    #[test]
    fn test_is_experiment_table_no_header_match() {
        let detector = TableDetector::new();
        let table = vec![
            vec!["Item".to_string(), "Count".to_string()],
            vec!["Foo".to_string(), "123".to_string()],
            vec!["Bar".to_string(), "456".to_string()],
            vec!["Baz".to_string(), "789".to_string()],
            vec!["Qux".to_string(), "012".to_string()],
        ];
        // Has numeric cells but no metric/dataset keywords in header or body
        assert!(!detector.is_experiment_table(&table));
    }

    #[test]
    fn test_table_detector_default() {
        let detector = TableDetector::default();
        let blocks: [PdfTextBlock; 0] = [];
        assert!(detector.detect_tables_from_blocks(&blocks, 0).is_empty());
    }

    #[test]
    fn test_extract_all_tables_returns_empty() {
        let detector = TableDetector::new();
        let result = detector.extract_all_tables(Path::new("nonexistent.pdf"), 5);
        assert!(result.is_empty());
    }

    #[test]
    fn test_pdf_text_block_serde() {
        let block = PdfTextBlock {
            block_type: 0,
            bbox: (0.0, 0.0, 100.0, 50.0),
            lines: vec![PdfTextLine {
                bbox: (0.0, 0.0, 100.0, 10.0),
                spans: vec![PdfTextSpan {
                    bbox: (0.0, 0.0, 50.0, 10.0),
                    text: "Hello".to_string(),
                }],
            }],
        };

        let json_str = serde_json::to_string(&block).unwrap();
        let parsed: PdfTextBlock = serde_json::from_str(&json_str).unwrap();
        assert_eq!(parsed.block_type, 0);
        assert_eq!(parsed.lines[0].spans[0].text, "Hello");
    }

    #[test]
    fn test_detected_table_serde() {
        let table = DetectedTable {
            bbox: (0.0, 0.0, 100.0, 50.0),
            data: vec![
                vec!["Model".to_string(), "Accuracy".to_string()],
                vec!["BERT".to_string(), "90.5".to_string()],
            ],
            is_experiment: true,
            page: 0,
        };

        let json_str = serde_json::to_string(&table).unwrap();
        let parsed: DetectedTable = serde_json::from_str(&json_str).unwrap();
        assert_eq!(parsed.data[0][0], "Model");
        assert!(parsed.is_experiment);
    }

    #[test]
    fn test_rows_sorted_by_y_coordinate() {
        let detector = TableDetector::new();
        // Insert lines out of order
        let block = make_block(
            (0.0, 0.0, 500.0, 90.0),
            vec![
                make_line(30.0, vec![make_span(0.0, 30.0, "Row 3")]),
                make_line(10.0, vec![make_span(0.0, 10.0, "Row 1")]),
                make_line(20.0, vec![make_span(0.0, 20.0, "Row 2")]),
            ],
        );
        let tables = detector.detect_tables_from_blocks(&[block], 0);
        assert_eq!(tables.len(), 1);
        assert_eq!(tables[0].data[0][0], "Row 1");
        assert_eq!(tables[0].data[1][0], "Row 2");
        assert_eq!(tables[0].data[2][0], "Row 3");
    }
}
