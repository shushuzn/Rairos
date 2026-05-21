//! SparksMatter Demo - Multi-Agent Materials Discovery Workflow
//!
//! This example demonstrates the complete SparksMatter-style multi-agent workflow
//! for autonomous materials research.
//!
//! # Prerequisites
//!
//! 1. Run Ollama: `ollama serve`
//! 2. Pull a model: `ollama pull llama3.2` or `ollama pull mistral`
//!
//! # Run
//!
//! ```bash
//! cd /tmp/Rairos
//! cargo run --example sparks_matter_demo --features tools
//! ```
//!
//! # What it does
//!
//! 1. **Ideation Phase**: Generates and critiques a hypothesis for thermoelectric materials
//! 2. **Planning Phase**: Creates and reviews a research plan
//! 3. **Execution Phase**: Executes the plan (with mock tool results)
//! 4. **Reporting Phase**: Generates a research report

use std::sync::Arc;

// Use the real Ollama client from rairos-llm
use rairos_llm::ollama::OllamaClient;
use rairos_llm::LlmClient;

use rairos_cortex_pro::sparks_crew::SparksCrew;
use rairos_cortex_pro::sparks_agents::{
    HypothesisAgent, HypothesisCriticAgent, PlannerAgent,
    PlanCriticAgent, ExecutorAgent, ReportWriterAgent,
};

/// Helper to create a fully configured SparksCrew for the demo.
fn create_demo_crew(llm: Arc<dyn LlmClient>) -> SparksCrew {
    SparksCrew::new(llm)
        .add_agent(Box::new(HypothesisAgent::new("Scientist1")))
        .add_agent(Box::new(HypothesisCriticAgent::new("Scientist2")))
        .add_agent(Box::new(PlannerAgent::new("Planner")))
        .add_agent(Box::new(PlanCriticAgent::new("PlanReviewer")))
        .add_agent(Box::new(ExecutorAgent::new("Assistant")))
        .add_agent(Box::new(ReportWriterAgent::new("ReportWriter")))
        .with_max_iterations(2)
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("╔══════════════════════════════════════════════════════════════╗");
    println!("║     SparksMatter Demo - Multi-Agent Materials Discovery      ║");
    println!("╚══════════════════════════════════════════════════════════════╝");
    println!();

    // Check if Ollama is available
    let ollama = OllamaClient::new();
    println!("🔍 Checking Ollama availability...");

    // Try a simple completion to verify Ollama is running
    match ollama.complete(
        vec![rairos_llm::Message {
            role: "user".into(),
            content: "Say 'Ollama is ready' in exactly those words.".into(),
        }],
        "llama3.2",
        0.7,
        50,
    ).await {
        Ok(response) => {
            let content = match response {
                rairos_llm::LlmResponse::NonStream(r) => r.content,
                rairos_llm::LlmResponse::Stream(_) => "streamed".to_string(),
            };
            println!("✅ Ollama is ready! Response: {}", content);
        }
        Err(e) => {
            println!("⚠️  Ollama not available: {}", e);
            println!("💡 Please ensure Ollama is running: `ollama serve`");
            println!("   And a model is pulled: `ollama pull llama3.2`");
            return Ok(());
        }
    }

    println!();
    println!("🚀 Starting SparksMatter Multi-Agent Workflow...");
    println!();

    // Create crew with Ollama
    let llm = Arc::new(OllamaClient::new()) as Arc<dyn LlmClient>;
    let mut crew = create_demo_crew(llm);

    // Define the research query
    let query = "Find high-performance thermoelectric materials for waste heat recovery in automotive applications";

    println!("📋 Research Query: {}", query);
    println!();
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!();

    // Phase 1: Ideation
    println!("📌 Phase 1: Ideation - Generating and critiquing hypothesis...");
    match crew.run_ideation(query).await {
        Ok(hypothesis) => {
            println!("✅ Hypothesis generated!");
            println!("   Length: {} characters", hypothesis.len());
            println!();
            println!("   Preview:");
            let preview = &hypothesis[..hypothesis.len().min(300)];
            println!("   {}", preview.replace("\n", "\n   "));
            if hypothesis.len() > 300 {
                println!("   [...]");
            }
        }
        Err(e) => {
            println!("❌ Ideation failed: {:?}", e);
            return Ok(());
        }
    }
    println!();

    // Phase 2: Planning
    println!("📌 Phase 2: Planning - Creating research plan...");
    match crew.run_planning().await {
        Ok(plan) => {
            println!("✅ Plan created!");
            println!("   Rationale: {}", plan.rationale);
            println!("   Steps: {}", plan.steps.len());
            for (i, step) in plan.steps.iter().enumerate() {
                let tool_name = if step.tool.is_empty() { "none" } else { &step.tool };
                println!("   {}. {} [tool: {}]", i + 1, step.task, tool_name);
            }
            if !plan.other_tasks.is_empty() {
                println!("   Other tasks: {:?}", plan.other_tasks);
            }
        }
        Err(e) => {
            println!("❌ Planning failed: {:?}", e);
            return Ok(());
        }
    }
    println!();

    // Phase 3: Execution
    println!("📌 Phase 3: Execution - Running research plan...");
    let plan = crew.run_planning().await.ok();
    if let Some(ref p) = plan {
        match crew.run_execution(p).await {
            Ok(result) => {
                println!("✅ Execution completed!");
                println!("   Overall success: {}", result.success);
                println!("   Steps completed: {}", result.step_results.len());
                for sr in &result.step_results {
                    println!("   - Step {}: {}",
                        sr.step,
                        if sr.success { "✅" } else { "❌" }
                    );
                }
            }
            Err(e) => {
                println!("⚠️  Execution error: {:?}", e);
            }
        }
    }
    println!();

    // Phase 4: Reporting
    println!("📌 Phase 4: Reporting - Generating research report...");
    match crew.run_reporting().await {
        Ok(report) => {
            println!("✅ Report generated!");
            println!();
            println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
            println!("📄 RESEARCH REPORT");
            println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
            println!("{}", report.full_document);
        }
        Err(e) => {
            println!("❌ Reporting failed: {:?}", e);
        }
    }

    println!();
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("✅ SparksMatter Demo completed!");
    println!();

    Ok(())
}
