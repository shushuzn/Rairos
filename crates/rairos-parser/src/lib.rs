//! Rairos Parser — Paper metadata fetching from multiple sources
//!
//! Supported sources: arXiv, CrossRef, Semantic Scholar
//! Replaces: parsers/arxiv.py, parsers/cross_search.py

use rairos_core::{Paper, ParseStatus};
use serde::{Deserialize, Serialize};
use std::time::Duration;
use thiserror::Error;

// ============================================================================
// Error Types
// ============================================================================

#[derive(Error, Debug)]
pub enum ParseError {
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),

    #[error("Rate limited, retry after {0}s")]
    RateLimited(u64),

    #[error("Paper not found: {0}")]
    NotFound(String),

    #[error("Parse failed: {0}")]
    ParseFailed(String),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
}

// ============================================================================
// ArXiv API
// ============================================================================

const ARXIV_API: &str = "http://export.arxiv.org/api/query";

/// ArXiv entry response fields we care about
#[derive(Debug, Deserialize)]
struct ArXivEntry {
    #[serde(rename = "id")]
    entry_id: String,
    #[serde(rename = "title")]
    title: String,
    #[serde(rename = "summary")]
    abstract_text: String,
    #[serde(rename = "author")]
    authors: Vec<ArXivAuthor>,
    #[serde(rename = "published")]
    published: String,
    #[serde(rename = "category")]
    categories: Vec<ArXivCategory>,
}

#[derive(Debug, Deserialize)]
struct ArXivAuthor {
    #[serde(rename = "name")]
    name: String,
}

#[derive(Debug, Deserialize)]
struct ArXivCategory {
    #[serde(rename = "term")]
    term: String,
}

#[derive(Debug, Deserialize)]
struct ArXivFeed {
    #[serde(rename = "entry")]
    entry: Option<ArXivEntry>,
    #[serde(rename = "error")]
    error: Option<String>,
}

/// Fetch paper from arXiv by ID
pub async fn fetch_arxiv(arxiv_id: &str) -> Result<Paper, ParseError> {
    let url = format!("{}?id_list={}", ARXIV_API, arxiv_id);

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(30))
        .build()?;

    let resp = client.get(&url).send().await?;
    let text = resp.text().await?;

    // Parse Atom feed manually since serde_xml doesn't handle namespaced elements well
    let title = extract_tag(&text, "title").unwrap_or_default();
    let summary = extract_tag(&text, "summary").unwrap_or_default();
    let published = extract_tag(&text, "published").unwrap_or_default();
    let entry_id = extract_tag(&text, "id").unwrap_or_default();

    let authors: Vec<String> = extract_authors(&text);
    let categories: Vec<String> = extract_categories(&text);

    if title.is_empty() {
        return Err(ParseError::NotFound(arxiv_id.to_string()));
    }

    let paper = Paper::new(
        Some(arxiv_id.to_string()),
        clean_arxiv_title(&title),
        clean_text(&summary),
    );

    // Note: in full impl, would update fields via Database
    Ok(paper)
}

fn extract_tag(xml: &str, tag: &str) -> Option<String> {
    let start = format!("<{}>", tag);
    let end = format!("</{}>", tag);
    xml.find(&start).and_then(|s| {
        xml[s + start.len()..].find(&end).map(|e| {
            xml[s + start.len()..s + start.len() + e].trim().to_string()
        })
    })
}

fn extract_authors(xml: &str) -> Vec<String> {
    let mut authors = Vec::new();
    let mut search = xml;
    while let Some(start) = search.find("<author>") {
        let chunk = &search[start..];
        if let Some(name_start) = chunk.find("<name>") {
            let name_end = chunk.find("</name>").unwrap_or(0);
            if name_start < 20 {
                // within <author> tag
                let name = &chunk[name_start + 6..name_end];
                authors.push(name.trim().to_string());
            }
        }
        search = &search[start + 1..];
    }
    authors
}

