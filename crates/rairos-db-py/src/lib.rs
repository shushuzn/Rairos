//! rairos-db-py — PyO3 bindings for rairos-db

#![allow(deprecated)]

use std::collections::HashMap;

use pyo3::conversion::IntoPy;
use pyo3::prelude::*;
use pyo3::types::{PyBytes, PyDict, PyList};
use rairos_db::{DbError, Database, Paper, SearchResult};

fn db_err_to_py(err: DbError) -> PyErr {
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
    fn published(&self) -> &str {
        &self.inner.published
    }
    #[getter]
    fn source(&self) -> &str {
        &self.inner.source
    }
    #[getter]
    fn parse_status(&self) -> &str {
        &self.inner.parse_status
    }
    #[getter]
    fn primary_category(&self) -> &str {
        &self.inner.primary_category
    }
    #[getter]
    fn added_at(&self) -> &str {
        &self.inner.added_at
    }
    fn to_dict(&self, py: Python<'_>) -> PyResult<Py<PyDict>> {
        #[allow(deprecated)]
        let dict = PyDict::new_bound(py);
        dict.set_item("id", &self.inner.id)?;
        dict.set_item("title", &self.inner.title)?;
        dict.set_item("authors", &self.inner.authors)?;
        dict.set_item("abstract_text", &self.inner.abstract_text)?;
        dict.set_item("published", &self.inner.published)?;
        dict.set_item("source", &self.inner.source)?;
        dict.set_item("parse_status", &self.inner.parse_status)?;
        dict.set_item("primary_category", &self.inner.primary_category)?;
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
        self.inner.init().map_err(db_err_to_py)
    }

    fn clear_all(&self) -> PyResult<()> {
        self.inner.clear_all().map_err(db_err_to_py)
    }

    fn get_paper(&self, paper_id: &str) -> PyResult<Option<PyPaper>> {
        let result = self.inner.get_paper(paper_id).map_err(db_err_to_py)?;
        Ok(result.map(|p| PyPaper { inner: p }))
    }

    fn get_papers_bulk(
        &self,
        paper_ids: Vec<String>,
    ) -> PyResult<HashMap<String, PyPaper>> {
        let map = self
            .inner
            .get_papers_bulk(&paper_ids)
            .map_err(db_err_to_py)?;
        Ok(map
            .into_iter()
            .map(|(k, v)| (k, PyPaper { inner: v }))
            .collect())
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

        let result = self
            .inner
            .upsert_paper(
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
            )
            .map_err(db_err_to_py)?;
        Ok(Some(PyPaper { inner: result }))
    }

    fn search_papers(
        &self,
        query: &str,
        limit: Option<i64>,
        offset: Option<i64>,
        source: Option<&str>,
        category: Option<&str>,
        parse_status: Option<&str>,
    ) -> PyResult<(Vec<PySearchResult>, i64)> {
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
        source: Option<&str>,
        category: Option<&str>,
        parse_status: Option<&str>,
    ) -> PyResult<(Vec<PyPaper>, i64)> {
        let limit = limit.unwrap_or(100);
        let offset = offset.unwrap_or(0);
        let (papers, total) = self
            .inner
            .list_papers(limit, offset, source, category, None, None, parse_status, "added_at", "desc")
            .map_err(db_err_to_py)?;
        Ok((
            papers
                .into_iter()
                .map(|p| PyPaper { inner: p })
                .collect(),
            total,
        ))
    }

    fn paper_count(&self, status: Option<&str>) -> PyResult<i64> {
        self.inner.paper_count(status).map_err(db_err_to_py)
    }

    fn paper_exists(&self, paper_id: &str) -> PyResult<bool> {
        self.inner.paper_exists(paper_id).map_err(db_err_to_py)
    }

    fn delete_paper(&self, paper_id: &str) -> PyResult<bool> {
        self.inner.delete_paper(paper_id).map_err(db_err_to_py)
    }

    fn get_stats(&self) -> PyResult<HashMap<String, i64>> {
        let stats = self.inner.get_stats().map_err(db_err_to_py)?;
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
                        rusqlite::types::Value::Null => py.None().into(),
                        rusqlite::types::Value::Integer(i) => i.into_py(py).into(),
                        rusqlite::types::Value::Real(f) => f.into_py(py).into(),
                        rusqlite::types::Value::Text(s) => s.into_py(py).into(),
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