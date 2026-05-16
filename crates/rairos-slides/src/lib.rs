//! rairos-slides — Paper to Slides generator.
//!
//! Ported from `llm/slides.py` + `cli/cmd/slides.py`.
//! Rule-based template engine + MD/HTML output.
//! LLM enhancement available as future upgrade via rairos-llm::slides.

#![allow(clippy::print_literal)]

use rairos_core::Database;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

// ============================================================================
// Types
// ============================================================================

/// Template/style for slide generation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SlideTemplate {
    #[serde(rename = "academic")]
    Academic,
    #[serde(rename = "minimal")]
    Minimal,
    #[serde(rename = "modern")]
    Modern,
}

impl SlideTemplate {
    pub fn from_str(s: &str) -> Self {
        match s.trim().to_lowercase().as_str() {
            "minimal" => SlideTemplate::Minimal,
            "modern" => SlideTemplate::Modern,
            _ => SlideTemplate::Academic,
        }
    }
}

/// Output format for slides.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SlideFormat {
    #[serde(rename = "md")]
    Markdown,
    #[serde(rename = "html")]
    Html,
}

impl SlideFormat {
    pub fn from_str(s: &str) -> Self {
        match s.trim().to_lowercase().as_str() {
            "html" => SlideFormat::Html,
            _ => SlideFormat::Markdown,
        }
    }
}

/// Output language.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SlideLanguage {
    #[serde(rename = "zh")]
    Chinese,
    #[serde(rename = "en")]
    English,
    #[serde(rename = "bilingual")]
    Bilingual,
}

impl SlideLanguage {
    pub fn from_str(s: &str) -> Self {
        match s.trim().to_lowercase().as_str() {
            "en" => SlideLanguage::English,
            "bilingual" => SlideLanguage::Bilingual,
            _ => SlideLanguage::Chinese,
        }
    }
}

/// Configuration for slide generation.
#[derive(Debug, Clone)]
pub struct SlidesConfig {
    pub template: SlideTemplate,
    pub num_slides: usize,
    pub format: SlideFormat,
    pub output_path: Option<PathBuf>,
    pub include_notes: bool,
    pub language: SlideLanguage,
}

impl Default for SlidesConfig {
    fn default() -> Self {
        Self {
            template: SlideTemplate::Academic,
            num_slides: 10,
            format: SlideFormat::Markdown,
            output_path: None,
            include_notes: false,
            language: SlideLanguage::Chinese,
        }
    }
}

/// A single slide.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Slide {
    pub title: String,
    pub content: String,
    pub notes: String,
    pub slide_type: String,
}

/// Result of slide generation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SlidesResult {
    pub output_path: String,
    pub slide_count: usize,
    pub paper_count: usize,
    pub slides: Vec<Slide>,
}

// ============================================================================
// Slide generator
// ============================================================================

/// Generate slides from paper data.
pub struct PaperSlidesGenerator<'a> {
    pub db: Option<&'a Database>,
}

impl<'a> PaperSlidesGenerator<'a> {
    pub fn new(db: Option<&'a Database>) -> Self {
        Self { db }
    }

    /// Main entry: generate slides for one or more papers.
    pub fn generate(
        &self,
        paper_ids: &[String],
        config: &SlidesConfig,
    ) -> SlidesResult {
        let papers = self.fetch_papers(paper_ids);
        let slides = if papers.len() <= 1 {
            papers.first()
                .map(|p| self.single_paper_slides(p, config))
                .unwrap_or_else(|| vec![empty_slide()])
        } else {
            self.comparison_slides(&papers, config)
        };

        let output_path = self.write_output(&slides, config);
        let count = slides.len();

        SlidesResult {
            output_path: output_path.to_string_lossy().to_string(),
            slide_count: count,
            paper_count: papers.len(),
            slides,
        }
    }

    // ── Paper fetching ─────────────────────────────────────────

    /// Fetch paper data from the database.
    fn fetch_papers(&self, paper_ids: &[String]) -> Vec<PaperContent> {
        let db = match self.db {
            Some(d) => d,
            None => return Vec::new(),
        };

        let mut papers = Vec::new();
        for pid in paper_ids {
            if let Ok(paper) = db.get_paper(pid) {
                let text = paper.abstract_text.clone();
                let sections = detect_sections(&text);
                papers.push(PaperContent {
                    id: paper.id.clone(),
                    title: paper.title.clone(),
                    authors: paper.authors.join(", "),
                    abstract_text: text,
                    year: paper.published.format("%Y").to_string(),
                    sections,
                });
            }
        }
        papers
    }

