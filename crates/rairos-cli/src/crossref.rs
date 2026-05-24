//! rairos-crossref — Crossref API metadata fetching.

#![allow(clippy::upper_case_acronyms)]
#![allow(dead_code)]
//!
//! Ported from `parsers/crossref.py` (228 LOC).
//!
//! Fetches paper metadata from Crossref API by DOI.

use chrono::NaiveDate;
use rairos_core::constants::{CROSSREF_WORKS, DOI_RESOLVER};
use rairos_core::identifiers::normalize_arxiv_id;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::sync::LazyLock;
use std::time::Duration;
use thiserror::Error;

static RE_HTML_TAG: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"<[^>]+>").expect("valid regex")
});

static RE_ARXIV_URL: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"arxiv\.org/(?:abs|pdf)/(\d{4}\.\d{4,5})(v\d+)?").expect("valid regex")
});

static RE_ARXIV_ID: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(\d{4}\.\d{4,5})(v\d+)?").expect("valid regex")
});

#[derive(Error, Debug)]
pub enum CrossrefError {
    #[error("HTTP error: {0}")]
    Http(String),
    #[error("Not found (DOI may not exist in Crossref): {0}")]
    NotFound(String),
    #[error("Parse error: {0}")]
    Parse(String),
    #[error("Network error: {0}")]
    Network(String),
    #[error("Circuit open: too many failures")]
    CircuitOpen,
}

impl From<reqwest::Error> for CrossrefError {
    fn from(e: reqwest::Error) -> Self {
        if e.is_timeout() {
            CrossrefError::Network(format!("timeout: {}", e))
        } else if e.is_request() {
            CrossrefError::Network(format!("request error: {}", e))
        } else {
            CrossrefError::Network(e.to_string())
        }
    }
}

// ─── Data Types ────────────────────────────────────────────────────────────────

/// Paper metadata fetched from Crossref.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrossrefPaper {
    pub source: String,
    pub uid: String,
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
    pub reference_count: i64,
    pub maybe_arxiv: Option<String>,
}

// ─── Crossref API Response ────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
struct CrossrefResponse {
    message: CrossrefItem,
}

