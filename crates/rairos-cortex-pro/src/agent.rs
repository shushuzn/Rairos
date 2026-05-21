//! Agent definitions and implementations.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use crate::state::ResearchState;
use crate::error::CortexProError;

/// Agent role in the crew
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentRole {
    /// Research agent - searches and extracts papers
    Researcher,
    /// Gap analysis agent - identifies research gaps
    GapAnalyzer,
    /// Citation graph agent - builds citation networks
    CitationGraph,
    /// Vector indexing agent - indexes to vector DB
    VectorIndexer,
    /// Report writing agent - synthesizes results
    ReportWriter,
    /// QA/validation agent - validates results
    QaAgent,
}

impl AgentRole {
    pub fn as_str(&self) -> &'static str {
        match self {
            AgentRole::Researcher => "researcher",
            AgentRole::GapAnalyzer => "gap_analyzer",
            AgentRole::CitationGraph => "citation_graph",
            AgentRole::VectorIndexer => "vector_indexer",
            AgentRole::ReportWriter => "report_writer",
            AgentRole::QaAgent => "qa_agent",
        }
    }
}

/// Configuration for an agent
#[derive(Debug, Clone)]
pub struct AgentConfig {
    /// Agent role
    pub role: AgentRole,
    /// Agent name/identifier
    pub name: String,
    /// LLM model to use
    pub model: String,
    /// Temperature for generation
    pub temperature: f32,
    /// Maximum tokens per response
    pub max_tokens: u32,
    /// Whether to enable verbose logging
    pub verbose: bool,
}

impl Default for AgentConfig {
    fn default() -> Self {
        Self {
            role: AgentRole::Researcher,
            name: "agent".to_string(),
            model: "gpt-4o".to_string(),
            temperature: 0.7,
            max_tokens: 4096,
            verbose: false,
        }
    }
}

impl AgentConfig {
    pub fn new(role: AgentRole, name: impl Into<String>) -> Self {
        Self {
            role,
            name: name.into(),
            ..Default::default()
        }
    }

    pub fn with_model(mut self, model: &str) -> Self {
        self.model = model.to_string();
        self
    }

    pub fn with_temperature(mut self, temperature: f32) -> Self {
        self.temperature = temperature;
        self
    }
}

/// Output from an agent execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentOutput {
    /// Role of the agent that produced this output
    pub role: AgentRole,
    /// Agent name
    pub agent_name: String,
    /// Generated content
    pub content: String,
    /// Confidence score (0.0 to 1.0)
    pub confidence: f32,
    /// References/citations used
    pub references: Vec<String>,
    /// Errors encountered (if any)
    pub errors: Vec<String>,
    /// Execution time in milliseconds
    pub execution_time_ms: u64,
}

impl AgentOutput {
    pub fn success(
        role: AgentRole,
        agent_name: &str,
        content: String,
        execution_time_ms: u64,
    ) -> Self {
        Self {
            role,
            agent_name: agent_name.to_string(),
            content,
            confidence: 1.0,
            references: vec![],
            errors: vec![],
            execution_time_ms,
        }
    }

    pub fn with_confidence(mut self, confidence: f32) -> Self {
        self.confidence = confidence;
        self
    }

    pub fn with_references(mut self, references: Vec<String>) -> Self {
        self.references = references;
        self
    }

    pub fn with_error(mut self, error: String) -> Self {
        self.errors.push(error);
        self
    }
}

/// Trait for all agents
#[async_trait]
pub trait Agent: Send + Sync {
    /// Get the agent configuration
    fn config(&self) -> &AgentConfig;

    /// Get the agent role
    fn role(&self) -> AgentRole {
        self.config().role
    }

    /// Execute the agent on the given state
    async fn execute(&self, state: &ResearchState) -> Result<AgentOutput, CortexProError>;

    /// Validate the agent's output (for QA agents)
    async fn validate(&self, _output: &AgentOutput) -> Result<bool, CortexProError> {
        Ok(true) // Default: always valid
    }
}

