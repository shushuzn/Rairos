//! Rairos Parsers — Extended paper metadata fetching from multiple sources
#![allow(dead_code)]
//!
//! This crate extends `rairos-parser` with search functionality:
//! - `arxiv_search`: Search arXiv by keyword query
//! - `cross_search`: Search multiple sources (arXiv, Semantic Scholar) concurrently
//! - `semantic_search`: Search Semantic Scholar by keyword
//!
//! Supported sources: arXiv, CrossRef, Semantic Scholar
//! Replaces: parsers/arxiv_search.py, parsers/cross_search.py, parsers/semantic_scholar.py

use rairos_core::{constants::ARXIV_API, Paper, PaperMetadata};
use serde::Deserialize;
use std::time::Duration;
use thiserror::Error;

// ============================================================================
// Error Types
// ============================================================================

#[derive(Error, Debug)]
pub enum SearchError {
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),

    #[error("Rate limited, retry after {0}s")]
    RateLimited(u64),

    #[error("Search failed: {0}")]
    SearchFailed(String),

    #[error("Parse failed: {0}")]
    ParseFailed(String),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("Runtime error: {0}")]
    Runtime(#[from] std::io::Error),
}

// ============================================================================
// ArXiv Search API
// ============================================================================

/// Search arXiv by keyword and return metadata for top papers.
///
/// # Arguments
/// * `query` - Search query (supports arXiv advanced operators like AND, OR, TITLE, ABS)
/// * `max_results` - Number of papers to return (default 5, max 100)
/// * `timeout_secs` - Request timeout in seconds
///
/// # Returns
/// Vector of Paper objects sorted by relevance (best match first)
///
/// # Errors
/// Returns `SearchError` if the search request fails or rate limited
pub async fn arxiv_search(
    query: &str,
    max_results: usize,
    timeout_secs: u64,
) -> Result<Vec<Paper>, SearchError> {
    if query.trim().is_empty() {
        return Err(SearchError::SearchFailed(
            "Query cannot be empty".to_string(),
        ));
    }

    let max_results = max_results.min(100);
    let encoded_query = urlencoding::encode(query);
    let url = format!(
        "{}?search_query=all:{}&start=0&max_results={}&sortBy=relevance&sortOrder=descending",
        ARXIV_API, encoded_query, max_results
    );

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(timeout_secs))
        .build()?;

    let resp = client.get(&url).send().await?;

    if resp.status() == 429 {
        return Err(SearchError::RateLimited(60));
    }

    let text = resp.text().await?;

    // Parse Atom feed manually since serde_xml doesn't handle namespaced elements well
    let papers = extract_papers_from_arxiv_feed(&text)?;

    Ok(papers)
}

/// Extract papers from arXiv Atom feed XML
fn extract_papers_from_arxiv_feed(xml: &str) -> Result<Vec<Paper>, SearchError> {
    let mut papers = Vec::new();

    // Find all <entry>...</entry> blocks
    let mut search = xml;
    while let Some(start) = search.find("<entry>") {
        let end_tag = "</entry>";
        if let Some(end) = search[start..].find(end_tag) {
            let entry_xml = &search[start..start + end + end_tag.len()];
            let paper = extract_paper_from_entry_xml(entry_xml);
            papers.push(paper);
            search = &search[start + 1..];
        } else {
            break;
        }
    }

    Ok(papers)
}

/// Extract paper data from a raw entry XML string
fn extract_paper_from_entry_xml(entry_xml: &str) -> Paper {
    let title = clean_arxiv_title(&extract_tag(entry_xml, "title").unwrap_or_default());
    let summary = extract_tag(entry_xml, "summary").unwrap_or_default();
    let entry_id = extract_tag(entry_xml, "id").unwrap_or_default();

    // Extract authors
    let authors = extract_authors(entry_xml);

    // Extract categories
    let categories = extract_categories(entry_xml);

    // Extract DOI
    let doi = extract_doi(entry_xml);

    // Extract PDF link
    let pdf_url = extract_pdf_url(entry_xml, &entry_id);

    let arxiv_id = entry_id
        .split('/')
        .next_back()
        .unwrap_or(&entry_id)
        .to_string();

    Paper::with_metadata(
        Some(arxiv_id),
        title,
        clean_text(&summary),
        authors,
        categories,
        PaperMetadata {
            cited_by: 0,
            references: 0,
            doi,
            pdf_url,
        },
    )
}

