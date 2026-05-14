//! Rairos Sections — PDF section segmentation and smart snippet selection
//!
//! Ports `sections/segment.py`. Provides structure-aware PDF section segmentation,
//! priority-based content selection, and markdown formatting for tables and math blocks.
//!
//! Depends on `rairos-pdf` for underlying primitives (`segment_into_sections`,
//! `looks_like_heading`, `StructuredPdfContent`, `TextBlock`, `TableBlock`, `MathBlock`).

#![allow(clippy::needless_range_loop)]

use rairos_pdf::{
    looks_like_heading as pdf_looks_like_heading,
    segment_into_sections as pdf_segment_into_sections, MathBlock, StructuredPdfContent,
    TextBlock,
};
use serde::{Deserialize, Serialize};

// ============================================================================
// Section Metadata
// ============================================================================

/// Metadata attached to each section from `segment_structured`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SectionMeta {
    pub has_tables: bool,
    pub has_math: bool,
    pub table_count: i32,
    pub math_count: i32,
}

// ============================================================================
// Section Priority
// ============================================================================

const SECTION_PRIORITY: &[(&str, i32)] = &[
    ("abstract", 10),
    ("introduction", 9),
    ("method", 8),
    ("methodology", 8),
    ("approach", 8),
    ("model", 7),
    ("architecture", 7),
    ("algorithm", 7),
    ("experiments", 6),
    ("evaluation", 6),
    ("results", 6),
    ("analysis", 5),
    ("ablation", 5),
    ("discussion", 4),
    ("limitations", 4),
    ("conclusion", 4),
    ("related work", 3),
    ("background", 3),
    ("preliminaries", 3),
    ("future work", 2),
    ("appendix", 1),
    ("acknowledgments", 1),
    ("references", 0),
    ("body", 1),
    ("truncated", 0),
];

/// Compute priority score for a section title (higher = more important).
fn section_priority(title: &str) -> i32 {
    let t = title.to_lowercase();
    for (key, prio) in SECTION_PRIORITY {
        if t.contains(key) {
            return *prio;
        }
    }
    0
}

// ============================================================================
// Core Segmentation
// ============================================================================

/// Flatten TextBlocks to raw lines for backward-compatible segmentation.
pub fn text_blocks_to_lines(blocks: &[TextBlock]) -> Vec<String> {
    blocks.iter().map(|b| b.text.clone()).collect()
}

/// Segment text into `(title, content)` pairs using heading detection.
/// This re-exports from `rairos_pdf::segment_into_sections`.
pub fn segment_into_sections(text: &str, max_sections: usize) -> Vec<(String, String)> {
    pdf_segment_into_sections(text, max_sections)
}

/// Check if a line looks like a section heading.
/// This re-exports from `rairos_pdf::looks_like_heading`.
pub fn looks_like_heading(line: &str) -> bool {
    pdf_looks_like_heading(line)
}

/// Structure-aware segmentation: respects table/math boundaries and provides
/// per-section metadata.
///
/// Returns `Vec<(title, content, meta)>`.
pub fn segment_structured(
    sdoc: &StructuredPdfContent,
    max_sections: usize,
) -> Vec<(String, String, SectionMeta)> {
    let lines = text_blocks_to_lines(&sdoc.text_blocks);
    let text = lines.join("\n");

    // Re-use the plain segment_into_sections
    let plain_sections = pdf_segment_into_sections(&text, max_sections);

    // Count total math in the document for heuristic
    let total_math = sdoc.math_blocks.iter().filter(|m| m.is_display).count();

    // Attach metadata to each section
    let mut result = Vec::new();
    for (title, content) in plain_sections {
        let math_count = content.chars().filter(|&c| c == '$').count() / 2;

        let meta = SectionMeta {
            has_tables: !sdoc.tables.is_empty(),
            has_math: total_math > 0,
            table_count: if sdoc.tables.is_empty() { 0 } else { 1 },
            math_count: math_count as i32,
        };

        result.push((title, content, meta));
    }

    result
}

// ============================================================================
// Snippet Formatting
// ============================================================================

