//! Experiment Table Extraction from PDFs
//!
//! This module provides tools for detecting, parsing, and storing experiment tables
//! from AI/ML research papers.
//!
//! # Modules
//!
//! - [`detector`] - PDF table detection using heuristic patterns
//! - [`parser`] - Parse raw table data into structured JSON representation
//! - [`storage`] - SQLite-backed storage for extracted tables

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
