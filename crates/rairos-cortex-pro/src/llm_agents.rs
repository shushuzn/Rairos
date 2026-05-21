//! LLM-based agent implementations for SparksCrew.
//!
//! These agents use real LLM calls via the LlmClient trait instead of mock responses.
//! They can be configured with system prompts and use the memory bank for context.

use std::sync::Arc;
use std::time::Instant;
use async_trait::async_trait;
use crate::agent::{Agent, AgentConfig, AgentOutput, AgentRole};
use crate::state::ResearchState;
use crate::error::CortexProError;
use crate::memory::MemoryBank;
use rairos_llm::{LlmClient, Message};

/// Hypothesis generation agent using real LLM
pub struct LlmHypothesisAgent {
    config: AgentConfig,
    llm: Arc<dyn LlmClient>,
    system_prompt: String,
    model: String,
    memory: Option<Arc<MemoryBank>>,
}

impl LlmHypothesisAgent {
    pub fn new(
        name: impl Into<String>,
        llm: Arc<dyn LlmClient>,
        model: impl Into<String>,
    ) -> Self {
        let system_prompt = r#"You are a materials science researcher tasked with generating a novel hypothesis for materials discovery.

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
4. Suggested validation approaches"#.to_string();

        Self {
            config: AgentConfig::new(AgentRole::Hypothesis, name),
            llm,
            system_prompt,
            model: model.into(),
            memory: None,
        }
    }

    pub fn with_memory(mut self, memory: Arc<MemoryBank>) -> Self {
        self.memory = Some(memory);
        self
    }

    pub fn with_system_prompt(mut self, prompt: impl Into<String>) -> Self {
        self.system_prompt = prompt.into();
        self
    }

    fn build_prompt(&self, query: &str) -> String {
        let mut prompt = format!("Research Query: {}\n\n", query);

        // Add context from memory if available
        if let Some(ref memory) = self.memory {
            let active = memory.get_active_directions();
            let failed = memory.get_failed_directions();

            if !active.is_empty() {
                prompt.push_str("Previous active research directions:\n");
                for dir in &active {
                    prompt.push_str(&format!("- {}\n", dir));
                }
                prompt.push('\n');
            }

            if !failed.is_empty() {
                prompt.push_str("Directions to avoid (failed):\n");
                for dir in &failed {
                    prompt.push_str(&format!("- {}\n", dir));
                }
                prompt.push('\n');
            }
        }

        prompt.push_str("Generate a novel hypothesis based on the research query above.");
        prompt
    }
}

#[async_trait]
impl Agent for LlmHypothesisAgent {
    fn config(&self) -> &AgentConfig {
        &self.config
    }

    async fn execute(&self, state: &ResearchState) -> Result<AgentOutput, CortexProError> {
        let start = Instant::now();

        let query = state
            .intermediate
            .get("query")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        let user_prompt = self.build_prompt(query);

        let messages = vec![
            Message {
                role: "system".to_string(),
                content: self.system_prompt.clone(),
            },
            Message {
                role: "user".to_string(),
                content: user_prompt,
            },
        ];

        let response = self
            .llm
            .complete(messages, &self.model, 0.7, 4096)
            .await
            .map_err(|e| CortexProError::AgentError(e.to_string()))?;

        let content = response.content().unwrap_or("No response").to_string();

        let execution_time_ms = start.elapsed().as_millis() as u64;

        Ok(AgentOutput {
            role: self.config.role,
            agent_name: self.config.name.clone(),
            content,
            confidence: 0.8,
            references: vec![],
            errors: vec![],
            execution_time_ms,
        })
    }
}

/// Hypothesis critic agent using real LLM
pub struct LlmHypothesisCriticAgent {
    config: AgentConfig,
    llm: Arc<dyn LlmClient>,
    system_prompt: String,
    model: String,
}

impl LlmHypothesisCriticAgent {
    pub fn new(
        name: impl Into<String>,
        llm: Arc<dyn LlmClient>,
        model: impl Into<String>,
    ) -> Self {
        let system_prompt = r#"You are a materials science expert reviewing a hypothesis for feasibility and scientific merit.

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
- REVISION NEEDED + detailed feedback explaining what's wrong and how to fix it"#.to_string();

        Self {
            config: AgentConfig::new(AgentRole::HypothesisCritic, name),
            llm,
            system_prompt,
            model: model.into(),
        }
    }

    pub fn with_system_prompt(mut self, prompt: impl Into<String>) -> Self {
        self.system_prompt = prompt.into();
        self
    }
}

#[async_trait]
impl Agent for LlmHypothesisCriticAgent {
    fn config(&self) -> &AgentConfig {
        &self.config
    }