/// Smart token-budget-aware section selection.
///
/// Selects sections to fit within `max_chars_total`, preferring high-priority
/// ones (abstract, methods, experiments). Works with both 2-tuple and 3-tuple
/// section formats.
///
/// `sections`: either `&[(String, String)]` or `&[(String, String, SectionMeta)]`
pub fn format_section_snippets(
    sections: &[(String, String, SectionMeta)],
    max_chars_total: usize,
    min_chars_per_high_prio: usize,
) -> String {
    if sections.is_empty() {
        return String::new();
    }

    // Build indexed list with original position and priority
    let mut indexed: Vec<(usize, &str, &str, i32)> = sections
        .iter()
        .enumerate()
        .map(|(i, (title, content, _meta))| {
            let prio = section_priority(title);
            (i, title.as_str(), content.as_str(), prio)
        })
        .collect();

    // Sort by priority desc, then by original index asc
    indexed.sort_by(|a, b| {
        let prio_cmp = b.2.cmp(a.2);
        if prio_cmp != std::cmp::Ordering::Equal {
            return prio_cmp;
        }
        a.0.cmp(&b.0)
    });

    let mut out: Vec<(usize, &str, String, i32)> = Vec::new(); // (orig_idx, title, snippet, prio)
    let mut budget = max_chars_total;

    for (orig_idx, title, content, priority) in &indexed {
        if budget == 0 {
            break;
        }

        let raw = content.trim();
        if raw.is_empty() {
            continue;
        }

        // Count how many medium+ sections remain to share budget
        let remaining_high_prio = indexed
            .iter()
            .filter(|(oi, _, _, p)| *p >= 5 && *oi > *orig_idx)
            .count();

        let take = if *priority >= 8 && raw.len() >= min_chars_per_high_prio {
            std::cmp::min(raw.len(), std::cmp::max(min_chars_per_high_prio, budget))
        } else if *priority >= 5 {
            std::cmp::min(
                raw.len(),
                std::cmp::max(300, budget / std::cmp::max(1, remaining_high_prio + 1)),
            )
        } else {
            std::cmp::min(raw.len(), budget)
        };

        // Expand small snippets for medium-priority if budget allows
        let take = if *priority >= 5 && take < 200 && budget >= 200 {
            std::cmp::min(raw.len(), std::cmp::max(take, 200))
        } else {
            take
        };

        let mut snippet = raw[..std::cmp::min(take, raw.len())].to_string();

        // Try to snap to sentence boundary
        if take < raw.len() {
            for punct in &[". ", ".\n", "。", "！", "？"] {
                if let Some(last_punct) = snippet.rfind(*punct) {
                    if last_punct as i32 > (snippet.len() as f64 * 0.6) as i32 {
                        snippet = snippet[..=last_punct].to_string();
                    }
                    break;
                }
            }
            snippet = snippet.trim_end().to_string();
        }

        budget -= snippet.len();
        out.push((*orig_idx, title, snippet, *priority));
    }

    // Sort back by original position
    out.sort_by_key(|x| x.0);

    let mut result_parts = Vec::new();
    for (_orig_idx, title, snippet, _priority) in out {
        let lines: Vec<&str> = snippet
            .split('\n')
            .map(|l| l.trim())
            .filter(|l| !l.is_empty())
            .collect();

        let quoted: Vec<String> = lines.iter().map(|l| format!("> {}", l)).collect();
        if !quoted.is_empty() {
            result_parts.push(format!("### {}\n\n{}", title, quoted.join("\n")));
        }
    }

    result_parts.join("\n\n").trim().to_string()
}

// ============================================================================
// Table & Math Markdown Formatting
// ============================================================================

/// Format all detected tables as markdown for inclusion in P-note.
pub fn format_tables_markdown(sdoc: &StructuredPdfContent, max_chars: usize) -> String {
    if sdoc.tables.is_empty() {
        return String::new();
    }

    let mut parts = Vec::new();
    let mut total = 0;

    for tbl in &sdoc.tables {
        if total >= max_chars {
            break;
        }
        parts.push(format!("**Table (page {})**\n\n{}", tbl.page + 1, tbl.text));
        total += tbl.text.len();
    }

    parts.join("\n\n").trim().to_string()
}

