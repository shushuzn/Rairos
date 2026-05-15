//! Rairos PDF — PDF parsing and extraction
//!
//! Replaces: pdf/parser.py, pdf/extract.py
//!
//! Provides: PDF download with resume, text extraction, structured content parsing
//! (LaTeX blocks, tables, figures).

pub mod extable;
pub mod paper_parser;
pub mod pdf_parser2;
pub mod provenance;

use regex::Regex;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::io::Read;
use std::path::Path;
use thiserror::Error;

// ============================================================================
// Error Types
// ============================================================================

#[derive(Error, Debug)]
pub enum PdfError {
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("PDF not found: {0}")]
    NotFound(String),

    #[error("PDF parse failed: {0}")]
    ParseFailed(String),

    #[error("Download failed: {0}")]
    DownloadFailed(String),
}

pub type Result<T> = std::result::Result<T, PdfError>;

// ============================================================================
// Data Structures
// ============================================================================

/// LaTeX math block extracted from a PDF
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LaTeXBlock {
    pub source: String,
    pub is_display: bool,
    pub page: usize,
    #[serde(default)]
    pub bbox: (f32, f32, f32, f32),
}

/// Table data extracted from a PDF
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TableData {
    pub headers: Vec<String>,
    pub rows: Vec<Vec<String>>,
    pub page: usize,
    #[serde(default)]
    pub bbox: (f32, f32, f32, f32),
    #[serde(default)]
    pub caption: String,
}

/// Figure/image data extracted from a PDF
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FigureData {
    pub caption: String,
    pub page: usize,
    #[serde(default)]
    pub bbox: (f32, f32, f32, f32),
    #[serde(default)]
    pub alt_text: String,
}

/// Block type for structured text extraction
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BlockType {
    Heading,
    Body,
    BodyType,
    Caption,
    ListItem,
    Footnote,
}

impl BlockType {
    /// Returns a string representation of the block type.
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

/// A text block with type classification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextBlock {
    #[serde(rename = "type")]
    pub block_type: BlockType,
    pub text: String,
    pub page: usize,
}

/// A table block (markdown-like format)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TableBlock {
    pub text: String,
    pub page: usize,
    pub bbox: (f32, f32, f32, f32),
}

/// A math block (LaTeX or Unicode math)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MathBlock {
    pub text: String,
    pub is_display: bool,
    pub page: usize,
}

/// Structured PDF content with text blocks, tables, and math
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct StructuredPdfContent {
    #[serde(default)]
    pub text_blocks: Vec<TextBlock>,
    #[serde(default)]
    pub tables: Vec<TableBlock>,
    #[serde(default)]
    pub math_blocks: Vec<MathBlock>,
}

/// Parsed paper result
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
    pub page_count: usize,
    pub word_count: usize,
    pub parse_version: usize,
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

// ============================================================================
// PDF Download
// ============================================================================

/// Download a PDF file with resume support.
/// Writes to `out_path` on success, uses `.part` suffix for partial downloads.
pub async fn download_pdf(pdf_url: &str, out_path: &Path) -> Result<()> {
    let out_path = out_path.to_path_buf();
    let resume_path = out_path.with_extension("part");

    let client = reqwest::Client::new();

    // Check for existing partial file
    let existing_size = if resume_path.exists() {
        std::fs::metadata(&resume_path)
            .ok()
            .map(|m| m.len())
            .unwrap_or(0)
    } else {
        0
    };

    let mut request = client.get(pdf_url);

    if existing_size > 0 {
        request = request.header("Range", format!("bytes={}-", existing_size));
    }

    let response = request.send().await?;

    // Check if server supports Range
    let supports_range = response.status() == 206
        || (existing_size > 0
            && response
                .headers()
                .get("Accept-Ranges")
                .map(|v| v != "none")
                .unwrap_or(false));

    if supports_range && existing_size > 0 {
        // Resume: append to existing partial file
        let mut file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&resume_path)?;

        let bytes = response.bytes().await?;
        std::io::Write::write_all(&mut file, &bytes)?;
    } else {
        // No resume or new download: overwrite
        response.error_for_status_ref()?;
        let bytes = response.bytes().await?;
        std::fs::write(&out_path, &bytes)?;
    }

    // Finalize: rename .part -> target
    if resume_path.exists() {
        let size = std::fs::metadata(&resume_path)?.len();
        if size > 0 {
            std::fs::rename(&resume_path, &out_path)?;
        }
    }

    if !out_path.exists() {
        return Err(PdfError::DownloadFailed(format!(
            "No content received for {}",
            pdf_url
        )));
    }

    Ok(())
}