    async fn execute(&self, state: &ResearchState) -> Result<AgentOutput, CortexProError> {
        let start = Instant::now();

        let hypothesis = state
            .intermediate
            .get("hypothesis")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        let user_prompt = format!(
            "Review this hypothesis:\n\n{}\n\nProvide your evaluation:",
            hypothesis
        );

        let messages = vec![
            Message {
                role: "system".to_string(),
                content: self.system_prompt.clone(),
            },
            Message {
                role: "user".to_string(),
                content: user_prompt,
            },
        ];

        let response = self
            .llm
            .complete(messages, &self.model, 0.3, 2048)
            .await
            .map_err(|e| CortexProError::AgentError(e.to_string()))?;

        let content = response.content().unwrap_or("No response").to_string();

        let execution_time_ms = start.elapsed().as_millis() as u64;

        // Check if approved
        let is_approved = content.to_lowercase().contains("approved");
        let confidence = if is_approved { 0.9 } else { 0.6 };

        Ok(AgentOutput {
            role: self.config.role,
            agent_name: self.config.name.clone(),
            content,
            confidence,
            references: vec![],
            errors: vec![],
            execution_time_ms,
        })
    }
}

/// Planner agent using real LLM
pub struct LlmPlannerAgent {
    config: AgentConfig,
    llm: Arc<dyn LlmClient>,
    system_prompt: String,
    model: String,
    tools_description: String,
}

impl LlmPlannerAgent {
    pub fn new(
        name: impl Into<String>,
        llm: Arc<dyn LlmClient>,
        model: impl Into<String>,
    ) -> Self {
        let system_prompt = r#"You are a research planner creating a detailed execution plan for materials discovery.

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
}"#.to_string();

        let tools_description = r#"
Available tools:
- materials_project: Search and download crystal structures from Materials Project
- cgcnn: Machine learning property prediction using Crystal Graph CNN
- mattergen: Generate novel crystal structures
- mattersim: Simulate material properties

Return ONLY valid JSON, no markdown formatting."#.to_string();

        Self {
            config: AgentConfig::new(AgentRole::Planner, name),
            llm,
            system_prompt,
            model: model.into(),
            tools_description,
        }
    }

    pub fn with_tools_description(mut self, desc: impl Into<String>) -> Self {
        self.tools_description = desc.into();
        self
    }
}

#[async_trait]
impl Agent for LlmPlannerAgent {
    fn config(&self) -> &AgentConfig {
        &self.config
    }

    async fn execute(&self, state: &ResearchState) -> Result<AgentOutput, CortexProError> {
        let start = Instant::now();

        let hypothesis = state
            .intermediate
            .get("hypothesis")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        let user_prompt = format!(
            "Create a research plan for this hypothesis:\n\n{}\n\n{}",
            hypothesis,
            self.tools_description
        );

        let messages = vec![
            Message {
                role: "system".to_string(),
                content: self.system_prompt.clone(),
            },
            Message {
                role: "user".to_string(),
                content: user_prompt,
            },
        ];

        let response = self
            .llm
            .complete(messages, &self.model, 0.5, 4096)
            .await
            .map_err(|e| CortexProError::AgentError(e.to_string()))?;

        let content = response.content().unwrap_or("No response").to_string();

        let execution_time_ms = start.elapsed().as_millis() as u64;

        Ok(AgentOutput {
            role: self.config.role,
            agent_name: self.config.name.clone(),
            content,
            confidence: 0.75,
            references: vec![],
            errors: vec![],
            execution_time_ms,
        })
    }
}

/// Plan critic agent using real LLM
pub struct LlmPlanCriticAgent {
    config: AgentConfig,
    llm: Arc<dyn LlmClient>,
    system_prompt: String,
    model: String,
}

impl LlmPlanCriticAgent {
    pub fn new(
        name: impl Into<String>,
        llm: Arc<dyn LlmClient>,
        model: impl Into<String>,
    ) -> Self {
        let system_prompt = r#"You are a research methodology expert reviewing a research plan.

Your role:
1. Evaluate whether the plan will effectively test the hypothesis
2. Identify gaps, redundancies, or logical issues
3. Suggest improvements
4. Approve or request revision

Output format:
Provide either:
- APPROVED + brief justification, OR
- REVISION NEEDED + detailed feedback explaining what's wrong and how to fix it"#.to_string();

        Self {
            config: AgentConfig::new(AgentRole::PlanCritic, name),
            llm,
            system_prompt,
            model: model.into(),
        }
    }
}

#[async_trait]
impl Agent for LlmPlanCriticAgent {
    fn config(&self) -> &AgentConfig {
        &self.config
    }

    async fn execute(&self, state: &ResearchState) -> Result<AgentOutput, CortexProError> {
        let start = Instant::now();

        let plan_json = state
            .intermediate
            .get("plan")
            .and_then(|v| v.as_str())
            .unwrap_or("{}");

        let user_prompt = format!(
            "Review this research plan:\n\n{}\n\nProvide your evaluation:",
            plan_json
        );

        let messages = vec![
            Message {
                role: "system".to_string(),
                content: self.system_prompt.clone(),
            },
            Message {
                role: "user".to_string(),
                content: user_prompt,
            },
        ];

        let response = self
            .llm
            .complete(messages, &self.model, 0.3, 2048)
            .await
            .map_err(|e| CortexProError::AgentError(e.to_string()))?;

        let content = response.content().unwrap_or("No response").to_string();

        let execution_time_ms = start.elapsed().as_millis() as u64;

        let is_approved = content.to_lowercase().contains("approved");
        let confidence = if is_approved { 0.85 } else { 0.6 };

        Ok(AgentOutput {
            role: self.config.role,
            agent_name: self.config.name.clone(),
            content,
            confidence,
            references: vec![],
            errors: vec![],
            execution_time_ms,
        })
    }
}

