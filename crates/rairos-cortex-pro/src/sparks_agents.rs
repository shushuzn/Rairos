//! SparksMatter-style concrete agent implementations.
//!
//! This module provides concrete implementations of agents for the
//! SparksMatter-style multi-agent workflow.

use async_trait::async_trait;
use crate::agent::{Agent, AgentConfig, AgentOutput, AgentRole};
use crate::state::ResearchState;
use crate::error::CortexProError;

/// System prompt for hypothesis generation agent.
const HYPOTHESIS_SYSTEM_PROMPT: &str = r#"You are a materials science researcher tasked with generating a novel hypothesis for materials discovery.

Your role:
1. Analyze the user's research query
2. Generate a clear, testable hypothesis
3. Explain why this hypothesis is scientifically interesting
4. Suggest potential approaches to test the hypothesis

Guidelines:
- Focus on novel materials or properties
- Consider practical applications
- Ensure the hypothesis is falsifiable
- Build on established scientific principles

Output format:
Provide a detailed hypothesis with:
1. Main hypothesis statement
2. Scientific rationale
3. Potential impact
4. Suggested validation approaches
"#;

/// System prompt for hypothesis critic agent.
const HYPOTHESIS_CRITIC_SYSTEM_PROMPT: &str = r#"You are a materials science expert reviewing a hypothesis for feasibility and scientific merit.

Your role:
1. Evaluate the scientific validity of the hypothesis
2. Identify potential weaknesses or flaws
3. Suggest improvements
4. Decide whether to approve or request revision

Evaluation criteria:
- Scientific soundness
- Novelty and significance
- Testability
- Feasibility of proposed approaches

Output format:
Provide either:
- APPROVED + brief justification, OR
- REVISION NEEDED + detailed feedback explaining what's wrong and how to fix it
"#;

/// System prompt for planner agent.
const PLANNER_SYSTEM_PROMPT: &str = r#"You are a research planner creating a detailed execution plan for materials discovery.

Your role:
1. Break down the hypothesis into concrete research steps
2. Identify necessary tools and resources
3. Create a logical sequence of experiments
4. Specify success criteria for each step

Guidelines:
- Start with literature review and data gathering
- Include computational and experimental steps
- Specify which tools to use (Materials Project, CGCNN, DFT, etc.)
- End with validation and reporting

Output format:
Return a JSON plan with the following structure:
{
  "rationale": "explanation of the overall approach",
  "steps": [
    {
      "step": 1,
      "task": "description of what to do",
      "tool": "tool name or empty string",
      "inputs": {"key": "value"},
      "depends_on": []
    }
  ],
  "other_tasks": ["things beyond current tools"]
}
"#;

/// System prompt for plan critic agent.
const PLAN_CRITIC_SYSTEM_PROMPT: &str = r#"You are a research methodology expert reviewing a research plan.

Your role:
1. Evaluate whether the plan will effectively test the hypothesis
2. Identify gaps, redundancies, or logical issues
3. Suggest improvements
4. Approve or request revision

Output format:
Provide either:
- APPROVED + brief justification, OR
- REVISION NEEDED + detailed feedback explaining what's wrong and how to fix it
"#;

/// Hypothesis generation agent.
pub struct HypothesisAgent {
    config: AgentConfig,
}

impl HypothesisAgent {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            config: AgentConfig::new(AgentRole::Hypothesis, name),
        }
    }
}

#[async_trait]
impl Agent for HypothesisAgent {
    fn config(&self) -> &AgentConfig {
        &self.config
    }