    // ── Single paper slides ────────────────────────────────────

    fn single_paper_slides(&self, paper: &PaperContent, config: &SlidesConfig) -> Vec<Slide> {
        let mut slides: Vec<Slide> = Vec::new();

        // 1. Title slide
        let title_content = if config.language == SlideLanguage::English {
            format!("{}\n{}", paper.authors, paper.year)
        } else if config.language == SlideLanguage::Bilingual {
            format!("{}\n{} / {}", paper.authors, paper.year, paper.authors)
        } else {
            format!("{}\n{}", paper.authors, paper.year)
        };
        slides.push(Slide {
            title: paper.title.clone(),
            content: title_content,
            notes: "开场介绍论文标题和作者".to_string(),
            slide_type: "title".to_string(),
        });

        // 2. Motivation/Abstract
        let abstract_label = match config.language {
            SlideLanguage::English => "Motivation",
            SlideLanguage::Bilingual => "研究动机 / Motivation",
            _ => "研究动机",
        };
        slides.push(Slide {
            title: abstract_label.to_string(),
            content: truncate(&paper.abstract_text, 500),
            notes: "介绍研究背景和动机，强调问题的重要性".to_string(),
            slide_type: "content".to_string(),
        });

        // 3. Method sections (up to 2)
        let method_kws = ["method", "approach", "model", "architecture", "方法", "模型", "架构"];
        let method_sections: Vec<&DetectedSection> = paper.sections.iter()
            .filter(|s| method_kws.iter().any(|kw| s.heading.to_lowercase().contains(kw)))
            .collect();
        for sec in method_sections.iter().take(2) {
            let method_label = match config.language {
                SlideLanguage::English => "Method".to_string(),
                SlideLanguage::Bilingual => format!("方法 / Method: {}", sec.heading),
                _ => format!("方法: {}", sec.heading),
            };
            slides.push(Slide {
                title: if method_sections.len() == 1 { "方法".to_string() } else { method_label },
                content: truncate(&sec.body, 800),
                notes: format!("详细讲解{}部分的技术细节", sec.heading),
                slide_type: "content".to_string(),
            });
        }

        // 4. Experiment sections (up to 1)
        let exp_kws = ["experiment", "result", "evaluation", "实验", "结果", "评估"];
        let exp_sections: Vec<&DetectedSection> = paper.sections.iter()
            .filter(|s| exp_kws.iter().any(|kw| s.heading.to_lowercase().contains(kw)))
            .collect();
        if let Some(sec) = exp_sections.first() {
            slides.push(Slide {
                title: match config.language {
                    SlideLanguage::English => "Experimental Results".to_string(),
                    SlideLanguage::Bilingual => "实验结果 / Results".to_string(),
                    _ => "实验结果".to_string(),
                },
                content: truncate(&sec.body, 600),
                notes: "展示关键实验数据和方法对比".to_string(),
                slide_type: "content".to_string(),
            });
        }

        // 5. Conclusion sections
        let conc_kws = ["conclusion", "讨论", "结论", "总结"];
        let conc_sections: Vec<&DetectedSection> = paper.sections.iter()
            .filter(|s| conc_kws.iter().any(|kw| s.heading.to_lowercase().contains(kw)))
            .collect();
        if let Some(sec) = conc_sections.first() {
            slides.push(Slide {
                title: match config.language {
                    SlideLanguage::English => "Conclusion".to_string(),
                    SlideLanguage::Bilingual => "结论 / Conclusion".to_string(),
                    _ => "结论".to_string(),
                },
                content: truncate(&sec.body, 500),
                notes: "总结论文贡献和未来工作方向".to_string(),
                slide_type: "summary".to_string(),
            });
        }

        // 6. References / closing
        slides.push(Slide {
            title: match config.language {
                SlideLanguage::English => "References & Further Reading".to_string(),
                SlideLanguage::Bilingual => "参考与引用 / References".to_string(),
                _ => "参考与引用".to_string(),
            },
            content: format!("Paper ID: {}\nYear: {}", paper.id, paper.year),
            notes: "提供进一步阅读的建议".to_string(),
            slide_type: "content".to_string(),
        });

        slides.truncate(config.num_slides);
        slides
    }

