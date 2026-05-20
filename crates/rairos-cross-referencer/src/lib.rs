//! rairos-cross-referencer — Cross-paper contradiction/synergy detection
//!
//! Detects relationships between a paper and existing papers.

use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::LazyLock;

static RE_CROSS_REF: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\[?(\S+?)\]?\s*\((\w+)\)").expect("valid regex")
});

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrossReferenceItem {
    pub relation: String,
    pub target_paper_id: String,
    pub target_title: String,
    pub description: String,
    pub confidence: f64,
    pub evidence: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrossReferenceResult {
    pub paper_id: String,
    pub related_papers_found: usize,
    pub items: Vec<CrossReferenceItem>,
    pub used_fallback: bool,
    pub error: String,
}

impl CrossReferenceResult {
    pub fn new(paper_id: &str) -> Self {
        Self {
            paper_id: paper_id.to_string(),
            related_papers_found: 0,
            items: Vec::new(),
            used_fallback: false,
            error: String::new(),
        }
    }

    pub fn with_error(paper_id: &str, error: &str) -> Self {
        Self {
            paper_id: paper_id.to_string(),
            related_papers_found: 0,
            items: Vec::new(),
            used_fallback: true,
            error: error.to_string(),
        }
    }
}

pub struct CrossReferencer {
    papers: HashMap<String, (String, String, Vec<String>)>,
}

impl CrossReferencer {
    pub fn new() -> Self {
        Self {
            papers: HashMap::new(),
        }
    }

    pub fn add_paper(
        &mut self,
        paper_id: &str,
        title: &str,
        abstract_text: &str,
        tags: Vec<String>,
    ) {
        self.papers.insert(
            paper_id.to_string(),
            (title.to_string(), abstract_text.to_string(), tags),
        );
    }

    pub fn analyze(
        &self,
        paper_id: &str,
        _title: &str,
        _abstract_text: &str,
        _body_text: &str,
        tags: Option<Vec<String>>,
        _use_llm: bool,
    ) -> CrossReferenceResult {
        if self.papers.is_empty() {
            return CrossReferenceResult::with_error(
                paper_id,
                "No database available for cross-referencing",
            );
        }

        let tags = tags.unwrap_or_default();
        let candidates = self.find_candidates(paper_id, &tags, 10);

        if candidates.is_empty() {
            return CrossReferenceResult {
                paper_id: paper_id.to_string(),
                related_papers_found: 0,
                items: Vec::new(),
                used_fallback: false,
                error: String::new(),
            };
        }

        self.analyze_fallback(paper_id, candidates)
    }

    fn find_candidates(
        &self,
        paper_id: &str,
        tags: &[String],
        max_candidates: usize,
    ) -> Vec<(String, String, String)> {
        let mut candidates = Vec::new();
        let mut seen = std::collections::HashSet::new();

        for tag in tags {
            for (pid, (title, abstract_text, paper_tags)) in &self.papers {
                if pid == paper_id || seen.contains(pid) {
                    continue;
                }
                if paper_tags.contains(tag) {
                    seen.insert(pid.clone());
                    candidates.push((pid.clone(), title.clone(), abstract_text.clone()));
                    if candidates.len() >= max_candidates {
                        return candidates;
                    }
                }
            }
        }

        candidates
    }

    fn analyze_fallback(
        &self,
        paper_id: &str,
        candidates: Vec<(String, String, String)>,
    ) -> CrossReferenceResult {
        let mut items = Vec::new();

        for (pid, title, _) in candidates.into_iter().take(5) {
            items.push(CrossReferenceItem {
                relation: "alignment".to_string(),
                target_paper_id: pid,
                target_title: title,
                description: "Same tag overlap — suggest manual review for relationship"
                    .to_string(),
                confidence: 0.3,
                evidence: String::new(),
            });
        }

        CrossReferenceResult {
            paper_id: paper_id.to_string(),
            related_papers_found: items.len(),
            items,
            used_fallback: true,
            error: String::new(),
        }
    }

    pub fn parse_response(
        &self,
        raw: &str,
        candidates: &[(String, String, String)],
    ) -> Vec<CrossReferenceItem> {
        let mut items = Vec::new();

        for cap in RE_CROSS_REF.captures_iter(raw) {
            let pid = cap
                .get(1)
                .map(|m| m.as_str().trim_matches(|c| c == '[' || c == ']'))
                .unwrap_or("");
            let relation = cap
                .get(2)
                .map(|m| m.as_str().to_lowercase())
                .unwrap_or_default();

            if !["contradiction", "alignment", "extension", "unrelated"]
                .contains(&relation.as_str())
            {
                continue;
            }

            let title = candidates
                .iter()
                .find(|(p, _, _)| p == pid)
                .map(|(_, t, _)| t.clone())
                .unwrap_or_else(|| pid.to_string());

            items.push(CrossReferenceItem {
                relation: relation.clone(),
                target_paper_id: pid.to_string(),
                target_title: title,
                description: "(parsed from LLM analysis)".to_string(),
                confidence: 0.5,
                evidence: String::new(),
            });
        }

        if items.is_empty() {
            for (pid, title, _) in candidates.iter().take(3) {
                items.push(CrossReferenceItem {
                    relation: "alignment".to_string(),
                    target_paper_id: pid.clone(),
                    target_title: title.clone(),
                    description: "Related by shared tags".to_string(),
                    confidence: 0.3,
                    evidence: String::new(),
                });
            }
        }

        items
    }
}

impl Default for CrossReferencer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cross_referencer_new() {
        let cr = CrossReferencer::new();
        assert!(cr.papers.is_empty());
    }

    #[test]
    fn test_add_paper() {
        let mut cr = CrossReferencer::new();
        cr.add_paper("p1", "Title 1", "Abstract 1", vec!["tag1".to_string()]);
        assert_eq!(cr.papers.len(), 1);
    }

    #[test]
    fn test_analyze_empty_database() {
        let cr = CrossReferencer::new();
        let result = cr.analyze(
            "p1",
            "Title",
            "Abstract",
            "Body",
            Some(vec!["tag1".to_string()]),
            false,
        );
        assert!(result.error.contains("No database"));
    }

    #[test]
    fn test_analyze_fallback() {
        let mut cr = CrossReferencer::new();
        cr.add_paper("p2", "Title 2", "Abstract 2", vec!["tag1".to_string()]);
        let result = cr.analyze(
            "p1",
            "Title 1",
            "Abstract 1",
            "Body",
            Some(vec!["tag1".to_string()]),
            false,
        );
        assert!(result.used_fallback);
        assert!(!result.items.is_empty());
    }

    #[test]
    fn test_parse_response_empty() {
        let cr = CrossReferencer::new();
        let candidates = vec![(
            "p1".to_string(),
            "Title 1".to_string(),
            "Abstract 1".to_string(),
        )];
        let items = cr.parse_response("Some random text", &candidates);
        assert!(!items.is_empty());
    }

    #[test]
    fn test_parse_response_with_relation() {
        let cr = CrossReferencer::new();
        let candidates = vec![(
            "p1".to_string(),
            "Title 1".to_string(),
            "Abstract 1".to_string(),
        )];
        let items = cr.parse_response("[p1] (alignment)", &candidates);
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].relation, "alignment");
    }

    #[test]
    fn test_cross_reference_result_new() {
        let result = CrossReferenceResult::new("p1");
        assert_eq!(result.paper_id, "p1");
        assert!(result.error.is_empty());
    }

    #[test]
    fn test_cross_reference_result_with_error() {
        let result = CrossReferenceResult::with_error("p1", "test error");
        assert_eq!(result.paper_id, "p1");
        assert_eq!(result.error, "test error");
        assert!(result.used_fallback);
    }
}
