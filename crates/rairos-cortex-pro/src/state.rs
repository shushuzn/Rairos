//! Research state management and context.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use chrono::{DateTime, Utc};

/// Research phase in the workflow
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Phase {
    /// Initial planning phase
    Planning,
    /// Paper search and retrieval phase
    Searching,
    /// Paper extraction and parsing phase
    Extracting,
    /// Gap analysis phase
    Analyzing,
    /// Citation graph building phase
    BuildingGraph,
    /// Vector indexing phase
    Indexing,
    /// Report writing phase
    Writing,
    /// QA/validation phase
    Validating,
    /// Research complete
    Complete,
    /// Research failed
    Failed,
}

impl Phase {
    pub fn as_str(&self) -> &'static str {
        match self {
            Phase::Planning => "planning",
            Phase::Searching => "searching",
            Phase::Extracting => "extracting",
            Phase::Analyzing => "analyzing",
            Phase::BuildingGraph => "building_graph",
            Phase::Indexing => "indexing",
            Phase::Writing => "writing",
            Phase::Validating => "validating",
            Phase::Complete => "complete",
            Phase::Failed => "failed",
        }
    }

    /// Get the next phase in the workflow
    pub fn next(&self) -> Option<Phase> {
        match self {
            Phase::Planning => Some(Phase::Searching),
            Phase::Searching => Some(Phase::Extracting),
            Phase::Extracting => Some(Phase::Analyzing),
            Phase::Analyzing => Some(Phase::BuildingGraph),
            Phase::BuildingGraph => Some(Phase::Indexing),
            Phase::Indexing => Some(Phase::Writing),
            Phase::Writing => Some(Phase::Validating),
            Phase::Validating => Some(Phase::Complete),
            Phase::Complete => None,
            Phase::Failed => None,
        }
    }

    /// Check if this is a terminal phase
    pub fn is_terminal(&self) -> bool {
        matches!(self, Phase::Complete | Phase::Failed)
    }
}

/// Research context passed through the pipeline
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResearchContext {
    /// Research topic/question
    pub topic: String,
    /// Keywords for search
    pub keywords: Vec<String>,
    /// Constraints/requirements
    pub constraints: Vec<String>,
    /// Creation timestamp
    pub created_at: DateTime<Utc>,
    /// Last update timestamp
    pub updated_at: DateTime<Utc>,
}

impl ResearchContext {
    pub fn new(topic: impl Into<String>) -> Self {
        let now = Utc::now();
        Self {
            topic: topic.into(),
            keywords: vec![],
            constraints: vec![],
            created_at: now,
            updated_at: now,
        }
    }

    pub fn with_keywords(mut self, keywords: Vec<String>) -> Self {
        self.keywords = keywords;
        self
    }

    pub fn with_constraints(mut self, constraints: Vec<String>) -> Self {
        self.constraints = constraints;
        self
    }

    pub fn touch(&mut self) {
        self.updated_at = Utc::now();
    }
}

/// Research state shared across all agents
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResearchState {
    /// Current phase
    pub phase: Phase,
    /// Research context
    pub context: ResearchContext,
    /// Papers discovered
    pub papers: Vec<PaperInfo>,
    /// Research gaps identified
    pub gaps: Vec<GapInfo>,
    /// Citations discovered
    pub citations: Vec<CitationInfo>,
    /// Agent outputs history
    pub outputs: Vec<crate::agent::AgentOutput>,
    /// Intermediate results
    pub intermediate: HashMap<String, serde_json::Value>,
    /// Validation results
    pub validations: Vec<ValidationResult>,
    /// Final report
    pub report: Option<String>,
    /// Errors encountered
    pub errors: Vec<String>,
    /// Iteration count
    pub iteration: usize,
}

impl ResearchState {
    pub fn new(topic: impl Into<String>) -> Self {
        Self {
            phase: Phase::Planning,
            context: ResearchContext::new(topic),
            papers: vec![],
            gaps: vec![],
            citations: vec![],
            outputs: vec![],
            intermediate: HashMap::new(),
            validations: vec![],
            report: None,
            errors: vec![],
            iteration: 0,
        }
    }

