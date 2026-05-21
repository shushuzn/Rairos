//! Crew orchestration for multi-agent collaboration.

use std::time::{Duration, Instant};
use crate::agent::{Agent, AgentConfig, AgentOutput, AgentRole};
use crate::state::{ResearchState, Phase};
use crate::error::CortexProError;

/// Configuration for a crew
#[derive(Debug, Clone)]
pub struct CrewConfig {
    /// Crew name
    pub name: String,
    /// Agents in the crew
    pub agents: Vec<AgentConfig>,
    /// Maximum iterations before halting
    pub max_iterations: usize,
    /// Timeout per agent execution
    pub agent_timeout: Duration,
    /// Crew-level timeout
    pub crew_timeout: Duration,
    /// Enable verbose logging
    pub verbose: bool,
}

impl Default for CrewConfig {
    fn default() -> Self {
        Self {
            name: "research_crew".to_string(),
            agents: vec![],
            max_iterations: 10,
            agent_timeout: Duration::from_secs(300),
            crew_timeout: Duration::from_secs(3600),
            verbose: false,
        }
    }
}

impl CrewConfig {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            ..Default::default()
        }
    }

    pub fn add_agent(mut self, role: AgentRole, name: impl Into<String>) -> Self {
        self.agents.push(AgentConfig::new(role, name));
        self
    }

    pub fn with_max_iterations(mut self, max: usize) -> Self {
        self.max_iterations = max;
        self
    }

    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.crew_timeout = timeout;
        self
    }
}

/// Result from crew execution
#[derive(Debug, Clone)]
pub struct CrewResult {
    /// Whether execution succeeded
    pub success: bool,
    /// Final state
    pub state: ResearchState,
    /// Execution time
    pub execution_time_ms: u64,
    /// Phases completed
    pub phases_completed: Vec<Phase>,
}

impl CrewResult {
    pub fn success(state: ResearchState, execution_time_ms: u64) -> Self {
        Self {
            success: true,
            state,
            execution_time_ms,
            phases_completed: vec![],
        }
    }

    pub fn failure(state: ResearchState, execution_time_ms: u64) -> Self {
        Self {
            success: false,
            state,
            execution_time_ms,
            phases_completed: vec![],
        }
    }
}

/// The research crew - a team of agents working together
pub struct ResearchCrew<A: Agent> {
    config: CrewConfig,
    agents: Vec<A>,
}

impl<A: Agent> ResearchCrew<A> {
    /// Create a new crew with configuration
    pub fn new(config: CrewConfig) -> Self {
        Self {
            config,
            agents: vec![],
        }
    }

    /// Add an agent to the crew
    pub fn add_agent(mut self, agent: A) -> Self {
        self.agents.push(agent);
        self
    }

    /// Get an agent by role
    pub fn get_agent(&self, role: AgentRole) -> Option<&A> {
        self.agents.iter().find(|a| a.role() == role)
    }

    /// Run the crew on a research topic
    pub async fn run(&self, topic: &str) -> Result<CrewResult, CortexProError> {
        let start = Instant::now();
        let mut state = ResearchState::new(topic);

        if self.config.verbose {
            tracing::info!("Crew '{}' starting research on: {}", self.config.name, topic);
        }

        // Execute phases in sequence
        let phases = [
            Phase::Planning,
            Phase::Searching,
            Phase::Extracting,
            Phase::Analyzing,
            Phase::BuildingGraph,
            Phase::Indexing,
            Phase::Writing,
            Phase::Validating,
        ];

        for phase in phases {
            if state.is_complete() {
                break;
            }

            state.set_phase(phase);

            if self.config.verbose {
                tracing::info!("Executing phase: {:?}", phase);
            }

            let phase_start = Instant::now();
            match self.execute_phase(phase, &state).await {
                Ok(output) => {
                    state.add_output(output);
                    if self.config.verbose {
                        tracing::info!(
                            "Phase {:?} completed in {:?}",
                            phase,
                            phase_start.elapsed()
                        );
                    }
                }
                Err(e) => {
                    state.add_error(format!("Phase {:?} failed: {}", phase, e));
                    state.set_phase(Phase::Failed);
                    break;
                }
            }

            // Check timeout
            if start.elapsed() > self.config.crew_timeout {
                state.add_error("Crew timeout exceeded");
                state.set_phase(Phase::Failed);
                break;
            }

            // Check max iterations
            state.iteration += 1;
            if state.iteration >= self.config.max_iterations {
                state.add_error("Max iterations exceeded");
                state.set_phase(Phase::Failed);
                break;
            }
        }

        let execution_time_ms = start.elapsed().as_millis() as u64;

        if !state.is_complete() {
            state.set_phase(Phase::Complete);
        }

        Ok(CrewResult {
            success: !state.errors.is_empty(),
            state,
            execution_time_ms,
            phases_completed: phases.into_iter().take_while(|p| !p.is_terminal()).collect(),
        })
    }

