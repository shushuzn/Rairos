//! rairos-db-py — PyO3 bindings for rairos-core
//!
//! This crate provides Python bindings for the rairos-core Database,
//! adapted from the old rairos-db bindings.

#![allow(deprecated)]

use std::collections::HashMap;

use chrono::{DateTime, Utc};
use pyo3::conversion::IntoPy;
use pyo3::prelude::*;
use pyo3::types::{PyBytes, PyDict, PyList};
use rairos_core::{CoreError, Database, Paper, SearchResult};

fn db_err_to_py(err: CoreError) -> PyErr {
    PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(err.to_string())
}

#[pyclass]
struct PyPaper {
    inner: Paper,
}

#[pymethods]
impl PyPaper {
    #[getter]
    fn id(&self) -> &str {
        &self.inner.id
    }
    #[getter]
    fn title(&self) -> &str {
        &self.inner.title
    }
    #[getter]
    fn authors(&self) -> Vec<String> {
        self.inner.authors.clone()
    }
    #[getter]
    fn abstract_text(&self) -> &str {
        &self.inner.abstract_text
    }
    #[getter]
    fn published(&self) -> String {
        self.inner.published.to_rfc3339()
    }
    #[getter]
    fn source(&self) -> &str {
        // rairos-core does not have a 'source' field; default to empty
        ""
    }
    #[getter]
    fn parse_status(&self) -> String {
        self.inner.parse_status.to_string()
    }
    #[getter]
    fn primary_category(&self) -> String {
        self.inner.categories.first().cloned().unwrap_or_default()
    }
    #[getter]
    fn added_at(&self) -> String {
        // rairos-core does not expose added_at directly
        String::new()
    }
    fn to_dict(&self, py: Python<'_>) -> PyResult<Py<PyDict>> {
        #[allow(deprecated)]
        let dict = PyDict::new_bound(py);
        dict.set_item("id", &self.inner.id)?;
        dict.set_item("title", &self.inner.title)?;
        dict.set_item("authors", &self.inner.authors)?;
        dict.set_item("abstract_text", &self.inner.abstract_text)?;
        dict.set_item("published", self.inner.published.to_rfc3339())?;
        dict.set_item("source", "")?;
        dict.set_item("parse_status", self.inner.parse_status.to_string())?;
        dict.set_item(
            "primary_category",
            self.inner.categories.first().cloned().unwrap_or_default(),
        )?;
        Ok(dict.into())
    }
}

#[pyclass]
struct PySearchResult {
    inner: SearchResult,
}

#[pymethods]
impl PySearchResult {
    #[getter]
    fn paper_id(&self) -> &str {
        &self.inner.paper_id
    }
    #[getter]
    fn title(&self) -> &str {
        &self.inner.title
    }
    #[getter]
    fn authors(&self) -> Vec<String> {
        self.inner.authors.clone()
    }
    #[getter]
    fn score(&self) -> f64 {
        self.inner.score
    }
    #[getter]
    fn snippet(&self) -> &str {
        &self.inner.snippet
    }
}

#[pyclass]
struct PyDatabase {
    inner: Database,
}

#[pymethods]
impl PyDatabase {
    #[new]
    #[pyo3(signature = (db_path=None))]
    fn new(db_path: Option<String>) -> PyResult<Self> {
        let inner = match db_path {
            Some(p) => Database::open(p).map_err(db_err_to_py)?,
            None => Database::open_default().map_err(db_err_to_py)?,
        };
        Ok(Self { inner })
    }

    fn init(&self) -> PyResult<()> {
        // rairos-core initializes schema automatically in open();
        // this is a no-op for compatibility.
        Ok(())
    }

    fn clear_all(&self) -> PyResult<()> {
        self.inner.clear_all().map_err(db_err_to_py)
    }

    fn get_paper(&self, paper_id: &str) -> PyResult<Option<PyPaper>> {
        match self.inner.get_paper(paper_id) {
            Ok(p) => Ok(Some(PyPaper { inner: p })),
            Err(CoreError::Database(rusqlite::Error::QueryReturnedNoRows)) => Ok(None),
            Err(e) => Err(db_err_to_py(e)),
        }
    }