    /// Transition to a new phase
    pub fn set_phase(&mut self, phase: Phase) {
        self.phase = phase;
    }

    /// Add a paper to the state
    pub fn add_paper(&mut self, paper: PaperInfo) {
        self.papers.push(paper);
    }

    /// Add a gap to the state
    pub fn add_gap(&mut self, gap: GapInfo) {
        self.gaps.push(gap);
    }

    /// Add an agent output
    pub fn add_output(&mut self, output: crate::agent::AgentOutput) {
        self.outputs.push(output);
    }

    /// Add an error
    pub fn add_error(&mut self, error: impl Into<String>) {
        self.errors.push(error.into());
    }

    /// Check if research is complete
    pub fn is_complete(&self) -> bool {
        self.phase.is_terminal()
    }

    /// Get the number of papers found
    pub fn paper_count(&self) -> usize {
        self.papers.len()
    }

    /// Get the number of gaps identified
    pub fn gap_count(&self) -> usize {
        self.gaps.len()
    }

    /// Get validation summary
    pub fn validation_summary(&self) -> ValidationSummary {
        let total = self.validations.len();
        let passed = self.validations.iter().filter(|v| v.passed).count();
        ValidationSummary {
            total,
            passed,
            failed: total - passed,
        }
    }
}

/// Information about a discovered paper
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaperInfo {
    /// arXiv ID or DOI
    pub id: String,
    /// Paper title
    pub title: String,
    /// Authors
    pub authors: Vec<String>,
    /// Abstract
    pub abstract_text: Option<String>,
    /// arXiv ID (if applicable)
    pub arxiv_id: Option<String>,
    /// DOI (if applicable)
    pub doi: Option<String>,
    /// Citation count
    pub citation_count: usize,
    /// Topics/keywords
    pub topics: Vec<String>,
    /// PDF URL
    pub pdf_url: Option<String>,
    /// Year published
    pub year: Option<i32>,
}

/// Information about a research gap
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GapInfo {
    /// Gap ID
    pub id: String,
    /// Gap type
    pub gap_type: GapType,
    /// Description
    pub description: String,
    /// Evidence supporting this gap
    pub evidence: Vec<String>,
    /// Potential approaches to address
    pub approaches: Vec<String>,
    /// Confidence level (0.0 to 1.0)
    pub confidence: f32,
}

/// Type of research gap
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GapType {
    /// Method limitation
    MethodLimitation,
    /// Unexplored application
    UnexploredApplication,
    /// Contradiction in literature
    Contradiction,
    /// Evaluation gap
    EvaluationGap,
    /// Scalability issue
    ScalabilityIssue,
    /// Dataset gap
    DatasetGap,
}

/// Information about a citation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CitationInfo {
    /// Citing paper ID
    pub citing_id: String,
    /// Cited paper ID
    pub cited_id: String,
    /// Citation context
    pub context: Option<String>,
}

/// Validation result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationResult {
    /// Validator agent name
    pub validator: String,
    /// What was validated
    pub target: String,
    /// Whether validation passed
    pub passed: bool,
    /// Validation message
    pub message: String,
}

/// Validation summary
#[derive(Debug, Clone)]
pub struct ValidationSummary {
    pub total: usize,
    pub passed: usize,
    pub failed: usize,
}

/// Crew context variables (inspired by SparksMatter's context_variables).
///
/// This tracks the state of a multi-agent crew workflow:
/// - idea_created: Whether a hypothesis has been generated
/// - idea_approved: Whether the hypothesis has passed review
/// - plan_created: Whether a research plan has been created
/// - plan_approved: Whether the plan has been approved
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrewContext {
    /// Query/task that started this workflow
    pub query: Option<String>,
    /// Whether task execution has started
    pub task_started: bool,
    /// Whether a hypothesis/idea has been created
    pub idea_created: bool,
    /// Whether the hypothesis has been approved by critic
    pub idea_approved: bool,
    /// Whether a research plan has been created
    pub plan_created: bool,
    /// Whether the plan has been approved
    pub plan_approved: bool,
    /// Whether user has approved to proceed
    pub user_approved: bool,
    /// The generated hypothesis/idea
    pub hypothesis: Option<String>,
    /// The generated research plan
    pub plan: Option<String>,
    /// Explanation of the query
    pub query_explanation: Option<String>,
    /// List of available tools
    pub tools: Vec<String>,
}

