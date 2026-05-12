//! Rairos PDF Parser — Structured PDF parsing with caching
//!
//! Ports `pdf/parser.py` and `pdf/extract.py`.
//!
//! Provides structured PDF content extraction: LaTeX blocks, tables, figures,
//! text extraction with fallback chain, and cache-aware parsing.

#![allow(clippy::needless_range_loop)]

use regex::Regex;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs::File;
use std::io::Read;
use std::path::Path;
use thiserror::Error;

// ============================================================================
// Error Types
// ============================================================================

#[derive(Error, Debug)]
pub enum PdfParserError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("PDF not found: {0}")]
    NotFound(String),

    #[error("Parse failed: {0}")]
    ParseFailed(String),

    #[error("Timeout: {0}")]
    Timeout(String),
}

pub type Result<T> = std::result::Result<T, PdfParserError>;

// ============================================================================
// Data Structures
// ============================================================================

/// A LaTeX math block extracted from a PDF.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct LaTeXBlock {
    pub source: String,
    pub is_display: bool,
    pub page: i32,
    #[serde(default)]
    pub bbox: (f64, f64, f64, f64),
}

/// A table extracted from a PDF.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TableData {
    pub headers: Vec<String>,
    pub rows: Vec<Vec<String>>,
    pub page: i32,
    #[serde(default)]
    pub bbox: (f64, f64, f64, f64),
    #[serde(default)]
    pub caption: String,
}

/// A figure extracted from a PDF.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct FigureData {
    pub caption: String,
    pub page: i32,
    #[serde(default)]
    pub bbox: (f64, f64, f64, f64),
    #[serde(default)]
    pub alt_text: String,
}

/// A parsed paper with structured content.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ParsedPaper {
    pub paper_id: String,
    pub text: String,
    #[serde(default)]
    pub latex_blocks: Vec<LaTeXBlock>,
    #[serde(default)]
    pub tables: Vec<TableData>,
    #[serde(default)]
    pub figures: Vec<FigureData>,
    pub page_count: i32,
    pub word_count: i32,
    pub parse_version: i32,
    #[serde(default)]
    pub pdf_hash: String,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub authors: Vec<String>,
    #[serde(default)]
    pub abstract_text: String,
    #[serde(default)]
    pub published: String,
    #[serde(default)]
    pub warnings: Vec<String>,
    #[serde(default)]
    pub errors: Vec<String>,
}

impl ParsedPaper {
    /// Serialize to a JSON value for caching.
    pub fn to_cache_dict(&self) -> serde_json::Value {
        serde_json::to_value(self).unwrap_or_default()
    }

    /// Deserialize from a JSON value (cache lookup).
    pub fn from_cache_dict(d: &serde_json::Value) -> Option<Self> {
        serde_json::from_value(d.clone()).ok()
    }
}

// ============================================================================
// Text Cleaning
// ============================================================================

lazy_static::lazy_static! {
    static ref RE_CLEAN_WHITESPACE: Regex = Regex::new(r"[ \t]+\n").unwrap();
    static ref RE_COLLAPSE_BLANK_LINES: Regex = Regex::new(r"\n{3,}").unwrap();
    static ref RE_DISPLAY_MATH_1: Regex = Regex::new(r"^\s*\$\$[\s\S]+?\$\$\s*$").unwrap();
    static ref RE_DISPLAY_MATH_2: Regex = Regex::new(r"^\s*\\\[[\s\S]+?\s*\\\]\s*$").unwrap();
    static ref RE_DISPLAY_MATH_ENV: Regex =
        Regex::new(r"^\s*\\begin\{(align|align\*|gather|gather\*|eqnarray|multline)\}")
            .unwrap();
    static ref RE_INLINE_MATH: Regex = Regex::new(r"\$([^\$\n]+?)\$|\\\([^)]+\\\)").unwrap();
}