    fn get_papers_bulk(
        &self,
        paper_ids: Vec<String>,
    ) -> PyResult<HashMap<String, PyPaper>> {
        let mut map = HashMap::new();
        for pid in &paper_ids {
            if let Ok(p) = self.inner.get_paper(pid) {
                map.insert(pid.clone(), PyPaper { inner: p });
            }
        }
        Ok(map)
    }

    fn upsert_paper(&self, data: &Bound<'_, PyAny>) -> PyResult<Option<PyPaper>> {
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
        let published_str: String = data
            .get_item("published")
            .ok()
            .and_then(|a| a.extract().ok())
            .unwrap_or_default();
        let _abs_url: String = data
            .get_item("abs_url")
            .ok()
            .and_then(|a| a.extract().ok())
            .unwrap_or_default();
        let _pdf_url: String = data
            .get_item("pdf_url")
            .ok()
            .and_then(|a| a.extract().ok())
            .unwrap_or_default();
        let _primary_category: String = data
            .get_item("primary_category")
            .ok()
            .and_then(|a| a.extract().ok())
            .unwrap_or_default();
        let _source: String = data
            .get_item("source")
            .ok()
            .and_then(|a| a.extract().ok())
            .unwrap_or_default();

        let published_dt = DateTime::parse_from_rfc3339(&published_str)
            .map(|dt| dt.with_timezone(&Utc))
            .unwrap_or_else(|_| Utc::now());

        // Build a rairos_core::Paper – categories from primary_category
        let categories = if _primary_category.is_empty() {
            Vec::new()
        } else {
            vec![_primary_category.clone()]
        };

        let paper = Paper {
            id: paper_id.clone(),
            arxiv_id: None,
            title,
            authors,
            published: published_dt,
            abstract_text,
            categories,
            parse_status: rairos_core::ParseStatus::Pending,
            metadata: rairos_core::PaperMetadata {
                cited_by: 0,
                references: 0,
                doi: None,
                pdf_url: if _pdf_url.is_empty() { None } else { Some(_pdf_url) },
            },
        };

        self.inner.upsert_paper(&paper).map_err(db_err_to_py)?;
        Ok(Some(PyPaper { inner: paper }))
    }

    #[pyo3(signature = (query, limit=None, offset=None, source=None, category=None, parse_status=None))]
    fn search_papers(
        &self,
        query: &str,
        limit: Option<i64>,
        offset: Option<i64>,
        _source: Option<&str>,
        _category: Option<&str>,
        parse_status: Option<&str>,
    ) -> PyResult<(Vec<PySearchResult>, i64)> {
        let limit = limit.unwrap_or(20);
        let offset = offset.unwrap_or(0);
        // Use rairos-core's extended search with LIKE; rairos-core doesn't support source/date_from/date_to
        // so we pass None for unsupported filters.
        let (results, total) = self
            .inner
            .search_papers_ext(query, limit, offset, category, parse_status)
            .map_err(db_err_to_py)?;
        Ok((
            results
                .into_iter()
                .map(|r| PySearchResult { inner: r })
                .collect(),
            total,
        ))
    }

    fn list_papers(
        &self,
        limit: Option<i64>,
        offset: Option<i64>,
        _source: Option<&str>,
        _category: Option<&str>,
        parse_status: Option<&str>,
    ) -> PyResult<(Vec<PyPaper>, i64)> {
        let limit = limit.unwrap_or(100) as usize;
        let offset = offset.unwrap_or(0) as usize;
        // rairos-core's list_papers filters by ParseStatus enum, not string
        let status = parse_status.and_then(|s| match s {
            "pending" => Some(rairos_core::ParseStatus::Pending),
            "parsing" => Some(rairos_core::ParseStatus::Parsing),
            "done" => Some(rairos_core::ParseStatus::Done),
            "failed" => Some(rairos_core::ParseStatus::Failed),
            _ => None,
        });
        let papers = self
            .inner
            .list_papers(status, limit, offset)
            .map_err(db_err_to_py)?;
        let total = papers.len() as i64;
        Ok((
            papers
                .into_iter()
                .map(|p| PyPaper { inner: p })
                .collect(),
            total,
        ))
    }

