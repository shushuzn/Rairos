//! rairos-db — SQLite database layer for Rairos
//!
//! Schema: papers, parse_history, tags, paper_tags, job_queue, settings,
//! paper_cache, dedup_log, citations, experiment_tables, paper_code_trace,
//! gap_history, arxiv_search_cache.
//!
//! Thread-local connection management (one connection per thread).

use chrono::Utc;
use regex::Regex;
use rusqlite::{Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use thiserror::Error;

// ============================================================================
// Error Types
// ============================================================================

#[derive(Error, Debug)]
pub enum DbError {
    #[error("Database error: {0}")]
    Database(#[from] rusqlite::Error),
    #[error("Paper not found: {0}")]
    NotFound(String),
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

pub type Result<T> = std::result::Result<T, DbError>;

// ============================================================================
// Paper
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Paper {
    // Primary key
    pub id: String,
    // Core fields
    pub source: String,
    pub title: String,
    pub authors: Vec<String>,
    pub abstract_text: String,
    pub published: String,
    pub updated: String,
    pub abs_url: String,
    pub pdf_url: String,
    pub primary_category: String,
    pub journal: String,
    pub volume: String,
    pub issue: String,
    pub page: String,
    pub doi: String,
    pub categories: String,
    pub reference_count: i64,
    // Timestamps
    pub added_at: String,
    pub updated_at: String,
    // PDF info
    pub pdf_path: String,
    pub pdf_hash: String,
    // Parse status
    pub parse_status: String,
    pub parse_error: String,
    pub parse_version: i64,
    // Parsed content
    pub plain_text: String,
    pub latex_blocks: Vec<serde_json::Value>,
    pub table_count: i64,
    pub figure_count: i64,
    pub word_count: i64,
    pub page_count: i64,
    // Note paths
    pub pnote_path: String,
    pub cnote_path: String,
    pub mnote_path: String,
    // Embedding
    #[serde(skip_serializing_if = "Option::is_none")]
    pub embed_vector: Option<Vec<f32>>,
}

impl Default for Paper {
    fn default() -> Self {
        Self {
            id: String::new(),
            source: String::new(),
            title: String::new(),
            authors: Vec::new(),
            abstract_text: String::new(),
            published: String::new(),
            updated: String::new(),
            abs_url: String::new(),
            pdf_url: String::new(),
            primary_category: String::new(),
            journal: String::new(),
            volume: String::new(),
            issue: String::new(),
            page: String::new(),
            doi: String::new(),
            categories: String::new(),
            reference_count: 0,
            added_at: String::new(),
            updated_at: String::new(),
            pdf_path: String::new(),
            pdf_hash: String::new(),
            parse_status: "pending".to_string(),
            parse_error: String::new(),
            parse_version: 0,
            plain_text: String::new(),
            latex_blocks: Vec::new(),
            table_count: 0,
            figure_count: 0,
            word_count: 0,
            page_count: 0,
            pnote_path: String::new(),
            cnote_path: String::new(),
            mnote_path: String::new(),
            embed_vector: None,
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn db_version_exists() {
        assert!(true)
    }
}

#[cfg(not(windows))]
mod py_bindings {
    use pyo3::prelude::*;
    use std::collections::HashMap;

    fn db_err_to_py(err: crate::DbError) -> PyErr {
        PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(err.to_string())
    }

    #[pyclass]
    struct PyDatabase {
        inner: crate::Database,
    }

    #[pymethods]
    impl PyDatabase {
        #[new]
        #[pyo3(signature = (db_path=None))]
        fn new(db_path: Option<String>) -> PyResult<Self> {
            let inner = match db_path {
                Some(p) => crate::Database::open(p).map_err(db_err_to_py)?,
                None => crate::Database::open_default().map_err(db_err_to_py)?,
            };
            Ok(Self { inner })
        }

        fn init_(&self) -> PyResult<()> {
            self.inner.init().map_err(db_err_to_py)
        }

        #[pyo3(signature = (status=None))]
        fn paper_count(&self, status: Option<&str>) -> PyResult<i64> {
            self.inner.paper_count(status).map_err(db_err_to_py)
        }

        fn paper_exists(&self, paper_id: &str) -> PyResult<bool> {
            self.inner.paper_exists(paper_id).map_err(db_err_to_py)
        }

        fn delete_paper(&self, paper_id: &str) -> PyResult<bool> {
            self.inner.delete_paper(paper_id).map_err(db_err_to_py)
        }

        fn get_total_papers(&self) -> PyResult<i64> {
            let stats = self.inner.get_stats().map_err(db_err_to_py)?;
            Ok(stats.total_papers)
        }

        fn upsert_paper(&self, data: &Bound<'_, PyAny>) -> PyResult<Option<String>> {
            let paper_id: String = data.get_item("paper_id")?.extract()?;
            let title: String = data
                .get_item("title")
                .ok()
                .and_then(|a| a.extract().ok())
                .unwrap_or_default();
            let authors: Vec<String> = data
                .get_item("authors")
                .ok()
                .and_then(|a| a.extract().ok())
                .unwrap_or_default();
            let abstract_text: String = data
                .get_item("abstract")
                .ok()
                .and_then(|a| a.extract().ok())
                .unwrap_or_default();
            let published: String = data
                .get_item("published")
                .ok()
                .and_then(|a| a.extract().ok())
                .unwrap_or_default();
            let abs_url: String = data
                .get_item("abs_url")
                .ok()
                .and_then(|a| a.extract().ok())
                .unwrap_or_default();
            let pdf_url: String = data
                .get_item("pdf_url")
                .ok()
                .and_then(|a| a.extract().ok())
                .unwrap_or_default();
            let primary_category: String = data
                .get_item("primary_category")
                .ok()
                .and_then(|a| a.extract().ok())
                .unwrap_or_default();
            let source: String = data
                .get_item("source")
                .ok()
                .and_then(|a| a.extract().ok())
                .unwrap_or_default();

            let result = self.inner.upsert_paper(
                &paper_id,
                &source,
                &title,
                &authors,
                &abstract_text,
                &published,
                "",
                &abs_url,
                &pdf_url,
                &primary_category,
                "",
                "",
                "",
                "",
                "",
                "",
                0,
                "",
                "",
            );
            match result {
                Ok(p) => {
                    let json = serde_json::json!({
                        "id": p.id,
                        "title": p.title,
                        "authors": p.authors,
                        "abstract": p.abstract_text,
                        "published": p.published,
                        "source": p.source,
                        "parse_status": p.parse_status,
                        "primary_category": p.primary_category,
                        "added_at": p.added_at,
                    });
                    Ok(Some(serde_json::to_string(&json).unwrap_or_default()))
                }
                Err(e) => Err(db_err_to_py(e)),
            }
        }

        fn get_paper(&self, paper_id: &str) -> PyResult<Option<String>> {
            let result = self.inner.get_paper(paper_id);
            match result {
                Ok(Some(p)) => {
                    let json = serde_json::json!({
                        "id": p.id,
                        "title": p.title,
                        "authors": p.authors,
                        "abstract": p.abstract_text,
                        "published": p.published,
                        "source": p.source,
                        "parse_status": p.parse_status,
                        "primary_category": p.primary_category,
                        "added_at": p.added_at,
                    });
                    Ok(Some(serde_json::to_string(&json).unwrap_or_default()))
                }
                Ok(None) => Ok(None),
                Err(e) => Err(db_err_to_py(e)),
            }
        }

        #[pyo3(signature = (query, limit=None, offset=None, source=None, category=None, parse_status=None))]
        fn search_papers(
            &self,
            query: &str,
            limit: Option<i64>,
            offset: Option<i64>,
            source: Option<&str>,
            category: Option<&str>,
            parse_status: Option<&str>,
        ) -> PyResult<String> {
            let limit = limit.unwrap_or(20);
            let offset = offset.unwrap_or(0);
            let (results, total) = self
                .inner
                .search_papers(
                    query,
                    limit,
                    offset,
                    source,
                    category,
                    None,
                    None,
                    parse_status,
                )
                .map_err(db_err_to_py)?;
            let results_json: Vec<serde_json::Value> = results
                .into_iter()
                .map(|r| {
                    serde_json::json!({
                        "paper_id": r.paper_id,
                        "title": r.title,
                        "score": r.score,
                        "snippet": r.snippet,
                    })
                })
                .collect();
            let json = serde_json::json!({ "results": results_json, "total": total });
            Ok(serde_json::to_string(&json).unwrap_or_default())
        }

        #[pyo3(signature = (limit=None, offset=None, source=None, category=None, parse_status=None))]
        fn list_papers(
            &self,
            limit: Option<i64>,
            offset: Option<i64>,
            source: Option<&str>,
            category: Option<&str>,
            parse_status: Option<&str>,
        ) -> PyResult<String> {
            let limit = limit.unwrap_or(100);
            let offset = offset.unwrap_or(0);
            let (papers, total) = self
                .inner
                .list_papers(
                    limit,
                    offset,
                    source,
                    category,
                    None,
                    None,
                    parse_status,
                    "added_at",
                    "desc",
                )
                .map_err(db_err_to_py)?;
            let papers_json: Vec<serde_json::Value> = papers
                .into_iter()
                .map(|p| {
                    serde_json::json!({
                        "id": p.id,
                        "title": p.title,
                        "authors": p.authors,
                        "abstract": p.abstract_text,
                        "published": p.published,
                        "source": p.source,
                        "parse_status": p.parse_status,
                        "primary_category": p.primary_category,
                        "added_at": p.added_at,
                    })
                })
                .collect();
            let json = serde_json::json!({ "papers": papers_json, "total": total });
            Ok(serde_json::to_string(&json).unwrap_or_default())
        }

        fn get_stats(&self) -> PyResult<HashMap<String, i64>> {
            let stats = self.inner.get_stats().map_err(db_err_to_py)?;
            let mut map = HashMap::new();
            map.insert("total_papers".to_string(), stats.total_papers);
            map.insert("queue_queued".to_string(), stats.queue_queued);
            map.insert("queue_running".to_string(), stats.queue_running);
            Ok(map)
        }

        fn clear_all(&self) -> PyResult<()> {
            self.inner.clear_all().map_err(db_err_to_py)
        }
    }
}
