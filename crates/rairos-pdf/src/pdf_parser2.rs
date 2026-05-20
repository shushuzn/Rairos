//! PDF Parser with file-based caching and fallback extraction.
//!
//! Provides:
//! - `PdfCache` — file-based JSON cache for parsed papers
//! - `PdfParser` — high-level orchestrator wrapping PdfCache + extraction logic
//! - `extract_pdf_text_with_fallback` — tries extraction, falls back if text is short

#![allow(dead_code)]

use regex::Regex;
use std::path::Path;
use std::sync::LazyLock;

use crate::{PdfError, Result};

static RE_CLEAN_WHITESPACE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"[ \t]+\n").expect("valid regex"));
static RE_COLLAPSE_BLANK_LINES: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\n{3,}").expect("valid regex"));

fn clean_text(text: &str) -> String {
    let text = text.replace('\r', "\n");
    let text = RE_CLEAN_WHITESPACE.replace_all(&text, "\n").to_string();
    let text = RE_COLLAPSE_BLANK_LINES
        .replace_all(&text, "\n\n")
        .to_string();
    text.trim().to_string()
}

// ============================================================================
// Cache Management
// ============================================================================

const PARSE_VERSION: usize = 1;

/// File-based cache for parsed papers.
pub struct PdfCache {
    cache_dir: std::path::PathBuf,
}

impl PdfCache {
    /// Create a new PdfCache with the given cache directory.
    /// The directory is created if it does not exist.
    pub fn new(cache_dir: impl AsRef<std::path::Path>) -> Self {
        let cache_dir = cache_dir.as_ref().to_path_buf();
        std::fs::create_dir_all(&cache_dir).ok();
        Self { cache_dir }
    }

    fn cache_path(&self, paper_id: &str) -> std::path::PathBuf {
        self.cache_dir.join(format!("{}.json", paper_id))
    }

    /// Load cached parse result if the PDF hash matches.
    pub fn load(&self, paper_id: &str, pdf_hash: &str) -> Option<crate::ParsedPaper> {
        let path = self.cache_path(paper_id);
        if !path.exists() {
            return None;
        }
        let Ok(contents) = std::fs::read_to_string(&path) else {
            return None;
        };
        let Ok(value) = serde_json::from_str::<serde_json::Value>(&contents) else {
            return None;
        };
        if value.get("pdf_hash").and_then(|v| v.as_str()) == Some(pdf_hash) {
            crate::ParsedPaper::from_cache_dict(&value)
        } else {
            None
        }
    }

    /// Save a parse result to the cache.
    pub fn save(&self, paper: &crate::ParsedPaper) -> std::io::Result<()> {
        let path = self.cache_path(&paper.paper_id);
        let json = serde_json::to_string_pretty(&paper.to_cache_dict())
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        std::fs::write(&path, json)
    }

    /// Invalidate (remove) the cache entry for a paper.
    pub fn invalidate(&self, paper_id: &str) -> std::io::Result<()> {
        let path = self.cache_path(paper_id);
        if path.exists() {
            std::fs::remove_file(path)?;
        }
        Ok(())
    }
}

// ============================================================================
// PDF Parser
// ============================================================================

/// PDF Parser with structured extraction and caching.
pub struct PdfParser {
    cache: Option<PdfCache>,
}

impl PdfParser {
    /// Create a new PdfParser with a cache directory.
    pub fn new(cache_dir: impl AsRef<std::path::Path>) -> Self {
        Self {
            cache: Some(PdfCache::new(cache_dir)),
        }
    }

    /// Create a PdfParser with an existing PdfCache instance.
    pub fn with_cache(cache: PdfCache) -> Self {
        Self { cache: Some(cache) }
    }

    /// Create a PdfParser without caching.
    pub fn without_cache() -> Self {
        Self { cache: None }
    }

    /// Parse a PDF and return structured content.
    ///
    /// Checks the cache first if `use_cache` is true. On cache miss,
    /// extracts structured content, builds a `ParsedPaper`, saves to cache,
    /// and returns the result.
    pub fn parse(
        &self,
        pdf_path: &Path,
        paper_id: &str,
        use_cache: bool,
    ) -> Result<crate::ParsedPaper> {
        if !pdf_path.exists() {
            return Err(PdfError::NotFound(
                pdf_path.to_string_lossy().to_string(),
            ));
        }

        let pdf_hash = super::compute_pdf_hash(pdf_path)?;

        // Cache check
        if use_cache {
            if let Some(ref cache) = self.cache {
                if let Some(cached) = cache.load(paper_id, &pdf_hash) {
                    return Ok(cached);
                }
            }
        }

        // Extract structured content
        let sdoc = super::extract_structured_content(pdf_path)?;

        let text = super::text_blocks_to_lines(&sdoc.text_blocks).join("\n");
        let word_count = text.split_whitespace().count();

        let paper = crate::ParsedPaper {
            paper_id: paper_id.to_string(),
            text: clean_text(&text),
            latex_blocks: sdoc
                .math_blocks
                .iter()
                .map(|m| crate::LaTeXBlock {
                    source: m.text.clone(),
                    is_display: m.is_display,
                    page: m.page,
                    bbox: (0.0_f32, 0.0_f32, 0.0_f32, 0.0_f32),
                })
                .collect(),
            tables: sdoc
                .tables
                .iter()
                .map(|t| {
                    let header_line = t.text.lines().next().unwrap_or("");
                    let headers: Vec<String> = header_line
                        .trim()
                        .trim_matches('|')
                        .split('|')
                        .map(|c| c.trim().to_string())
                        .collect();
                    let rows: Vec<Vec<String>> = t
                        .text
                        .lines()
                        .skip(2)
                        .map(|l| {
                            l.trim()
                                .trim_matches('|')
                                .split('|')
                                .map(|c| c.trim().to_string())
                                .collect()
                        })
                        .collect();
                    crate::TableData {
                        headers,
                        rows,
                        page: t.page,
                        bbox: t.bbox,
                        caption: String::new(),
                    }
                })
                .collect(),
            figures: Vec::new(),
            page_count: 0,
            word_count,
            parse_version: PARSE_VERSION,
            pdf_hash,
            title: String::new(),
            authors: Vec::new(),
            abstract_text: String::new(),
            published: String::new(),
            warnings: Vec::new(),
            errors: Vec::new(),
        };

        // Save to cache
        if let Some(ref cache) = self.cache {
            cache.save(&paper).ok();
        }

        Ok(paper)
    }
}

