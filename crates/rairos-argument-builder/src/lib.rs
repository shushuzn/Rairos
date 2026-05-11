use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EvidenceType {
    #[serde(rename = "support")]
    Support,
    #[serde(rename = "contradict")]
    Contradict,
    #[serde(rename = "qualify")]
    Qualify,
    #[serde(rename = "methodological")]
    Methodological,
}

impl Default for EvidenceType {
    fn default() -> Self {
        EvidenceType::Support
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ArgumentSection {
    #[serde(rename = "introduction")]
    Introduction,
    #[serde(rename = "related_work")]
    RelatedWork,
    #[serde(rename = "methodology")]
    Methodology,
    #[serde(rename = "experiments")]
    Experiments,
    #[serde(rename = "discussion")]
    Discussion,
    #[serde(rename = "limitation")]
    Limitation,
}

impl std::fmt::Display for ArgumentSection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ArgumentSection::Introduction => write!(f, "introduction"),
            ArgumentSection::RelatedWork => write!(f, "related_work"),
            ArgumentSection::Methodology => write!(f, "methodology"),
            ArgumentSection::Experiments => write!(f, "experiments"),
            ArgumentSection::Discussion => write!(f, "discussion"),
            ArgumentSection::Limitation => write!(f, "limitation"),
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Evidence {
    pub evidence_type: EvidenceType,
    #[serde(default)]
    pub source: String,
    #[serde(default)]
    pub content: String,
    #[serde(default)]
    pub citation: String,
    #[serde(default = "default_weight")]
    pub weight: f64,
}

fn default_weight() -> f64 {
    1.0
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claim {
    pub text: String,
    #[serde(default)]
    pub evidence: Vec<Evidence>,
    #[serde(default = "default_confidence")]
    pub confidence: f64,
}

fn default_confidence() -> f64 {
    0.5
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Argument {
    pub thesis: String,
    #[serde(default)]
    pub claims: Vec<Claim>,
    #[serde(default)]
    pub supporting_evidence: Vec<Evidence>,
    #[serde(default)]
    pub contradicting_evidence: Vec<Evidence>,
    #[serde(default)]
    pub related_gaps: Vec<String>,
    #[serde(default)]
    pub paper_suggestions: Vec<ArgumentSection>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArgumentResult {
    pub topic: String,
    pub argument: Argument,
    #[serde(default)]
    pub summary: String,
    #[serde(default)]
    pub section_guidance: HashMap<ArgumentSection, String>,
}

pub struct ArgumentBuilder {
    db: Option<()>,
    insight_manager: Option<()>,
    gap_analyzer: Option<()>,
}

impl ArgumentBuilder {
    pub fn new() -> Self {
        Self {
            db: None,
            insight_manager: None,
            gap_analyzer: None,
        }
    }

    pub fn build(&self, thesis: &str, _use_llm: bool, _model: Option<&str>) -> ArgumentResult {
        let paper_evidence = self.search_paper_evidence(thesis);
        let insight_evidence = self.collect_insight_evidence(thesis);
        let related_gaps = self.find_related_gaps(thesis);

        let all_evidence: Vec<Evidence> = paper_evidence
            .into_iter()
            .chain(insight_evidence)
            .collect();

        let (supporting, contradicting) = self.categorize_evidence(&all_evidence);
        let section_guidance = self.generate_section_guidance(&supporting, &contradicting);

        let argument = Argument {
            thesis: thesis.to_string(),
            claims: Vec::new(),
            supporting_evidence: supporting.clone(),
            contradicting_evidence: contradicting.clone(),
            related_gaps,
            paper_suggestions: self.suggest_sections(&contradicting),
        };

        let summary = self.summarize(&argument);

        ArgumentResult {
            topic: thesis.to_string(),
            argument,
            summary,
            section_guidance,
        }
    }

    fn search_paper_evidence(&self, _thesis: &str) -> Vec<Evidence> {
        Vec::new()
    }

    fn collect_insight_evidence(&self, _thesis: &str) -> Vec<Evidence> {
        Vec::new()
    }

    fn classify_insight(&self, content: &str, _thesis: &str) -> EvidenceType {
        let contradict_keywords = [
            "局限", "问题", "失败", "缺陷", "limitation", "problem", "fail",
        ];
        let content_lower = content.to_lowercase();
        for kw in &contradict_keywords {
            if content_lower.contains(*kw) {
                return EvidenceType::Contradict;
            }
        }
        EvidenceType::Support
    }

    fn find_related_gaps(&self, _thesis: &str) -> Vec<String> {
        Vec::new()
    }

    fn categorize_evidence(&self, evidence_list: &[Evidence]) -> (Vec<Evidence>, Vec<Evidence>) {
        let mut supporting = Vec::new();
        let mut contradicting = Vec::new();

        for e in evidence_list {
            match e.evidence_type {
                EvidenceType::Support | EvidenceType::Qualify => supporting.push(e.clone()),
                _ => contradicting.push(e.clone()),
            }
        }

        supporting.sort_by(|a, b| b.weight.partial_cmp(&a.weight).unwrap_or(std::cmp::Ordering::Equal));
        contradicting.sort_by(|a, b| b.weight.partial_cmp(&a.weight).unwrap_or(std::cmp::Ordering::Equal));

        (supporting, contradicting)
    }

    fn generate_section_guidance(
        &self,
        _supporting: &[Evidence],
        contradicting: &[Evidence],
    ) -> HashMap<ArgumentSection, String> {
        let mut guidance = HashMap::new();

        guidance.insert(
            ArgumentSection::Introduction,
            "开篇应明确研究动机：为什么这个问题重要？引用主要支持证据说明该方向的潜力。".to_string(),
        );

        guidance.insert(
            ArgumentSection::RelatedWork,
            format!("综述现有工作，区分本文与前人贡献。识别 {} 个需要回应的质疑。", contradicting.len()),
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
                contradicting.len()
            ),
        );

        guidance
    }

    fn suggest_sections(&self, contradicting: &[Evidence]) -> Vec<ArgumentSection> {
        let mut sections = vec![ArgumentSection::Introduction, ArgumentSection::Discussion];
        if !contradicting.is_empty() {
            sections.push(ArgumentSection::Limitation);
        }
        sections
    }

    fn summarize(&self, argument: &Argument) -> String {
        let support_count = argument.supporting_evidence.len();
        let contradict_count = argument.contradicting_evidence.len();
        let thesis_preview = if argument.thesis.len() > 50 {
            format!("{}...", &argument.thesis[..50])
        } else {
            argument.thesis.clone()
        };
        format!(
            "论点「{}」有 {} 条支持证据，{} 条反驳证据。涉及 {} 个相关研究空白。",
            thesis_preview,
            support_count,
            contradict_count,
            argument.related_gaps.len()
        )
    }
}

impl Default for ArgumentBuilder {
    fn default() -> Self {
        Self::new()
    }
}

pub fn render_argument(result: &ArgumentResult) -> String {
    let mut lines = Vec::new();
    let arg = &result.argument;

    lines.push("=".repeat(70));
    lines.push("论点论证".to_string());
    lines.push("=".repeat(70));
    lines.push(String::new());
    lines.push(format!("论点：{}", arg.thesis));
    lines.push(String::new());

    lines.push("✅ 支持证据:".to_string());
    if arg.supporting_evidence.is_empty() {
        lines.push("   暂无支持证据".to_string());
    } else {
        for (i, e) in arg.supporting_evidence.iter().take(5).enumerate() {
            lines.push(format!("   {}. [{}]", i + 1, e.source));
            let content_preview = if e.content.len() > 80 {
                format!("{}...", &e.content[..80])
            } else {
                e.content.clone()
            };
            lines.push(format!("      {}...", content_preview));
        }
    }
    lines.push(String::new());

    lines.push("❌ 反驳/质疑证据:".to_string());
    if arg.contradicting_evidence.is_empty() {
        lines.push("   暂无明显反驳证据".to_string());
    } else {
        for (i, e) in arg.contradicting_evidence.iter().take(5).enumerate() {
            lines.push(format!("   {}. [{}]", i + 1, e.source));
            let content_preview = if e.content.len() > 80 {
                format!("{}...", &e.content[..80])
            } else {
                e.content.clone()
            };
            lines.push(format!("      {}...", content_preview));
        }
    }
    lines.push(String::new());

    if !arg.related_gaps.is_empty() {
        lines.push("🔗 相关研究空白:".to_string());
        for gap in &arg.related_gaps {
            lines.push(format!("   • {}", gap));
        }
        lines.push(String::new());
    }

    if !result.section_guidance.is_empty() {
        lines.push("📚 论文章节建议:".to_string());
        let section_names: HashMap<ArgumentSection, &str> = [
            (ArgumentSection::Introduction, "引言"),
            (ArgumentSection::RelatedWork, "相关工作"),
            (ArgumentSection::Methodology, "方法论"),
            (ArgumentSection::Experiments, "实验"),
            (ArgumentSection::Discussion, "讨论"),
            (ArgumentSection::Limitation, "局限"),
        ]
        .into_iter()
        .collect();

        for (section, guidance) in &result.section_guidance {
            let section_name = match section_names.get(section) {
                Some(&s) => s.to_string(),
                None => section.to_string(),
            };
            lines.push(format!("   {}:", section_name));
            let guidance_chars: Vec<char> = guidance.chars().collect();
            let guidance_preview = if guidance_chars.len() > 100 {
                format!("{}...", guidance_chars[..100].iter().collect::<String>())
            } else {
                guidance.clone()
            };
            lines.push(format!("      {}...", guidance_preview));
        }
        lines.push(String::new());
    }

    lines.push("=".repeat(70));
    lines.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_evidence_type_serialization() {
        let e = Evidence {
            evidence_type: EvidenceType::Support,
            source: "test".to_string(),
            content: "test content".to_string(),
            citation: "test citation".to_string(),
            weight: 0.8,
        };
        let json = serde_json::to_string(&e).unwrap();
        assert!(json.contains("support"));
    }

    #[test]
    fn test_categorize_evidence() {
        let builder = ArgumentBuilder::new();
        let evidence = vec![
            Evidence {
                evidence_type: EvidenceType::Support,
                source: "paper1".to_string(),
                content: "content".to_string(),
                weight: 0.8,
                ..Default::default()
            },
            Evidence {
                evidence_type: EvidenceType::Contradict,
                source: "paper2".to_string(),
                content: "content".to_string(),
                weight: 0.6,
                ..Default::default()
            },
            Evidence {
                evidence_type: EvidenceType::Qualify,
                source: "paper3".to_string(),
                content: "content".to_string(),
                weight: 0.5,
                ..Default::default()
            },
        ];

        let (supporting, contradicting) = builder.categorize_evidence(&evidence);
        assert_eq!(supporting.len(), 2);
        assert_eq!(contradicting.len(), 1);
    }

    #[test]
    fn test_classify_insight() {
        let builder = ArgumentBuilder::new();
        assert_eq!(
            builder.classify_insight("This has a limitation in the approach", "thesis"),
            EvidenceType::Contradict
        );
        assert_eq!(
            builder.classify_insight("This is a great paper", "thesis"),
            EvidenceType::Support
        );
    }

    #[test]
    fn test_build_argument() {
        let builder = ArgumentBuilder::new();
        let result = builder.build("Test thesis", false, None);
        assert_eq!(result.topic, "Test thesis");
        assert_eq!(result.argument.thesis, "Test thesis");
    }

    #[test]
    fn test_render_argument() {
        let builder = ArgumentBuilder::new();
        let result = builder.build("Test thesis", false, None);
        let rendered = render_argument(&result);
        assert!(rendered.contains("论点论证"));
        assert!(rendered.contains("Test thesis"));
    }

    #[test]
    fn test_suggest_sections() {
        let builder = ArgumentBuilder::new();
        let sections = builder.suggest_sections(&[]);
        assert!(sections.contains(&ArgumentSection::Introduction));
        assert!(sections.contains(&ArgumentSection::Discussion));
        assert!(!sections.contains(&ArgumentSection::Limitation));

        let sections_with_contradicting = builder.suggest_sections(&[Evidence {
            evidence_type: EvidenceType::Contradict,
            source: "test".to_string(),
            content: "test".to_string(),
            weight: 1.0,
            ..Default::default()
        }]);
        assert!(sections_with_contradicting.contains(&ArgumentSection::Limitation));
    }
}