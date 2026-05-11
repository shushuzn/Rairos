use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct BriefingSection {
    pub title: String,
    pub content: String,
    #[serde(default = "default_level")]
    pub level: i32,
}

fn default_level() -> i32 {
    2
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Briefing {
    pub paper_arxiv_id: String,
    pub paper_title: String,
    #[serde(default)]
    pub sections: Vec<BriefingSection>,
    #[serde(default)]
    pub gene_pool_matches: Vec<serde_json::Map<String, serde_json::Value>>,
    #[serde(default)]
    pub memory_stances: Vec<serde_json::Map<String, serde_json::Value>>,
    #[serde(default)]
    pub verdict: String,
    #[serde(default)]
    pub verdict_reason: String,
    #[serde(default)]
    pub generated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BriefingResult {
    pub success: bool,
    pub briefing: Option<Briefing>,
    #[serde(default)]
    pub markdown: String,
    #[serde(default)]
    pub error: String,
}

const GENE_POOL_FILE: &str = "gene_pool/capsules.json";
const RESEARCH_MEMORY_FILE: &str = "research_memory/stances.json";

fn gene_pool_path() -> PathBuf {
    dirs::home_dir()
        .map(|p| p.join(".ai_research_os").join(GENE_POOL_FILE))
        .unwrap_or_else(|| PathBuf::from(GENE_POOL_FILE))
}

fn research_memory_path() -> PathBuf {
    dirs::home_dir()
        .map(|p| p.join(".ai_research_os").join(RESEARCH_MEMORY_FILE))
        .unwrap_or_else(|| PathBuf::from(RESEARCH_MEMORY_FILE))
}

fn load_gene_pool() -> Vec<serde_json::Map<String, serde_json::Value>> {
    let path = gene_pool_path();
    if !path.exists() {
        return Vec::new();
    }
    match std::fs::read_to_string(&path) {
        Ok(contents) => {
            let data: serde_json::Value = match serde_json::from_str(&contents) {
                Ok(v) => v,
                Err(_) => return Vec::new(),
            };
            if let Some(arr) = data.as_array() {
                arr.iter().filter_map(|v| v.as_object().cloned()).collect()
            } else if let Some(obj) = data.as_object() {
                obj.get("capsules")
                    .and_then(|v| v.as_array())
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|v| v.as_object().cloned())
                            .collect()
                    })
                    .unwrap_or_default()
            } else {
                Vec::new()
            }
        }
        Err(_) => Vec::new(),
    }
}

fn load_research_memory() -> Vec<serde_json::Map<String, serde_json::Value>> {
    let path = research_memory_path();
    if !path.exists() {
        return Vec::new();
    }
    match std::fs::read_to_string(&path) {
        Ok(contents) => {
            let data: serde_json::Value = match serde_json::from_str(&contents) {
                Ok(v) => v,
                Err(_) => return Vec::new(),
            };
            if let Some(arr) = data.as_array() {
                arr.iter().filter_map(|v| v.as_object().cloned()).collect()
            } else {
                Vec::new()
            }
        }
        Err(_) => Vec::new(),
    }
}

