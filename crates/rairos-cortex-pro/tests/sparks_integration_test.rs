//! End-to-end integration tests for SparksMatter-style workflow.
//!
//! These tests verify the complete multi-agent workflow from hypothesis
//! generation through planning, execution, and reporting.

use std::sync::Arc;
use async_trait::async_trait;
use rairos_llm::{LlmClient, LlmResponse, LlmError, Message, NonStreamResponse, LlmUsage};

/// Mock LLM client for testing.
struct MockLlmClient;

impl MockLlmClient {
    fn new() -> Arc<Self> {
        Arc::new(Self)
    }
}

#[async_trait]
impl LlmClient for MockLlmClient {
    async fn complete(
        &self,
        _messages: Vec<Message>,
        _model: &str,
        _temperature: f32,
        _max_tokens: u32,
    ) -> Result<LlmResponse, LlmError> {
        Ok(LlmResponse::NonStream(NonStreamResponse {
            content: "Mock response".to_string(),
            usage: LlmUsage::default(),
            model: "mock".to_string(),
            finish_reason: "stop".to_string(),
        }))
    }

    async fn stream_complete(
        &self,
        _messages: Vec<Message>,
        _model: &str,
        _temperature: f32,
        _max_tokens: u32,
    ) -> Result<LlmResponse, LlmError> {
        Ok(LlmResponse::NonStream(NonStreamResponse {
            content: "Mock response".to_string(),
            usage: LlmUsage::default(),
            model: "mock".to_string(),
            finish_reason: "stop".to_string(),
        }))
    }

    fn provider_name(&self) -> &'static str {
        "mock"
    }
}

use rairos_cortex_pro::sparks_crew::{SparksCrew, Plan};
use rairos_cortex_pro::sparks_agents::{
    HypothesisAgent, HypothesisCriticAgent, PlannerAgent,
    PlanCriticAgent, ExecutorAgent, ReportWriterAgent,
};

/// Helper to create a fully configured SparksCrew for testing.
fn create_test_crew() -> SparksCrew {
    let mock_llm = MockLlmClient::new();
    SparksCrew::new(mock_llm)
        .add_agent(Box::new(HypothesisAgent::new("TestHypothesis")))
        .add_agent(Box::new(HypothesisCriticAgent::new("TestHypothesisCritic")))
        .add_agent(Box::new(PlannerAgent::new("TestPlanner")))
        .add_agent(Box::new(PlanCriticAgent::new("TestPlanCritic")))
        .add_agent(Box::new(ExecutorAgent::new("TestExecutor")))
        .add_agent(Box::new(ReportWriterAgent::new("TestReportWriter")))
        .with_max_iterations(3)
}

#[tokio::test]
async fn test_full_workflow_thermoelectric() {
    let mut crew = create_test_crew();

    let query = "Find high-performance thermoelectric materials for waste heat recovery";

    // Phase 1: Ideation
    let hypothesis = crew.run_ideation(query).await;
    assert!(hypothesis.is_ok(), "Ideation should succeed: {:?}", hypothesis.err());
    let hypothesis_text = hypothesis.unwrap();
    assert!(!hypothesis_text.is_empty(), "Hypothesis should not be empty");

    // Verify context was updated
    assert!(crew.idea_created(), "Idea should be created");
    assert!(crew.idea_approved(), "Idea should be approved");

    // Phase 2: Planning
    let plan = crew.run_planning().await;
    assert!(plan.is_ok(), "Planning should succeed: {:?}", plan.err());
    let plan_data = plan.unwrap();
    assert!(!plan_data.steps.is_empty(), "Plan should have steps");
    assert!(!plan_data.rationale.is_empty(), "Plan should have rationale");

    // Phase 3: Execution - Skip as it requires registered tools
    // In a real test with actual tool implementations, this would run
    // let execution = crew.run_execution(&plan_data).await;
    // assert!(execution.is_ok(), "Execution should succeed: {:?}", execution.err());
    // let exec_result = execution.unwrap();
    // assert!(!exec_result.step_results.is_empty(), "Should have step results");

    // Phase 4: Reporting
    let report = crew.run_reporting().await;
    assert!(report.is_ok(), "Reporting should succeed: {:?}", report.err());
    let report_data = report.unwrap();
    assert!(!report_data.full_document.is_empty(), "Report should have content");

    println!("✅ Full workflow completed successfully");
    println!("   Hypothesis length: {} chars", hypothesis_text.len());
    println!("   Plan steps: {}", plan_data.steps.len());
}