// ============================================================================
// Text Extraction (lopdf-based)
// ============================================================================

/// Extract plain text from a PDF file using lopdf.
///
/// Iterates over all pages and concatenates extracted text with newline
/// separators. Per-page extraction failures are silently skipped (graceful
/// degradation) — callers should check the returned length to detect
/// complete extraction failure.
pub fn extract_pdf_text(pdf_path: &Path) -> Result<String> {
    if !pdf_path.exists() {
        return Err(PdfError::NotFound(format!(
            "PDF not found: {}",
            pdf_path.display()
        )));
    }
    let doc = lopdf::Document::load(pdf_path).map_err(|e| {
        PdfError::ParseFailed(format!("Failed to load PDF: {}", e))
    })?;
    let mut text = String::new();
    for page_num in doc.get_pages().into_keys() {
        if let Ok(page_text) = doc.extract_text(&[page_num]) {
            if !text.is_empty() {
                text.push('\n');
            }
            text.push_str(&page_text);
        }
    }
    if text.is_empty() {
        return Err(PdfError::ParseFailed("No text extracted from PDF".into()));
    }
    Ok(text)
}

/// Compute SHA256 hash of a PDF file for cache validation.
pub fn compute_pdf_hash(pdf_path: &Path) -> Result<String> {
    let mut file = std::fs::File::open(pdf_path)?;
    let mut hasher = Sha256::new();
    let mut buffer = [0u8; 8192];

    loop {
        let bytes_read = Read::read(&mut file, &mut buffer)?;
        if bytes_read == 0 {
            break;
        }
        hasher.update(&buffer[..bytes_read]);
    }

    Ok(hex::encode(hasher.finalize()))
}

// ============================================================================
// Regex Helpers
// ============================================================================

#[allow(dead_code)]
fn make_display_math_patterns() -> Vec<Regex> {
    vec![
        Regex::new(r"^\s*\$\$[\s\S]+?\$\$\s*$").unwrap(),
        Regex::new(r"^\s*\\[\s\S]+?\s*\\]\s*$").unwrap(),
        // Use separate patterns for each environment (no backreferences)
        Regex::new(r"^\s*\\begin\{align\*?\}[\s\S]+?\\end\{align\*?\}\s*$").unwrap(),
        Regex::new(r"^\s*\\begin\{gather\*?\}[\s\S]+?\\end\{gather\*?\}\s*$").unwrap(),
        Regex::new(r"^\s*\\begin\{eqnarray\*?\}[\s\S]+?\\end\{eqnarray\*?\}\s*$").unwrap(),
        Regex::new(r"^\s*\\begin\{multline\*?\}[\s\S]+?\\end\{multline\*?\}\s*$").unwrap(),
    ]
}

#[allow(dead_code)]
fn make_inline_math_pat() -> Regex {
    Regex::new(r"\$([^\$\n]+?)\$|\\\([^)]+\\\)").unwrap()
}

#[allow(dead_code)]
fn is_display_math(line: &str) -> bool {
    let s = line.trim();
    for pat in make_display_math_patterns() {
        if pat.is_match(s) {
            return true;
        }
    }
    false
}

#[allow(dead_code)]
fn extract_inline_math(line: &str) -> Vec<MathBlock> {
    let pat = make_inline_math_pat();
    pat.captures_iter(line)
        .map(|m| MathBlock {
            text: m.get(0).map(|x| x.as_str().to_string()).unwrap_or_default(),
            is_display: false,
            page: 0, // Will be set by caller
        })
        .collect()
}

// ============================================================================
// Structured Content Extraction
// ============================================================================