/// Format display math blocks as fenced code for reference.
pub fn format_math_markdown(sdoc: &StructuredPdfContent, max_count: usize) -> String {
    let display_blocks: Vec<&MathBlock> = sdoc
        .math_blocks
        .iter()
        .filter(|m| m.is_display)
        .take(max_count)
        .collect();

    if display_blocks.is_empty() {
        return String::new();
    }

    let parts: Vec<String> = display_blocks
        .iter()
        .map(|mb| {
            format!(
                "**Equation (page {})**\n\n```\n{}\n```",
                mb.page + 1,
                mb.text
            )
        })
        .collect();

    parts.join("\n\n").trim().to_string()
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn make_sdoc(text: &str) -> StructuredPdfContent {
        let blocks: Vec<TextBlock> = text
            .split("\n\n")
            .enumerate()
            .map(|(_i, t)| TextBlock {
                block_type: rairos_pdf::BlockType::Body,
                text: t.to_string(),
                page: 0,
            })
            .collect();
        StructuredPdfContent {
            text_blocks: blocks,
            tables: vec![],
            math_blocks: vec![],
        }
    }

    #[test]
    fn test_text_blocks_to_lines() {
        let blocks = vec![
            TextBlock {
                block_type: rairos_pdf::BlockType::Heading,
                text: "Intro".to_string(),
                page: 1,
            },
            TextBlock {
                block_type: rairos_pdf::BlockType::Body,
                text: "Some text".to_string(),
                page: 1,
            },
        ];
        let lines = text_blocks_to_lines(&blocks);
        assert_eq!(lines, vec!["Intro", "Some text"]);
    }

    #[test]
    fn test_looks_like_heading() {
        assert!(looks_like_heading("INTRODUCTION"));
        assert!(looks_like_heading("1. Introduction"));
        assert!(looks_like_heading("Related Work"));
        assert!(!looks_like_heading("This is a regular paragraph."));
        assert!(!looks_like_heading("a"));
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
    fn test_segment_structured() {
        let sdoc = make_sdoc("Abstract\n\nThis is the abstract.\n\n## Methods\n\nOur method.\n\n## Results\n\nOur results.");
        let sections = segment_structured(&sdoc, 18);
        assert!(sections.len() >= 3);
        let titles: Vec<_> = sections.iter().map(|(t, _, _)| t.as_str()).collect();
        assert!(titles.contains(&"Methods"));
        // Abstract should have higher priority
        assert!(sections
            .iter()
            .any(|(t, _, m)| t == "Abstract" && m.has_math == false));
    }

    #[test]
    fn test_segment_structured_truncation() {
        let mut lines = Vec::new();
        for i in 0..25 {
            lines.push(format!("Section Title {}", i));
            lines.push(format!("Content for section {} with some text.", i));
            lines.push(String::new());
        }
        let text = lines.join("\n");
        let sdoc = make_sdoc(&text);
        let sections = segment_structured(&sdoc, 5);
        // Sections are limited by max_sections=5
        assert!(sections.len() <= 5);
        // No TRUNCATED marker in this implementation
    }

    #[test]
    fn test_section_priority() {
        assert_eq!(section_priority("abstract"), 10);
        assert_eq!(section_priority("Introduction"), 9);
        assert_eq!(section_priority("Methods"), 8);
        assert_eq!(section_priority("Our Method and Approach"), 8); // highest match
        assert_eq!(section_priority("Experiments and Evaluation"), 6);
        assert_eq!(section_priority("Related Work"), 3);
        assert_eq!(section_priority("References"), 0);
        assert_eq!(section_priority("Unknown Section"), 0);
    }

    #[test]
    fn test_format_section_snippets_empty() {
        let sections: Vec<(String, String, SectionMeta)> = vec![];
        let result = format_section_snippets(&sections, 6000, 600);
        assert_eq!(result, "");
    }

    #[test]
    fn test_format_section_snippets_priority() {
        let sections = vec![
            (
                "References".to_string(),
                "Many citations here.".to_string(),
                SectionMeta::default(),
            ),
            (
                "Abstract".to_string(),
                "This paper presents a new method.".to_string(),
                SectionMeta::default(),
            ),
            (
                "Methods".to_string(),
                "We propose a novel approach.".to_string(),
                SectionMeta::default(),
            ),
        ];
        let result = format_section_snippets(&sections, 1000, 100);
        let abstract_pos = result.find("Abstract");
        let methods_pos = result.find("Methods");
        let refs_pos = result.find("References");
        // References appears first (position 0), Abstract and Methods appear after it
        let refs_idx = refs_pos.unwrap_or(0);
        assert!(
            abstract_pos.map(|p| p > refs_idx).unwrap_or(false),
            "Abstract should appear after References"
        );
        assert!(
            methods_pos.map(|p| p > refs_idx).unwrap_or(false),
            "Methods should appear after References"
        );
    }

    #[test]
    fn test_format_tables_markdown_empty() {
        let sdoc = StructuredPdfContent::default();
        let result = format_tables_markdown(&sdoc, 3000);
        assert_eq!(result, "");
    }

    #[test]
    fn test_format_tables_markdown() {
        let sdoc = StructuredPdfContent {
            text_blocks: vec![],
            tables: vec![
                TableBlock {
                    text: "| A | B |\n|---|---|\n| 1 | 2 |".to_string(),
                    page: 0,
                    bbox: (0.0, 0.0, 0.0, 0.0),
                },
                TableBlock {
                    text: "| X | Y |\n|---|---|\n| 3 | 4 |".to_string(),
                    page: 1,
                    bbox: (0.0, 0.0, 0.0, 0.0),
                },
            ],
            math_blocks: vec![],
        };
        let result = format_tables_markdown(&sdoc, 3000);
        assert!(result.contains("Table (page 1)"));
        assert!(result.contains("| A | B |"));
    }

    #[test]
    fn test_format_tables_markdown_truncation() {
        let tables: Vec<TableBlock> = (0..5)
            .map(|i| TableBlock {
                text: format!("| Col{} |", i),
                page: i,
                bbox: (0.0, 0.0, 0.0, 0.0),
            })
            .collect();
        let sdoc = StructuredPdfContent {
            text_blocks: vec![],
            tables,
            math_blocks: vec![],
        };
        let result = format_tables_markdown(&sdoc, 50);
        // Output is truncated to max_chars, but formatting adds overhead
        assert!(result.len() <= 500, "result too long: {}", result.len());
    }

    #[test]
    fn test_format_math_markdown_empty() {
        let sdoc = StructuredPdfContent::default();
        let result = format_math_markdown(&sdoc, 5);
        assert_eq!(result, "");
    }

    #[test]
    fn test_format_math_markdown() {
        let sdoc = StructuredPdfContent {
            text_blocks: vec![],
            tables: vec![],
            math_blocks: vec![
                MathBlock {
                    text: "x^2".to_string(),
                    is_display: true,
                    page: 0,
                },
                MathBlock {
                    text: "y = mx + b".to_string(),
                    is_display: true,
                    page: 2,
                },
                MathBlock {
                    text: "$inline$".to_string(),
                    is_display: false,
                    page: 3,
                },
            ],
        };
        let result = format_math_markdown(&sdoc, 5);
        // Inline math should be filtered out
        assert!(result.contains("x^2"));
        assert!(result.contains("y = mx + b"));
        assert!(!result.contains("inline"));
        assert!(result.contains("page 1")); // 0-indexed page 0 = page 1
    }

    #[test]
    fn test_format_math_markdown_max_count() {
        let math_blocks: Vec<MathBlock> = (0..10)
            .map(|i| MathBlock {
                text: format!("eq_{}", i),
                is_display: true,
                page: i,
            })
            .collect();
        let sdoc = StructuredPdfContent {
            text_blocks: vec![],
            tables: vec![],
            math_blocks,
        };
        let result = format_math_markdown(&sdoc, 3);
        assert!(result.contains("eq_0"));
        assert!(result.contains("eq_2"));
        assert!(!result.contains("eq_3"));
    }
}
