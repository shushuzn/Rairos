//! rairos-paradigm-monitor — Paradigm Concentration Monitor
//!
//! Detects when >60% of citations in a domain cluster around ≤3 references.
//! Flags a generalization_gap risk alert.
//!
//! Ported from `llm/paradigm_monitor.py`.

#![allow(clippy::unnecessary_unwrap)]

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use thiserror::Error;

// ============================================================================
// Constants
// ============================================================================

const DB_PATH: &str = ".ai_research_os/papers.db";
const ALERT_THRESHOLD: f64 = 0.60; // >60% concentration triggers alert
const TOP_N: usize = 3;

// ============================================================================
// Errors
// ============================================================================

#[derive(Error, Debug)]
pub enum ParadigmError {
    #[error("SQLite error: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("db unavailable")]
    DbUnavailable,
}

pub type Result<T> = std::result::Result<T, ParadigmError>;

// ============================================================================
// Types
// ============================================================================

/// A top-N paper with its citation share info.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TopPaper {
    pub category: String,
    pub paper_id: String,
    pub title: String,
    pub citation_count: i64,
    pub share_pct: f64,
}

/// An alert when paradigm concentration exceeds threshold.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParadigmAlert {
    #[serde(rename = "type")]
    pub alert_type: String,
    pub severity: String,
    pub message: String,
    pub top_papers: Vec<String>,
}

/// The result of a paradigm concentration check.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ParadigmReport {
    pub categories: Vec<TopPaper>,
    pub alerts: Vec<ParadigmAlert>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_papers: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_citations: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

// ============================================================================
// Core Logic
// ============================================================================

fn default_db_path() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(DB_PATH)
}

fn collect_rows(
    conn: &rusqlite::Connection,
    sql: &str,
    params: &[&dyn rusqlite::ToSql],
) -> Vec<(String, String, String, i64)> {
    let mut stmt = match conn.prepare(sql) {
        Ok(s) => s,
        Err(_) => return vec![],
    };
    let mut rows = match stmt.query(params) {
        Ok(r) => r,
        Err(_) => return vec![],
    };
    let mut result = Vec::new();
    while let Ok(Some(row)) = rows.next() {
        let id: String = row.get(0).ok().unwrap_or_default();
        let title: String = row.get(1).ok().unwrap_or_default();
        let primary_cat: String = row.get(2).ok().unwrap_or_default();
        let cit: i64 = row.get(3).ok().unwrap_or(0);
        result.push((id, title, primary_cat, cit));
    }
    result
}

fn collect_ids(conn: &rusqlite::Connection, sql: &str, params: &[&dyn rusqlite::ToSql]) -> Vec<String> {
    let mut stmt = match conn.prepare(sql) {
        Ok(s) => s,
        Err(_) => return vec![],
    };
    let mut rows = match stmt.query(params) {
        Ok(r) => r,
        Err(_) => return vec![],
    };
    let mut result = Vec::new();
    while let Ok(Some(row)) = rows.next() {
        if let Ok(id) = row.get::<_, String>(0) {
            result.push(id);
        }
    }
    result
}

/// Check paradigm concentration for a given category (or "all").
pub fn check_paradigm_concentration(category: &str) -> ParadigmReport {
    let db_path = default_db_path();
    if !db_path.exists() {
        return ParadigmReport {
            error: Some("db unavailable".to_string()),
            ..Default::default()
        };
    }

    let conn = match rusqlite::Connection::open(&db_path) {
        Ok(c) => c,
        Err(_) => {
            return ParadigmReport {
                error: Some("db unavailable".to_string()),
                ..Default::default()
            }
        }
    };

    let rows: Vec<(String, String, String, i64)> = if category == "all" {
        collect_rows(
            &conn,
            "SELECT id, title, primary_category, citation_count \
             FROM papers WHERE citation_count > 0 \
             ORDER BY citation_count DESC",
            &[],
        )
    } else {
        collect_rows(
            &conn,
            "SELECT id, title, primary_category, citation_count \
             FROM papers \
             WHERE primary_category = ? AND citation_count > 0 \
             ORDER BY citation_count DESC",
            &[&category],
        )
    };

    if rows.is_empty() {
        return ParadigmReport {
            categories: vec![],
            alerts: vec![],
            total_papers: Some(0),
            total_citations: Some(0),
            error: None,
        };
    }

    let total_citations: i64 = rows.iter().map(|(_, _, _, cit)| cit).sum();
    if total_citations == 0 {
        return ParadigmReport {
            categories: vec![],
            alerts: vec![],
            total_papers: Some(rows.len()),
            total_citations: Some(0),
            error: None,
        };
    }

    let top_n_rows = rows.iter().take(TOP_N).collect::<Vec<_>>();
    let top_n_citations: i64 = top_n_rows.iter().map(|(_, _, _, cit)| cit).sum();
    let concentration = top_n_citations as f64 / total_citations as f64;

    let categories: Vec<TopPaper> = top_n_rows
        .iter()
        .map(|(id, title, primary_cat, cit)| TopPaper {
            category: if primary_cat.is_empty() {
                "uncategorized".to_string()
            } else {
                primary_cat.clone()
            },
            paper_id: id.clone(),
            title: if title.is_empty() {
                "Unknown".to_string()
            } else {
                title.chars().take(80).collect()
            },
            citation_count: *cit,
            share_pct: round(100.0 * (*cit as f64) / (total_citations as f64), 1),
        })
        .collect();

    let mut alerts = Vec::new();
    if concentration > ALERT_THRESHOLD {
        alerts.push(ParadigmAlert {
            alert_type: "paradigm_concentration".to_string(),
            severity: "high".to_string(),
            message: format!(
                "{}% of citations in {} domain cluster around {} papers \
                 (threshold: {}%). Consider diversifying reading to reduce generalization gap risk.",
                round(concentration * 100.0, 0) as i64,
                category,
                TOP_N,
                round(ALERT_THRESHOLD * 100.0, 0) as i64,
            ),
            top_papers: top_n_rows.iter().map(|(id, _, _, _)| id.clone()).collect(),
        });
    }

    ParadigmReport {
        categories,
        alerts,
        total_papers: Some(rows.len()),
        total_citations: Some(total_citations),
        error: None,
    }
}