    // ── Multi-paper comparison slides ──────────────────────────

    fn comparison_slides(&self, papers: &[PaperContent], config: &SlidesConfig) -> Vec<Slide> {
        let mut slides = Vec::new();

        // Title slide
        let titles: Vec<&str> = papers.iter().map(|p| p.title.as_str()).collect();
        let bullet_list = titles.iter().enumerate()
            .map(|(i, t)| format!("{}. {}", i + 1, t))
            .collect::<Vec<_>>()
            .join("\n");
        slides.push(Slide {
            title: match config.language {
                SlideLanguage::English => "Paper Comparison".to_string(),
                _ => "论文对比分析".to_string(),
            },
            content: bullet_list,
            notes: "介绍即将对比的论文".to_string(),
            slide_type: "title".to_string(),
        });

        // Comparison table slide
        slides.push(Slide {
            title: match config.language {
                SlideLanguage::English => "Overview Comparison".to_string(),
                _ => "论文概览对比".to_string(),
            },
            content: render_comparison_table(papers),
            notes: "展示各论文基本信息".to_string(),
            slide_type: "comparison".to_string(),
        });

        // Per-paper detail slides
        for paper in papers {
            slides.push(Slide {
                title: truncate(&paper.title, 50),
                content: format!("Year: {}\n\n{}", paper.year, truncate(&paper.abstract_text, 400)),
                notes: format!("介绍{}的核心内容", paper.title),
                slide_type: "content".to_string(),
            });
        }

        slides.truncate(config.num_slides);
        slides
    }

    // ── Output writers ─────────────────────────────────────────

    fn write_output(&self, slides: &[Slide], config: &SlidesConfig) -> PathBuf {
        let output_path = config.output_path.clone().unwrap_or_else(|| {
            let ext = match config.format {
                SlideFormat::Markdown => "md",
                SlideFormat::Html => "html",
            };
            let ts = chrono::Utc::now().format("%Y%m%d_%H%M%S");
            PathBuf::from(format!("slides_output/slides_{}.{}", ts, ext))
        });

        match config.format {
            SlideFormat::Markdown => write_markdown(slides, &output_path, config),
            SlideFormat::Html => write_html(slides, &output_path, config),
        }
    }
}

// ============================================================================
// Section Detection (keyword heuristic)
// ============================================================================

#[derive(Debug, Clone)]
struct DetectedSection {
    heading: String,
    body: String,
}

struct PaperContent {
    id: String,
    title: String,
    authors: String,
    abstract_text: String,
    year: String,
    sections: Vec<DetectedSection>,
}

