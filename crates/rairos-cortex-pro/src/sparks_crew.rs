//! SparksMatter-style multi-agent crew for materials discovery.
//!
//! This module provides a multi-stage crew workflow inspired by SparksMatter:
//! 1. **Ideation**: Hypothesis generation and critique
//! 2. **Planning**: Research plan creation and review
//! 3. **Execution**: Tool-based experimentation
//! 4. **Reporting**: Structured report generation
//!
//! ## Architecture
//!
//! ```text
//! User Query
//!     │
//!     ▼
//! ┌─────────────────┐
//! │     Manager     │ ◄─── Orchestrates workflow
//! └────────┬────────┘
//!          │
//!     ┌────┴────┬────────────┐
//!     ▼          ▼            ▼
//! ┌───────┐ ┌────────┐ ┌──────────┐
//! │Scientist│ │Scientist│ │ Planner  │ ◄─── Hypothesis/Plan agents
//! │   1   │ │   2    │ └────┬─────┘
//! └───────┘ └────┬───┘      │
//!                 ▼            ▼
//!           ┌──────────┐ ┌───────┐
//!           │ Critic   │ │Critic │ ◄─── Review agents
//!           └────┬─────┘ └───┬───┘
//!                │            │
//!                 └─────┬─────┘
//!                       ▼
//!                 ┌──────────┐
//!                 │Assistant │ ◄─── Execution agent
//!                 └────┬─────┘
//!                      │
//!                      ▼
//!               ┌──────────────┐
//!               │ MaterialTools │ ◄─── MP, CGCNN, etc.
//!               └──────────────┘
//! ```

use std::sync::Arc;
use std::time::Duration;
use async_trait::async_trait;
use tokio::time::sleep;
use crate::state::{CrewContext, ResearchState, Phase};
use crate::agent::{Agent, AgentConfig, AgentOutput, AgentRole};
use crate::crew::CrewResult;
use crate::error::CortexProError;
use crate::pipeline::Pipeline;

/// Retry configuration for agent calls
#[derive(Debug, Clone)]
pub struct RetryConfig {
    /// Maximum number of retry attempts
    pub max_attempts: u32,
    /// Base delay in milliseconds
    pub base_delay_ms: u64,
    /// Maximum delay in milliseconds
    pub max_delay_ms: u64,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_attempts: 3,
            base_delay_ms: 100,
            max_delay_ms: 5000,
        }
    }
}

impl RetryConfig {
    /// Create a new retry config with custom max attempts
    pub fn with_max_attempts(mut self, max: u32) -> Self {
        self.max_attempts = max;
        self
    }

    /// Calculate delay for a given attempt using exponential backoff
    fn delay_for_attempt(&self, attempt: u32) -> Duration {
        let exp_delay = self.base_delay_ms * 2u64.pow(attempt.saturating_sub(1));
        let delay = exp_delay.min(self.max_delay_ms);
        // Add jitter (0-25% of delay)
        let jitter = (delay as f64 * 0.25 * rand_simple()) as u64;
        Duration::from_millis(delay + jitter)
    }
}

/// Simple pseudo-random for jitter (0.0 to 1.0)
fn rand_simple() -> f64 {
    use std::time::Instant;
    let now = Instant::now();
    let nanos = now.elapsed().as_nanos();
    (nanos % 1000) as f64 / 1000.0
}

/// Callback for streaming agent progress updates
pub type StreamingCallback = Box<dyn Fn(AgentRole, &str) + Send + Sync>;

/// Phase result for tracking progress
#[derive(Debug, Clone)]
pub struct PhaseResult {
    /// Phase name
    pub phase: String,
    /// Whether the phase succeeded
    pub success: bool,
    /// Output content (if successful)
    pub output: Option<String>,
    /// Error message (if failed)
    pub error: Option<String>,
    /// Duration in milliseconds
    pub duration_ms: u64,
}

impl PhaseResult {
    /// Create a successful phase result
    pub fn success(phase: &str, output: String, duration_ms: u64) -> Self {
        Self {
            phase: phase.to_string(),
            success: true,
            output: Some(output),
            error: None,
            duration_ms,
        }
    }

    /// Create a failed phase result
    pub fn failure(phase: &str, error: String, duration_ms: u64) -> Self {
        Self {
            phase: phase.to_string(),
            success: false,
            output: None,
            error: Some(error),
            duration_ms,
        }
    }
}

/// Plan step for research execution.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PlanStep {
    /// Step number
    pub step: usize,
    /// Task description
    pub task: String,
    /// Tool name to use (empty if no tool needed)
    pub tool: String,
    /// Tool inputs
    pub inputs: std::collections::HashMap<String, serde_json::Value>,
    /// Step numbers this depends on
    #[serde(default)]
    pub depends_on: Vec<usize>,
}

