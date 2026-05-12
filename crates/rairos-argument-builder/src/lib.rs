//! rairos-argument-builder — Research Argument Builder
//!
//! Builds structured arguments from research evidence.
//! Ports `llm/argument_builder.py`.
//!
//! Core functionality:
//! 1. Claim management: core thesis + supporting/contradicting evidence
//! 2. Evidence collection: paper evidence + citation relationships
//! 3. Argument generation: structured arguments + section guidance
//! 4. Evidence categorization and weight-based sorting

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use thiserror::Error;

// ============================================================================
// Errors
// ============================================================================

#[derive(Error, Debug)]
pub enum ArgumentBuilderError {
    #[error("Evidence not found: {0}")]
    EvidenceNotFound(String),
    #[error("Invalid input: {0}")]
    InvalidInput(String),
}

pub type Result<T> = std::result::Result<T, ArgumentBuilderError>;

// ============================================================================
// Enums
// ============================================================================

/// Type of evidence for or against a claim.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum EvidenceType {
    Support,       // 支持证据
    Contradict,    // 反驳证据
    Qualify,       // 限定条件
    Methodological, // 方法论问题
}

impl Default for EvidenceType {
    fn default() -> Self {
        EvidenceType::Support
    }
}

/// Standard argument sections in a research paper.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ArgumentSection {
    Introduction,
    RelatedWork,
    Methodology,
    Experiments,
    Discussion,
    Limitation,
}

impl ArgumentSection {
    /// Human-readable name (Chinese).
    pub fn label_zh(&self) -> &'static str {
        match self {
            ArgumentSection::Introduction => "引言",
            ArgumentSection::RelatedWork => "相关工作",
            ArgumentSection::Methodology => "方法论",
            ArgumentSection::Experiments => "实验",
            ArgumentSection::Discussion => "讨论",
            ArgumentSection::Limitation => "局限",
        }
    }
}

impl std::fmt::Display for ArgumentSection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

// ============================================================================
// Core Data Structures
// ============================================================================

/// A piece of evidence for or against a claim.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[derive(Default)]
pub struct Evidence {
    /// Type of evidence.
    pub evidence_type: EvidenceType,
    /// Source identifier (paper title, insight ID, etc.).
    pub source: String,
    /// The actual evidence content.
    pub content: String,
    /// Citation information.
    #[serde(default)]
    pub citation: String,
    /// Evidence strength (0-1), default 1.0.
    #[serde(default = "default_evidence_weight")]
    pub weight: f64,
}

fn default_evidence_weight() -> f64 {
    1.0
}

/// A single claim in an argument.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claim {
    /// The claim text.
    pub text: String,
    /// Evidence supporting or contradicting this claim.
    #[serde(default)]
    pub evidence: Vec<Evidence>,
    /// Confidence level (0-1), default 0.5.
    #[serde(default = "default_confidence")]
    pub confidence: f64,
}

fn default_confidence() -> f64 {
    0.5
}

/// A structured research argument.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Argument {
    /// Core thesis statement.
    pub thesis: String,
    /// Individual claims within the argument.
    #[serde(default)]
    pub claims: Vec<Claim>,
    /// All supporting evidence.
    #[serde(default)]
    pub supporting_evidence: Vec<Evidence>,
    /// All contradicting evidence.
    #[serde(default)]
    pub contradicting_evidence: Vec<Evidence>,
    /// Related research gaps.
    #[serde(default)]
    pub related_gaps: Vec<String>,
    /// Suggested paper sections to include.
    #[serde(default)]
    pub paper_suggestions: Vec<ArgumentSection>,
}

/// Complete argument building result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArgumentResult {
    /// Research topic/thesis.
    pub topic: String,
    /// The constructed argument.
    pub argument: Argument,
    /// Human-readable summary.
    #[serde(default)]
    pub summary: String,
    /// Section-specific guidance.
    #[serde(default)]
    pub section_guidance: HashMap<ArgumentSection, String>,
}

// ============================================================================
// Contradiction Keywords (English + Chinese)
// ============================================================================

const CONTRADICT_KEYWORDS: &[&str] = &[
    // Chinese
    "局限",
    "问题",
    "失败",
    "缺陷",
    // English
    "limitation",
    "problem",
    "fail",
    "issue",
    "drawback",
    "weakness",
    "缺点",
    "不足",
];