/// Heuristic section detection from plain text.
fn detect_sections(text: &str) -> Vec<DetectedSection> {
    let section_markers = [
        "introduction", "background", "related work", "method", "approach",
        "model", "architecture", "framework", "algorithm", "experiment",
        "result", "evaluation", "discussion", "conclusion", "limitation",
        "future work", "contribution",
        "introduction", "background", "method", "experiment",
        "result", "discussion", "conclusion",
        "介绍", "背景", "方法", "模型", "算法", "架构",
        "实验", "结果", "讨论", "结论", "分析",
    ];

    let mut sections = Vec::new();
    let mut current_heading = String::new();
    let mut current_body = Vec::new();
    let mut in_section = false;

    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            if in_section {
                current_body.push(String::new());
            }
            continue;
        }

        // Check if this line looks like a section heading
        let looks_like_heading = {
            let trimmed_len = trimmed.len();
            if !(5..=60).contains(&trimmed_len) {
                false
            } else if trimmed.starts_with(|c: char| c.is_ascii_digit() || c == '.')
                && trimmed.contains(". ")
            {
                // Numbered heading: "1. Introduction", "2.1 Method"
                let suffix = trimmed.trim_start_matches(|c: char| c.is_ascii_digit() || c == '.' || c == ' ');
                let suffix_lower = suffix.to_lowercase();
                trimmed_len < 50 && section_markers.iter().any(|m| {
                    suffix_lower.find(m).map(|i| {
                        let post_ok = suffix_lower.as_bytes().get(i + m.len())
                            .is_none_or(|&b| !b.is_ascii_alphanumeric());
                        (i == 0 || " .-:(".contains(suffix_lower.as_bytes().get(i - 1).map(|&b| b as char).unwrap_or(' '))) && post_ok
                    }).unwrap_or(false)
                })
            } else {
                // Plain heading — must be very short (<25) and marker-proximate
                trimmed_len < 25 && section_markers.iter().any(|m| {
                    let lower = trimmed.to_lowercase();
                    lower.find(m).is_some_and(|_idx| {
                        // Marker must be >40% of line length
                        m.len() as f64 / trimmed_len as f64 > 0.4
                    })
                })
            }
        };

        if looks_like_heading {
            if !current_heading.is_empty() && current_body.iter().any(|l| !l.is_empty()) {
                sections.push(DetectedSection {
                    heading: current_heading,
                    body: current_body.join("\n").trim().to_string(),
                });
            }
            current_heading = trimmed.to_string();
            current_body.clear();
            in_section = true;
        } else if in_section
            && (!trimmed.starts_with(|c: char| c.is_uppercase()) || trimmed.len() > 3) {
                current_body.push(trimmed.to_string());
            }
    }

    // Last section
    if !current_heading.is_empty() && current_body.iter().any(|l| !l.is_empty()) {
        sections.push(DetectedSection {
            heading: current_heading,
            body: current_body.join("\n").trim().to_string(),
        });
    }

    sections
}

// ============================================================================
// Output Renderers
// ============================================================================

fn write_markdown(slides: &[Slide], output_path: &PathBuf, config: &SlidesConfig) -> PathBuf {
    let mut lines = Vec::new();
    for (i, slide) in slides.iter().enumerate() {
        lines.push(format!("# Slide {}: {}", i + 1, slide.title));
        lines.push(String::new());
        if slide.slide_type == "title" {
            lines.push(format!("## {}", slide.content));
        } else {
            lines.push(slide.content.clone());
        }
        lines.push(String::new());
        if config.include_notes && !slide.notes.is_empty() {
            lines.push(format!("**演讲备注**: {}", slide.notes));
            lines.push(String::new());
        }
        lines.push("---".to_string());
        lines.push(String::new());
    }
    ensure_parent(output_path);
    std::fs::write(output_path, lines.join("\n")).ok();
    output_path.clone()
}

