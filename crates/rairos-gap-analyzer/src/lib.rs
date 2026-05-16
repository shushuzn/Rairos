//! rairos-gap-analyzer — Gap Analyzer re-export.

#![allow(clippy::should_implement_trait)]
//!
//! Ported from `llm/gap_analyzer.py`.

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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResearchGapV2 {
    pub gap_type: String,
    pub title: String,
    pub description: String,
    pub severity: String,
    #[serde(default)]
    pub paper_ids: Vec<String>,
    #[serde(default)]
    pub evidence: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GapAnalysisResultV2 {
    pub gaps: Vec<ResearchGapV2>,
    #[serde(default)]
    pub summary: String,
    #[serde(default)]
    pub recommendations: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GapAnalyzerV2 {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub gaps: Vec<ResearchGapV2>,
}

impl GapAnalyzerV2 {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            gaps: Vec::new(),
        }
    }

    pub fn add_gap(&mut self, gap: ResearchGapV2) {
        self.gaps.push(gap);
    }

    pub fn get_gaps(&self) -> &[ResearchGapV2] {
        &self.gaps
    }
}

pub fn render_gap_report(result: &GapAnalysisResultV2) -> String {
    let mut lines = vec!["# Gap Analysis Report".to_string(), String::new()];

    if !result.summary.is_empty() {
        lines.push(result.summary.clone());
        lines.push(String::new());
    }

    lines.push("## Identified Gaps".to_string());
    lines.push(String::new());

    for (i, gap) in result.gaps.iter().enumerate() {
        lines.push(format!("{}. **[{}]** {}", i + 1, gap.gap_type, gap.title));
        lines.push(format!("   {}", gap.description));
        if !gap.evidence.is_empty() {
            lines.push(format!("   Evidence: {}", gap.evidence.join(", ")));
        }
        lines.push(String::new());
    }

    if !result.recommendations.is_empty() {
        lines.push("## Recommendations".to_string());
        lines.push(String::new());
        for rec in &result.recommendations {
            lines.push(format!("- {}", rec));
        }
    }

    lines.join("\n")
}

pub fn render_combined_report(results: &[GapAnalysisResultV2]) -> String {
    let mut lines = vec!["# Combined Gap Analysis Report".to_string(), String::new()];

    let total_gaps: usize = results.iter().map(|r| r.gaps.len()).sum();
    lines.push(format!("Total gaps identified: {}\n", total_gaps));

    for (i, result) in results.iter().enumerate() {
        lines.push(format!("## Analysis {}", i + 1));
        lines.push(String::new());
        lines.push(render_gap_report(result));
        lines.push(String::new());
    }

    lines.join("\n")
}

pub const _GAP_TYPE_NAMES: &[&str] = &[
    "unexplored_application",
    "scalability_issue",
    "evaluation_gap",
    "method_limitation",
    "theoretical_gap",
    "reproducibility_gap",
    "robustness_gap",
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
    fn test_gap_analyzer_v2_new() {
        let analyzer = GapAnalyzerV2::new("Test Analyzer");
        assert_eq!(analyzer.name, "Test Analyzer");
        assert!(analyzer.gaps.is_empty());
    }

    #[test]
    fn test_gap_analyzer_v2_add_gap() {
        let mut analyzer = GapAnalyzerV2::new("Test");
        let gap = ResearchGapV2 {
            gap_type: "evaluation_gap".to_string(),
            title: "Missing benchmark".to_string(),
            description: "No standard benchmark exists".to_string(),
            severity: "high".to_string(),
            paper_ids: vec![],
            evidence: vec![],
        };
        analyzer.add_gap(gap);
        assert_eq!(analyzer.gaps.len(), 1);
    }

    #[test]
    fn test_render_gap_report_empty() {
        let result = GapAnalysisResultV2 {
            gaps: vec![],
            summary: String::new(),
            recommendations: vec![],
        };
        let report = render_gap_report(&result);
        assert!(report.contains("Gap Analysis Report"));
    }

    #[test]
    fn test_render_gap_report_with_gaps() {
        let result = GapAnalysisResultV2 {
            gaps: vec![ResearchGapV2 {
                gap_type: "evaluation_gap".to_string(),
                title: "Test Gap".to_string(),
                description: "Test description".to_string(),
                severity: "high".to_string(),
                paper_ids: vec!["paper1".to_string()],
                evidence: vec!["evidence1".to_string()],
            }],
            summary: "Summary text".to_string(),
            recommendations: vec!["Rec 1".to_string()],
        };
        let report = render_gap_report(&result);
        assert!(report.contains("Test Gap"));
        assert!(report.contains("evaluation_gap"));
        assert!(report.contains("Summary text"));
    }

    #[test]
    fn test_render_combined_report() {
        let results = vec![
            GapAnalysisResultV2 {
                gaps: vec![ResearchGapV2 {
                    gap_type: "method_limitation".to_string(),
                    title: "Gap 1".to_string(),
                    description: "Desc 1".to_string(),
                    severity: "medium".to_string(),
                    paper_ids: vec![],
                    evidence: vec![],
                }],
                summary: String::new(),
                recommendations: vec![],
            },
            GapAnalysisResultV2 {
                gaps: vec![],
                summary: String::new(),
                recommendations: vec![],
            },
        ];
        let report = render_combined_report(&results);
        assert!(report.contains("Combined Gap Analysis Report"));
        assert!(report.contains("Total gaps identified: 1"));
    }

    #[test]
    fn test_gap_type_names() {
        assert_eq!(_GAP_TYPE_NAMES.len(), 7);
        assert!(_GAP_TYPE_NAMES.contains(&"unexplored_application"));
        assert!(_GAP_TYPE_NAMES.contains(&"scalability_issue"));
    }
}