fn extract_categories(xml: &str) -> Vec<String> {
    let mut cats = Vec::new();
    let mut search = xml;
    while let Some(start) = search.find("<category") {
        let chunk = &search[start..];
        if let Some(term_start) = chunk.find("term=\"") {
            let term_end = chunk[term_start + 6..].find('"').unwrap_or(0);
            let term = &chunk[term_start + 6..term_start + 6 + term_end];
            if !term.contains('-') {
                // skip subcategories like "cs.AI"
                cats.push(term.to_string());
            }
        }
        search = &search[start + 1..];
    }
    cats
}

fn clean_arxiv_title(title: &str) -> String {
    title.lines()
        .map(|l| l.trim())
        .collect::<Vec<_>>()
        .join(" ")
        .trim()
        .to_string()
}

fn clean_text(text: &str) -> String {
    text.replace('\n', " ")
        .replace("  ", " ")
        .trim()
        .to_string()
}

// ============================================================================
// CrossRef API
// ============================================================================

const CROSSREF_API: &str = "https://api.crossref.org/works";

/// CrossRef response
#[derive(Debug, Deserialize)]
struct CrossRefResponse {
    #[serde(rename = "message")]
    message: Option<CrossRefMessage>,
}

#[derive(Debug, Deserialize)]
struct CrossRefMessage {
    #[serde(rename = "DOI")]
    doi: Option<String>,
    #[serde(rename = "title")]
    title: Option<Vec<String>>,
    #[serde(rename = "abstract")]
    abstract_text: Option<String>,
    #[serde(rename = "author")]
    authors: Option<Vec<CrossRefAuthor>>,
    #[serde(rename = "published-print")]
    published: Option<CrossRefDate>,
    #[serde(rename = "published-online")]
    published_online: Option<CrossRefDate>,
    #[serde(rename = "subject")]
    categories: Option<Vec<String>>,
    #[serde(rename = "is-referenced-by-count")]
    cited_by: Option<u32>,
}

#[derive(Debug, Deserialize)]
struct CrossRefAuthor {
    #[serde(rename = "given")]
    given: Option<String>,
    #[serde(rename = "family")]
    family: Option<String>,
}

#[derive(Debug, Deserialize)]
struct CrossRefDate {
    #[serde(rename = "date-parts")]
    date_parts: Option<Vec<Vec<u16>>>,
}

#[derive(Debug, Deserialize)]
struct CrossRefError {
    #[serde(rename = "message")]
    message: Option<String>,
    #[serde(rename = "status")]
    status: Option<String>,
}

/// Fetch paper from CrossRef by DOI
pub async fn fetch_crossref(doi: &str) -> Result<Paper, ParseError> {
    let url = format!("{}/{}", CROSSREF_API, doi);

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(30))
        .build()?;

    let resp = client
        .get(&url)
        .header("User-Agent", "Rairos/1.0 (mailto:rairos@example.com)")
        .send()
        .await?;

    if resp.status() == 404 {
        return Err(ParseError::NotFound(doi.to_string()));
    }

    let text = resp.text().await?;

    // Check for error response
    if let Ok(err) = serde_json::from_str::<CrossRefError>(&text) {
        if err.status.as_deref() == Some("failed") {
            return Err(ParseError::NotFound(doi.to_string()));
        }
    }

    let data: CrossRefResponse = serde_json::from_str(&text)?;
    let msg = data.message.ok_or_else(|| ParseError::NotFound(doi.to_string()))?;

    let title = msg.title.and_then(|t| t.into_iter().next()).unwrap_or_default();
    let abstract_text = msg.abstract_text.unwrap_or_default();
    let authors: Vec<String> = msg.authors
        .unwrap_or_default()
        .into_iter()
        .map(|a| {
            let given = a.given.unwrap_or_default();
            let family = a.family.unwrap_or_default();
            if given.is_empty() {
                family
            } else {
                format!("{} {}", given, family)
            }
        })
        .collect();

    let categories = msg.categories.unwrap_or_default();

    let paper = Paper::new(
        None, // arXiv ID not available from CrossRef
        title,
        clean_text(&abstract_text),
    );

    Ok(paper)
}

// ============================================================================
// Semantic Scholar API
// ============================================================================

const SEMANTIC_API: &str = "https://api.semanticscholar.org/graph/v1";

