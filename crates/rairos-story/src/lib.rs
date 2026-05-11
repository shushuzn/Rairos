//! rairos-story — Research Story Weaver
//!
//! Generates narrative understanding from research papers.
//! Core algorithm:
//! 1. Narrative extraction: identify core contributions, turning points, contradictions
//! 2. Relationship graph: logical relationships between papers
//! 3. Story generation: timeline orchestration + LLM narrative
//! 4. Comparison patterns: divergences and consensus between two storylines
//!
//! Ported from `llm/story_weaver.py`.

use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum NarrativeRole {
    Protagonist,
    Antagonist,
    TurningPoint,
    Divergence,
    Synthesis,
}

impl NarrativeRole {
    pub fn as_str(&self) -> &'static str {
        match self {
            NarrativeRole::Protagonist => "protagonist",
            NarrativeRole::Antagonist => "antagonist",
            NarrativeRole::TurningPoint => "turning_point",
            NarrativeRole::Divergence => "divergence",
            NarrativeRole::Synthesis => "synthesis",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RelationshipType {
    Inherits,
    Extends,
    Contrasts,
    Contradicts,
    Synthesizes,
    Cites,
}

impl RelationshipType {
    pub fn as_str(&self) -> &'static str {
        match self {
            RelationshipType::Inherits => "inherits",
            RelationshipType::Extends => "extends",
            RelationshipType::Contrasts => "contrasts",
            RelationshipType::Contradicts => "contradicts",
            RelationshipType::Synthesizes => "synthesizes",
            RelationshipType::Cites => "cites",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaperNarrative {
    pub paper_id: String,
    pub title: String,
    pub year: i32,
    pub role: NarrativeRole,
    pub core_contribution: String,
    pub key_insight: String,
    #[serde(default)]
    pub turning_point_type: String,
    #[serde(default)]
    pub conflicts_with: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Chapter {
    pub title: String,
    pub time_range: (i32, i32),
    pub papers: Vec<PaperNarrative>,
    #[serde(default)]
    pub summary: String,
    #[serde(default)]
    pub theme: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Relationship {
    pub from_paper: String,
    pub to_paper: String,
    pub relationship: RelationshipType,
    #[serde(default)]
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct StoryResult {
    pub topic: String,
    #[serde(default)]
    pub chapters: Vec<Chapter>,
    #[serde(default)]
    pub relationships: Vec<Relationship>,
    #[serde(default)]
    pub protagonist_arc: String,
    #[serde(default)]
    pub contradictions: Vec<(String, String)>,
    #[serde(default)]
    pub themes: Vec<String>,
    #[serde(default)]
    pub summary: String,
}

const TURNING_POINT_PATTERNS: &[&str] = &[
    r"breakthrough|revolution|paradigm shift|game changer|state-of-the-art",
    r"outperforms?|surpasses?|exceeds? previous",
    r"first to|for the first time|introduces? a new",
    r"despite|however|but|nevertheless|contradict",
];

const DIVERGENCE_PATTERNS: &[&str] = &[
    r"alternative|instead|rather|unlike|contrast",
    r"different approach|different from|diverges",
    r"on the other hand|meanwhile|conversely",
];

fn compile_patterns(patterns: &[&str]) -> Vec<Regex> {
    patterns
        .iter()
        .filter_map(|p| Regex::new(&format!("(?i){}", p)).ok())
        .collect()
}

fn matches_any(text: &str, regexes: &[Regex]) -> bool {
    regexes.iter().any(|r| r.is_match(text))
}

fn extract_match(pattern: &str, text: &str) -> Option<String> {
    if let Ok(re) = Regex::new(&format!("(?i){}", pattern)) {
        re.captures(text).map(|c| c.get(1).map(|m| m.as_str().to_string()).unwrap_or_default())
    } else {
        None
    }
}

fn extract_insight(text: &str) -> String {
    let patterns = [
        r"(?:key|central|core) insight:?\s*(.+?)(?:\.|$)",
        r"we find that (.+?)(?:\.|$)",
        r"discover(?:y|ed) that (.+?)(?:\.|$)",
        r"((?:the|this) .+? is(?: all| the) .+?)(?:\.|$)",
    ];
    for pattern in &patterns {
        if let Some(cap) = extract_match(pattern, text) {
            if !cap.is_empty() {
                return cap.chars().take(80).collect();
            }
        }
    }
    "Provides new approach to the problem".to_string()
}

fn detect_turning_point(text: &str) -> String {
    if text.contains("breakthrough") || text.contains("revolution") {
        return "颠覆性突破".to_string();
    }
    if text.contains("paradigm shift") {
        return "范式转变".to_string();
    }
    if text.contains("state-of-the-art") || text.contains("sota") {
        return "性能突破".to_string();
    }
    if text.contains("first") && text.contains("time") {
        return "首次实现".to_string();
    }
    String::new()
}

fn extract_contribution(title: &str, abstract_text: &str) -> String {
    let text = format!("{} {}", title, &abstract_text[..abstract_text.len().min(200)]).to_lowercase();
    let patterns = [
        r"we (?:propose|present|introduce|develop) (.+?)\.",
        r"this paper (.+?)\.",
        r"we show that (.+?)\.",
        r"(?:propose|present|introduce) (.+?)(?:\.|$)",
    ];
    for pattern in &patterns {
        if let Some(cap) = extract_match(pattern, &text) {
            if !cap.is_empty() {
                return cap.chars().take(100).collect();
            }
        }
    }
    if !title.is_empty() {
        title.chars().take(60).collect()
    } else {
        "Unknown contribution".to_string()
    }
}

fn determine_role(text: &str, year: i32) -> NarrativeRole {
    if year <= 2018 {
        let lower = text.to_lowercase();
        if lower.contains("attention is all you need") || lower.contains("bert") || lower.contains("gpt") {
            return NarrativeRole::Protagonist;
        }
    }
    let turning_re = compile_patterns(TURNING_POINT_PATTERNS);
    if matches_any(text, &turning_re) {
        return NarrativeRole::TurningPoint;
    }
    let divergence_re = compile_patterns(DIVERGENCE_PATTERNS);
    if matches_any(text, &divergence_re) {
        return NarrativeRole::Divergence;
    }
    NarrativeRole::Protagonist
}

fn infer_relationship(a: &PaperNarrative, b: &PaperNarrative) -> (Option<RelationshipType>, String) {
    let a_lower = a.title.to_lowercase();
    let b_lower = b.title.to_lowercase();

    if b.year > a.year && (a_lower.contains("extends") || b_lower.contains("building")) {
        return (
            Some(RelationshipType::Extends),
            format!("{} work extends {} work", b.year, a.year),
        );
    }

    if a.year <= 2017 && b.year > 2019 {
        return (
            Some(RelationshipType::Inherits),
            format!("Based on foundational work from {}", a.year),
        );
    }

    let divergence_kws = ["instead", "alternative", "rather", "unlike"];
    if divergence_kws.iter().any(|k| b_lower.contains(k)) {
        return (
            Some(RelationshipType::Contrasts),
            format!("Proposes alternative to {}...", &a.title[..a.title.len().min(30)]),
        );
    }

    let contrast_kws = ["vs", "versus", "对比", "比较"];
    if contrast_kws.iter().any(|k| a_lower.contains(k) || b_lower.contains(k)) {
        return (
            Some(RelationshipType::Contrasts),
            format!("Contrasts with {}...", &a.title[..a.title.len().min(30)]),
        );
    }

    (None, String::new())
}

fn organize_chapters(narratives: &[PaperNarrative]) -> Vec<Chapter> {
    let mut periods: HashMap<(i32, i32), Vec<&PaperNarrative>> = HashMap::new();

    for n in narratives {
        let period = if n.year < 2015 {
            (2008, 2014)
        } else if n.year < 2018 {
            (2015, 2017)
        } else if n.year < 2020 {
            (2018, 2019)
        } else if n.year < 2022 {
            (2020, 2021)
        } else if n.year < 2024 {
            (2022, 2023)
        } else {
            (2024, 2026)
        };
        periods.entry(period).or_default().push(n);
    }

    let titles: HashMap<(i32, i32), &str> = [
        ((2008, 2014), "萌芽期 - Attention 机制的发现"),
        ((2015, 2017), "突破期 - Attention Is All You Need"),
        ((2018, 2019), "扩散期 - BERT 与预训练革命"),
        ((2020, 2021), "规模化初期 - GPT-3 的里程碑"),
        ((2022, 2023), "百模大战 - 开源与闭源的对抗"),
        ((2024, 2026), "AGI 探索 - 超越 Transformer?"),
    ]
    .into_iter()
    .collect();

    let mut chapters: Vec<Chapter> = Vec::new();
    let mut period_list: Vec<_> = periods.iter().collect();
    period_list.sort_by_key(|k| k.0);
    for (period, papers) in period_list {
        let mut sorted_papers: Vec<PaperNarrative> = papers.iter().map(|p| (*p).clone()).collect();
        sorted_papers.sort_by_key(|p| p.year);
        chapters.push(Chapter {
            title: titles.get(period).copied().unwrap_or("未知时期").to_string(),
            time_range: *period,
            papers: sorted_papers,
            summary: String::new(),
            theme: String::new(),
        });
    }

    chapters.sort_by_key(|c| c.time_range.0);
    chapters
}

fn find_contradictions(narratives: &[PaperNarrative]) -> Vec<(String, String)> {
    let efficiency_kws = ["efficient", "fast", "lightweight", "small", "distill"];
    let scale_kws = ["large", "massive", "scale", "billions", "parameters"];

    let mut contradictions = Vec::new();
    for (i, a) in narratives.iter().enumerate() {
        for b in narratives.iter().skip(i + 1) {
            let a_lower = a.title.to_lowercase();
            let b_lower = b.title.to_lowercase();

            let a_efficient = efficiency_kws.iter().any(|k| a_lower.contains(k));
            let b_scale = scale_kws.iter().any(|k| b_lower.contains(k));
            if a_efficient && b_scale {
                contradictions.push((a.title.clone(), b.title.clone()));
            }

            let a_scale = scale_kws.iter().any(|k| a_lower.contains(k));
            let b_efficient = efficiency_kws.iter().any(|k| b_lower.contains(k));
            if a_scale && b_efficient {
                contradictions.push((a.title.clone(), b.title.clone()));
            }
        }
    }
    contradictions.truncate(5);
    contradictions
}

fn identify_themes(narratives: &[PaperNarrative]) -> Vec<String> {
    let theme_keywords: HashMap<&str, Vec<&str>> = [
        ("Attention 机制", vec!["attention", "self-attention", "multi-head"]),
        ("预训练范式", vec!["pre-train", "fine-tun", "mask"]),
        ("规模化", vec!["scale", "large", "billions", "parameters"]),
        ("效率优化", vec!["efficient", "fast", "distill", "prune", "quantize"]),
        ("多模态", vec!["multimodal", "vision", "image", "text"]),
        ("推理能力", vec!["reason", "chain-of-thought", "cot"]),
        ("对齐与安全", vec!["align", "rlhf", "safety", "value"]),
    ]
    .into_iter()
    .collect();

    let all_text: String = narratives.iter().map(|n| n.title.to_lowercase()).collect::<Vec<_>>().join(" ");

    let mut themes = Vec::new();
    for (theme, keywords) in theme_keywords.iter() {
        if keywords.iter().any(|k| all_text.contains(k)) {
            themes.push(theme.to_string());
        }
    }
    themes.truncate(5);
    themes
}

fn generate_summary(result: &StoryResult) -> String {
    if result.chapters.is_empty() {
        return "暂无足够数据生成故事".to_string();
    }

    let themes = if !result.themes.is_empty() {
        result.themes.iter().take(3).cloned().collect::<Vec<_>>().join(", ")
    } else {
        "技术演进".to_string()
    };

    let first_year = result.chapters.first().map(|c| c.time_range.0).unwrap_or(0);
    let last_year = result.chapters.last().map(|c| c.time_range.1).unwrap_or(0);

    let mut summary = format!(
        "《{}》的演进是一场关于{}的探索。\n从 {} 年的开创性工作，到 {} 年的最新突破，\n领域经历了从理论验证到工程化应用，从单一模型到多元化生态的转变。",
        result.topic, themes, first_year, last_year
    );

    if !result.contradictions.is_empty() {
        summary += &format!("\n核心张力: 发现 {} 个主要矛盾点，体现了领域内不同技术路线的竞争与融合。", result.contradictions.len());
    }

    summary
}

pub struct StoryWeaver;

impl StoryWeaver {
    pub fn weave(&self, topic: &str, papers: Vec<PaperInput>) -> StoryResult {
        if papers.is_empty() {
            return StoryResult {
                topic: topic.to_string(),
                ..Default::default()
            };
        }

        let narratives: Vec<PaperNarrative> = papers
            .iter()
            .map(|p| {
                let text = format!("{} {}", p.title.to_lowercase(), p.abstract_text.to_lowercase());
                let role = determine_role(&text, p.year);
                PaperNarrative {
                    paper_id: p.id.clone(),
                    title: p.title.chars().take(60).collect(),
                    year: p.year,
                    role,
                    core_contribution: extract_contribution(&p.title, &p.abstract_text),
                    key_insight: extract_insight(&text),
                    turning_point_type: detect_turning_point(&text),
                    conflicts_with: Vec::new(),
                }
            })
            .collect();

        let relationships: Vec<Relationship> = narratives
            .iter()
            .enumerate()
            .flat_map(|(i, a)| {
                narratives.iter().skip(i + 1).filter_map(|b| {
                    let (rel_type, desc) = infer_relationship(a, b);
                    rel_type.map(|rt| Relationship {
                        from_paper: a.paper_id.clone(),
                        to_paper: b.paper_id.clone(),
                        relationship: rt,
                        description: desc,
                    })
                })
            })
            .collect();

        let chapters = organize_chapters(&narratives);
        let contradictions = find_contradictions(&narratives);
        let themes = identify_themes(&narratives);

        let mut result = StoryResult {
            topic: topic.to_string(),
            chapters,
            relationships,
            protagonist_arc: String::new(),
            contradictions,
            themes,
            summary: String::new(),
        };

        result.summary = generate_summary(&result);
        result
    }

    pub fn render_result(&self, result: &StoryResult) -> String {
        let mut lines = Vec::new();
        lines.push(format!("研究故事: {}", result.topic));
        lines.push(String::new());

        for (i, chapter) in result.chapters.iter().enumerate() {
            lines.push(format!("第{}章: {}", i + 1, chapter.title));
            lines.push(format!("   时间: {}-{}", chapter.time_range.0, chapter.time_range.1));

            if !chapter.summary.is_empty() {
                lines.push(format!("   {}", chapter.summary));
            } else {
                let contributions: Vec<String> = chapter.papers.iter().take(3).map(|p| {
                    p.core_contribution.chars().take(50).collect()
                }).collect();
                lines.push(format!("   关键贡献: {}", contributions.join(" | ")));
            }
            lines.push(String::new());

            for paper in chapter.papers.iter().take(3) {
                let role_icon = match paper.role {
                    NarrativeRole::Protagonist | NarrativeRole::Divergence | NarrativeRole::Antagonist => "├─",
                    NarrativeRole::TurningPoint | NarrativeRole::Synthesis => "└─",
                };
                lines.push(format!("   {} {} ({})", role_icon, paper.title, paper.year));
                lines.push(format!("   │  └─ {}", &paper.key_insight[..paper.key_insight.len().min(60)]));
                if !paper.turning_point_type.is_empty() {
                    lines.push(format!("   │     🔥 {}", paper.turning_point_type));
                }
            }
            lines.push(String::new());
        }

        if !result.contradictions.is_empty() {
            lines.push("⚡ 核心矛盾:".to_string());
            for (a, b) in result.contradictions.iter().take(3) {
                lines.push(format!("   • {}...", &a[..a.len().min(40)]));
                lines.push(format!("     ↔ {}...", &b[..b.len().min(40)]));
            }
            lines.push(String::new());
        }

        if !result.themes.is_empty() {
            lines.push(format!("🧭 核心主题: {}", result.themes.iter().take(4).cloned().collect::<Vec<_>>().join(", ")));
            lines.push(String::new());
        }

        if !result.summary.is_empty() {
            lines.push(format!("📝 {}", result.summary));
        }

        lines.join("\n")
    }

    pub fn render_mermaid(&self, result: &StoryResult) -> String {
        let mut lines = vec![
            "```mermaid".to_string(),
            "flowchart TD".to_string(),
            format!("    title[\"📖 {}\"]", result.topic),
            String::new(),
        ];

        for (i, chapter) in result.chapters.iter().enumerate() {
            for paper in chapter.papers.iter().take(2) {
                let node_id = format!("P{}{}", paper.year, i);
                let role_class = match paper.role {
                    NarrativeRole::TurningPoint => "fill:#ff6b6b",
                    NarrativeRole::Divergence => "fill:#4ecdc4",
                    _ => "fill:#ddd",
                };
                let title_short = paper.title.chars().take(30).collect::<String>();
                lines.push(format!("    {}[\"{}\":::{}]", node_id, title_short, paper.role.as_str()));
                lines.push(format!("    classDef {} {}", paper.role.as_str(), role_class));
            }
        }

        lines.push("```".to_string());
        lines.join("\n")
    }

    pub fn compare_stories(&self, a: &StoryResult, b: &StoryResult) -> String {
        let mut lines = Vec::new();
        lines.push(format!("📖 故事线对比: {} vs {}", a.topic, b.topic));
        lines.push(String::new());

        let shared_themes: HashSet<&str> = a.themes.iter().map(|s| s.as_str()).collect();
        let shared: Vec<&str> = b.themes.iter().filter(|t| shared_themes.contains(&t.as_str())).map(|s| s.as_str()).collect();
        if !shared.is_empty() {
            lines.push(format!("🔗 共同主题: {}", shared.join(", ")));
        }

        lines.push(String::new());

        let a_first = a.chapters.first().map(|c| c.time_range.0).unwrap_or(0);
        let a_last = a.chapters.last().map(|c| c.time_range.1).unwrap_or(0);
        let b_first = b.chapters.first().map(|c| c.time_range.0).unwrap_or(0);
        let b_last = b.chapters.last().map(|c| c.time_range.1).unwrap_or(0);

        lines.push(format!("📅 {}: {}-{}", a.topic, a_first, a_last));
        lines.push(format!("📅 {}: {}-{}", b.topic, b_first, b_last));
        lines.push(String::new());
        lines.push("🎭 主角发展弧线:".to_string());
        lines.push(format!("  • {}: {}", a.topic, if !a.protagonist_arc.is_empty() { &a.protagonist_arc[..a.protagonist_arc.len().min(80)] } else { "传统方法演进" }));
        lines.push(format!("  • {}: {}", b.topic, if !b.protagonist_arc.is_empty() { &b.protagonist_arc[..b.protagonist_arc.len().min(80)] } else { "新方法探索" }));

        lines.join("\n")
    }
}

#[derive(Debug, Clone)]
pub struct PaperInput {
    pub id: String,
    pub title: String,
    pub abstract_text: String,
    pub year: i32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_contribution() {
        let title = "BERT: Pre-training of Deep Bidirectional Transformers";
        let abstract_text = "We propose a new method. This paper presents a novel approach.";
        let result = extract_contribution(title, abstract_text);
        assert!(!result.is_empty());
    }

    #[test]
    fn test_extract_insight() {
        let text = "The key insight is that attention mechanisms are effective.";
        let result = extract_insight(text);
        assert!(!result.is_empty());
    }

    #[test]
    fn test_detect_turning_point_breakthrough() {
        let text = "This is a breakthrough in AI research.";
        assert_eq!(detect_turning_point(text), "颠覆性突破");
    }

    #[test]
    fn test_detect_turning_point_sota() {
        let text = "Our method achieves state-of-the-art results.";
        assert_eq!(detect_turning_point(text), "性能突破");
    }

    #[test]
    fn test_determine_role_turning_point() {
        let text = "This is a breakthrough that outperforms all previous methods.";
        assert_eq!(determine_role(text, 2020), NarrativeRole::TurningPoint);
    }

    #[test]
    fn test_determine_role_divergence() {
        let text = "Instead of attention, we propose an alternative approach.";
        assert_eq!(determine_role(text, 2020), NarrativeRole::Divergence);
    }

    #[test]
    fn test_story_weaver_empty() {
        let weaver = StoryWeaver;
        let result = weaver.weave("transformer", vec![]);
        assert_eq!(result.topic, "transformer");
        assert!(result.chapters.is_empty());
    }

    #[test]
    fn test_story_weaver_with_papers() {
        let weaver = StoryWeaver;
        let papers = vec![
            PaperInput {
                id: "p1".to_string(),
                title: "Attention Is All You Need".to_string(),
                abstract_text: "We propose the transformer architecture. breakthrough method.".to_string(),
                year: 2017,
            },
            PaperInput {
                id: "p2".to_string(),
                title: "BERT".to_string(),
                abstract_text: "We present BERT a new method for pre-training.".to_string(),
                year: 2018,
            },
        ];
        let result = weaver.weave("NLP", papers);
        assert_eq!(result.topic, "NLP");
        assert!(!result.chapters.is_empty());
        assert!(!result.summary.is_empty());
    }

    #[test]
    fn test_render_result() {
        let weaver = StoryWeaver;
        let papers = vec![
            PaperInput {
                id: "p1".to_string(),
                title: "Attention Is All You Need".to_string(),
                abstract_text: "We propose the transformer.".to_string(),
                year: 2017,
            },
        ];
        let result = weaver.weave("transformer", papers);
        let rendered = weaver.render_result(&result);
        assert!(rendered.contains("transformer"));
    }

    #[test]
    fn test_render_mermaid() {
        let weaver = StoryWeaver;
        let papers = vec![
            PaperInput {
                id: "p1".to_string(),
                title: "Attention Is All You Need".to_string(),
                abstract_text: "We propose the transformer.".to_string(),
                year: 2017,
            },
        ];
        let result = weaver.weave("transformer", papers);
        let mermaid = weaver.render_mermaid(&result);
        assert!(mermaid.contains("mermaid"));
        assert!(mermaid.contains("transformer"));
    }

    #[test]
    fn test_find_contradictions() {
        let narratives = vec![
            PaperNarrative {
                paper_id: "p1".to_string(),
                title: "Efficient Transformer".to_string(),
                year: 2020,
                role: NarrativeRole::Protagonist,
                core_contribution: "".to_string(),
                key_insight: "".to_string(),
                turning_point_type: "".to_string(),
                conflicts_with: vec![],
            },
            PaperNarrative {
                paper_id: "p2".to_string(),
                title: "Large Scale Transformer".to_string(),
                year: 2021,
                role: NarrativeRole::Protagonist,
                core_contribution: "".to_string(),
                key_insight: "".to_string(),
                turning_point_type: "".to_string(),
                conflicts_with: vec![],
            },
        ];
        let contrad = find_contradictions(&narratives);
        assert!(!contrad.is_empty());
    }

    #[test]
    fn test_identify_themes() {
        let narratives = vec![
            PaperNarrative {
                paper_id: "p1".to_string(),
                title: "Attention Mechanism".to_string(),
                year: 2017,
                role: NarrativeRole::Protagonist,
                core_contribution: "".to_string(),
                key_insight: "".to_string(),
                turning_point_type: "".to_string(),
                conflicts_with: vec![],
            },
        ];
        let themes = identify_themes(&narratives);
        assert!(!themes.is_empty());
    }

    #[test]
    fn test_narrative_role_as_str() {
        assert_eq!(NarrativeRole::Protagonist.as_str(), "protagonist");
        assert_eq!(NarrativeRole::TurningPoint.as_str(), "turning_point");
    }

    #[test]
    fn test_relationship_type_as_str() {
        assert_eq!(RelationshipType::Inherits.as_str(), "inherits");
        assert_eq!(RelationshipType::Contrasts.as_str(), "contrasts");
    }
}