/// Research plan.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Plan {
    /// Plan steps in order
    pub steps: Vec<PlanStep>,
    /// Scientific rationale
    pub rationale: String,
    /// Tasks beyond tool capabilities
    #[serde(default)]
    pub other_tasks: Vec<String>,
}

/// Execution result from running a plan.
#[derive(Debug, Clone)]
pub struct ExecutionResult {
    /// Whether execution succeeded
    pub success: bool,
    /// Results from each step
    pub step_results: Vec<StepResult>,
    /// Error message if failed
    pub error: Option<String>,
}

/// Result from a single plan step.
#[derive(Debug, Clone)]
pub struct StepResult {
    pub step: usize,
    pub success: bool,
    pub output: Option<serde_json::Value>,
    pub error: Option<String>,
}

/// Research report with structured sections.
#[derive(Debug, Clone)]
pub struct ResearchReport {
    /// Introduction section
    pub introduction: String,
    /// Methods section
    pub methods: String,
    /// Results section
    pub results: String,
    /// Outlook section
    pub outlook: String,
    /// Final compiled document
    pub full_document: String,
}

/// SparksMatter-style crew for materials discovery.
///
/// This crew orchestrates multiple agents through a structured workflow:
/// 1. Generate and critique hypothesis
/// 2. Create and review plan
/// 3. Execute with tools
/// 4. Generate report
pub struct SparksCrew {
    /// LLM client for agent calls
    llm: Arc<dyn rairos_llm::LlmClient>,
    /// Agents in the crew
    agents: Vec<Box<dyn Agent>>,
    /// Available tools (when rairos-tools is enabled)
    tools: Vec<Arc<dyn crate::tools::MaterialTool>>,
    /// Current crew context
    context: CrewContext,
    /// Maximum iterations per phase
    max_iterations: u32,
    /// Execution timeout per step
    step_timeout_secs: u64,
}

impl SparksCrew {
    /// Create a new SparksCrew.
    pub fn new(llm: Arc<dyn rairos_llm::LlmClient>) -> Self {
        Self {
            llm,
            agents: Vec::new(),
            tools: Vec::new(),
            context: CrewContext::default(),
            max_iterations: 5,
            step_timeout_secs: 300,
        }
    }

    /// Add an agent to the crew.
    pub fn add_agent(mut self, agent: Box<dyn Agent>) -> Self {
        self.agents.push(agent);
        self
    }

    /// Add a material tool.
    #[cfg(feature = "tools")]
    pub fn add_tool(mut self, tool: Arc<dyn crate::tools::MaterialTool>) -> Self {
        let tool_name = tool.name().to_string();
        self.tools.push(tool);
        self.context.tools.push(tool_name);
        self
    }

    /// Set maximum iterations per phase.
    pub fn with_max_iterations(mut self, max: u32) -> Self {
        self.max_iterations = max;
        self
    }

    /// Set step timeout in seconds.
    pub fn with_step_timeout(mut self, secs: u64) -> Self {
        self.step_timeout_secs = secs;
        self
    }

    /// Get a reference to the crew context (for testing).
    pub fn context(&self) -> &CrewContext {
        &self.context
    }

    /// Get a mutable reference to the crew context (for testing).
    pub fn context_mut(&mut self) -> &mut CrewContext {
        &mut self.context
    }

    /// Get whether the task has started.
    pub fn task_started(&self) -> bool {
        self.context.task_started
    }

    /// Get whether an idea has been created.
    pub fn idea_created(&self) -> bool {
        self.context.idea_created
    }

    /// Get whether the idea was approved.
    pub fn idea_approved(&self) -> bool {
        self.context.idea_approved
    }

    /// Get whether a plan has been created.
    pub fn plan_created(&self) -> bool {
        self.context.plan_created
    }

    /// Get whether the plan was approved.
    pub fn plan_approved(&self) -> bool {
        self.context.plan_approved
    }

    /// Get the current hypothesis.
    pub fn hypothesis(&self) -> Option<&str> {
        self.context.hypothesis.as_deref()
    }

    /// Get the current plan JSON.
    pub fn plan_json(&self) -> Option<&str> {
        self.context.plan.as_deref()
    }

    /// Find an agent by role.
    fn find_agent(&self, role: AgentRole) -> Option<&dyn Agent> {
        for agent in &self.agents {
            if agent.config().role == role {
                return Some(agent.as_ref());
            }
        }
        None
    }

    /// Find a tool by name.
    #[cfg(feature = "tools")]
    fn find_tool(&self, name: &str) -> Option<&dyn crate::tools::MaterialTool> {
        for tool in &self.tools {
            if tool.name() == name {
                return Some(tool.as_ref());
            }
        }
        None
    }