/// Semantic Scholar paper response
#[derive(Debug, Deserialize)]
struct SemanticPaper {
    #[serde(rename = "paperId")]
    paper_id: Option<String>,
    #[serde(rename = "externalIds")]
    external_ids: Option<SemanticExternalIds>,
    #[serde(rename = "title")]
    title: Option<String>,
    #[serde(rename = "abstract")]
    abstract_text: Option<String>,
    #[serde(rename = "authors")]
    authors: Option<Vec<SemanticAuthor>>,
    #[serde(rename = "year")]
    year: Option<u16>,
    #[serde(rename = "citationCount")]
    citation_count: Option<u32>,
    #[serde(rename = "fieldsOfStudy")]
    fields: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
struct SemanticExternalIds {
    #[serde(rename = "DOI")]
    doi: Option<String>,
    #[serde(rename = "ArXiv")]
    arxiv: Option<String>,
    #[serde(rename = "PubMed")]
    pubmed: Option<String>,
}

#[derive(Debug, Deserialize)]
struct SemanticAuthor {
    #[serde(rename = "authorId")]
    id: Option<String>,
    #[serde(rename = "name")]
    name: Option<String>,
}

/// Fetch paper from Semantic Scholar by ID
pub async fn fetch_semantic(paper_id: &str) -> Result<Paper, ParseError> {
    let url = format!(
        "{}/paper/{}?fields=title,abstract,authors,year,citationCount,fieldsOfStudy,externalIds",
        SEMANTIC_API,
        paper_id
    );

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(30))
        .build()?;

    let resp = client.get(&url).send().await?;

    if resp.status() == 404 {
        return Err(ParseError::NotFound(paper_id.to_string()));
    }

    let data: SemanticPaper = resp.json().await?;

    let title = data.title.unwrap_or_default();
    if title.is_empty() {
        return Err(ParseError::NotFound(paper_id.to_string()));
    }

    let arxiv_id = data.external_ids.as_ref()
        .and_then(|ids| ids.arxiv.clone());

    let abstract_text = data.abstract_text.unwrap_or_default();

    let authors: Vec<String> = data.authors
        .unwrap_or_default()
        .into_iter()
        .filter_map(|a| a.name)
        .collect();

    let paper = Paper::new(
        arxiv_id,
        title,
        clean_text(&abstract_text),
    );

    Ok(paper)
}

// ============================================================================
// Unified Parser
// ============================================================================

/// Parse status for tracking
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Source {
    ArXiv,
    CrossRef,
    SemanticScholar,
}

impl std::fmt::Display for Source {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Source::ArXiv => write!(f, "arxiv"),
            Source::CrossRef => write!(f, "crossref"),
            Source::SemanticScholar => write!(f, "semantic-scholar"),
        }
    }
}

/// Detect source from ID format
pub fn detect_source(id: &str) -> Option<Source> {
    if id.starts_with("10.") {
        Some(Source::CrossRef)
    } else if id.chars().all(|c| c.is_ascii_digit() || c == '.' || c == '-') {
        Some(Source::ArXiv)
    } else if id.len() == 40 && id.chars().all(|c| c.is_ascii_hexdigit()) {
        Some(Source::SemanticScholar)
    } else {
        None
    }
}

/// Fetch paper from the appropriate source based on ID format
pub async fn fetch_paper(id: &str) -> Result<Paper, ParseError> {
    match detect_source(id) {
        Some(Source::ArXiv) => fetch_arxiv(id).await,
        Some(Source::CrossRef) => fetch_crossref(id).await,
        Some(Source::SemanticScholar) => fetch_semantic(id).await,
        None => Err(ParseError::NotFound(format!("Unknown ID format: {}", id))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_source() {
        assert_eq!(detect_source("2301.00001"), Some(Source::ArXiv));
        assert_eq!(detect_source("10.1038/nature12373"), Some(Source::CrossRef));
        assert_eq!(
            detect_source("https://www.semanticscholar.org/paper/abc123"),
            Some(Source::SemanticScholar)
        );
        assert_eq!(detect_source("abc123"), None);
    }

    #[test]
    fn test_clean_text() {
        assert_eq!(clean_text("hello\n\nworld  "), "hello world");
        assert_eq!(clean_text("  test  "), "test");
    }
}