#[derive(Debug, Deserialize)]
struct CrossrefItem {
    title: Option<Vec<String>>,
    author: Option<Vec<CrossrefAuthor>>,
    #[serde(rename = "abstract")]
    abstract_field: Option<String>,
    #[serde(rename = "published-print")]
    published_print: Option<CrossrefDate>,
    #[serde(rename = "published-online")]
    published_online: Option<CrossrefDate>,
    published: Option<CrossrefDate>,
    issued: Option<CrossrefDate>,
    created: Option<CrossrefDate>,
    deposited: Option<CrossrefDate>,
    #[serde(rename = "URL")]
    pub url: Option<String>,
    #[serde(rename = "container-title")]
    container_title: Option<Vec<String>>,
    volume: Option<String>,
    issue: Option<String>,
    page: Option<String>,
    #[serde(rename = "is-referenced-by-count")]
    is_referenced_by_count: Option<i64>,
    link: Option<Vec<CrossrefLink>>,
    #[serde(rename = "relation")]
    relation: Option<serde_json::Value>,
    #[serde(rename = "alternative-id")]
    alternative_id: Option<Vec<String>>,
    archive: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
struct CrossrefAuthor {
    given: Option<String>,
    family: Option<String>,
}

#[derive(Debug, Deserialize)]
struct CrossrefDate {
    #[serde(rename = "date-parts")]
    date_parts: Option<Vec<Vec<i64>>>,
}

#[derive(Debug, Deserialize)]
struct CrossrefLink {
    #[serde(rename = "content-type")]
    content_type: Option<String>,
    #[serde(rename = "URL")]
    pub url: Option<String>,
}

// ─── Parsing Helpers ──────────────────────────────────────────────────────────

fn parse_date(date: Option<&CrossrefDate>) -> String {
    let date = match date {
        Some(d) => d,
        None => return String::new(),
    };
    let parts = match &date.date_parts {
        Some(p) if !p.is_empty() && !p[0].is_empty() => &p[0],
        _ => return String::new(),
    };
    let year = parts[0];
    let month = parts.get(1).copied().unwrap_or(1);
    let day = parts.get(2).copied().unwrap_or(1);
    if let Some(date) = NaiveDate::from_ymd_opt(year as i32, month as u32, day as u32) {
        date.to_string()
    } else {
        String::new()
    }
}

fn best_effort_date(item: &CrossrefItem) -> String {
    // Try in order of preference
    for key in &[
        "published-print",
        "published-online",
        "published",
        "issued",
        "created",
        "deposited",
    ] {
        let date = match *key {
            "published-print" => item.published_print.as_ref(),
            "published-online" => item.published_online.as_ref(),
            "published" => item.published.as_ref(),
            "issued" => item.issued.as_ref(),
            "created" => item.created.as_ref(),
            "deposited" => item.deposited.as_ref(),
            _ => None,
        };
        if !parse_date(date).is_empty() {
            return parse_date(Some(date.unwrap()));
        }
    }
    String::new()
}

fn parse_authors(item: &CrossrefItem) -> Vec<String> {
    let mut out = Vec::new();
    if let Some(authors) = &item.author {
        for a in authors {
            let given = a.given.as_deref().unwrap_or("").trim();
            let family = a.family.as_deref().unwrap_or("").trim();
            let name = format!("{} {}", given, family).trim().to_string();
            if !name.is_empty() {
                out.push(name);
            }
        }
    }
    out
}

fn parse_title(item: &CrossrefItem) -> String {
    item.title
        .as_ref()
        .and_then(|t| t.first())
        .map(|s| s.trim().to_string())
        .unwrap_or_default()
}

fn parse_abstract(item: &CrossrefItem) -> String {
    let ab = item.abstract_field.as_deref().unwrap_or("");
    if ab.is_empty() {
        return String::new();
    }
    // Strip HTML tags
    let cleaned = RE_HTML_TAG.replace_all(ab, "");
    let cleaned = cleaned.split_whitespace().collect::<Vec<_>>().join(" ");
    cleaned.trim().to_string()
}

fn try_find_arxiv_id(item: &CrossrefItem, doi: &str) -> Option<String> {
    // Try DOI directly
    if let Some(arxid) = normalize_arxiv_id(doi) {
        return Some(arxid);
    }

    // Try relation field
    if let Some(rel) = &item.relation {
        let blob = rel.to_string();
        if let Some(m) = RE_ARXIV_URL.captures(&blob) {
            let id = m.get(1).map(|g| g.as_str()).unwrap_or("");
            let ver = m.get(2).map(|g| g.as_str()).unwrap_or("");
            return Some(format!("{}{}", id, ver));
        }
    }

    // Try alternative-id, archive, link
    for key in &["alternative-id", "archive"] {
        if let Some(val) = match *key {
            "alternative-id" => item.alternative_id.as_ref(),
            "archive" => item.archive.as_ref(),
            _ => None,
        } {
            for v in val {
                if let Some(m) = RE_ARXIV_URL.captures(v) {
                    let id = m.get(1).map(|g| g.as_str()).unwrap_or("");
                    let ver = m.get(2).map(|g| g.as_str()).unwrap_or("");
                    return Some(format!("{}{}", id, ver));
                }
                // Check if the string itself looks like an arxiv ID in a sea of "arxiv"
                let blob = v.to_lowercase();
                if blob.contains("arxiv") {
                    if let Some(m) = RE_ARXIV_ID.captures(&blob) {
                        return Some(format!(
                            "{}{}",
                            m.get(1).map(|g| g.as_str()).unwrap_or(""),
                            m.get(2).map(|g| g.as_str()).unwrap_or("")
                        ));
                    }
                }
            }
        }
    }

    None
}

// ─── Main Fetch Function ───────────────────────────────────────────────────────

static CIRCUIT_FAILURE_COUNT: std::sync::atomic::AtomicUsize =
    std::sync::atomic::AtomicUsize::new(0);
static CIRCUIT_OPEN: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
static LAST_FAILURE: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

const CIRCUIT_FAILURE_THRESHOLD: usize = 5;
const CIRCUIT_RECOVERY_SECS: u64 = 60;

fn check_circuit() -> bool {
    if CIRCUIT_OPEN.load(std::sync::atomic::Ordering::Relaxed) {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        let last = LAST_FAILURE.load(std::sync::atomic::Ordering::Relaxed);
        if now - last >= CIRCUIT_RECOVERY_SECS {
            CIRCUIT_OPEN.store(false, std::sync::atomic::Ordering::Relaxed);
            CIRCUIT_FAILURE_COUNT.store(0, std::sync::atomic::Ordering::Relaxed);
            return false;
        }
        return true;
    }
    false
}

fn record_failure() {
    let count = CIRCUIT_FAILURE_COUNT.fetch_add(1, std::sync::atomic::Ordering::Relaxed) + 1;
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    LAST_FAILURE.store(now, std::sync::atomic::Ordering::Relaxed);
    if count >= CIRCUIT_FAILURE_THRESHOLD {
        CIRCUIT_OPEN.store(true, std::sync::atomic::Ordering::Relaxed);
    }
}

fn record_success() {
    CIRCUIT_FAILURE_COUNT.store(0, std::sync::atomic::Ordering::Relaxed);
}

/// Fetch paper metadata from Crossref by DOI.
/// Returns (CrossrefPaper, maybe_arxiv_id).
/// Does NOT crash on 404/network error — returns minimal fallback paper.
pub fn fetch_crossref_metadata(
    doi: &str,
    timeout_secs: u64,
) -> Result<(CrossrefPaper, Option<String>), CrossrefError> {
    if check_circuit() {
        return Err(CrossrefError::CircuitOpen);
    }

    let url = CROSSREF_WORKS.replace("{doi}", doi);

    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(timeout_secs))
        .build()
        .map_err(|e| CrossrefError::Network(e.to_string()))?;