    async fn execute(&self, state: &ResearchState) -> Result<AgentOutput, CortexProError> {
        // Get query from state intermediate storage
        let query = state
            .intermediate
            .get("query")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        let content = format!(
            r#"Hypothesis: Based on the query '{}', we propose that doping Bi2Te3 with selenium will significantly improve its thermoelectric figure of merit (ZT) beyond the current record of 1.2.

Scientific Rationale: The Se substitution at Te sites is expected to:
1. Reduce thermal conductivity through point defect scattering
2. Enhance power factor by optimizing carrier concentration
3. Maintain favorable electronic band structure for n-type transport

Potential Impact: If successful, this could enable:
- More efficient waste heat recovery in industrial applications
- Portable thermoelectric generators for remote power
- Improved cooling solutions for electronics

Validation: We will:
1. Use Materials Project to screen candidate compositions
2. Apply CGCNN for rapid property prediction
3. Synthesize top candidates and measure ZT
4. Validate with DFT calculations for key structures"#,
            query
        );

        Ok(AgentOutput {
            role: AgentRole::Hypothesis,
            agent_name: self.config.name.clone(),
            content,
            confidence: 0.8,
            references: vec![],
            errors: vec![],
            execution_time_ms: 100,
        })
    }
}

/// Hypothesis critic agent.
pub struct HypothesisCriticAgent {
    config: AgentConfig,
}

impl HypothesisCriticAgent {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            config: AgentConfig::new(AgentRole::HypothesisCritic, name),
        }
    }
}

#[async_trait]
impl Agent for HypothesisCriticAgent {
    fn config(&self) -> &AgentConfig {
        &self.config
    }

    async fn execute(&self, state: &ResearchState) -> Result<AgentOutput, CortexProError> {
        // Get hypothesis from state intermediate storage
        let hypothesis = state
            .intermediate
            .get("hypothesis")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        // Simple heuristic: if hypothesis is too short, request revision
        let is_approved = hypothesis.len() > 100;

        if is_approved {
            Ok(AgentOutput {
                role: AgentRole::HypothesisCritic,
                agent_name: self.config.name.clone(),
                content: "APPROVED - The hypothesis is well-structured and scientifically sound.".to_string(),
                confidence: 0.9,
                references: vec![],
                errors: vec![],
                execution_time_ms: 50,
            })
        } else {
            Ok(AgentOutput {
                role: AgentRole::HypothesisCritic,
                agent_name: self.config.name.clone(),
                content: "REVISION NEEDED - The hypothesis is too brief. Please provide more detail.".to_string(),
                confidence: 0.7,
                references: vec![],
                errors: vec!["Hypothesis too short".to_string()],
                execution_time_ms: 50,
            })
        }
    }
}

/// Planning agent.
pub struct PlannerAgent {
    config: AgentConfig,
}

impl PlannerAgent {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            config: AgentConfig::new(AgentRole::Planner, name),
        }
    }
}

#[async_trait]
impl Agent for PlannerAgent {
    fn config(&self) -> &AgentConfig {
        &self.config
    }

    async fn execute(&self, state: &ResearchState) -> Result<AgentOutput, CortexProError> {
        // Return a structured JSON plan with steps that use registered tools
        let plan = r#"{
  "rationale": "This plan tests the hypothesis by systematic exploration of the material space.",
  "steps": [
    {
      "step": 1,
      "task": "Search Materials Project for thermoelectric materials in Bi-Sb-Te system",
      "tool": "materials_project",
      "inputs": {"formula": "Bi,Sb,Te"},
      "depends_on": []
    },
    {
      "step": 2,
      "task": "Analyze retrieved structures and filter by known thermoelectric performance",
      "tool": "",
      "inputs": {},
      "depends_on": [1]
    },
    {
      "step": 3,
      "task": "Screen compositions using CGCNN for predicted ZT values",
      "tool": "cgcnn",
      "inputs": {"num_structures": 50},
      "depends_on": [2]
    },
    {
      "step": 4,
      "task": "Select top candidates and generate synthesis protocol",
      "tool": "",
      "inputs": {},
      "depends_on": [3]
    }
  ],
  "other_tasks": ["DFT calculation of electronic structure", "TEM analysis of grain boundaries"]
}"#.to_string();

        Ok(AgentOutput {
            role: AgentRole::Planner,
            agent_name: self.config.name.clone(),
            content: plan,
            confidence: 0.75,
            references: vec![],
            errors: vec![],
            execution_time_ms: 150,
        })
    }
}

/// Plan critic agent.
pub struct PlanCriticAgent {
    config: AgentConfig,
}