fn match_gene_pool(_arxiv_id: &str, title: &str, abstract_text: &str) -> Vec<serde_json::Map<String, serde_json::Value>> {
    let gene_pool = load_gene_pool();
    if gene_pool.is_empty() {
        return Vec::new();
    }

    let text = format!("{} {}", title, abstract_text).to_lowercase();

    let mut matches: Vec<(serde_json::Map<String, serde_json::Value>, f64)> = Vec::new();

    for capsule in &gene_pool {
        let gap_title = capsule
            .get("gap_title")
            .or_else(|| capsule.get("action_gap_title"))
            .or_else(|| capsule.get("trigger_gap_title"))
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_lowercase();
        let gap_type = capsule
            .get("gap_type")
            .or_else(|| capsule.get("trigger_gap_type"))
            .or_else(|| capsule.get("action_gap_type"))
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let keywords: Vec<String> = capsule
            .get("keywords")
            .or_else(|| capsule.get("trigger_keywords"))
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(|s| s.to_lowercase()))
                    .collect()
            })
            .unwrap_or_default();
        let outcome_score = capsule
            .get("outcome_success_score")
            .or_else(|| capsule.get("score"))
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0);

        let overlap = keywords.iter().filter(|kw| text.contains(kw.as_str())).count() as f64;

        if overlap >= 1.0 || (!gap_title.is_empty() && overlap > 0.0) {
            let mut m = serde_json::Map::new();
            m.insert("gap_title".to_string(), serde_json::json!(gap_title));
            m.insert("gap_type".to_string(), serde_json::json!(gap_type));
            m.insert("outcome_score".to_string(), serde_json::json!(outcome_score));
            m.insert(
                "match_reason".to_string(),
                serde_json::json!(if overlap > 0.0 {
                    format!("keyword overlap: {}", overlap as i32)
                } else {
                    "topic match".to_string()
                }),
            );
            matches.push((m, outcome_score));
        }
    }

    matches.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    matches.into_iter().take(3).map(|(m, _)| m).collect()
}

fn match_research_memory(
    _arxiv_id: &str,
    title: &str,
    abstract_text: &str,
) -> Vec<serde_json::Map<String, serde_json::Value>> {
    let stances = load_research_memory();
    if stances.is_empty() {
        return Vec::new();
    }

    let text = format!("{} {}", title, abstract_text).to_lowercase();

    let mut matches: Vec<serde_json::Map<String, serde_json::Value>> = Vec::new();

    for stance in &stances {
        let claim = stance
            .get("claim")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_lowercase();
        let topic = stance
            .get("topic")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_lowercase();

        let claim_words: HashSet<_> = claim.split_whitespace().collect();
        let text_words: HashSet<_> = text.split_whitespace().collect();
        let topic_words: HashSet<_> = topic.split_whitespace().collect();

        let claim_overlap = claim_words.intersection(&text_words).count();
        let topic_overlap = topic_words.intersection(&text_words).count();

        if claim_overlap > 0 || topic_overlap > 0 {
            let mut m = serde_json::Map::new();
            m.insert(
                "stance_id".to_string(),
                stance.get("stance_id").cloned().unwrap_or(serde_json::Value::Null),
            );
            m.insert(
                "topic".to_string(),
                stance.get("topic").cloned().unwrap_or(serde_json::Value::Null),
            );
            m.insert(
                "claim".to_string(),
                stance.get("claim").cloned().unwrap_or(serde_json::Value::Null),
            );
            m.insert(
                "stance".to_string(),
                stance.get("stance").cloned().unwrap_or(serde_json::Value::Null),
            );
            m.insert(
                "confidence".to_string(),
                stance.get("confidence").cloned().unwrap_or(serde_json::json!(0.0)),
            );
            m.insert(
                "evidence_refs".to_string(),
                stance.get("evidence_refs").cloned().unwrap_or(serde_json::Value::Null),
            );
            matches.push(m);
        }
    }

    matches.into_iter().take(3).collect()
}

pub struct BriefingGenerator;

impl BriefingGenerator {
    pub fn new() -> Self {
        Self
    }