impl Default for CrewContext {
    fn default() -> Self {
        Self {
            query: None,
            task_started: false,
            idea_created: false,
            idea_approved: false,
            plan_created: false,
            plan_approved: false,
            user_approved: false,
            hypothesis: None,
            plan: None,
            query_explanation: None,
            tools: Vec::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent::AgentRole;

    #[test]
    fn test_phase_next() {
        assert_eq!(Phase::Planning.next(), Some(Phase::Searching));
        assert_eq!(Phase::Searching.next(), Some(Phase::Extracting));
        assert_eq!(Phase::Extracting.next(), Some(Phase::Analyzing));
        assert_eq!(Phase::Analyzing.next(), Some(Phase::BuildingGraph));
        assert_eq!(Phase::BuildingGraph.next(), Some(Phase::Indexing));
        assert_eq!(Phase::Indexing.next(), Some(Phase::Writing));
        assert_eq!(Phase::Writing.next(), Some(Phase::Validating));
        assert_eq!(Phase::Validating.next(), Some(Phase::Complete));
        assert_eq!(Phase::Complete.next(), None);
        assert_eq!(Phase::Failed.next(), None);
    }

    #[test]
    fn test_phase_is_terminal() {
        assert!(!Phase::Planning.is_terminal());
        assert!(!Phase::Searching.is_terminal());
        assert!(!Phase::Extracting.is_terminal());
        assert!(!Phase::Analyzing.is_terminal());
        assert!(!Phase::BuildingGraph.is_terminal());
        assert!(!Phase::Indexing.is_terminal());
        assert!(!Phase::Writing.is_terminal());
        assert!(!Phase::Validating.is_terminal());
        assert!(Phase::Complete.is_terminal());
        assert!(Phase::Failed.is_terminal());
    }

    #[test]
    fn test_phase_as_str() {
        assert_eq!(Phase::Planning.as_str(), "planning");
        assert_eq!(Phase::Searching.as_str(), "searching");
        assert_eq!(Phase::Extracting.as_str(), "extracting");
        assert_eq!(Phase::Analyzing.as_str(), "analyzing");
        assert_eq!(Phase::BuildingGraph.as_str(), "building_graph");
        assert_eq!(Phase::Indexing.as_str(), "indexing");
        assert_eq!(Phase::Writing.as_str(), "writing");
        assert_eq!(Phase::Validating.as_str(), "validating");
        assert_eq!(Phase::Complete.as_str(), "complete");
        assert_eq!(Phase::Failed.as_str(), "failed");
    }

    #[test]
    fn test_research_state_new() {
        let state = ResearchState::new("machine learning optimization");
        assert_eq!(state.phase, Phase::Planning);
        assert_eq!(state.context.topic, "machine learning optimization");
        assert!(state.papers.is_empty());
        assert!(state.gaps.is_empty());
        assert!(state.citations.is_empty());
        assert!(state.outputs.is_empty());
        assert!(state.intermediate.is_empty());
        assert!(state.validations.is_empty());
        assert!(state.report.is_none());
        assert!(state.errors.is_empty());
        assert_eq!(state.iteration, 0);
    }

    #[test]
    fn test_research_state_is_complete() {
        let mut state = ResearchState::new("test topic");
        assert!(!state.is_complete());

        state.set_phase(Phase::Complete);
        assert!(state.is_complete());

        state.set_phase(Phase::Failed);
        assert!(state.is_complete());

        state.set_phase(Phase::Writing);
        assert!(!state.is_complete());
    }

    #[test]
    fn test_research_state_set_phase() {
        let mut state = ResearchState::new("test");
        assert_eq!(state.phase, Phase::Planning);

        state.set_phase(Phase::Searching);
        assert_eq!(state.phase, Phase::Searching);

        state.set_phase(Phase::Analyzing);
        assert_eq!(state.phase, Phase::Analyzing);
    }

    #[test]
    fn test_research_state_add_paper() {
        let mut state = ResearchState::new("test");
        let paper = PaperInfo {
            id: "paper1".to_string(),
            title: "Test Paper".to_string(),
            authors: vec!["Author 1".to_string()],
            abstract_text: Some("Abstract".to_string()),
            arxiv_id: Some("1234.5678".to_string()),
            doi: None,
            citation_count: 10,
            topics: vec!["ML".to_string()],
            pdf_url: None,
            year: Some(2024),
        };
        state.add_paper(paper);
        assert_eq!(state.paper_count(), 1);
        assert_eq!(state.papers[0].title, "Test Paper");
    }

    #[test]
    fn test_research_state_add_gap() {
        let mut state = ResearchState::new("test");
        let gap = GapInfo {
            id: "gap1".to_string(),
            gap_type: GapType::MethodLimitation,
            description: "Limited scalability".to_string(),
            evidence: vec!["Paper 1".to_string()],
            approaches: vec!["Approach A".to_string()],
            confidence: 0.8,
        };
        state.add_gap(gap);
        assert_eq!(state.gap_count(), 1);
        assert_eq!(state.gaps[0].gap_type, GapType::MethodLimitation);
    }

    #[test]
    fn test_research_state_add_output() {
        use crate::agent::AgentOutput;
        let mut state = ResearchState::new("test");
        let output = AgentOutput::success(
            AgentRole::Researcher,
            "agent1",
            "content".to_string(),
            100,
        );
        state.add_output(output);
        assert_eq!(state.outputs.len(), 1);
    }

    #[test]
    fn test_research_state_add_error() {
        let mut state = ResearchState::new("test");
        state.add_error("Test error");
        assert_eq!(state.errors.len(), 1);
        assert_eq!(state.errors[0], "Test error");
    }

    #[test]
    fn test_research_context_new() {
        let ctx = ResearchContext::new("AI research");
        assert_eq!(ctx.topic, "AI research");
        assert!(ctx.keywords.is_empty());
        assert!(ctx.constraints.is_empty());
    }

    #[test]
    fn test_research_context_builder() {
        let ctx = ResearchContext::new("AI research")
            .with_keywords(vec!["ML".to_string(), "NLP".to_string()])
            .with_constraints(vec!["Must be recent".to_string()]);
        assert_eq!(ctx.keywords, vec!["ML", "NLP"]);
        assert_eq!(ctx.constraints, vec!["Must be recent"]);
    }

    #[test]
    fn test_validation_summary() {
        let mut state = ResearchState::new("test");
        state.validations.push(ValidationResult {
            validator: "qa".to_string(),
            target: "report".to_string(),
            passed: true,
            message: "OK".to_string(),
        });
        state.validations.push(ValidationResult {
            validator: "qa".to_string(),
            target: "gaps".to_string(),
            passed: false,
            message: "Missing gaps".to_string(),
        });

        let summary = state.validation_summary();
        assert_eq!(summary.total, 2);
        assert_eq!(summary.passed, 1);
        assert_eq!(summary.failed, 1);
    }

    #[test]
    fn test_gap_type_as_str() {
        assert_eq!(format!("{:?}", GapType::MethodLimitation), "MethodLimitation");
        assert_eq!(format!("{:?}", GapType::UnexploredApplication), "UnexploredApplication");
        assert_eq!(format!("{:?}", GapType::Contradiction), "Contradiction");
        assert_eq!(format!("{:?}", GapType::EvaluationGap), "EvaluationGap");
        assert_eq!(format!("{:?}", GapType::ScalabilityIssue), "ScalabilityIssue");
        assert_eq!(format!("{:?}", GapType::DatasetGap), "DatasetGap");
    }
}