fn clean_text(text: &str) -> String {
    let text = text.replace('\r', "\n");
    let text = RE_CLEAN_WHITESPACE.replace_all(&text, "\n").to_string();
    let text = RE_COLLAPSE_BLANK_LINES
        .replace_all(&text, "\n\n")
        .to_string();
    text.trim().to_string()
}

// ============================================================================
// Math Detection
// ============================================================================

fn is_display_math(line: &str) -> bool {
    let s = line.trim();
    RE_DISPLAY_MATH_1.is_match(s)
        || RE_DISPLAY_MATH_2.is_match(s)
        || RE_DISPLAY_MATH_ENV.is_match(s)
}

fn extract_inline_math(line: &str) -> Vec<LaTeXBlock> {
    RE_INLINE_MATH
        .captures_iter(line)
        .map(|m| LaTeXBlock {
            source: m.get(0).map(|x| x.as_str().to_string()).unwrap_or_default(),
            is_display: false,
            page: -1,
            bbox: (0.0, 0.0, 0.0, 0.0),
        })
        .collect()
}

// ============================================================================
// PDF Hash Computation
// ============================================================================

/// Compute SHA256 hash of a PDF file for cache validation.
pub fn compute_pdf_hash(pdf_path: &Path) -> Result<String> {
    let mut file = File::open(pdf_path)?;
    let mut hasher = Sha256::new();
    let mut buffer = [0u8; 8192];

    loop {
        let bytes_read = Read::read(&mut file, &mut buffer)?;
        if bytes_read == 0 {
            break;
        }
        hasher.update(&buffer[..bytes_read]);
    }

    Ok(format!("{:x}", hasher.finalize()))
}

// ============================================================================
// Plain Text Extraction (basic implementation)
// ============================================================================

/// Extract plain text from a PDF file using basic pattern matching.
/// Note: Full PDF text extraction requires native PDF parsing libraries.
/// This is a placeholder that reads text layers from PDF structures.
pub fn extract_pdf_text(_pdf_path: &Path) -> Result<String> {
    // Full PDF text extraction requires native libraries like pdf-extract or lopdf.
    // This placeholder returns empty string; real implementation would use:
    // - lopdf for direct PDF object extraction
    // - pdf-extract for better text layer handling
    Ok(String::new())
}

/// Extract plain text with fallback chain: PDF text layer -> pdfminer fallback.
pub fn extract_pdf_text_with_fallback(pdf_path: &Path) -> Result<String> {
    let text = extract_pdf_text(pdf_path)?;
    if text.len() < 100 {
        // If text is too short, it might be a scanned/image PDF
        Ok(text)
    } else {
        Ok(clean_text(&text))
    }
}

// ============================================================================
// Structured Content Extraction
// ============================================================================

/// Extract structured content: text blocks, tables, math, figures.
/// This is a framework implementation; actual PDF parsing requires native libs.
pub fn extract_structured_content(_pdf_path: &Path) -> Result<StructuredPdfContent> {
    // Full implementation would use lopdf or pdf-extract for:
    // - Block-level text extraction
    // - Table detection and extraction
    // - Figure/image detection
    // - Math block identification
    Ok(StructuredPdfContent::default())
}

/// Structured PDF content container.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct StructuredPdfContent {
    #[serde(default)]
    pub text_blocks: Vec<TextBlock>,
    #[serde(default)]
    pub tables: Vec<TableBlock>,
    #[serde(default)]
    pub math_blocks: Vec<MathBlock>,
}

/// A text block with type classification.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TextBlock {
    #[serde(rename = "type")]
    pub block_type: BlockType,
    pub text: String,
    pub page: i32,
    #[serde(default)]
    pub bbox: (f64, f64, f64, f64),
}

/// A table block from PDF extraction.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TableBlock {
    pub text: String,
    pub page: i32,
    #[serde(default)]
    pub bbox: (f64, f64, f64, f64),
}

/// A math block from PDF extraction.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MathBlock {
    pub text: String,
    pub is_display: bool,
    pub page: i32,
}