    pub fn generate(
        &self,
        arxiv_id: &str,
        _use_llm: bool,
        _api_key: Option<&str>,
        _base_url: Option<&str>,
        _model: Option<&str>,
        _output_dir: Option<&Path>,
    ) -> BriefingResult {
        let paper_title = format!("Paper {}", arxiv_id);
        let abstract_text = "Abstract not available without database".to_string();
        let authors: Vec<String> = vec!["Unknown".to_string()];

        let gene_pool_matches = match_gene_pool(arxiv_id, &paper_title, &abstract_text);
        let memory_stances = match_research_memory(arxiv_id, &paper_title, &abstract_text);

        let (verdict, verdict_reason) = self.compute_verdict(&paper_title, &abstract_text, &gene_pool_matches, &memory_stances);

        let sections = self.generate_metadata_briefing(
            &paper_title,
            &abstract_text,
            &authors,
            arxiv_id,
            &gene_pool_matches,
            &memory_stances,
        );

        let now = chrono::Utc::now().to_rfc3339();
        let briefing = Briefing {
            paper_arxiv_id: arxiv_id.to_string(),
            paper_title: paper_title.clone(),
            sections,
            gene_pool_matches,
            memory_stances,
            verdict: verdict.clone(),
            verdict_reason: verdict_reason.clone(),
            generated_at: now.clone(),
        };

        let markdown = self.render_markdown(&briefing, &authors);

        BriefingResult {
            success: true,
            briefing: Some(briefing),
            markdown,
            error: String::new(),
        }
    }

    fn compute_verdict(
        &self,
        title: &str,
        abstract_text: &str,
        gene_pool_matches: &[serde_json::Map<String, serde_json::Value>],
        memory_stances: &[serde_json::Map<String, serde_json::Value>],
    ) -> (String, String) {
        if gene_pool_matches.is_empty() && memory_stances.is_empty() {
            return (
                "neutral".to_string(),
                "No matching Gene Pool entries or Research Memory stances".to_string(),
            );
        }

        let text = format!("{} {}", title, abstract_text).to_lowercase();

        let gap_type_matches: Vec<_> = gene_pool_matches
            .iter()
            .filter(|m| {
                m.get("gap_type")
                    .and_then(|v| v.as_str())
                    .map(|gt| text.contains(&gt.to_lowercase()))
                    .unwrap_or(false)
            })
            .collect();

        if !gap_type_matches.is_empty() {
            return (
                "opportunity_seized".to_string(),
                "Paper directly addresses a known research gap in Gene Pool".to_string(),
            );
        }

        let validates_gaps = gene_pool_matches
            .iter()
            .any(|m| m.get("outcome_score").and_then(|v| v.as_f64()).unwrap_or(0.0) >= 0.5);

        let contradiction_signals = [
            "fail to",
            "does not",
            "cannot",
            "ineffective",
            "worse than",
            "no evidence",
            "contrary to",
            "challenges",
        ];

        let contradicts = contradiction_signals.iter().any(|sig| text.contains(sig));

        if contradicts {
            return (
                "contradicts".to_string(),
                "Paper contains language suggesting it challenges existing approaches".to_string(),
            );
        }

        if validates_gaps {
            return (
                "validates".to_string(),
                "Paper addresses Gene Pool gaps with high outcome scores".to_string(),
            );
        }

        (
            "neutral".to_string(),
            "Paper is related but does not directly validate or contradict existing knowledge"
                .to_string(),
        )
    }