// ============================================================================
// Text Extraction with Fallback
// ============================================================================

/// Extract plain text with a fallback chain.
///
/// First attempts standard PDF text extraction via `extract_pdf_text`.
/// If the extracted text is very short (fewer than 100 chars), it may be
/// a scanned/image PDF; the short text is returned as-is without cleaning.
/// Otherwise, whitespace is normalized and cleaned before returning.
pub fn extract_pdf_text_with_fallback(pdf_path: &Path) -> Result<String> {
    let text = super::extract_pdf_text(pdf_path)?;
    if text.len() < 100 {
        // If text is too short, it might be a scanned/image PDF
        Ok(text)
    } else {
        Ok(clean_text(&text))
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_clean_text() {
        let input = "Hello  \n\n\n\nWorld\n";
        let result = clean_text(input);
        assert!(result.contains("Hello"));
        assert!(!result.contains("  "));
    }

    #[test]
    fn test_pdf_cache_save_load() {
        let temp_dir = tempfile::tempdir().unwrap();
        let cache = PdfCache::new(temp_dir.path());

        let paper = crate::ParsedPaper {
            paper_id: "2301.00001".to_string(),
            text: "Test content".to_string(),
            page_count: 10,
            word_count: 100,
            parse_version: 1,
            pdf_hash: "abc123".to_string(),
            ..Default::default()
        };

        cache.save(&paper).unwrap();

        let loaded = cache.load("2301.00001", "abc123").unwrap();
        assert_eq!(loaded.paper_id, "2301.00001");
        assert_eq!(loaded.word_count, 100);
    }

    #[test]
    fn test_pdf_cache_hash_mismatch() {
        let temp_dir = tempfile::tempdir().unwrap();
        let cache = PdfCache::new(temp_dir.path());

        let paper = crate::ParsedPaper {
            paper_id: "2301.00001".to_string(),
            text: "Test content".to_string(),
            pdf_hash: "abc123".to_string(),
            ..Default::default()
        };

        cache.save(&paper).unwrap();

        // Different hash should not load
        let loaded = cache.load("2301.00001", "different_hash");
        assert!(loaded.is_none());
    }

    #[test]
    fn test_pdf_cache_invalidate() {
        let temp_dir = tempfile::tempdir().unwrap();
        let cache = PdfCache::new(temp_dir.path());

        let paper = crate::ParsedPaper {
            paper_id: "2301.00001".to_string(),
            text: "Test content".to_string(),
            pdf_hash: "abc123".to_string(),
            ..Default::default()
        };

        cache.save(&paper).unwrap();
        assert!(cache.load("2301.00001", "abc123").is_some());

        cache.invalidate("2301.00001").unwrap();
        assert!(cache.load("2301.00001", "abc123").is_none());
    }

    #[test]
    fn test_parsed_paper_cache_dict() {
        let paper = crate::ParsedPaper {
            paper_id: "test".to_string(),
            text: "content".to_string(),
            page_count: 5,
            word_count: 50,
            parse_version: 1,
            pdf_hash: "hash123".to_string(),
            ..Default::default()
        };

        let dict = paper.to_cache_dict();
        let restored = crate::ParsedPaper::from_cache_dict(&dict).unwrap();
        assert_eq!(restored.paper_id, "test");
        assert_eq!(restored.word_count, 50);
    }

    #[test]
    fn test_block_type_as_str() {
        assert_eq!(crate::BlockType::Heading.as_str(), "heading");
        assert_eq!(crate::BlockType::Body.as_str(), "body");
        assert_eq!(crate::BlockType::BodyType.as_str(), "body");
        assert_eq!(crate::BlockType::Caption.as_str(), "caption");
        assert_eq!(crate::BlockType::ListItem.as_str(), "list_item");
        assert_eq!(crate::BlockType::Footnote.as_str(), "footnote");
    }
}