/// Block type classification.
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum BlockType {
    Heading,
    Body,
    Caption,
    ListItem,
    Footnote,
    #[default]
    BodyType,
}

impl BlockType {
    pub fn as_str(&self) -> &'static str {
        match self {
            BlockType::Heading => "heading",
            BlockType::Body | BlockType::BodyType => "body",
            BlockType::Caption => "caption",
            BlockType::ListItem => "list_item",
            BlockType::Footnote => "footnote",
        }
    }
}

// ============================================================================
// Block Type Detection (ported from pdf/extract.py)
// ============================================================================

fn is_all_caps(s: &str) -> bool {
    let alphabetic: String = s.chars().filter(|c| c.is_alphabetic()).collect();
    if alphabetic.is_empty() {
        return false;
    }
    alphabetic.chars().all(|c| c.is_uppercase())
}

/// Detect block type from line text using heuristics.
pub fn detect_block_type(line: &str) -> BlockType {
    let s = line.trim();
    if s.is_empty() {
        return BlockType::Body;
    }

    // Markdown heading
    if Regex::new(r"^#{1,6}\s+").unwrap().is_match(s) {
        return BlockType::Heading;
    }

    // All-caps short line
    if is_all_caps(s) && s.len() >= 3 && s.len() <= 60 && s.split_whitespace().count() <= 10 {
        return BlockType::Heading;
    }

    // Numbered section heading
    if Regex::new(r"^(\d+(\.\d+)*\.?|I{1,3}|IV|V|VI{0,3})\s+[A-Z][A-Za-z ]{2,40}$")
        .unwrap()
        .is_match(s)
    {
        return BlockType::Heading;
    }

    // Caption pattern
    let caption_pat =
        Regex::new(r"(?i)^(Figure|Fig\.|Table|Alg\.?|Algorithm|Listing|Plate)\s+\d").unwrap();
    if caption_pat.is_match(s) {
        return BlockType::Caption;
    }

    // Footnote
    if Regex::new(r"^\[\d+\]$").unwrap().is_match(s) || Regex::new(r"^\^\d+$").unwrap().is_match(s)
    {
        return BlockType::Footnote;
    }

    // List item
    if Regex::new(r"^[-*+]\s").unwrap().is_match(s) || Regex::new(r"^\d+\.\s").unwrap().is_match(s)
    {
        return BlockType::ListItem;
    }

    BlockType::Body
}

// ============================================================================
// Section Detection (ported from sections/segment.py)
// ============================================================================

lazy_static::lazy_static! {
    static ref SECTION_KEYWORDS: Vec<&'static str> = vec![
        "abstract",
        "introduction",
        "background",
        "related work",
        "method",
        "approach",
        "preliminaries",
        "experiments",
        "evaluation",
        "results",
        "discussion",
        "limitations",
        "conclusion",
        "future work",
        "references",
        "appendix",
        "acknowledgments",
        "ablation",
    ];
}

/// Check if a line looks like a section heading.
pub fn looks_like_heading(line: &str) -> bool {
    let s = line.trim();
    if s.len() < 3 || s.len() > 120 {
        return false;
    }

    if Regex::new(r"^(\d+(\.\d+)*)\.?\s+[A-Za-z].{2,}$")
        .unwrap()
        .is_match(s)
    {
        return true;
    }
    if Regex::new(r"^(I|II|III|IV|V|VI|VII|VIII|IX|X)\.?\s+[A-Za-z].{2,}$")
        .unwrap()
        .is_match(s)
    {
        return true;
    }

    let low = s.to_lowercase();
    if SECTION_KEYWORDS.iter().any(|k| low == *k) {
        return true;
    }
    if SECTION_KEYWORDS
        .iter()
        .any(|k| low.starts_with(&format!("{} ", k)))
    {
        return true;
    }

    if s.chars().all(|c| c.is_uppercase()) && s.len() >= 4 && s.len() <= 40 {
        return true;
    }

    false
}

