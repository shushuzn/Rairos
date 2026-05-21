//! Tool implementations wrapping Rairos components
//!
//! Each tool wraps a rairos function and implements the TraitClaw Tool trait.

use async_trait::async_trait;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use traitclaw_core::prelude::*;

// ============================================================================
// Tool Input/Output Types
// ============================================================================

/// Input for arXiv search tool
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ArxivSearchInput {
    /// Search query (topic, keywords, etc.)
    pub query: String,
    /// Maximum number of results
    #[serde(default = "default_max_results")]
    pub max_results: usize,
}

fn default_max_results() -> usize {
    10
}

/// Output from arXiv search
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArxivSearchOutput {
    pub papers: Vec<PaperInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaperInfo {
    pub id: String,
    pub title: String,
    pub abstract_text: String,
    pub url: String,
}

/// Input for gap analysis tool
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct GapAnalysisInput {
    /// Research topic
    pub topic: String,
    /// Papers to analyze (JSON strings)
    pub papers: Vec<String>,
}

/// Output from gap analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GapAnalysisOutput {
    pub gaps: Vec<GapInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GapInfo {
    pub gap_type: String,
    pub title: String,
    pub description: String,
    pub matched_papers: Vec<String>,
}

// ============================================================================
// Tool Implementations
// ============================================================================

/// Search academic papers from arXiv
pub struct ArxivSearchTool;

impl ArxivSearchTool {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl Tool for ArxivSearchTool {
    type Input = ArxivSearchInput;
    type Output = ArxivSearchOutput;

    fn name(&self) -> &str {
        "arxiv_search"
    }

    fn description(&self) -> &str {
        "Search academic papers from arXiv by topic or keywords"
    }

    async fn execute(&self, input: Self::Input) -> traitclaw_core::Result<Self::Output> {
        // TODO: Wire up to rairos_parser::search_arxiv()
        // For now, return mock data
        Ok(ArxivSearchOutput {
            papers: vec![PaperInfo {
                id: "mock".to_string(),
                title: format!("Paper about {}", input.query),
                abstract_text: "Mock abstract".to_string(),
                url: format!("https://arxiv.org/abs/mock"),
            }],
        })
    }
}

/// Analyze papers to detect research gaps
pub struct GapAnalysisTool;

impl GapAnalysisTool {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl Tool for GapAnalysisTool {
    type Input = GapAnalysisInput;
    type Output = GapAnalysisOutput;

    fn name(&self) -> &str {
        "gap_analysis"
    }

    fn description(&self) -> &str {
        "Analyze papers to identify research gaps and opportunities"
    }

    async fn execute(&self, _input: Self::Input) -> traitclaw_core::Result<Self::Output> {
        // TODO: Wire up to GapAnalyzerV2
        // For now, return mock data
        Ok(GapAnalysisOutput {
            gaps: vec![GapInfo {
                gap_type: "method_limitation".to_string(),
                title: "Mock gap".to_string(),
                description: "Mock description".to_string(),
                matched_papers: vec![],
            }],
        })
    }
}

/// Generate research report
pub struct ReportGenerationTool;

impl ReportGenerationTool {
    pub fn new() -> Self {
        Self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ReportGenerationInput {
    pub topic: String,
    pub gaps: Vec<String>,
    pub papers: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReportGenerationOutput {
    pub report: String,
}

#[async_trait]
impl Tool for ReportGenerationTool {
    type Input = ReportGenerationInput;
    type Output = ReportGenerationOutput;

    fn name(&self) -> &str {
        "generate_report"
    }

    fn description(&self) -> &str {
        "Generate a structured research report from analyzed gaps"
    }

    async fn execute(&self, input: Self::Input) -> traitclaw_core::Result<Self::Output> {
        // TODO: Wire up to report generation
        Ok(ReportGenerationOutput {
            report: format!(
                "# Research Report: {}\n\n## Gaps:\n{}\n\n## Papers:\n{}",
                input.topic,
                input.gaps.join("\n- "),
                input.papers.join("\n- ")
            ),
        })
    }
}
