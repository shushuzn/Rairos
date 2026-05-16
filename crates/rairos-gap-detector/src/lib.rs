//! rairos-gap-detector — Gap Detector re-export.

#![allow(clippy::should_implement_trait)]
//!
//! Ported from `llm/gap_detector.py`.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GapType {
    UnexploredApplication,
    ScalabilityIssue,
    EvaluationGap,
    MethodLimitation,
    TheoreticalGap,
    ReproducibilityGap,
    RobustnessGap,
}

impl GapType {
    pub fn as_str(&self) -> &'static str {
        match self {
            GapType::UnexploredApplication => "unexplored_application",
            GapType::ScalabilityIssue => "scalability_issue",
            GapType::EvaluationGap => "evaluation_gap",
            GapType::MethodLimitation => "method_limitation",
            GapType::TheoreticalGap => "theoretical_gap",
            GapType::ReproducibilityGap => "reproducibility_gap",
            GapType::RobustnessGap => "robustness_gap",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "unexplored_application" => Some(GapType::UnexploredApplication),
            "scalability_issue" => Some(GapType::ScalabilityIssue),
            "evaluation_gap" => Some(GapType::EvaluationGap),
            "method_limitation" => Some(GapType::MethodLimitation),
            "theoretical_gap" => Some(GapType::TheoreticalGap),
            "reproducibility_gap" => Some(GapType::ReproducibilityGap),
            "robustness_gap" => Some(GapType::RobustnessGap),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GapSeverity {
    Low,
    Medium,
    High,
    Critical,
}

impl GapSeverity {
    pub fn as_str(&self) -> &'static str {
        match self {
            GapSeverity::Low => "low",
            GapSeverity::Medium => "medium",
            GapSeverity::High => "high",
            GapSeverity::Critical => "critical",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "low" => Some(GapSeverity::Low),
            "medium" => Some(GapSeverity::Medium),
            "high" => Some(GapSeverity::High),
            "critical" => Some(GapSeverity::Critical),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResearchGap {
    pub gap_type: String,
    pub title: String,
    pub description: String,
    pub severity: String,
    #[serde(default)]
    pub evidence: Vec<String>,
}

impl ResearchGap {
    pub fn new(gap_type: &str, title: &str, description: &str, severity: &str) -> Self {
        Self {
            gap_type: gap_type.to_string(),
            title: title.to_string(),
            description: description.to_string(),
            severity: severity.to_string(),
            evidence: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResearchQuestion {
    pub question: String,
    pub gap_type: String,
    #[serde(default)]
    pub suggested_papers: Vec<String>,
}

impl ResearchQuestion {
    pub fn new(question: &str, gap_type: &str) -> Self {
        Self {
            question: question.to_string(),
            gap_type: gap_type.to_string(),
            suggested_papers: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GapAnalysisResult {
    pub gaps: Vec<ResearchGap>,
    #[serde(default)]
    pub questions: Vec<ResearchQuestion>,
    #[serde(default)]
    pub summary: String,
}

pub struct GapDetector {
    #[allow(dead_code)]
    name: String,
}

impl GapDetector {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
        }
    }

    pub fn detect_gaps(&self, _text: &str) -> GapAnalysisResult {
        GapAnalysisResult {
            gaps: Vec::new(),
            questions: Vec::new(),
            summary: String::new(),
        }
    }
}

pub const _GAP_DETECTION_SYSTEM_PROMPT: &str = "You are a research gap detection assistant.";

pub const _GAP_DETECTION_USER_PROMPT_TEMPLATE: &str =
    "Analyze the following research text and identify gaps:\n\n{text}";

pub const _QUESTION_GENERATION_SYSTEM_PROMPT: &str =
    "You are a research question generation assistant.";

pub const _QUESTION_GENERATION_USER_PROMPT_TEMPLATE: &str =
    "Based on the following gaps, generate research questions:\n\n{gaps}";

pub const _GAP_TYPE_PATTERNS: &[(&str, &[&str])] = &[
    (
        "unexplored_application",
        &["no prior work", "not yet explored", "not studied"],
    ),
    (
        "scalability_issue",
        &["scalability", "scale", "large-scale", "computational"],
    ),
    (
        "evaluation_gap",
        &[
            "no benchmark",
            "evaluation",
            "missing evaluation",
            "not evaluated",
        ],
    ),
    (
        "method_limitation",
        &["limitation", "cannot handle", "fails when", "weakness"],
    ),
    (
        "theoretical_gap",
        &["theoretical", "theory", "prove", "formal"],
    ),
    (
        "reproducibility_gap",
        &[
            "reproduce",
            "replication",
            "code not available",
            "open source",
        ],
    ),
    (
        "robustness_gap",
        &["robustness", "adversarial", "noise", "attack"],
    ),
];

pub const _GAP_QUESTION_TEMPLATES: &[(&str, &str)] = &[
    (
        "unexplored_application",
        "How can {topic} be applied to {domain}?",
    ),
    (
        "scalability_issue",
        "How does {topic} scale with {resource}?",
    ),
    ("evaluation_gap", "What benchmarks exist for {topic}?"),
    ("method_limitation", "What are the limitations of {method}?"),
    ("theoretical_gap", "Can we prove {property} for {topic}?"),
    ("reproducibility_gap", "How can {result} be reproduced?"),
    (
        "robustness_gap",
        "How robust is {method} to {perturbation}?",
    ),
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gap_type_as_str() {
        assert_eq!(
            GapType::UnexploredApplication.as_str(),
            "unexplored_application"
        );
        assert_eq!(GapType::ScalabilityIssue.as_str(), "scalability_issue");
        assert_eq!(GapType::EvaluationGap.as_str(), "evaluation_gap");
        assert_eq!(GapType::MethodLimitation.as_str(), "method_limitation");
        assert_eq!(GapType::TheoreticalGap.as_str(), "theoretical_gap");
        assert_eq!(GapType::ReproducibilityGap.as_str(), "reproducibility_gap");
        assert_eq!(GapType::RobustnessGap.as_str(), "robustness_gap");
    }

    #[test]
    fn test_gap_type_from_str() {
        assert_eq!(
            GapType::from_string("unexplored_application"),
            Some(GapType::UnexploredApplication)
        );
        assert_eq!(
            GapType::from_string("scalability_issue"),
            Some(GapType::ScalabilityIssue)
        );
        assert_eq!(GapType::from_string("invalid"), None);
    }

    #[test]
    fn test_gap_severity_as_str() {
        assert_eq!(GapSeverity::Low.as_str(), "low");
        assert_eq!(GapSeverity::Medium.as_str(), "medium");
        assert_eq!(GapSeverity::High.as_str(), "high");
        assert_eq!(GapSeverity::Critical.as_str(), "critical");
    }

    #[test]
    fn test_gap_severity_from_str() {
        assert_eq!(GapSeverity::from_str("low"), Some(GapSeverity::Low));
        assert_eq!(GapSeverity::from_str("medium"), Some(GapSeverity::Medium));
        assert_eq!(GapSeverity::from_str("high"), Some(GapSeverity::High));
        assert_eq!(
            GapSeverity::from_str("critical"),
            Some(GapSeverity::Critical)
        );
        assert_eq!(GapSeverity::from_str("invalid"), None);
    }

    #[test]
    fn test_research_gap_new() {
        let gap = ResearchGap::new(
            "evaluation_gap",
            "No benchmark",
            "Missing standard benchmark",
            "high",
        );
        assert_eq!(gap.gap_type, "evaluation_gap");
        assert_eq!(gap.title, "No benchmark");
        assert_eq!(gap.severity, "high");
        assert!(gap.evidence.is_empty());
    }

    #[test]
    fn test_research_question_new() {
        let q = ResearchQuestion::new("What is the benchmark?", "evaluation_gap");
        assert_eq!(q.question, "What is the benchmark?");
        assert_eq!(q.gap_type, "evaluation_gap");
        assert!(q.suggested_papers.is_empty());
    }

    #[test]
    fn test_gap_detector_new() {
        let detector = GapDetector::new("Test");
        assert_eq!(detector.name, "Test");
    }

    #[test]
    fn test_gap_detector_detect_gaps() {
        let detector = GapDetector::new("Test");
        let result = detector.detect_gaps("some text");
        assert!(result.gaps.is_empty());
        assert!(result.questions.is_empty());
    }

    #[test]
    fn test_gap_type_patterns_length() {
        assert_eq!(_GAP_TYPE_PATTERNS.len(), 7);
        for (gap_type, patterns) in _GAP_TYPE_PATTERNS {
            assert!(
                !patterns.is_empty(),
                "Gap type {} has no patterns",
                gap_type
            );
        }
    }

    #[test]
    fn test_gap_question_templates_length() {
        assert_eq!(_GAP_QUESTION_TEMPLATES.len(), 7);
    }

    #[test]
    fn test_gap_detection_prompts() {
        assert!(!_GAP_DETECTION_SYSTEM_PROMPT.is_empty());
        assert!(!_GAP_DETECTION_USER_PROMPT_TEMPLATE.is_empty());
        assert!(!_QUESTION_GENERATION_SYSTEM_PROMPT.is_empty());
        assert!(!_QUESTION_GENERATION_USER_PROMPT_TEMPLATE.is_empty());
    }
}