    /// Call an agent and get its output.
    async fn call_agent(&self, role: AgentRole, query: &str) -> Result<String, CortexProError> {
        self.call_agent_with_retry(role, query, &RetryConfig::default()).await
    }

    /// Call an agent with retry logic and exponential backoff.
    async fn call_agent_with_retry(
        &self,
        role: AgentRole,
        query: &str,
        retry_config: &RetryConfig,
    ) -> Result<String, CortexProError> {
        let agent = self
            .find_agent(role)
            .ok_or_else(|| CortexProError::AgentNotFound(format!("Agent {:?} not found", role)))?;

        let mut last_error = None;
        for attempt in 1..=retry_config.max_attempts {
            let state = ResearchState::new(query);
            match agent.execute(&state).await {
                Ok(output) => {
                    if output.errors.is_empty() {
                        return Ok(output.content);
                    } else {
                        last_error = Some(CortexProError::AgentError(output.errors.join("; ")));
                    }
                }
                Err(e) => {
                    last_error = Some(e);
                }
            }

            // Don't sleep after the last attempt
            if attempt < retry_config.max_attempts {
                let delay = retry_config.delay_for_attempt(attempt);
                sleep(delay).await;
            }
        }

        Err(last_error.unwrap_or_else(|| CortexProError::AgentError("Unknown error".to_string())))
    }

    /// Call an agent with additional intermediate data.
    async fn call_agent_with_intermediate(
        &self,
        role: AgentRole,
        query: &str,
        intermediate_key: &str,
        intermediate_value: &str,
    ) -> Result<String, CortexProError> {
        let agent = self
            .find_agent(role)
            .ok_or_else(|| CortexProError::AgentNotFound(format!("Agent {:?} not found", role)))?;

        let retry_config = RetryConfig::default();
        let mut last_error = None;

        for attempt in 1..=retry_config.max_attempts {
            let mut state = ResearchState::new(query);
            state.intermediate.insert(intermediate_key.to_string(), serde_json::json!(intermediate_value));

            match agent.execute(&state).await {
                Ok(output) => {
                    if output.errors.is_empty() {
                        return Ok(output.content);
                    } else {
                        last_error = Some(CortexProError::AgentError(output.errors.join("; ")));
                    }
                }
                Err(e) => {
                    last_error = Some(e);
                }
            }

            if attempt < retry_config.max_attempts {
                let delay = retry_config.delay_for_attempt(attempt);
                sleep(delay).await;
            }
        }

        Err(last_error.unwrap_or_else(|| CortexProError::AgentError("Unknown error".to_string())))
    }

    /// Phase 1: Ideation - generate and approve hypothesis.
    pub async fn run_ideation(&mut self, query: &str) -> Result<String, CortexProError> {
        self.context.task_started = true;
        self.context.query = Some(query.to_string());

        // Manager → Scientist1: Generate hypothesis
        let hypothesis = self.call_agent(AgentRole::Hypothesis, query).await?;
        self.context.hypothesis = Some(hypothesis.clone());

        // Scientist1 → Scientist2: Critic review
        for _ in 0..self.max_iterations {
            let feedback = self.call_agent_with_intermediate(
                AgentRole::HypothesisCritic,
                &hypothesis,
                "hypothesis",
                &hypothesis,
            ).await?;

            // Check if approved (feedback contains "approved" or "yes")
            if feedback.to_lowercase().contains("approved") ||
               feedback.to_lowercase().contains("yes") {
                self.context.idea_approved = true;
                self.context.idea_created = true;
                return Ok(hypothesis);
            }

            // Not approved - revise hypothesis
            let revised = self.call_agent(AgentRole::Hypothesis, &format!(
                "Revise based on feedback: {}\n\nOriginal: {}",
                feedback, hypothesis
            )).await?;

            self.context.hypothesis = Some(revised.clone());
        }

        // Max iterations reached - proceed anyway
        self.context.idea_created = true;
        Ok(hypothesis)
    }