/// Extract structured content from a PDF (text blocks, tables, math).
/// Note: Full implementation requires native PDF parsing library.
pub fn extract_structured_content(_pdf_path: &Path) -> Result<StructuredPdfContent> {
    Ok(StructuredPdfContent::default())
}

// ============================================================================
// Block Type Detection
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

    // Markdown heading: "# text"
    if Regex::new(r"^#{1,6}\s+").unwrap().is_match(s) {
        return BlockType::Heading;
    }

    // All-caps short line (likely section header)
    if is_all_caps(s) && s.len() >= 3 && s.len() <= 60 && s.split_whitespace().count() <= 10 {
        return BlockType::Heading;
    }

    // Numbered section heading: "1. Introduction" or Roman numerals
    if Regex::new(r"^(\d+(\.\d+)*\.?|I{1,3}|IV|V|VI{0,3})\s+[A-Z][A-Za-z ]{2,40}$")
        .unwrap()
        .is_match(s)
    {
        return BlockType::Heading;
    }

    // Figure / Table caption pattern (case insensitive)
    let caption_pat =
        Regex::new(r"(?i)^(Figure|Fig\.|Table|Alg\.?|Algorithm|Listing|Plate)\s+\d").unwrap();
    if caption_pat.is_match(s) {
        return BlockType::Caption;
    }

    // Footnote / reference mark
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
// Section Segmentation
// ============================================================================