// ============================================================================
// Template Guidance (Chinese)
// ============================================================================

fn template_guidance(
    _supporting_count: usize,
    contradicting_count: usize,
) -> HashMap<ArgumentSection, String> {
    let mut guidance = HashMap::new();

    guidance.insert(
        ArgumentSection::Introduction,
        "开篇应明确研究动机：为什么这个问题重要？引用主要支持证据说明该方向的潜力。".to_string(),
    );

    guidance.insert(
        ArgumentSection::RelatedWork,
        format!(
            "综述现有工作，区分本文与前人贡献。识别 {} 个需要回应的质疑。",
            contradicting_count
        ),
    );

    guidance.insert(
        ArgumentSection::Methodology,
        "方法论需针对反驳证据设计消融实验。说明如何衡量论点成立的条件边界。".to_string(),
    );

    guidance.insert(
        ArgumentSection::Discussion,
        "承认局限性（尤其是反驳证据指出的）。解释为什么在特定条件下论点仍然成立。".to_string(),
    );

    guidance.insert(
        ArgumentSection::Limitation,
        format!(
            "诚实讨论适用边界：基于 {} 条反驳证据，明确指出哪些场景下论点可能不成立。",
            contradicting_count
        ),
    );

    guidance
}

// ============================================================================
// Argument Builder
// ============================================================================

pub struct ArgumentBuilder;

impl ArgumentBuilder {
    pub fn new() -> Self {
        Self
    }

    /// Build an argument from a thesis.
    pub fn build(&self, thesis: &str) -> ArgumentResult {
        let argument = Argument {
            thesis: thesis.to_string(),
            claims: Vec::new(),
            supporting_evidence: Vec::new(),
            contradicting_evidence: Vec::new(),
            related_gaps: Vec::new(),
            paper_suggestions: Vec::new(),
        };

        ArgumentResult {
            topic: thesis.to_string(),
            argument,
            summary: String::new(),
            section_guidance: HashMap::new(),
        }
    }

    /// Build from pre-collected evidence.
    ///
    /// Takes a thesis and pre-classified supporting/contradicting evidence lists.
    /// Generates section guidance and summary automatically.
    pub fn build_with_evidence(
        &self,
        thesis: &str,
        supporting: Vec<Evidence>,
        contradicting: Vec<Evidence>,
    ) -> ArgumentResult {
        // Categorize by evidence type
        let (supporting_evidence, contradicting_evidence) =
            Self::categorize_evidence(supporting, contradicting);

        // Generate section guidance
        let section_guidance = template_guidance(
            supporting_evidence.len(),
            contradicting_evidence.len(),
        );

        // Generate summary
        let summary = Self::summarize(
            thesis,
            supporting_evidence.len(),
            contradicting_evidence.len(),
            0, // related_gaps.len() — passed as 0 since caller didn't provide
        );

        // Suggest sections
        let paper_suggestions = Self::suggest_sections(contradicting_evidence.len() > 0);

        let argument = Argument {
            thesis: thesis.to_string(),
            claims: Vec::new(),
            supporting_evidence,
            contradicting_evidence,
            related_gaps: Vec::new(),
            paper_suggestions,
        };

        ArgumentResult {
            topic: thesis.to_string(),
            argument,
            summary,
            section_guidance,
        }
    }

    /// Categorize evidence into supporting and contradicting based on evidence type.
    pub fn categorize_evidence(
        mut supporting: Vec<Evidence>,
        mut contradicting: Vec<Evidence>,
    ) -> (Vec<Evidence>, Vec<Evidence>) {
        // Sort both by weight descending
        supporting.sort_by(|a, b| b.weight.partial_cmp(&a.weight).unwrap_or(std::cmp::Ordering::Equal));
        contradicting.sort_by(|a, b| b.weight.partial_cmp(&a.weight).unwrap_or(std::cmp::Ordering::Equal));
        (supporting, contradicting)
    }

    /// Classify if content supports or contradicts a thesis using keyword matching.
    pub fn classify_content(content: &str, _thesis: &str) -> EvidenceType {
        let content_lower = content.to_lowercase();

        for kw in CONTRADICT_KEYWORDS {
            if content_lower.contains(&kw.to_lowercase()) {
                return EvidenceType::Contradict;
            }
        }

        EvidenceType::Support
    }