    /// Phase 2: Planning - create and approve research plan.
    pub async fn run_planning(&mut self) -> Result<Plan, CortexProError> {
        let hypothesis = self
            .context
            .hypothesis
            .as_ref()
            .ok_or_else(|| CortexProError::AgentError("No hypothesis yet".to_string()))?;

        // Manager → Planner: Create plan
        let plan_json = self.call_agent(AgentRole::Planner, hypothesis).await?;

        // Parse plan JSON
        let plan: Plan = serde_json::from_str(&plan_json)
            .map_err(|e| CortexProError::AgentError(format!("Failed to parse plan: {}", e)))?;

        // Planner → Critic: Review plan
        for _ in 0..self.max_iterations {
            let feedback = self.call_agent_with_intermediate(
                AgentRole::PlanCritic,
                &plan_json,
                "plan",
                &plan_json,
            ).await?;

            if feedback.to_lowercase().contains("approved") ||
               feedback.to_lowercase().contains("yes") {
                self.context.plan_approved = true;
                self.context.plan_created = true;
                self.context.plan = Some(plan_json);
                return Ok(plan);
            }

            // Not approved - revise plan
            let revised_json = self.call_agent(AgentRole::Planner, &format!(
                "Revise based on feedback: {}\n\nOriginal: {}",
                feedback, plan_json
            )).await?;

            // Try to parse the revision
            if let Ok(revised_plan) = serde_json::from_str::<Plan>(&revised_json) {
                self.context.plan = Some(revised_json);
                return Ok(revised_plan);
            }
        }

        self.context.plan_created = true;
        self.context.plan = Some(plan_json);
        Ok(plan)
    }

    /// Phase 3: Execute plan using tools.
    #[cfg(feature = "tools")]
    pub async fn run_execution(&mut self, plan: &Plan) -> Result<ExecutionResult, CortexProError> {
        let mut step_results = Vec::new();

        for step in &plan.steps {
            if step.tool.is_empty() {
                // No tool - just code execution via Executor agent
                let output = self.call_agent(AgentRole::Executor, &step.task).await?;
                step_results.push(StepResult {
                    step: step.step,
                    success: true,
                    output: Some(serde_json::json!({ "output": output })),
                    error: None,
                });
            } else {
                // Execute tool
                let tool = self.find_tool(&step.tool)
                    .ok_or_else(|| CortexProError::AgentError(format!("Tool not found: {}", step.tool)))?;

                let params = crate::tools::ToolParams::new(step.tool.clone(), step.inputs.clone());
                match tool.execute(params).await {
                    Ok(tool_output) => {
                        step_results.push(StepResult {
                            step: step.step,
                            success: tool_output.success,
                            output: Some(tool_output.result),
                            error: tool_output.error,
                        });
                    }
                    Err(e) => {
                        step_results.push(StepResult {
                            step: step.step,
                            success: false,
                            output: None,
                            error: Some(e.to_string()),
                        });
                    }
                }
            }
        }

        let all_success = step_results.iter().all(|r| r.success);
        Ok(ExecutionResult {
            success: all_success,
            step_results,
            error: None,
        })
    }

    /// Phase 4: Generate research report.
    pub async fn run_reporting(&self) -> Result<ResearchReport, CortexProError> {
        // Use ReportWriter agent to generate sections
        // Pass hypothesis through the prompt
        let hypothesis = self.context.hypothesis.as_deref().unwrap_or("N/A");
        let prompt = format!(
            "Generate a research report for hypothesis: {}",
            hypothesis
        );

        let report_text = self.call_agent_with_intermediate(
            AgentRole::ReportWriter,
            &prompt,
            "hypothesis",
            hypothesis,
        ).await?;

        // Parse or structure the report
        // In a full implementation, this would call the agent multiple times
        // for each section (introduction, methods, results, outlook)
        Ok(ResearchReport {
            introduction: report_text.clone(),
            methods: "Methods section".to_string(),
            results: "Results section".to_string(),
            outlook: "Outlook section".to_string(),
            full_document: report_text,
        })
    }

    /// Run the complete SparksMatter-style workflow.
    pub async fn run(&mut self, query: &str) -> Result<CrewResult, CortexProError> {
        // Phase 1: Ideation
        let hypothesis = self.run_ideation(query).await?;

        // Phase 2: Planning
        let plan = self.run_planning().await?;

        // Phase 3: Execution (if tools enabled)
        #[cfg(feature = "tools")]
        let execution_result = self.run_execution(&plan).await?;

        // Phase 4: Reporting
        let _report = self.run_reporting().await?;

        let mut phases = vec![Phase::Planning, Phase::Searching];
        if self.context.idea_created {
            phases.push(Phase::Analyzing);
        }
        if self.context.plan_created {
            phases.push(Phase::Indexing);
        }

        Ok(CrewResult {
            success: true,
            state: ResearchState::new("sparks"),
            execution_time_ms: 0,
            phases_completed: phases,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_plan_step_serialization() {
        let step = PlanStep {
            step: 1,
            task: "Get structure from Materials Project".to_string(),
            tool: "download_structures_from_mp".to_string(),
            inputs: vec![("formula".to_string(), serde_json::json!("Bi2Te3"))]
                .into_iter()
                .collect(),
            depends_on: vec![],
        };

        let json = serde_json::to_string(&step).unwrap();
        assert!(json.contains("Bi2Te3"));
    }

    #[test]
    fn test_crew_context_default() {
        let ctx = CrewContext::default();
        assert!(!ctx.task_started);
        assert!(!ctx.idea_created);
    }
}