    let response = match client
        .get(&url)
        .header("User-Agent", "AI-Research-OS/1.0")
        .send()
    {
        Ok(r) => r,
        Err(e) => {
            record_failure();
            return Err(CrossrefError::from(e));
        }
    };

    if response.status() == reqwest::StatusCode::NOT_FOUND {
        record_failure();
        return Err(CrossrefError::NotFound(format!(
            "DOI {} not found in Crossref",
            doi
        )));
    }

    if !response.status().is_success() {
        record_failure();
        return Err(CrossrefError::Http(format!("HTTP {}", response.status())));
    }

    record_success();

    let data: CrossrefResponse = match response.json() {
        Ok(d) => d,
        Err(e) => return Err(CrossrefError::Parse(format!("JSON parse error: {}", e))),
    };

    let item = data.message;
    let title = parse_title(&item);
    let authors = parse_authors(&item);
    let abstract_text = parse_abstract(&item);
    let published = best_effort_date(&item);
    let abs_url = item
        .url
        .clone()
        .unwrap_or_else(|| format!("{}{}", DOI_RESOLVER, doi));
    let pdf_url = item
        .link
        .as_ref()
        .and_then(|links| {
            links
                .iter()
                .find(|l| l.content_type.as_deref() == Some("application/pdf"))
                .and_then(|l| l.url.clone())
        })
        .unwrap_or_default();

    let maybe_arxiv = try_find_arxiv_id(&item, doi);
    let journal = item
        .container_title
        .as_ref()
        .and_then(|t| t.first().cloned())
        .unwrap_or_default();
    let volume = item.volume.clone().unwrap_or_default();
    let issue = item.issue.clone().unwrap_or_default();
    let page = item.page.clone().unwrap_or_default();
    let ref_count = item.is_referenced_by_count.unwrap_or(0);

    let paper = CrossrefPaper {
        source: "doi".to_string(),
        uid: doi.to_string(),
        title,
        authors,
        abstract_text,
        published,
        updated: String::new(),
        abs_url,
        pdf_url,
        primary_category: String::new(),
        journal,
        volume,
        issue,
        page,
        reference_count: ref_count,
        maybe_arxiv: maybe_arxiv.clone(),
    };

    Ok((paper, maybe_arxiv))
}

// ─── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_date() {
        let json = r#"{"date-parts": [[2024, 3, 15]]}"#;
        let date: CrossrefDate = serde_json::from_str(json).unwrap();
        assert_eq!(parse_date(Some(&date)), "2024-03-15");
    }

    #[test]
    fn test_parse_date_empty() {
        let json = r#"{"date-parts": [[]]}"#;
        let date: CrossrefDate = serde_json::from_str(json).unwrap();
        assert_eq!(parse_date(Some(&date)), "");
    }

    #[test]
    fn test_parse_authors() {
        let json = r#"{"author": [{"given": "John", "family": "Doe"}, {"given": "Jane", "family": "Smith"}]}"#;
        let item: CrossrefItem = serde_json::from_str(json).unwrap();
        let authors = parse_authors(&item);
        assert_eq!(authors, vec!["John Doe", "Jane Smith"]);
    }

    #[test]
    fn test_parse_title() {
        let json = r#"{"title": ["  My Awesome Paper  "], "author": []}"#;
        let item: CrossrefItem = serde_json::from_str(json).unwrap();
        assert_eq!(parse_title(&item), "My Awesome Paper");
    }

    #[test]
    fn test_parse_abstract_strips_html() {
        let json = r#"{"abstract": "<jats:p>This is <i>italic</i> text.</jats:p>", "title": ["Test"], "author": []}"#;
        let item: CrossrefItem = serde_json::from_str(json).unwrap();
        let abstract_text = parse_abstract(&item);
        assert_eq!(abstract_text, "This is italic text.");
    }

    #[test]
    fn test_normalize_arxiv_id_from_doi() {
        assert_eq!(
            normalize_arxiv_id("10.48550/arXiv.2301.12345"),
            Some("2301.12345".to_string())
        );
        assert_eq!(
            normalize_arxiv_id("10.48550/arXiv.2301.12345v1"),
            Some("2301.12345v1".to_string())
        );
        assert_eq!(normalize_arxiv_id("10.1007/something"), None);
    }

    #[test]
    fn test_circuit_breaker_check() {
        // Initially should be false
        assert!(!check_circuit());
    }
}