const SECTION_KEYWORDS: &[&str] = &[
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

fn is_section_keyword(s: &str) -> bool {
    let low = s.to_lowercase();
    SECTION_KEYWORDS
        .iter()
        .any(|k| low == *k || low.starts_with(&format!("{} ", k)))
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

    if is_section_keyword(s) {
        return true;
    }

    if s.chars().all(|c| c.is_uppercase())
        && s.len() >= 4
        && s.len() <= 40
        && s.split_whitespace().count() <= 8
    {
        return true;
    }

    false
}

/// Flatten TextBlocks to raw lines for backward-compatible segmentation.
pub fn text_blocks_to_lines(blocks: &[TextBlock]) -> Vec<String> {
    blocks.iter().map(|b| b.text.clone()).collect()
}

/// Segment text into sections based on heading detection.
pub fn segment_into_sections(text: &str, max_sections: usize) -> Vec<(String, String)> {
    let lines: Vec<&str> = text.lines().collect();
    let mut sections: Vec<(String, Vec<String>)> = Vec::new();
    let mut cur_title = "BODY".to_string();
    let mut cur_buf: Vec<String> = Vec::new();
    let md_heading_pat = Regex::new(r"^(#{1,6})\s+(.+)$").unwrap();

    for line in &lines {
        let stripped = line.trim();
        let md_heading_match = md_heading_pat.captures(stripped);
        if let Some(caps) = md_heading_match {
            if !cur_buf.is_empty() {
                sections.push((cur_title.clone(), cur_buf));
            }
            cur_title = caps
                .get(2)
                .map(|m| m.as_str().trim().to_string())
                .unwrap_or_default();
            cur_buf = Vec::new();
        } else if looks_like_heading(line) {
            if !cur_buf.is_empty() {
                sections.push((cur_title.clone(), cur_buf));
            }
            cur_title = stripped.to_string();
            cur_buf = Vec::new();
        } else {
            cur_buf.push(line.to_string());
        }
    }

    if !cur_buf.is_empty() {
        sections.push((cur_title, cur_buf));
    }

    let mut merged: Vec<(String, String)> = Vec::new();
    for (title, buf) in sections {
        let content = buf.join("\n").trim().to_string();
        if content.is_empty() {
            continue;
        }
        merged.push((title, content));
    }

    if merged.len() > max_sections {
        let truncated = merged[..max_sections].to_vec();
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
// Table Parsing Helpers
// ============================================================================

/// Parse markdown-like table text into structured TableData.
pub fn parse_markdown_table(table_text: &str, page: usize) -> TableData {
    let mut headers = Vec::new();
    let mut rows = Vec::new();
    let mut in_header = true;

    for line in table_text.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        // Skip markdown separator lines (|---|---)
        if line.starts_with('|') && line.chars().all(|c| c == '|' || c == '-' || c == ' ') {
            continue;
        }

        let cells: Vec<String> = line
            .split('|')
            .skip(1) // Skip leading |
            .map(|c| c.trim().to_string())
            .collect();

        // Remove trailing empty cells (caused by trailing pipe: | A | B | C |)
        let mut cells = cells;
        while cells.len() > 1 && cells.last().is_some_and(|s| s.is_empty()) {
            cells.pop();
        }

        if cells.is_empty() {
            continue;
        }

        if in_header {
            headers = cells;
            in_header = false;
        } else {
            rows.push(cells);
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
// Serialization Helpers
// ============================================================================

impl ParsedPaper {
    /// Convert to a JSON-serializable dictionary.
    pub fn to_cache_dict(&self) -> serde_json::Value {
        serde_json::to_value(self).unwrap_or_default()
    }

    /// Reconstruct from a JSON dictionary (e.g., from cache).
    #[allow(dead_code)]
    pub fn from_cache_dict(d: &serde_json::Value) -> Option<Self> {
        serde_json::from_value(d.clone()).ok()
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // Helpers for math detection — only needed by tests
    fn make_display_math_patterns() -> Vec<Regex> {
        vec![
            Regex::new(r"^\s*\$\$[\s\S]+?\$\$\s*$").unwrap(),
            Regex::new(r"^\s*\\[\s\S]+?\s*\\]\s*$").unwrap(),
            Regex::new(r"^\s*\\begin\{align\*?\}[\s\S]+?\\end\{align\*?\}\s*$").unwrap(),
            Regex::new(r"^\s*\\begin\{gather\*?\}[\s\S]+?\\end\{gather\*?\}\s*$").unwrap(),
            Regex::new(r"^\s*\\begin\{eqnarray\*?\}[\s\S]+?\\end\{eqnarray\*?\}\s*$").unwrap(),
            Regex::new(r"^\s*\\begin\{multline\*?\}[\s\S]+?\\end\{multline\*?\}\s*$").unwrap(),
        ]
    }

    fn make_inline_math_pat() -> Regex {
        Regex::new(r"\$([^\$\n]+?)\$|\\\([^)]+\\\)").unwrap()
    }

    fn is_display_math(line: &str) -> bool {
        let s = line.trim();
        for pat in make_display_math_patterns() {
            if pat.is_match(s) {
                return true;
            }
        }
        false
    }

    fn extract_inline_math(line: &str) -> Vec<MathBlock> {
        let pat = make_inline_math_pat();
        pat.captures_iter(line)
            .map(|m| MathBlock {
                text: m.get(0).map(|x| x.as_str().to_string()).unwrap_or_default(),
                is_display: false,
                page: 0,
            })
            .collect()
    }

    #[test]
    fn test_detect_block_type_heading() {
        assert_eq!(detect_block_type("# Introduction"), BlockType::Heading);
        assert_eq!(detect_block_type("1. Introduction"), BlockType::Heading);
        assert_eq!(detect_block_type("INTRODUCTION"), BlockType::Heading);
        assert_eq!(detect_block_type("III. BACKGROUND"), BlockType::Heading);
    }

    #[test]
    fn test_detect_block_type_caption() {
        assert_eq!(
            detect_block_type("Figure 1: Architecture overview"),
            BlockType::Caption
        );
        assert_eq!(detect_block_type("TABLE 2: Results"), BlockType::Caption);
        assert_eq!(
            detect_block_type("Fig. 5 Neural network"),
            BlockType::Caption
        );
    }

    #[test]
    fn test_detect_block_type_list() {
        assert_eq!(detect_block_type("- item 1"), BlockType::ListItem);
        assert_eq!(detect_block_type("1. first"), BlockType::ListItem);
        assert_eq!(detect_block_type("* bullet"), BlockType::ListItem);
    }

    #[test]
    fn test_detect_block_type_footnote() {
        assert_eq!(detect_block_type("[1]"), BlockType::Footnote);
        assert_eq!(detect_block_type("^42"), BlockType::Footnote);
    }

    #[test]
    fn test_parse_markdown_table() {
        let md = r"| A | B | C |
|---|---|---|
| 1 | 2 | 3 |
| 4 | 5 | 6 |";
        let table = parse_markdown_table(md, 0);
        assert_eq!(table.headers, vec!["A", "B", "C"]);
        assert_eq!(table.rows.len(), 2);
        assert_eq!(table.rows[0], vec!["1", "2", "3"]);
    }

    #[test]
    fn test_is_display_math() {
        assert!(is_display_math("$$ x^2 $$"));
        assert!(is_display_math("\\[ x + y \\]"));
        assert!(!is_display_math("$x^2$"));
        assert!(is_display_math(
            r"$$
a &= b + c
$$"
        ));
    }

    #[test]
    fn test_extract_inline_math() {
        let blocks = extract_inline_math("The equation $E = mc^2$ is famous.");
        assert_eq!(blocks.len(), 1);
        assert!(!blocks[0].is_display);
    }

    #[test]
    fn test_parsed_paper_serialization() {
        let paper = ParsedPaper {
            paper_id: "test-123".to_string(),
            text: "Hello world".to_string(),
            latex_blocks: vec![LaTeXBlock {
                source: "$x^2$".to_string(),
                is_display: false,
                page: 0,
                bbox: (0.0, 0.0, 0.0, 0.0),
            }],
            tables: vec![],
            figures: vec![],
            page_count: 1,
            word_count: 2,
            parse_version: 1,
            pdf_hash: "abc123".to_string(),
            title: "Test".to_string(),
            authors: vec!["Alice".to_string()],
            abstract_text: "Abstract".to_string(),
            published: "2024-01-01".to_string(),
            warnings: vec![],
            errors: vec![],
        };

        let dict = paper.to_cache_dict();
        let restored = ParsedPaper::from_cache_dict(&dict).unwrap();
        assert_eq!(restored.paper_id, "test-123");
        assert_eq!(restored.latex_blocks.len(), 1);
    }

    #[test]
    fn test_block_type_body() {
        assert_eq!(
            detect_block_type("This is a regular paragraph."),
            BlockType::Body
        );
        assert_eq!(detect_block_type(""), BlockType::Body);
        assert_eq!(detect_block_type("   "), BlockType::Body);
    }

    #[test]
    fn test_looks_like_heading() {
        assert!(looks_like_heading("INTRODUCTION"));
        assert!(looks_like_heading("1. Introduction"));
        assert!(looks_like_heading("1.2 Methods"));
        assert!(looks_like_heading("III. BACKGROUND"));
        assert!(looks_like_heading("Abstract"));
        assert!(looks_like_heading("Related Work"));
        assert!(looks_like_heading("CONCLUSION"));
        assert!(!looks_like_heading("This is a regular paragraph."));
        assert!(!looks_like_heading("a")); // too short
        assert!(!looks_like_heading(&"x".repeat(150))); // too long
    }

    #[test]
    fn test_segment_into_sections() {
        let text = "Introduction\n\nSome intro text.\n\n## Methods\n\nOur method consists of...";
        let sections = segment_into_sections(text, 18);
        assert!(sections.len() >= 2);
        let titles: Vec<_> = sections.iter().map(|(t, _)| t.as_str()).collect();
        assert!(titles.contains(&"Methods"));
    }

    #[test]
    fn test_segment_into_sections_truncation() {
        let mut lines = Vec::new();
        for i in 0..25 {
            lines.push(format!(
                "{}. Section Title {}\n\nContent for section {}.",
                i, i, i
            ));
        }
        let text = lines.join("\n\n");
        let sections = segment_into_sections(&text, 5);
        assert!(sections.len() <= 6);
        let last_title = sections.last().map(|(t, _)| t.as_str()).unwrap_or("");
        assert_eq!(last_title, "TRUNCATED");
    }

    #[test]
    fn test_text_blocks_to_lines() {
        let blocks = vec![
            TextBlock {
                block_type: BlockType::Heading,
                text: "Intro".to_string(),
                page: 1,
            },
            TextBlock {
                block_type: BlockType::Body,
                text: "Some text".to_string(),
                page: 1,
            },
        ];
        let lines = text_blocks_to_lines(&blocks);
        assert_eq!(lines, vec!["Intro", "Some text"]);
    }
}