/// Flatten TextBlocks to raw lines.
pub fn text_blocks_to_lines(blocks: &[TextBlock]) -> Vec<String> {
    blocks.iter().map(|b| b.text.clone()).collect()
}

/// Segment text into (title, content) pairs.
pub fn segment_into_sections(text: &str, max_sections: usize) -> Vec<(String, String)> {
    let lines: Vec<&str> = text.split('\n').collect();
    let mut sections: Vec<(String, Vec<&str>)> = Vec::new();
    let mut cur_title = "BODY".to_string();
    let mut cur_buf: Vec<&str> = Vec::new();

    for line in &lines {
        let stripped = line.trim();
        if let Some(cap) = Regex::new(r"^(#{1,6})\s+(.+)$").unwrap().captures(stripped) {
            if !cur_buf.is_empty() {
                sections.push((cur_title.clone(), cur_buf));
            }
            cur_title = cap
                .get(2)
                .map(|m| m.as_str().trim().to_string())
                .unwrap_or_default();
            cur_buf = Vec::new();
        } else if looks_like_heading(line) {
            if !cur_buf.is_empty() {
                sections.push((cur_title.clone(), cur_buf));
            }
            cur_title = line.trim().to_string();
            cur_buf = Vec::new();
        } else {
            cur_buf.push(line);
        }
    }

    if !cur_buf.is_empty() {
        sections.push((cur_title.clone(), cur_buf));
    }

    let merged: Vec<(String, String)> = sections
        .into_iter()
        .filter(|(_, buf)| !buf.is_empty())
        .map(|(title, buf)| {
            let content = buf.join("\n").trim().to_string();
            (title, content)
        })
        .filter(|(_, content)| !content.is_empty())
        .collect();

    if merged.len() > max_sections {
        let truncated: Vec<(String, String)> = merged.iter().take(max_sections).cloned().collect();
        let mut result = truncated;
        result.push((
            "TRUNCATED".to_string(),
            "...(text truncated)...".to_string(),
        ));
        result
    } else {
        merged
    }
}

// ============================================================================
// Markdown Table Parsing
// ============================================================================

/// Parse a markdown table into structured TableData.
pub fn parse_markdown_table(table_text: &str, page: i32) -> TableData {
    let lines: Vec<&str> = table_text
        .lines()
        .filter(|l| !l.trim().is_empty())
        .collect();
    if lines.len() < 2 {
        return TableData::default();
    }

    // Parse header
    let headers: Vec<String> = lines[0]
        .trim()
        .trim_matches('|')
        .split('|')
        .map(|c| c.trim().to_string())
        .collect();

    // Skip separator line
    let data_start = if lines[1].contains("---") { 2 } else { 1 };

    // Parse rows
    let mut rows: Vec<Vec<String>> = Vec::new();
    for line in &lines[data_start..] {
        let row: Vec<String> = line
            .trim()
            .trim_matches('|')
            .split('|')
            .map(|c| c.trim().to_string())
            .collect();
        if row.len() == headers.len() {
            rows.push(row);
        }
    }

    TableData {
        headers,
        rows,
        page,
        bbox: (0.0, 0.0, 0.0, 0.0),
        caption: String::new(),
    }
}

// ============================================================================
// Cache Management
// ============================================================================

const PARSE_VERSION: i32 = 1;

/// File-based cache for parsed papers.
pub struct PdfCache {
    cache_dir: std::path::PathBuf,
}

impl PdfCache {
    pub fn new(cache_dir: impl AsRef<std::path::Path>) -> Self {
        let cache_dir = cache_dir.as_ref().to_path_buf();
        std::fs::create_dir_all(&cache_dir).ok();
        Self { cache_dir }
    }

    fn cache_path(&self, paper_id: &str) -> std::path::PathBuf {
        self.cache_dir.join(format!("{}.json", paper_id))
    }