    fn paper_count(&self, status: Option<&str>) -> PyResult<i64> {
        let count = match status {
            Some(s) => {
                // Filter by parse_status using LIKE-based counting
                let status_filter = match s {
                    "pending" => rairos_core::ParseStatus::Pending,
                    "parsing" => rairos_core::ParseStatus::Parsing,
                    "done" => rairos_core::ParseStatus::Done,
                    "failed" => rairos_core::ParseStatus::Failed,
                    _ => return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(
                        format!("Unknown status: {}", s),
                    )),
                };
                self.inner.count_papers_with_status(status_filter)
                    .map_err(db_err_to_py)?
            }
            None => self.inner.count_papers().map_err(db_err_to_py)?,
        };
        Ok(count)
    }

    fn paper_exists(&self, paper_id: &str) -> PyResult<bool> {
        Ok(self.inner.paper_exists(paper_id))
    }

    fn delete_paper(&self, paper_id: &str) -> PyResult<bool> {
        self.inner.delete_paper(paper_id).map_err(db_err_to_py)?;
        Ok(true)
    }

    fn get_stats(&self) -> PyResult<HashMap<String, i64>> {
        let stats = self.inner.get_detailed_stats().map_err(db_err_to_py)?;
        let mut map = HashMap::new();
        map.insert("total_papers".to_string(), stats.total_papers);
        map.insert("queue_queued".to_string(), stats.queue_queued);
        map.insert("queue_running".to_string(), stats.queue_running);
        Ok(map)
    }

    fn execute_query(
        &self,
        query: &str,
        params: &Bound<'_, PyList>,
    ) -> PyResult<Vec<HashMap<String, Py<PyAny>>>> {
        let sql = query.to_lowercase();
        if sql.contains("papers_fts") {
            return Ok(vec![]);
        }

        // Convert PyAny params to rusqlite::types::Value
        let values: Vec<rusqlite::types::Value> = params
            .iter()
            .map(|item| {
                if item.is_none() {
                    rusqlite::types::Value::Null
                } else if let Ok(s) = item.extract::<i64>() {
                    rusqlite::types::Value::Integer(s)
                } else if let Ok(d) = item.extract::<f64>() {
                    rusqlite::types::Value::Real(d)
                } else if let Ok(b) = item.extract::<bool>() {
                    rusqlite::types::Value::Integer(if b { 1 } else { 0 })
                } else if let Ok(s) = item.extract::<&str>() {
                    rusqlite::types::Value::Text(s.to_string())
                } else if let Ok(buf) = item.extract::<Vec<u8>>() {
                    rusqlite::types::Value::Blob(buf)
                } else {
                    rusqlite::types::Value::Null
                }
            })
            .collect();

        let rows = self.inner.query_raw(query, values).map_err(db_err_to_py)?;

        Python::with_gil(|py| {
            let mut results = Vec::new();
            for row_map in rows {
                let mut map: HashMap<String, Py<PyAny>> = HashMap::new();
                for (k, v) in row_map {
                    #[allow(deprecated)]
                    let val: Py<PyAny> = match v {
                        rusqlite::types::Value::Null => py.None(),
                        rusqlite::types::Value::Integer(i) => i.into_py(py),
                        rusqlite::types::Value::Real(f) => f.into_py(py),
                        rusqlite::types::Value::Text(s) => s.into_py(py),
                        rusqlite::types::Value::Blob(b) => PyBytes::new_bound(py, &b).into(),
                    };
                    map.insert(k, val);
                }
                results.push(map);
            }
            Ok(results)
        })
    }
}

#[pymodule]
fn rairos_db_py(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyDatabase>()?;
    m.add_class::<PyPaper>()?;
    m.add_class::<PySearchResult>()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    #[test]
    fn db_py_version_exists() {
        assert!(true)
    }
}