/// Search arXiv by keyword (sync wrapper using blocking client)
pub fn arxiv_search_blocking(query: &str, max_results: usize) -> Result<Vec<Paper>, SearchError> {
    let rt = tokio::runtime::Runtime::new()?;
    rt.block_on(arxiv_search(query, max_results, 30))
}

// ============================================================================
// Semantic Scholar Search API
// ============================================================================

const SEMANTIC_API: &str = "https://api.semanticscholar.org/graph/v1";
const S2_FIELDS: &str =
    "title,authors,abstract,year,venue,citationCount,openAccessPdf,paperId,externalIds";

/// Semantic Scholar search response
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct SemanticSearchResponse {
    #[serde(rename = "total")]
    total: Option<i32>,
    #[serde(rename = "data")]
    data: Vec<SemanticPaper>,
}

/// Semantic Scholar paper response
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct SemanticPaper {
    #[serde(rename = "paperId")]
    paper_id: String,
    #[serde(rename = "title")]
    title: Option<String>,
    #[serde(rename = "abstract")]
    abstract_text: Option<String>,
    #[serde(rename = "authors")]
    authors: Option<Vec<SemanticAuthor>>,
    #[serde(rename = "year")]
    year: Option<i32>,
    #[serde(rename = "venue")]
    venue: Option<String>,
    #[serde(rename = "citationCount")]
    citation_count: Option<i32>,
    #[serde(rename = "openAccessPdf")]
    open_access_pdf: Option<OpenAccessPdf>,
    #[serde(rename = "externalIds")]
    external_ids: Option<ExternalIds>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct SemanticAuthor {
    #[serde(rename = "authorId")]
    author_id: Option<String>,
    #[serde(rename = "name")]
    name: String,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct OpenAccessPdf {
    #[serde(rename = "url")]
    url: Option<String>,
    #[serde(rename = "status")]
    status: Option<String>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct ExternalIds {
    #[serde(rename = "DOI")]
    doi: Option<String>,
    #[serde(rename = "ArXiv")]
    arxiv: Option<String>,
    #[serde(rename = "PubMed")]
    pubmed: Option<String>,
}

/// Search Semantic Scholar by keyword and return papers.
///
/// # Arguments
/// * `query` - Search query
/// * `max_results` - Number of papers to return (default 10, max 100)
/// * `timeout_secs` - Request timeout in seconds
///
/// # Returns
/// Vector of Paper objects sorted by relevance
///
/// # Errors
/// Returns `SearchError` if the search request fails or rate limited
pub async fn semantic_search(
    query: &str,
    max_results: usize,
    timeout_secs: u64,
) -> Result<Vec<Paper>, SearchError> {
    if query.trim().is_empty() {
        return Err(SearchError::SearchFailed(
            "Query cannot be empty".to_string(),
        ));
    }

    let max_results = max_results.min(100);
    let encoded_query = urlencoding::encode(query);
    let url = format!(
        "{}/paper/search?query={}&limit={}&fields={}",
        SEMANTIC_API, encoded_query, max_results, S2_FIELDS
    );

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(timeout_secs))
        .build()?;

    let resp = client.get(&url).send().await?;

    if resp.status() == 429 {
        let retry_after = resp
            .headers()
            .get("retry-after")
            .and_then(|v| v.to_str().ok())
            .and_then(|s| s.parse().ok())
            .unwrap_or(60);
        return Err(SearchError::RateLimited(retry_after));
    }

    let data: SemanticSearchResponse = resp.json().await?;

    let papers: Vec<Paper> = data
        .data
        .into_iter()
        .map(semantic_paper_to_paper)
        .filter(|p| !p.title.is_empty())
        .collect();

    Ok(papers)
}

/// Convert a Semantic Scholar paper to our Paper type
fn semantic_paper_to_paper(s2: SemanticPaper) -> Paper {
    let title = s2.title.unwrap_or_default();
    let abstract_text = s2.abstract_text.unwrap_or_default();

    let authors: Vec<String> = s2
        .authors
        .unwrap_or_default()
        .into_iter()
        .map(|a| a.name)
        .collect();

    let categories: Vec<String> = s2.venue.iter().cloned().collect();

    let arxiv_id = s2.external_ids.as_ref().and_then(|ids| ids.arxiv.clone());
    let doi = s2.external_ids.as_ref().and_then(|ids| ids.doi.clone());

    let pdf_url = s2
        .open_access_pdf
        .as_ref()
        .and_then(|pdf| pdf.url.clone())
        .or_else(|| {
            arxiv_id
                .as_ref()
                .map(|aid| format!("https://arxiv.org/pdf/{}.pdf", aid))
        });

    let cited_by = s2.citation_count.unwrap_or(0) as usize;

    Paper::with_metadata(
        arxiv_id,
        title,
        clean_text(&abstract_text),
        authors,
        categories,
        PaperMetadata {
            cited_by,
            references: 0,
            doi,
            pdf_url,
        },
    )
}

/// Search Semantic Scholar (sync wrapper)
pub fn semantic_search_blocking(
    query: &str,
    max_results: usize,
) -> Result<Vec<Paper>, SearchError> {
    let rt = tokio::runtime::Runtime::new()?;
    rt.block_on(semantic_search(query, max_results, 30))
}

// ============================================================================
// Cross-Source Search (arXiv + Semantic Scholar)
// ============================================================================

/// Search result with source tracking
#[derive(Debug, Clone)]
pub struct SearchResult {
    pub paper: Paper,
    pub source: Source,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Source {
    ArXiv,
    SemanticScholar,
}

impl std::fmt::Display for Source {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Source::ArXiv => write!(f, "arxiv"),
            Source::SemanticScholar => write!(f, "semantic-scholar"),
        }
    }
}

/// Search multiple sources concurrently and merge results.
///
/// # Arguments
/// * `query` - Search query
/// * `max_per_source` - Maximum papers per source (default 5)
/// * `sources` - Which sources to query (default: both arXiv and Semantic Scholar)
///
/// # Returns
/// Combined vector of SearchResult objects (no dedup, sorted by source order)
pub async fn cross_search(
    query: &str,
    max_per_source: usize,
    sources: &[Source],
) -> Result<Vec<SearchResult>, SearchError> {
    if query.trim().is_empty() {
        return Err(SearchError::SearchFailed(
            "Query cannot be empty".to_string(),
        ));
    }

    if sources.is_empty() {
        return Err(SearchError::SearchFailed(
            "At least one source must be specified".to_string(),
        ));
    }

    let mut search_results = Vec::new();

    for &source in sources {
        let result = match source {
            Source::ArXiv => arxiv_search(query, max_per_source, 30).await,
            Source::SemanticScholar => semantic_search(query, max_per_source, 30).await,
        };

        match result {
            Ok(papers) => {
                for paper in papers {
                    search_results.push(SearchResult { paper, source });
                }
            }
            Err(e) => {
                tracing::warn!("Search failed for source {:?}: {}", source, e);
            }
        }
    }

    Ok(search_results)
}

/// Synchronous wrapper for cross_search
pub fn cross_search_blocking(
    query: &str,
    max_per_source: usize,
    sources: &[Source],
) -> Result<Vec<SearchResult>, SearchError> {
    let rt = tokio::runtime::Runtime::new()?;
    rt.block_on(cross_search(query, max_per_source, sources))
}

/// Convenience function for searching both arXiv and Semantic Scholar
pub fn search_papers_multi(
    query: &str,
    max_per_source: usize,
) -> Result<Vec<SearchResult>, SearchError> {
    cross_search_blocking(
        query,
        max_per_source,
        &[Source::ArXiv, Source::SemanticScholar],
    )
}

// ============================================================================
// Helper Functions
// ============================================================================

fn extract_tag(xml: &str, tag: &str) -> Option<String> {
    let start = format!("<{}>", tag);
    let end = format!("</{}>", tag);
    xml.find(&start).and_then(|s| {
        xml[s + start.len()..]
            .find(&end)
            .map(|e| xml[s + start.len()..s + start.len() + e].trim().to_string())
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

fn extract_pdf_url(xml: &str, entry_id: &str) -> Option<String> {
    // Look for PDF link in links
    let mut search = xml;
    while let Some(start) = search.find("<link") {
        let chunk = &search[start..];
        if chunk.contains("type=\"application/pdf\"") {
            if let Some(href_start) = chunk.find("href=\"") {
                let href_end = chunk[href_start + 6..].find('"').unwrap_or(0);
                let href = &chunk[href_start + 6..href_start + 6 + href_end];
                return Some(href.to_string());
            }
        }
        search = &search[start + 1..];
    }
    // Fallback
    Some(format!(
        "https://arxiv.org/pdf/{}.pdf",
        entry_id.split('/').next_back().unwrap_or("")
    ))
}

fn extract_doi(xml: &str) -> Option<String> {
    extract_tag(xml, "arxiv:doi")
}

fn clean_arxiv_title(title: &str) -> String {
    title
        .lines()
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
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_clean_text() {
        assert_eq!(clean_text("hello\n\nworld  "), "hello world");
        assert_eq!(clean_text("  test  "), "test");
        assert_eq!(clean_text("multi\nline\ntext"), "multi line text");
    }

    #[test]
    fn test_clean_arxiv_title() {
        assert_eq!(
            clean_arxiv_title("Title\nWith\nNewlines"),
            "Title With Newlines"
        );
        assert_eq!(clean_arxiv_title("  Trimmed  "), "Trimmed");
    }

    #[test]
    fn test_extract_tag() {
        let xml = "<title>Test Title</title> more text";
        assert_eq!(extract_tag(xml, "title"), Some("Test Title".to_string()));
    }

    #[test]
    fn test_extract_tag_not_found() {
        let xml = "<title>Test</title>";
        assert_eq!(extract_tag(xml, "author"), None);
    }

    #[test]
    fn test_extract_authors() {
        let xml = r#"
            <author><name>John Doe</name></author>
            <author><name>Jane Smith</name></author>
        "#;
        let authors = extract_authors(xml);
        assert_eq!(authors, vec!["John Doe", "Jane Smith"]);
    }

    #[test]
    fn test_extract_categories() {
        let xml = r#"
            <category term="csAI" />
            <category term="csLG" />
            <category term="statML" />
        "#;
        let cats = extract_categories(xml);
        assert_eq!(cats.len(), 3);
        assert!(cats.contains(&"csAI".to_string()));
        assert!(cats.contains(&"csLG".to_string()));
        assert!(cats.contains(&"statML".to_string()));
    }

    #[test]
    fn test_extract_pdf_url() {
        let xml = r#"
            <link href="https://arxiv.org/pdf/2301.12345.pdf" type="application/pdf" />
        "#;
        let pdf_url = extract_pdf_url(xml, "http://arxiv.org/abs/2301.12345v1");
        assert!(pdf_url.is_some());
        assert!(pdf_url.unwrap().contains("2301.12345.pdf"));
    }

    #[test]
    fn test_source_display() {
        assert_eq!(Source::ArXiv.to_string(), "arxiv");
        assert_eq!(Source::SemanticScholar.to_string(), "semantic-scholar");
    }

    #[test]
    fn test_search_error_display() {
        let err = SearchError::SearchFailed("test error".to_string());
        assert_eq!(err.to_string(), "Search failed: test error");

        let err = SearchError::RateLimited(60);
        assert_eq!(err.to_string(), "Rate limited, retry after 60s");
    }

    #[test]
    fn test_cross_search_empty_sources() {
        // Test that empty sources returns error
        let rt = tokio::runtime::Runtime::new().unwrap();
        let result = rt.block_on(cross_search("test", 5, &[]));
        assert!(result.is_err());
    }

    #[test]
    fn test_arxiv_search_empty_query() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let result = rt.block_on(arxiv_search("", 5, 30));
        assert!(result.is_err());
    }

    #[test]
    fn test_semantic_search_empty_query() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let result = rt.block_on(semantic_search("", 5, 30));
        assert!(result.is_err());
    }
}