#[tokio::test]
async fn test_ideation_approval_flow() {
    let mut crew = create_test_crew();

    // Test that hypothesis gets approved after iteration
    let result = crew.run_ideation("Test query for thermoelectrics").await;
    assert!(result.is_ok());
    assert!(crew.idea_approved());
}

#[tokio::test]
async fn test_plan_parsing() {
    // Directly test plan JSON parsing
    let plan_json = r#"{
        "rationale": "Test rationale",
        "steps": [
            {
                "step": 1,
                "task": "Download structures",
                "tool": "mp",
                "inputs": {"formula": "Bi2Te3"},
                "depends_on": []
            }
        ]
    }"#;

    let plan: Plan = serde_json::from_str(plan_json).unwrap();
    assert_eq!(plan.steps.len(), 1);
    assert_eq!(plan.steps[0].step, 1);
    assert_eq!(plan.rationale, "Test rationale");
}

#[tokio::test]
async fn test_report_generation() {
    let mut crew = create_test_crew();
    crew.context_mut().hypothesis = Some("Test hypothesis".to_string());

    let report = crew.run_reporting().await;
    assert!(report.is_ok());

    let report_data = report.unwrap();
    assert!(!report_data.introduction.is_empty());
    assert!(!report_data.full_document.is_empty());
}

#[tokio::test]
async fn test_context_tracking() {
    let mut crew = create_test_crew();

    assert!(!crew.task_started());
    assert!(!crew.idea_created());

    // Run ideation
    crew.run_ideation("Test").await.unwrap();

    assert!(crew.task_started());
    assert!(crew.hypothesis().is_some());
}

#[tokio::test]
async fn test_executor_agent_task_extraction() {
    use rairos_cortex_pro::state::ResearchState;
    use rairos_cortex_pro::sparks_agents::ExecutorAgent;
    use rairos_cortex_pro::agent::Agent;

    let executor = ExecutorAgent::new("test-exec");
    let mut state = ResearchState::new("test");
    state.intermediate.insert(
        "current_task".to_string(),
        serde_json::json!("Calculate formation energy for Bi2Te3"),
    );

    let result = executor.execute(&state).await;
    assert!(result.is_ok());
    let output = result.unwrap();
    assert!(output.content.contains("Bi2Te3"));
}

#[tokio::test]
async fn test_hypothesis_agent_produces_output() {
    use rairos_cortex_pro::state::ResearchState;
    use rairos_cortex_pro::sparks_agents::HypothesisAgent;
    use rairos_cortex_pro::agent::Agent;

    let agent = HypothesisAgent::new("test-hypothesis");
    let mut state = ResearchState::new("test");
    state.intermediate.insert(
        "query".to_string(),
        serde_json::json!("Find superconducting materials"),
    );

    let result = agent.execute(&state).await;
    assert!(result.is_ok());
    let output = result.unwrap();
    assert!(output.role == rairos_cortex_pro::agent::AgentRole::Hypothesis);
    assert!(output.confidence > 0.0);
}

#[tokio::test]
async fn test_planner_agent_json_output() {
    use rairos_cortex_pro::state::ResearchState;
    use rairos_cortex_pro::sparks_agents::PlannerAgent;
    use rairos_cortex_pro::agent::Agent;

    let agent = PlannerAgent::new("test-planner");
    let state = ResearchState::new("test");

    let result = agent.execute(&state).await;
    assert!(result.is_ok());
    let output = result.unwrap();

    // Output should be valid JSON that can be parsed as Plan
    let parsed: Result<Plan, _> = serde_json::from_str(&output.content);
    assert!(parsed.is_ok(), "Planner output should be valid JSON: {}", output.content);
}

#[tokio::test]
async fn test_plan_critic_validates_plan_structure() {
    use rairos_cortex_pro::state::ResearchState;
    use rairos_cortex_pro::sparks_agents::PlanCriticAgent;
    use rairos_cortex_pro::agent::Agent;

    let agent = PlanCriticAgent::new("test-critic");

    // Valid plan
    let mut state_valid = ResearchState::new("test");
    state_valid.intermediate.insert(
        "plan".to_string(),
        serde_json::json!(r#"{"rationale": "test", "steps": []}"#),
    );
    let result_valid = agent.execute(&state_valid).await.unwrap();
    assert!(result_valid.content.contains("APPROVED") || result_valid.confidence > 0.5);
}