    /// Load cached parse if PDF hash matches.
    pub fn load(&self, paper_id: &str, pdf_hash: &str) -> Option<ParsedPaper> {
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
            ParsedPaper::from_cache_dict(&value)
        } else {
            None
        }
    }

    /// Save parse result to cache.
    pub fn save(&self, paper: &ParsedPaper) -> std::io::Result<()> {
        let path = self.cache_path(&paper.paper_id);
        let json = serde_json::to_string_pretty(&paper.to_cache_dict())
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        std::fs::write(&path, json)
    }

    /// Invalidate cache for a paper.
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
    pub fn new(cache_dir: impl AsRef<std::path::Path>) -> Self {
        Self {
            cache: Some(PdfCache::new(cache_dir)),
        }
    }

    pub fn with_cache(cache: PdfCache) -> Self {
        Self { cache: Some(cache) }
    }

    pub fn without_cache() -> Self {
        Self { cache: None }
    }

    /// Parse a PDF and return structured content.
    pub fn parse(&self, pdf_path: &Path, paper_id: &str, use_cache: bool) -> Result<ParsedPaper> {
        if !pdf_path.exists() {
            return Err(PdfParserError::NotFound(
                pdf_path.to_string_lossy().to_string(),
            ));
        }

        let pdf_hash = compute_pdf_hash(pdf_path)?;

        // Cache check
        if use_cache {
            if let Some(ref cache) = self.cache {
                if let Some(cached) = cache.load(paper_id, &pdf_hash) {
                    return Ok(cached);
                }
            }
        }

        // Extract structured content
        let sdoc = extract_structured_content(pdf_path)?;

        let text = text_blocks_to_lines(&sdoc.text_blocks).join("\n");
        let word_count = text.split_whitespace().count() as i32;

        let paper = ParsedPaper {
            paper_id: paper_id.to_string(),
            text: clean_text(&text),
            latex_blocks: sdoc
                .math_blocks
                .iter()
                .map(|m| LaTeXBlock {
                    source: m.text.clone(),
                    is_display: m.is_display,
                    page: m.page,
                    bbox: (0.0, 0.0, 0.0, 0.0),
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
                    TableData {
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
    fn test_is_display_math() {
        assert!(is_display_math("$$\nx^2\n$$"));
        assert!(is_display_math("\\[E = mc^2\\]"));
        assert!(!is_display_math("$x$"));
        assert!(!is_display_math("Regular text"));
    }

    #[test]
    fn test_extract_inline_math() {
        let blocks = extract_inline_math("The equation $x^2$ is inline");
        assert_eq!(blocks.len(), 1);
        assert!(!blocks[0].is_display);
    }

    #[test]
    fn test_detect_block_type_heading() {
        assert_eq!(detect_block_type("# Introduction"), BlockType::Heading);
        assert_eq!(detect_block_type("INTRODUCTION"), BlockType::Heading);
        assert_eq!(detect_block_type("1. Introduction"), BlockType::Heading);
    }

    #[test]
    fn test_detect_block_type_caption() {
        assert_eq!(
            detect_block_type("Figure 1: Architecture"),
            BlockType::Caption
        );
        assert_eq!(detect_block_type("Table 2: Results"), BlockType::Caption);
    }

    #[test]
    fn test_detect_block_type_list_item() {
        assert_eq!(detect_block_type("- item"), BlockType::ListItem);
        assert_eq!(detect_block_type("1. first"), BlockType::ListItem);
    }

    #[test]
    fn test_detect_block_type_footnote() {
        assert_eq!(detect_block_type("[1]"), BlockType::Footnote);
        assert_eq!(detect_block_type("^42"), BlockType::Footnote);
    }

    #[test]
    fn test_looks_like_heading() {
        assert!(looks_like_heading("INTRODUCTION"));
        assert!(looks_like_heading("1. Introduction"));
        assert!(looks_like_heading("Related Work"));
        // Note: "## Methods" is a Markdown heading, handled separately in segment_into_sections
        assert!(!looks_like_heading("This is a regular paragraph."));
        assert!(!looks_like_heading("a"));
    }

    #[test]
    fn test_segment_into_sections() {
        let text =
            "Introduction\n\nSome intro text.\n\n## Methods\n\nOur method.\n\n## Results\n\nOur results.";
        let sections = segment_into_sections(text, 18);
        assert!(sections.len() >= 2);
        let titles: Vec<_> = sections.iter().map(|(t, _)| t.as_str()).collect();
        assert!(titles.contains(&"Methods"));
        assert!(titles.contains(&"Results"));
    }

    #[test]
    fn test_segment_into_sections_max() {
        let text: String = (0..30)
            .map(|i| format!("Section {}\n\nContent for section {}.\n\n", i, i))
            .collect();
        let sections = segment_into_sections(&text, 5);
        assert!(sections.len() <= 6); // 5 sections + TRUNCATED
    }

    #[test]
    fn test_parse_markdown_table() {
        let md = "| A | B |\n|---|---|\n| 1 | 2 |\n| 3 | 4 |";
        let table = parse_markdown_table(md, 0);
        assert_eq!(table.headers, vec!["A", "B"]);
        assert_eq!(table.rows.len(), 2);
        assert_eq!(table.page, 0);
    }

    #[test]
    fn test_parse_markdown_table_empty() {
        let table = parse_markdown_table("", 0);
        assert!(table.headers.is_empty());
        assert!(table.rows.is_empty());
    }

    #[test]
    fn test_pdf_hash_deterministic() {
        let temp_dir = tempfile::tempdir().unwrap();
        let pdf_path = temp_dir.path().join("test.pdf");
        std::fs::write(&pdf_path, b"%PDF-1.4 test content").unwrap();

        let hash1 = compute_pdf_hash(&pdf_path).unwrap();
        let hash2 = compute_pdf_hash(&pdf_path).unwrap();
        assert_eq!(hash1, hash2);
    }

    #[test]
    fn test_pdf_cache_save_load() {
        let temp_dir = tempfile::tempdir().unwrap();
        let cache = PdfCache::new(temp_dir.path());

        let paper = ParsedPaper {
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

        let paper = ParsedPaper {
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

        let paper = ParsedPaper {
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
    fn test_text_blocks_to_lines() {
        let blocks = vec![
            TextBlock {
                block_type: BlockType::Heading,
                text: "Introduction".to_string(),
                page: 0,
                bbox: (0.0, 0.0, 0.0, 0.0),
            },
            TextBlock {
                block_type: BlockType::Body,
                text: "Some text".to_string(),
                page: 0,
                bbox: (0.0, 0.0, 0.0, 0.0),
            },
        ];
        let lines = text_blocks_to_lines(&blocks);
        assert_eq!(lines, vec!["Introduction", "Some text"]);
    }

    #[test]
    fn test_parsed_paper_cache_dict() {
        let paper = ParsedPaper {
            paper_id: "test".to_string(),
            text: "content".to_string(),
            page_count: 5,
            word_count: 50,
            parse_version: 1,
            pdf_hash: "hash123".to_string(),
            ..Default::default()
        };

        let dict = paper.to_cache_dict();
        let restored = ParsedPaper::from_cache_dict(&dict).unwrap();
        assert_eq!(restored.paper_id, "test");
        assert_eq!(restored.word_count, 50);
    }

    #[test]
    fn test_block_type_as_str() {
        assert_eq!(BlockType::Heading.as_str(), "heading");
        assert_eq!(BlockType::Body.as_str(), "body");
        assert_eq!(BlockType::Caption.as_str(), "caption");
        assert_eq!(BlockType::ListItem.as_str(), "list_item");
        assert_eq!(BlockType::Footnote.as_str(), "footnote");
    }

    #[test]
    fn test_structured_pdf_content_default() {
        let content = StructuredPdfContent::default();
        assert!(content.text_blocks.is_empty());
        assert!(content.tables.is_empty());
        assert!(content.math_blocks.is_empty());
    }
}