    fn generate_metadata_briefing(
        &self,
        title: &str,
        abstract_text: &str,
        authors: &[String],
        arxiv_id: &str,
        gene_pool_matches: &[serde_json::Map<String, serde_json::Value>],
        memory_stances: &[serde_json::Map<String, serde_json::Value>],
    ) -> Vec<BriefingSection> {
        let mut sections = vec![
            BriefingSection {
                title: "TL;DR".to_string(),
                content: format!(
                    "**{}**\n\nAuthors: {}\n\n{}...",
                    title,
                    authors.iter().take(3).map(|s| s.as_str()).collect::<Vec<_>>().join(", "),
                    &abstract_text[..abstract_text.len().min(300)]
                ),
                ..Default::default()
            },
            BriefingSection {
                title: "Key Metadata".to_string(),
                content: format!(
                    "- arXiv: {}\n- Authors: {}\n- Gene Pool Matches: {}\n- Research Memory Stances: {}",
                    arxiv_id,
                    authors.join(", "),
                    gene_pool_matches.len(),
                    memory_stances.len()
                ),
                ..Default::default()
            },
        ];

        if !gene_pool_matches.is_empty() {
            let mut lines = vec![format!("**Relevant Gene Pool Gaps ({}):**", gene_pool_matches.len())];
            for m in gene_pool_matches {
                let gap_title = m.get("gap_title").and_then(|v| v.as_str()).unwrap_or("(unknown)");
                let gap_type = m.get("gap_type").and_then(|v| v.as_str()).unwrap_or("unknown");
                let score = m.get("outcome_score").and_then(|v| v.as_f64()).unwrap_or(0.0);
                lines.push(format!("- [{}] {} (score: {:.2})", gap_type, gap_title, score));
            }
            sections.push(BriefingSection {
                title: "Gene Pool Relevance".to_string(),
                content: lines.join("\n"),
                ..Default::default()
            });
        }

        if !memory_stances.is_empty() {
            let mut lines = vec![format!(
                "**Relevant Research Memory Stances ({}):**",
                memory_stances.len()
            )];
            for s in memory_stances {
                let stance_type = s.get("stance").and_then(|v| v.as_str()).unwrap_or("unknown");
                let topic = s.get("topic").and_then(|v| v.as_str()).unwrap_or("(unknown)");
                let claim = s.get("claim").and_then(|v| v.as_str()).unwrap_or("");
                lines.push(format!(
                    "- [{}] {}: {}",
                    stance_type.to_uppercase(),
                    topic,
                    &claim[..claim.len().min(80)]
                ));
            }
            sections.push(BriefingSection {
                title: "Memory Alignment".to_string(),
                content: lines.join("\n"),
                ..Default::default()
            });
        }

        sections
    }

    fn render_markdown(&self, briefing: &Briefing, authors: &[String]) -> String {
        let now = chrono::Utc::now().format("%Y-%m-%d").to_string();

        let verdict_emoji = HashMap::from([
            ("validates", "✅"),
            ("contradicts", "❌"),
            ("neutral", "⚪"),
            ("irrelevant", "🚫"),
            ("opportunity_seized", "🎯"),
        ]);
        let emoji = verdict_emoji
            .get(briefing.verdict.as_str())
            .copied()
            .unwrap_or("⚪");

        let mut lines = vec![
            format!("# Research Briefing: {}", briefing.paper_title),
            String::new(),
            format!(
                "**arXiv:** [{}](https://arxiv.org/abs/{}) | \
**Authors:** {} | **Generated:** {}",
                briefing.paper_arxiv_id,
                briefing.paper_arxiv_id,
                authors.iter().take(3).map(|s| s.as_str()).collect::<Vec<_>>().join(", "),
                now
            ),
            String::new(),
            format!("**Verdict:** {} **{}** — {}", emoji, briefing.verdict.to_uppercase(), briefing.verdict_reason),
            String::new(),
        ];

        for section in &briefing.sections {
            lines.push(format!("## {}", section.title));
            lines.push(String::new());
            lines.push(section.content.clone());
            lines.push(String::new());
        }

        if !briefing.gene_pool_matches.is_empty() {
            lines.push("## Gene Pool Matches".to_string());
            lines.push(String::new());
            for m in &briefing.gene_pool_matches {
                let gap_type = m.get("gap_type").and_then(|v| v.as_str()).unwrap_or("unknown");
                let gap_title = m.get("gap_title").and_then(|v| v.as_str()).unwrap_or("(unknown)");
                let score = m.get("outcome_score").and_then(|v| v.as_f64()).unwrap_or(0.0);
                let reason = m.get("match_reason").and_then(|v| v.as_str()).unwrap_or("");
                lines.push(format!("- **[{}]** {} (score: {:.2}) — {}", gap_type, gap_title, score, reason));
            }
            lines.push(String::new());
        }

        if !briefing.memory_stances.is_empty() {
            lines.push("## Research Memory Alignment".to_string());
            lines.push(String::new());
            for s in &briefing.memory_stances {
                let stance_type = s.get("stance").and_then(|v| v.as_str()).unwrap_or("unknown");
                let topic = s.get("topic").and_then(|v| v.as_str()).unwrap_or("(unknown)");
                let claim = s.get("claim").and_then(|v| v.as_str()).unwrap_or("");
                lines.push(format!("- **[{}]** {}: {}", stance_type.to_uppercase(), topic, &claim[..claim.len().min(80)]));
            }
            lines.push(String::new());
        }

        lines.push("---".to_string());
        lines.push(format!("_Generated by Rairos BriefingGenerator on {}_", now));

        lines.join("\n")
    }
}