    /// Execute a single phase with the appropriate agent
    async fn execute_phase(&self, phase: Phase, state: &ResearchState) -> Result<AgentOutput, CortexProError> {
        let role = match phase {
            Phase::Planning => AgentRole::Researcher,
            Phase::Searching => AgentRole::Researcher,
            Phase::Extracting => AgentRole::Researcher,
            Phase::Analyzing => AgentRole::GapAnalyzer,
            Phase::BuildingGraph => AgentRole::CitationGraph,
            Phase::Indexing => AgentRole::VectorIndexer,
            Phase::Writing => AgentRole::ReportWriter,
            Phase::Validating => AgentRole::QaAgent,
            _ => return Err(CortexProError::PipelineError(format!("Unknown phase: {:?}", phase))),
        };

        let agent = self.get_agent(role)
            .ok_or_else(|| CortexProError::AgentNotFound(role.as_str().to_string()))?;

        let start = Instant::now();
        let output = agent.execute(state).await?;
        let execution_time_ms = start.elapsed().as_millis() as u64;

        Ok(AgentOutput {
            role,
            agent_name: agent.config().name.clone(),
            execution_time_ms,
            ..output
        })
    }
}

/// Builder for creating a research crew
pub struct CrewBuilder<A: Agent> {
    config: CrewConfig,
    agents: Vec<A>,
}

impl<A: Agent> CrewBuilder<A> {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            config: CrewConfig::new(name),
            agents: vec![],
        }
    }

    pub fn with_agent(mut self, agent: A) -> Self {
        self.agents.push(agent);
        self
    }

    pub fn with_max_iterations(mut self, max: usize) -> Self {
        self.config.max_iterations = max;
        self
    }

    pub fn build(self) -> ResearchCrew<A> {
        ResearchCrew {
            config: self.config,
            agents: self.agents,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn test_crew_config_new() {
        let config = CrewConfig::new("test_crew");
        assert_eq!(config.name, "test_crew");
        assert!(config.agents.is_empty());
        assert_eq!(config.max_iterations, 10);
        assert_eq!(config.agent_timeout, Duration::from_secs(300));
        assert_eq!(config.crew_timeout, Duration::from_secs(3600));
        assert!(!config.verbose);
    }

    #[test]
    fn test_crew_config_builder_pattern() {
        let config = CrewConfig::new("research_crew")
            .add_agent(AgentRole::Researcher, "researcher1")
            .add_agent(AgentRole::GapAnalyzer, "gap_analyzer")
            .with_max_iterations(5);

        assert_eq!(config.name, "research_crew");
        assert_eq!(config.agents.len(), 2);
        assert_eq!(config.agents[0].role, AgentRole::Researcher);
        assert_eq!(config.agents[0].name, "researcher1");
        assert_eq!(config.agents[1].role, AgentRole::GapAnalyzer);
        assert_eq!(config.agents[1].name, "gap_analyzer");
        assert_eq!(config.max_iterations, 5);
    }

    #[test]
    fn test_crew_config_add_agent_returns_self() {
        let config = CrewConfig::new("crew")
            .add_agent(AgentRole::ReportWriter, "writer");

        assert_eq!(config.agents.len(), 1);
        assert_eq!(config.agents[0].role, AgentRole::ReportWriter);
    }

    #[test]
    fn test_crew_config_with_timeout() {
        let config = CrewConfig::new("crew")
            .with_timeout(Duration::from_secs(7200));

        assert_eq!(config.crew_timeout, Duration::from_secs(7200));
    }

    #[test]
    fn test_crew_config_default() {
        let config = CrewConfig::default();
        assert_eq!(config.name, "research_crew");
        assert_eq!(config.max_iterations, 10);
        assert!(config.agents.is_empty());
    }

    #[test]
    fn test_crew_result_success() {
        let state = ResearchState::new("test");
        let result = CrewResult::success(state.clone(), 5000);

        assert!(result.success);
        assert_eq!(result.state.context.topic, "test");
        assert_eq!(result.execution_time_ms, 5000);
        assert!(result.phases_completed.is_empty());
    }

    #[test]
    fn test_crew_result_failure() {
        let state = ResearchState::new("test");
        let result = CrewResult::failure(state, 3000);

        assert!(!result.success);
        assert_eq!(result.execution_time_ms, 3000);
    }

    #[test]
    fn test_agent_role_as_str_in_crew_context() {
        // Verify AgentRole::as_str works correctly
        assert_eq!(AgentRole::Researcher.as_str(), "researcher");
        assert_eq!(AgentRole::GapAnalyzer.as_str(), "gap_analyzer");
        assert_eq!(AgentRole::CitationGraph.as_str(), "citation_graph");
        assert_eq!(AgentRole::VectorIndexer.as_str(), "vector_indexer");
        assert_eq!(AgentRole::ReportWriter.as_str(), "report_writer");
        assert_eq!(AgentRole::QaAgent.as_str(), "qa_agent");
    }
}