    /// Suggest which sections need most attention.
    pub fn suggest_sections(has_contradicting: bool) -> Vec<ArgumentSection> {
        let mut sections = vec![
            ArgumentSection::Introduction,
            ArgumentSection::Discussion,
        ];

        if has_contradicting {
            sections.push(ArgumentSection::Limitation);
        }

        sections
    }

    /// Generate a summary of the argument.
    pub fn summarize(
        thesis: &str,
        support_count: usize,
        contradict_count: usize,
        gap_count: usize,
    ) -> String {
        let thesis_preview = if thesis.len() > 50 {
            format!("{}...", &thesis[..50])
        } else {
            thesis.to_string()
        };

        format!(
            "论点「{}」有 {} 条支持证据，{} 条反驳证据。涉及 {} 个相关研究空白。",
            thesis_preview,
            support_count,
            contradict_count,
            gap_count
        )
    }

    /// Parse LLM guidance response into section guidance map.
    /// This is a best-effort parser that looks for section keywords.
    pub fn parse_guidance_response(response: &str) -> HashMap<ArgumentSection, String> {
        let sections = [
            (ArgumentSection::Introduction, vec!["introduction", "引言", "动机"]),
            (
                ArgumentSection::RelatedWork,
                vec!["related", "相关工作", "贡献"],
            ),
            (ArgumentSection::Methodology, vec!["method", "方法", "实验"]),
            (ArgumentSection::Discussion, vec!["discussion", "讨论", "回应"]),
            (
                ArgumentSection::Limitation,
                vec!["limitation", "局限", "边界"],
            ),
        ];

        let mut guidance = HashMap::new();
        let mut current_section: Option<ArgumentSection> = None;
        let mut current_content: Vec<String> = Vec::new();

        for line in response.lines() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }

            let line_lower = line.to_lowercase();
            let mut matched = false;

            for (section, keywords) in &sections {
                if keywords.iter().any(|kw| line_lower.contains(kw)) {
                    // Save previous section
                    if let Some(prev) = current_section.take() {
                        if !current_content.is_empty() {
                            guidance.insert(prev, current_content.join("\n"));
                        }
                    }

                    // Extract content after the marker
                    current_section = Some(*section);
                    current_content = if let Some(idx) = line.find('.') {
                        vec![line[idx + 1..].trim().to_string()]
                    } else {
                        Vec::new()
                    };
                    matched = true;
                    break;
                }
            }

            if !matched {
                if let Some(_) = current_section {
                    current_content.push(line.to_string());
                }
            }
        }

        // Save last section
        if let Some(section) = current_section.take() {
            if !current_content.is_empty() {
                guidance.insert(section, current_content.join("\n"));
            }
        }

        // Fill missing sections with templates
        for section in &[
            ArgumentSection::Introduction,
            ArgumentSection::RelatedWork,
            ArgumentSection::Methodology,
            ArgumentSection::Discussion,
            ArgumentSection::Limitation,
        ] {
            if !guidance.contains_key(section) {
                guidance.insert(
                    *section,
                    format!("建议在 {} 部分讨论相关内容。", section.label_zh()),
                );
            }
        }

        guidance
    }
}