impl Default for BriefingGenerator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_briefing_generator_new() {
        let bg = BriefingGenerator::new();
        let result = bg.generate("test.12345", false, None, None, None, None);
        assert!(result.success);
        assert!(result.briefing.is_some());
    }

    #[test]
    fn test_compute_verdict_neutral() {
        let bg = BriefingGenerator::new();
        let (verdict, reason) = bg.compute_verdict("Test", "Abstract", &[], &[]);
        assert_eq!(verdict, "neutral");
        assert!(reason.contains("No matching"));
    }

    #[test]
    fn test_compute_verdict_validates() {
        let bg = BriefingGenerator::new();
        let mut match1 = serde_json::Map::new();
        match1.insert("outcome_score".to_string(), serde_json::json!(0.7));
        match1.insert("gap_type".to_string(), serde_json::json!("theoretical_gap"));
        match1.insert("gap_title".to_string(), serde_json::json!("test gap"));

        let (verdict, _reason) = bg.compute_verdict("Test", "Abstract", &[match1], &[]);
        assert_eq!(verdict, "validates");
    }

    #[test]
    fn test_compute_verdict_contradicts() {
        let bg = BriefingGenerator::new();
        let mut match1 = serde_json::Map::new();
        match1.insert("outcome_score".to_string(), serde_json::json!(0.3));
        match1.insert("gap_type".to_string(), serde_json::json!("theoretical_gap"));
        match1.insert("gap_title".to_string(), serde_json::json!("test gap"));

        let (verdict, _) = bg.compute_verdict(
            "Test",
            "This paper does not achieve good results",
            &[match1],
            &[],
        );
        assert_eq!(verdict, "contradicts");
    }

    #[test]
    fn test_generate_metadata_briefing() {
        let bg = BriefingGenerator::new();
        let sections = bg.generate_metadata_briefing(
            "Test Title",
            "Test Abstract Content",
            &["Author1".to_string(), "Author2".to_string()],
            "1234.56789",
            &[],
            &[],
        );
        assert!(!sections.is_empty());
        assert!(sections.iter().any(|s| s.title == "TL;DR"));
        assert!(sections.iter().any(|s| s.title == "Key Metadata"));
    }

    #[test]
    fn test_render_markdown() {
        let bg = BriefingGenerator::new();
        let briefing = Briefing {
            paper_arxiv_id: "1234.56789".to_string(),
            paper_title: "Test Paper".to_string(),
            sections: vec![BriefingSection {
                title: "Test Section".to_string(),
                content: "Test content".to_string(),
                ..Default::default()
            }],
            gene_pool_matches: vec![],
            memory_stances: vec![],
            verdict: "neutral".to_string(),
            verdict_reason: "No matching entries".to_string(),
            generated_at: chrono::Utc::now().to_rfc3339(),
        };
        let markdown = bg.render_markdown(&briefing, &["Author".to_string()]);
        assert!(markdown.contains("Test Paper"));
        assert!(markdown.contains("Research Briefing"));
    }

    #[test]
    fn test_briefing_section_serialization() {
        let section = BriefingSection {
            title: "Test".to_string(),
            content: "Content".to_string(),
            level: 2,
        };
        let json = serde_json::to_string(&section).unwrap();
        assert!(json.contains("Test"));
    }

    #[test]
    fn test_briefing_result_serialization() {
        let result = BriefingResult {
            success: true,
            briefing: None,
            markdown: "# Test".to_string(),
            error: String::new(),
        };
        let json = serde_json::to_string(&result).unwrap();
        assert!(json.contains("success"));
    }
}