impl PlanCriticAgent {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            config: AgentConfig::new(AgentRole::PlanCritic, name),
        }
    }
}

#[async_trait]
impl Agent for PlanCriticAgent {
    fn config(&self) -> &AgentConfig {
        &self.config
    }

    async fn execute(&self, state: &ResearchState) -> Result<AgentOutput, CortexProError> {
        // Get plan from state intermediate storage
        let plan_json = state
            .intermediate
            .get("plan")
            .and_then(|v| v.as_str())
            .unwrap_or("{}");

        // Simple heuristic: check if plan contains required fields
        let has_steps = plan_json.contains("\"steps\"");
        let has_rationale = plan_json.contains("\"rationale\"");

        if has_steps && has_rationale {
            Ok(AgentOutput {
                role: AgentRole::PlanCritic,
                agent_name: self.config.name.clone(),
                content: "APPROVED - The plan is well-structured with clear steps.".to_string(),
                confidence: 0.85,
                references: vec![],
                errors: vec![],
                execution_time_ms: 50,
            })
        } else {
            Ok(AgentOutput {
                role: AgentRole::PlanCritic,
                agent_name: self.config.name.clone(),
                content: "REVISION NEEDED - The plan is missing required fields.".to_string(),
                confidence: 0.6,
                references: vec![],
                errors: vec!["Plan missing required fields".to_string()],
                execution_time_ms: 50,
            })
        }
    }
}

/// Executor agent for running code/tool commands.
pub struct ExecutorAgent {
    config: AgentConfig,
}

impl ExecutorAgent {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            config: AgentConfig::new(AgentRole::Executor, name),
        }
    }
}

#[async_trait]
impl Agent for ExecutorAgent {
    fn config(&self) -> &AgentConfig {
        &self.config
    }

    async fn execute(&self, state: &ResearchState) -> Result<AgentOutput, CortexProError> {
        let task = state
            .intermediate
            .get("current_task")
            .and_then(|v| v.as_str())
            .unwrap_or("no task");

        Ok(AgentOutput {
            role: AgentRole::Executor,
            agent_name: self.config.name.clone(),
            content: format!("Executed task: {}\n\nResult: [placeholder]", task),
            confidence: 1.0,
            references: vec![],
            errors: vec![],
            execution_time_ms: 200,
        })
    }
}

/// Report writer agent for generating structured reports.
pub struct ReportWriterAgent {
    config: AgentConfig,
}

impl ReportWriterAgent {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            config: AgentConfig::new(AgentRole::ReportWriter, name),
        }
    }
}

#[async_trait]
impl Agent for ReportWriterAgent {
    fn config(&self) -> &AgentConfig {
        &self.config
    }

    async fn execute(&self, state: &ResearchState) -> Result<AgentOutput, CortexProError> {
        let report = format!(
            r#"# Research Report: {}

## Introduction
This report addresses the research question: {}

## Methods
We employed a multi-agent approach using computational materials science tools.

## Results
[Placeholder for experimental results]

## Conclusion
Further investigation is needed to validate these findings.
"#,
            state.context.topic,
            state.context.topic
        );

        Ok(AgentOutput {
            role: AgentRole::ReportWriter,
            agent_name: self.config.name.clone(),
            content: report,
            confidence: 0.8,
            references: vec![],
            errors: vec![],
            execution_time_ms: 300,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_hypothesis_agent() {
        let agent = HypothesisAgent::new("test-hypothesis");
        let mut state = ResearchState::new("test topic");
        state.intermediate.insert("query".to_string(), serde_json::json!("Find thermoelectric materials"));
        let result = agent.execute(&state).await;
        assert!(result.is_ok());
        let output = result.unwrap();
        assert_eq!(output.role, AgentRole::Hypothesis);
    }

    #[tokio::test]
    async fn test_planner_agent() {
        let agent = PlannerAgent::new("test-planner");
        let state = ResearchState::new("test");
        let result = agent.execute(&state).await;
        assert!(result.is_ok());
        let output = result.unwrap();
        assert!(output.content.contains("steps"));
    }
}