fn write_html(slides: &[Slide], output_path: &PathBuf, config: &SlidesConfig) -> PathBuf {
    let mut slide_htmls = Vec::new();
    for (i, slide) in slides.iter().enumerate() {
        let notes_html = if config.include_notes && !slide.notes.is_empty() {
            format!("<div class=\"notes\">{}</div>", slide.notes)
        } else {
            String::new()
        };
        slide_htmls.push(format!(
            r#"<div class="slide" id="slide-{}">
    <h1>{}</h1>
    <div class="content">
        <pre>{}</pre>
    </div>
    {}
</div>"#,
            i + 1, slide.title, slide.content, notes_html
        ));
    }

    let html = format!(
        r#"<!DOCTYPE html>
<html>
<head>
    <meta charset="utf-8">
    <title>Paper Slides</title>
    <style>
        body {{ font-family: Arial, sans-serif; margin: 0; padding: 20px; background: #f5f5f5; }}
        .slide {{ page-break-after: always; margin-bottom: 40px; border: 1px solid #ddd; padding: 20px; background: white; border-radius: 8px; box-shadow: 0 2px 8px rgba(0,0,0,0.1); }}
        h1 {{ color: #333; border-bottom: 2px solid #0066cc; padding-bottom: 10px; }}
        .content pre {{ white-space: pre-wrap; font-family: inherit; font-size: 16px; line-height: 1.6; }}
        .notes {{ background: #fffde7; padding: 10px; margin-top: 20px; font-style: italic; border-left: 4px solid #fdd835; }}
        @media print {{ .slide {{ box-shadow: none; border: 1px solid #ccc; }} }}
    </style>
</head>
<body>
{}
</body>
</html>"#,
        slide_htmls.join("\n")
    );

    ensure_parent(output_path);
    std::fs::write(output_path, html).ok();
    output_path.clone()
}

fn ensure_parent(path: &PathBuf) {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).ok();
    }
}

// ============================================================================
// Helpers
// ============================================================================

fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        let mut truncated = s[..max].to_string();
        if let Some(last_space) = truncated.rfind(' ') {
            truncated.truncate(last_space);
        }
        truncated.push_str("...");
        truncated
    }
}

fn empty_slide() -> Slide {
    Slide {
        title: "No Content".to_string(),
        content: "No papers found".to_string(),
        notes: String::new(),
        slide_type: "content".to_string(),
    }
}

/// Render a comparison table as Markdown text.
fn render_comparison_table(papers: &[PaperContent]) -> String {
    let headers = ["Paper", "Year", "Authors"];
    let rows: Vec<Vec<String>> = papers.iter().map(|p| {
        vec![
            truncate(&p.title, 30),
            p.year.clone(),
            truncate(&p.authors, 25),
        ]
    }).collect();

    let col_widths: Vec<usize> = (0..headers.len()).map(|i| {
        let max_data = rows.iter()
            .map(|r| r[i].len())
            .max()
            .unwrap_or(0);
        std::cmp::max(headers[i].len(), max_data) + 2
    }).collect();

    let mut lines = Vec::new();
    let header_line: Vec<String> = headers.iter().enumerate()
        .map(|(i, h)| pad_right(h, col_widths[i]))
        .collect();
    lines.push(header_line.join(" | "));
    lines.push(format!("|{}|", col_widths.iter().map(|w| "-".repeat(*w)).collect::<Vec<_>>().join("|")));
    for row in &rows {
        let cells: Vec<String> = row.iter().enumerate()
            .map(|(i, c)| pad_right(c, col_widths[i]))
            .collect();
        lines.push(cells.join(" | "));
    }
    lines.join("\n")
}

fn pad_right(s: &str, width: usize) -> String {
    if s.len() >= width {
        s[..width].to_string()
    } else {
        let mut result = s.to_string();
        result.push_str(&" ".repeat(width - s.len()));
        result
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_slide_template_from_str() {
        assert!(matches!(SlideTemplate::from_str("academic"), SlideTemplate::Academic));
        assert!(matches!(SlideTemplate::from_str("minimal"), SlideTemplate::Minimal));
        assert!(matches!(SlideTemplate::from_str("modern"), SlideTemplate::Modern));
        assert!(matches!(SlideTemplate::from_str("unknown"), SlideTemplate::Academic));
    }

    #[test]
    fn test_slide_format_from_str() {
        assert!(matches!(SlideFormat::from_str("md"), SlideFormat::Markdown));
        assert!(matches!(SlideFormat::from_str("html"), SlideFormat::Html));
        assert!(matches!(SlideFormat::from_str("unknown"), SlideFormat::Markdown));
    }

    #[test]
    fn test_slide_language_from_str() {
        assert!(matches!(SlideLanguage::from_str("zh"), SlideLanguage::Chinese));
        assert!(matches!(SlideLanguage::from_str("en"), SlideLanguage::English));
        assert!(matches!(SlideLanguage::from_str("bilingual"), SlideLanguage::Bilingual));
    }

    #[test]
    fn test_detect_sections_empty() {
        let sections = detect_sections("");
        assert!(sections.is_empty());
    }

    #[test]
    fn test_detect_sections_basic() {
        let text = "1. Introduction\nThis is the intro.\n\n2. Method\nOur method is great.\n\n3. Results\nWe got great results.";
        let sections = detect_sections(text);
        assert!(sections.len() >= 2);
        assert!(sections.iter().any(|s| s.heading.contains("Method")));
    }

    #[test]
    fn test_detect_sections_intro() {
        let text = "Introduction\nThis paper introduces a new method.\n\nMethod\nWe propose X.\n\nExperiment\nTests confirm.";
        let sections = detect_sections(text);
        assert!(!sections.is_empty());
        assert!(sections.iter().any(|s| s.heading.to_lowercase().contains("method")));
    }

    #[test]
    fn test_truncate_short() {
        assert_eq!(truncate("hello", 10), "hello");
    }

    #[test]
    fn test_truncate_long() {
        let long = "a".repeat(100);
        let result = truncate(&long, 50);
        assert!(result.len() <= 53); // 50 + "..."
    }

    #[test]
    fn test_empty_slide() {
        let s = empty_slide();
        assert_eq!(s.title, "No Content");
    }

    #[test]
    fn test_single_paper_no_db() {
        let gen = PaperSlidesGenerator::new(None);
        let config = SlidesConfig::default();
        let result = gen.generate(&["test123".to_string()], &config);
        // No DB, so no paper found → empty slides + no-content slide
        assert_eq!(result.paper_count, 0);
        assert!(result.slides.is_empty() || result.slides[0].title == "No Content");
    }

    #[test]
    fn test_comparison_table() {
        let papers = vec![
            PaperContent {
                id: "1".to_string(),
                title: "Paper A".to_string(),
                authors: "Alice".to_string(),
                abstract_text: "Abstract A".to_string(),
                year: "2023".to_string(),
                sections: vec![],
            },
            PaperContent {
                id: "2".to_string(),
                title: "Paper B".to_string(),
                authors: "Bob".to_string(),
                abstract_text: "Abstract B".to_string(),
                year: "2024".to_string(),
                sections: vec![],
            },
        ];
        let table = render_comparison_table(&papers);
        assert!(table.contains("Paper A"));
        assert!(table.contains("Paper B"));
        assert!(table.contains("2023"));
        assert!(table.contains("2024"));
    }

    #[test]
    fn test_markdown_output() {
        let slides = vec![
            Slide {
                title: "Test Title".to_string(),
                content: "Test content".to_string(),
                notes: "Test notes".to_string(),
                slide_type: "title".to_string(),
            },
        ];
        let config = SlidesConfig {
            include_notes: true,
            ..SlidesConfig::default()
        };
        let path = PathBuf::from("/tmp/test_slides_out.md");
        let result = write_markdown(&slides, &path, &config);
        assert!(result.exists());
        let content = std::fs::read_to_string(&result).unwrap();
        assert!(content.contains("Test Title"));
        assert!(content.contains("Slide 1"));
        assert!(content.contains("Test notes"));
        std::fs::remove_file(&result).ok();
    }

    #[test]
    fn test_html_output() {
        let slides = vec![
            Slide {
                title: "HTML Test".to_string(),
                content: "HTML content".to_string(),
                notes: String::new(),
                slide_type: "content".to_string(),
            },
        ];
        let config = SlidesConfig {
            format: SlideFormat::Html,
            ..SlidesConfig::default()
        };
        let path = PathBuf::from("/tmp/test_slides_out.html");
        let result = write_html(&slides, &path, &config);
        assert!(result.exists());
        let content = std::fs::read_to_string(&result).unwrap();
        assert!(content.contains("HTML Test"));
        assert!(content.contains("<html>"));
        std::fs::remove_file(&result).ok();
    }

    #[test]
    fn test_comparison_slides_structure() {
        let papers = vec![
            PaperContent {
                id: "1".to_string(), title: "Paper X".to_string(), authors: "Alice".to_string(),
                abstract_text: "Abstract of X".to_string(), year: "2023".to_string(), sections: vec![],
            },
            PaperContent {
                id: "2".to_string(), title: "Paper Y".to_string(), authors: "Bob".to_string(),
                abstract_text: "Abstract of Y".to_string(), year: "2024".to_string(), sections: vec![],
            },
        ];
        let gen = PaperSlidesGenerator::new(None);
        let slides = gen.comparison_slides(&papers, &SlidesConfig::default());
        assert!(slides.len() >= 3); // title + table + per-paper
        assert_eq!(slides[0].slide_type, "title");
        assert!(slides[0].content.contains("Paper X"));
    }

    #[test]
    fn test_pad_right() {
        let result = pad_right("hello", 10);
        assert_eq!(result.len(), 10);
        assert!(result.starts_with("hello"));
    }

    #[test]
    fn test_detect_sections_markers() {
        // With Chinese markers
        let text = "介绍\n背景介绍\n\n方法\n我们提出新方法\n\n实验\n结果很好";
        let sections = detect_sections(text);
        assert!(sections.iter().any(|s| s.heading.contains("方法")));
    }
}
