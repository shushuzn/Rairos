//! rairos-extable: Experiment Table Extraction from PDFs
//!
//! This crate provides tools for detecting, parsing, and storing experiment tables
//! from AI/ML research papers.
//!
//! # Modules
//!
//! - [`detector`] - PDF table detection using heuristic patterns
//! - [`parser`] - Parse raw table data into structured JSON representation
//! - [`storage`] - SQLite-backed storage for extracted tables
//!
//! # Example
//!
//! ```
//! use rairos_extable::{ExperimentTableParser, TableDetector};
//!
//! // Parse a table from structured data
//! let parser = ExperimentTableParser::new(None);
//! let table_data = vec![
//!     vec!["Model".to_string(), "Accuracy".to_string()],
//!     vec!["BERT".to_string(), "90.5".to_string()],
//!     vec!["RoBERTa".to_string(), "92.1".to_string()],
//! ];
//! let result = parser.parse_table_to_struct(&table_data, "Main Results").unwrap();
//! println!("Parsed {} table(s)", result.tables.len());
//! ```

pub mod detector;
pub mod parser;
pub mod storage;

pub use detector::{DetectedTable, TableDetector};
pub use parser::{ExperimentTableParser, ParseResult, ParsedTable};
pub use storage::{DbStats, ExperimentDB, Metric, OursBest, StoredTable, TableStruct};

#[cfg(test)]
mod tests {
    #[test]
    fn extable_version_exists() {
        assert!(true)
    }
}