/// Report writer agent using real LLM
pub struct LlmReportWriterAgent {
    config: AgentConfig,
    llm: Arc<dyn LlmClient>,
    system_prompt: String,
    model: String,
}

impl LlmReportWriterAgent {
    pub fn new(
        name: impl Into<String>,
        llm: Arc<dyn LlmClient>,
        model: impl Into<String>,
    ) -> Self {
        let system_prompt = r#"You are a research report writer specializing in materials science.

Your role:
1. Synthesize research findings into a coherent report
2. Structure the report with Introduction, Methods, Results, Discussion, Conclusion
3. Highlight key findings and their implications
4. Identify limitations and future work

Output format:
Markdown report with the following sections:
1. Introduction - Research context and objectives
2. Methods - Computational and experimental approaches
3. Results - Key findings with supporting data
4. Discussion - Interpretation and implications
5. Conclusion - Summary and future directions"#.to_string();

        Self {
            config: AgentConfig::new(AgentRole::ReportWriter, name),
            llm,
            system_prompt,
            model: model.into(),
        }
    }
}

#[async_trait]
impl Agent for LlmReportWriterAgent {
    fn config(&self) -> &AgentConfig {
        &self.config
    }

    async fn execute(&self, state: &ResearchState) -> Result<AgentOutput, CortexProError> {
        let start = Instant::now();

        let hypothesis = state
            .intermediate
            .get("hypothesis")
            .and_then(|v| v.as_str())
            .unwrap_or("Not available");

        let experiment_results = state
            .intermediate
            .get("experiment_results")
            .and_then(|v| v.as_str())
            .unwrap_or("No experimental results available yet.");

        let user_prompt = format!(
            "Write a research report based on:\n\n\
            Hypothesis:\n{}\n\n\
            Experimental Results:\n{}\n\n\
            Generate a comprehensive markdown report.",
            hypothesis,
            experiment_results
        );

        let messages = vec![
            Message {
                role: "system".to_string(),
                content: self.system_prompt.clone(),
            },
            Message {
                role: "user".to_string(),
                content: user_prompt,
            },
        ];

        let response = self
            .llm
            .complete(messages, &self.model, 0.7, 8192)
            .await
            .map_err(|e| CortexProError::AgentError(e.to_string()))?;

        let content = response.content().unwrap_or("No response").to_string();

        let execution_time_ms = start.elapsed().as_millis() as u64;

        Ok(AgentOutput {
            role: self.config.role,
            agent_name: self.config.name.clone(),
            content,
            confidence: 0.9,
            references: vec!["Materials Project".to_string()],
            errors: vec![],
            execution_time_ms,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    // Mock LLM client for testing
    struct MockLlmClient {
        response: String,
    }

    #[async_trait::async_trait]
    impl LlmClient for MockLlmClient {
        async fn complete(
            &self,
            _messages: Vec<Message>,
            _model: &str,
            _temperature: f32,
            _max_tokens: u32,
        ) -> Result<rairos_llm::LlmResponse, rairos_llm::LlmError> {
            Ok(rairos_llm::LlmResponse::NonStream(
                rairos_llm::NonStreamResponse {
                    content: self.response.clone(),
                    usage: rairos_llm::LlmUsage::default(),
                    model: "mock-model".to_string(),
                    finish_reason: "stop".to_string(),
                },
            ))
        }

        async fn stream_complete(
            &self,
            _messages: Vec<Message>,
            _model: &str,
            _temperature: f32,
            _max_tokens: u32,
        ) -> Result<rairos_llm::LlmResponse, rairos_llm::LlmError> {
            self.complete(_messages, _model, _temperature, _max_tokens).await
        }

        fn provider_name(&self) -> &'static str {
            "mock"
        }
    }

    #[tokio::test]
    async fn test_llm_hypothesis_agent() {
        let mock = Arc::new(MockLlmClient {
            response: "Hypothesis: Based on the query, we propose that doping Bi2Te3 with Se will improve ZT.".to_string(),
        });

        let agent = LlmHypothesisAgent::new("test-scientist", mock, "test-model");
        let mut state = ResearchState::new("test");
        state.intermediate.insert(
            "query".to_string(),
            serde_json::json!("Find thermoelectric materials"),
        );

        let result = agent.execute(&state).await;
        assert!(result.is_ok());
        let output = result.unwrap();
        assert!(output.content.contains("Bi2Te3"));
    }
}