/// Shared agent context for tool access
#[derive(Clone)]
pub struct AgentContext {
    /// LLM client
    pub llm: Arc<dyn rairos_llm::LlmClient>,
    /// Optional vector store
    pub vector_store: Option<Arc<dyn rairos_vector::client::VectorStore>>,
    /// Optional KG client
    pub kg_client: Option<Arc<rairos_kg_neo4j::Neo4jKgClient>>,
    /// Optional GraphRAG pipeline
    pub graphrag: Option<Arc<dyn GraphRagTrait>>,
}

/// Trait for GraphRAG operations (used by agents)
#[async_trait]
pub trait GraphRagTrait: Send + Sync {
    async fn query(&self, question: &str) -> Result<String, CortexProError>;
}

/// Adapter for rairos-graphrag pipeline
pub struct GraphRagAdapter {
    // In a full implementation, this would hold the actual GraphRagPipeline
}

impl Default for GraphRagAdapter {
    fn default() -> Self {
        Self::new()
    }
}

impl GraphRagAdapter {
    pub fn new() -> Self {
        Self {}
    }
}

#[async_trait]
impl GraphRagTrait for GraphRagAdapter {
    async fn query(&self, question: &str) -> Result<String, CortexProError> {
        Ok(format!("GraphRAG query: {}", question))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_agent_config_new() {
        let config = AgentConfig::new(AgentRole::Researcher, "test_agent");
        assert_eq!(config.role, AgentRole::Researcher);
        assert_eq!(config.name, "test_agent");
        assert_eq!(config.model, "gpt-4o");
        assert_eq!(config.temperature, 0.7);
        assert_eq!(config.max_tokens, 4096);
        assert!(!config.verbose);
    }

    #[test]
    fn test_agent_config_builder_pattern() {
        let config = AgentConfig::new(AgentRole::GapAnalyzer, "gap_agent")
            .with_model("gpt-4o-mini")
            .with_temperature(0.5);
        assert_eq!(config.role, AgentRole::GapAnalyzer);
        assert_eq!(config.name, "gap_agent");
        assert_eq!(config.model, "gpt-4o-mini");
        assert_eq!(config.temperature, 0.5);
    }

    #[test]
    fn test_agent_config_default() {
        let config = AgentConfig::default();
        assert_eq!(config.role, AgentRole::Researcher);
        assert_eq!(config.name, "agent");
        assert_eq!(config.model, "gpt-4o");
    }

    #[test]
    fn test_agent_output_success() {
        let output = AgentOutput::success(
            AgentRole::Researcher,
            "researcher_1",
            "Found 10 papers".to_string(),
            1500,
        );
        assert_eq!(output.role, AgentRole::Researcher);
        assert_eq!(output.agent_name, "researcher_1");
        assert_eq!(output.content, "Found 10 papers");
        assert_eq!(output.confidence, 1.0);
        assert!(output.references.is_empty());
        assert!(output.errors.is_empty());
        assert_eq!(output.execution_time_ms, 1500);
    }

    #[test]
    fn test_agent_output_builder_pattern() {
        let output = AgentOutput::success(
            AgentRole::GapAnalyzer,
            "gap_agent",
            "Identified 3 gaps".to_string(),
            2000,
        )
        .with_confidence(0.85)
        .with_references(vec!["paper1".to_string(), "paper2".to_string()])
        .with_error("Minor warning".to_string());

        assert_eq!(output.confidence, 0.85);
        assert_eq!(output.references, vec!["paper1", "paper2"]);
        assert_eq!(output.errors, vec!["Minor warning"]);
    }

    #[test]
    fn test_agent_role_as_str() {
        assert_eq!(AgentRole::Researcher.as_str(), "researcher");
        assert_eq!(AgentRole::GapAnalyzer.as_str(), "gap_analyzer");
        assert_eq!(AgentRole::CitationGraph.as_str(), "citation_graph");
        assert_eq!(AgentRole::VectorIndexer.as_str(), "vector_indexer");
        assert_eq!(AgentRole::ReportWriter.as_str(), "report_writer");
        assert_eq!(AgentRole::QaAgent.as_str(), "qa_agent");
    }
}