impl Default for ArgumentBuilder {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Render Functions
// ============================================================================

/// Render argument result as formatted text.
pub fn render_argument(result: &ArgumentResult) -> String {
    let mut lines = Vec::new();
    let arg = &result.argument;

    lines.push("=".repeat(70));
    lines.push("📝 论点论证".to_string());
    lines.push("=".repeat(70));
    lines.push(String::new());
    lines.push(format!("论点：{}", arg.thesis));
    lines.push(String::new());

    // Supporting evidence
    lines.push("✅ 支持证据:".to_string());
    if arg.supporting_evidence.is_empty() {
        lines.push("   暂无支持证据".to_string());
    } else {
        for (i, e) in arg.supporting_evidence.iter().take(5).enumerate().map(|(i, e)| (i + 1, e))
        {
            lines.push(format!("   {}. [{}]", i, e.source));
            let preview: String = e.content.chars().take(80).collect();
            let preview = if preview.len() < e.content.len() {
                format!("{}...", preview)
            } else {
                preview
            };
            lines.push(format!("      {}", preview));
        }
    }
    lines.push(String::new());

    // Contradicting evidence
    lines.push("❌ 反驳/质疑证据:".to_string());
    if arg.contradicting_evidence.is_empty() {
        lines.push("   暂无明显反驳证据".to_string());
    } else {
        for (i, e) in arg
            .contradicting_evidence
            .iter()
            .take(5)
            .enumerate()
            .map(|(i, e)| (i + 1, e))
        {
            lines.push(format!("   {}. [{}]", i, e.source));
            let preview: String = e.content.chars().take(80).collect();
            let preview = if preview.len() < e.content.len() {
                format!("{}...", preview)
            } else {
                preview
            };
            lines.push(format!("      {}", preview));
        }
    }
    lines.push(String::new());

    // Related gaps
    if !arg.related_gaps.is_empty() {
        lines.push("🔗 相关研究空白:".to_string());
        for gap in &arg.related_gaps {
            lines.push(format!("   • {}", gap));
        }
        lines.push(String::new());
    }

    // Section guidance
    if !result.section_guidance.is_empty() {
        lines.push("📚 论文章节建议:".to_string());
        for (section, guidance_text) in &result.section_guidance {
            lines.push(format!("   {}:", section.label_zh()));
            let preview: String = guidance_text.chars().take(100).collect();
            let preview = if preview.len() < guidance_text.len() {
                format!("{}...", preview)
            } else {
                preview
            };
            lines.push(format!("      {}", preview));
        }
        lines.push(String::new());
    }

    lines.push("=".repeat(70));
    lines.join("\n")
}

/// Render as Markdown.
pub fn render_argument_markdown(result: &ArgumentResult) -> String {
    let mut lines = Vec::new();
    let arg = &result.argument;

    lines.push(format!("# 论点论证：{}\n", arg.thesis));
    lines.push(String::new());

    lines.push("## 支持证据\n".to_string());
    if arg.supporting_evidence.is_empty() {
        lines.push("*暂无支持证据*\n".to_string());
    } else {
        for e in &arg.supporting_evidence {
            lines.push(format!(
                "- **{}** (权重: {:.2})  \n  {}",
                e.source,
                e.weight,
                e.content
            ));
        }
        lines.push(String::new());
    }

    lines.push("## 反驳/质疑证据\n".to_string());
    if arg.contradicting_evidence.is_empty() {
        lines.push("*暂无明显反驳证据*\n".to_string());
    } else {
        for e in &arg.contradicting_evidence {
            lines.push(format!(
                "- **{}** (权重: {:.2})  \n  {}",
                e.source,
                e.weight,
                e.content
            ));
        }
        lines.push(String::new());
    }

    if !arg.related_gaps.is_empty() {
        lines.push("## 相关研究空白\n".to_string());
        for gap in &arg.related_gaps {
            lines.push(format!("- {}", gap));
        }
        lines.push(String::new());
    }

    if !result.section_guidance.is_empty() {
        lines.push("## 论文章节建议\n".to_string());
        for (section, text) in &result.section_guidance {
            lines.push(format!("### {}\n{}\n", section.label_zh(), text));
        }
    }

    lines.join("\n")
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_empty() {
        let builder = ArgumentBuilder::new();
        let result = builder.build("Test thesis");
        assert_eq!(result.topic, "Test thesis");
        assert_eq!(result.argument.thesis, "Test thesis");
        assert!(result.argument.supporting_evidence.is_empty());
        assert!(result.argument.contradicting_evidence.is_empty());
    }

    #[test]
    fn test_build_with_evidence() {
        let builder = ArgumentBuilder::new();
        let supporting = vec![Evidence {
            evidence_type: EvidenceType::Support,
            source: "Paper A".to_string(),
            content: "Shows significant improvements".to_string(),
            citation: "Paper A (2024)".to_string(),
            weight: 0.9,
        }];
        let contradicting = vec![Evidence {
            evidence_type: EvidenceType::Contradict,
            source: "Paper B".to_string(),
            content: "Has some limitations".to_string(),
            citation: "Paper B (2023)".to_string(),
            weight: 0.7,
        }];

        let result = builder.build_with_evidence("Test thesis", supporting, contradicting);

        assert_eq!(result.argument.supporting_evidence.len(), 1);
        assert_eq!(result.argument.contradicting_evidence.len(), 1);
        assert!(!result.summary.is_empty());
        assert!(!result.section_guidance.is_empty());
    }

    #[test]
    fn test_categorize_evidence() {
        let supporting = vec![
            Evidence {
                evidence_type: EvidenceType::Support,
                source: "A".to_string(),
                content: "a".to_string(),
                weight: 0.5,
                ..Default::default()
            },
            Evidence {
                evidence_type: EvidenceType::Support,
                source: "B".to_string(),
                content: "b".to_string(),
                weight: 0.9,
                ..Default::default()
            },
        ];
        let contradicting = vec![
            Evidence {
                evidence_type: EvidenceType::Contradict,
                source: "C".to_string(),
                content: "c".to_string(),
                weight: 0.3,
                ..Default::default()
            },
        ];

        let (sup, con) = ArgumentBuilder::categorize_evidence(supporting, contradicting);

        assert_eq!(sup.len(), 2);
        assert_eq!(con.len(), 1);
        // Check sorting: higher weight first
        assert_eq!(sup[0].source, "B");
        assert_eq!(sup[1].source, "A");
    }

    #[test]
    fn test_classify_content_support() {
        assert_eq!(
            ArgumentBuilder::classify_content("This method achieves SOTA results", "test"),
            EvidenceType::Support
        );
    }

    #[test]
    fn test_classify_content_contradict() {
        assert_eq!(
            ArgumentBuilder::classify_content("This method has limitations", "test"),
            EvidenceType::Contradict
        );
        assert_eq!(
            ArgumentBuilder::classify_content("存在问题的原因", "test"),
            EvidenceType::Contradict
        );
        assert_eq!(
            ArgumentBuilder::classify_content("The limitation is clear", "test"),
            EvidenceType::Contradict
        );
    }

    #[test]
    fn test_suggest_sections() {
        let no_contradict = ArgumentBuilder::suggest_sections(false);
        assert!(no_contradict.contains(&ArgumentSection::Introduction));
        assert!(no_contradict.contains(&ArgumentSection::Discussion));
        assert!(!no_contradict.contains(&ArgumentSection::Limitation));

        let with_contradict = ArgumentBuilder::suggest_sections(true);
        assert!(with_contradict.contains(&ArgumentSection::Limitation));
    }

    #[test]
    fn test_summarize() {
        let summary = ArgumentBuilder::summarize("Transformer attention is great", 5, 2, 3);
        assert!(summary.contains("5 条支持证据"));
        assert!(summary.contains("2 条反驳证据"));
        assert!(summary.contains("3 个相关研究空白"));
    }

    #[test]
    fn test_render_argument() {
        let builder = ArgumentBuilder::new();
        let supporting = vec![Evidence {
            evidence_type: EvidenceType::Support,
            source: "Test Paper".to_string(),
            content: "Shows improvement".to_string(),
            weight: 0.8,
            ..Default::default()
        }];
        let result = builder.build_with_evidence("Test thesis", supporting, vec![]);

        let rendered = render_argument(&result);
        assert!(rendered.contains("Test thesis"));
        assert!(rendered.contains("Test Paper"));
        assert!(rendered.contains("✅ 支持证据"));
    }

    #[test]
    fn test_render_markdown() {
        let builder = ArgumentBuilder::new();
        let result = builder.build("Test thesis");
        let rendered = render_argument_markdown(&result);
        assert!(rendered.contains("# 论点论证"));
        assert!(rendered.contains("Test thesis"));
    }

    #[test]
    fn test_parse_guidance_response() {
        let response = "Introduction: Start with motivation. \nMethodology: Design experiments carefully. \nDiscussion: Address limitations.";
        let guidance = ArgumentBuilder::parse_guidance_response(response);

        assert!(guidance.contains_key(&ArgumentSection::Introduction));
        assert!(guidance.contains_key(&ArgumentSection::Methodology));
        assert!(guidance.contains_key(&ArgumentSection::Discussion));
    }

    #[test]
    fn test_evidence_default_weight() {
        let e: Evidence = serde_json::from_str(r#"{"evidence_type":"support","source":"test","content":"test"}"#).unwrap();
        assert!((e.weight - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_argument_section_display() {
        assert_eq!(ArgumentSection::Introduction.label_zh(), "引言");
        assert_eq!(ArgumentSection::RelatedWork.label_zh(), "相关工作");
    }
}