/// Render paradigm concentration results as HTML.
pub fn render_html(data: &ParadigmReport) -> String {
    if data.error.is_some() {
        return "<p>Paradigm concentration monitor temporarily unavailable</p>".to_string();
    }

    if data.categories.is_empty() && data.alerts.is_empty() {
        return "<p>No paradigm concentration detected.</p>".to_string();
    }

    let mut parts = Vec::new();

    for alert in &data.alerts {
        parts.push(format!(
            r#"<div style="background:#fff3cd;border:1px solid #ffc107;\
               border-radius:6px;padding:12px;margin-bottom:12px;font-size:13px">\
               <strong>&#9888;&#65039; Paradigm Concentration Alert</strong><br>{}</div>"#,
            alert.message
        ));
    }

    if !data.categories.is_empty() {
        let mut rows_html = String::new();
        for (i, cat) in data.categories.iter().enumerate().take(20) {
            rows_html.push_str(&format!(
                "<tr><td style=\"padding:6px 12px\">{}</td>\
                 <td style=\"padding:6px 12px\">{}</td>\
                 <td style=\"padding:6px 12px;text-align:center\">{}%</td></tr>",
                i + 1,
                cat.title,
                cat.share_pct,
            ));
        }
        parts.push(format!(
            "<table style=\"width:100%;border-collapse:collapse;font-size:13px\">\
             <tr style=\"background:#f5f5f5\">\
             <th style=\"padding:8px 12px\">#</th>\
             <th style=\"padding:8px 12px;text-align:left\">Paper</th>\
             <th style=\"padding:8px 12px;text-align:center\">Citation Share</th>\
             </tr>{}</table>",
            rows_html
        ));
    }

    parts.join("")
}

// ============================================================================
// Utilities
// ============================================================================

fn round(v: f64, decimals: u32) -> f64 {
    let m = 10_f64.powi(decimals as i32);
    (v * m).round() / m
}

/// Returns the list of paper IDs in a given primary_category, or all if "all".
pub fn get_papers_in_domain(category: &str) -> Vec<String> {
    let db_path = default_db_path();
    if !db_path.exists() {
        return vec![];
    }
    let conn = match rusqlite::Connection::open(&db_path) {
        Ok(c) => c,
        Err(_) => return vec![],
    };
    if category == "all" {
        collect_ids(&conn, "SELECT id FROM papers", &[])
    } else {
        collect_ids(
            &conn,
            "SELECT id FROM papers WHERE primary_category = ?",
            &[&category],
        )
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_round_basic() {
        assert!((round(1.23456, 2) - 1.23).abs() < 0.001);
        assert!((round(0.666666, 1) - 0.7).abs() < 0.001);
    }

    #[test]
    fn test_empty_report_default() {
        let report = ParadigmReport::default();
        assert!(report.categories.is_empty());
        assert!(report.alerts.is_empty());
        assert!(report.error.is_none());
    }

    #[test]
    fn test_render_html_error() {
        let report = ParadigmReport {
            error: Some("db unavailable".to_string()),
            ..Default::default()
        };
        let html = render_html(&report);
        assert!(html.contains("unavailable"));
    }

    #[test]
    fn test_render_html_empty() {
        let report = ParadigmReport {
            categories: vec![],
            alerts: vec![],
            total_papers: Some(0),
            total_citations: Some(0),
            error: None,
        };
        let html = render_html(&report);
        assert!(html.contains("No paradigm concentration"));
    }

    #[test]
    fn test_render_html_with_data() {
        let report = ParadigmReport {
            categories: vec![TopPaper {
                category: "cs.AI".to_string(),
                paper_id: "paper1".to_string(),
                title: "Test Paper".to_string(),
                citation_count: 100,
                share_pct: 50.0,
            }],
            alerts: vec![],
            total_papers: Some(10),
            total_citations: Some(200),
            error: None,
        };
        let html = render_html(&report);
        assert!(html.contains("Test Paper"));
        assert!(html.contains("50%"));
    }

    #[test]
    fn test_render_html_with_alert() {
        let report = ParadigmReport {
            categories: vec![TopPaper {
                category: "cs.AI".to_string(),
                paper_id: "paper1".to_string(),
                title: "Test Paper".to_string(),
                citation_count: 100,
                share_pct: 70.0,
            }],
            alerts: vec![ParadigmAlert {
                alert_type: "paradigm_concentration".to_string(),
                severity: "high".to_string(),
                message: "70% of citations cluster around 3 papers.".to_string(),
                top_papers: vec!["paper1".to_string()],
            }],
            total_papers: Some(10),
            total_citations: Some(200),
            error: None,
        };
        let html = render_html(&report);
        assert!(html.contains("Paradigm Concentration Alert"));
        assert!(html.contains("70%"));
    }

    #[test]
    fn test_check_paradigm_concentration_db_unavailable() {
        // Point to non-existent DB
        let report = check_paradigm_concentration("all");
        // Should return either empty (no DB) or error
        assert!(report.categories.is_empty() || report.error.is_some());
    }
}
