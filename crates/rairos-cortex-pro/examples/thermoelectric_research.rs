//! Thermoelectric Materials Research - Mock LLM Demo with Tool Execution
//!
//! This example demonstrates the SparksMatter multi-agent workflow for
//! thermoelectric materials discovery with tool execution.
//!
//! # Run
//!
//! ```bash
//! cargo run --example thermoelectric_research -p rairos-cortex-pro --features tools
//! ```

use std::sync::Arc;
use async_trait::async_trait;
use rairos_llm::{LlmClient, LlmResponse, LlmError, Message, NonStreamResponse, LlmUsage};
use rairos_cortex_pro::sparks_crew::SparksCrew;
use rairos_cortex_pro::sparks_agents::{
    HypothesisAgent, HypothesisCriticAgent, PlannerAgent,
    PlanCriticAgent, ExecutorAgent, ReportWriterAgent,
};
use rairos_cortex_pro::tools::{MaterialTool, ToolParams, ToolOutput};
use rairos_tools::ToolError;

/// Mock LLM that returns predefined responses
#[derive(Clone)]
struct MockLlm;

#[async_trait]
impl LlmClient for MockLlm {
    async fn complete(
        &self,
        messages: Vec<Message>,
        model: &str,
        _temperature: f32,
        _max_tokens: u32,
    ) -> Result<LlmResponse, LlmError> {
        let last_message = messages.last()
            .map(|m| m.content.as_str())
            .unwrap_or("");

        let content = if last_message.contains("hypothesis") || last_message.contains("Hypothesis") {
            "Hypothesis: Based on the research query, we propose that nanostructured Bi2-xSbxTe3 alloys with grain boundary engineering will achieve a thermoelectric figure of merit (ZT) > 1.8 at 300K through synergistic optimization of electrical and thermal transport properties.\n\nScientific Rationale:\n1. Nanostructuring introduces coherent grain boundaries that scatter mid-wavelength phonons, reducing lattice thermal conductivity by 50-70% without significantly degrading carrier mobility\n2. Sb alloying at Bi sites optimizes carrier concentration and enhances band degeneracy, improving power factor\n3. The nanostructured architecture maintains high Seebeck coefficient through energy filtering effect at grain boundaries\n\nPotential Impact:\n- Waste heat recovery in automotive and industrial applications\n- Compact, maintenance-free power generators for remote sensing\n- Wearable thermoelectric harvesters for IoT devices\n\nValidation:\n1. Use CGCNN to screen 50 compositions across the Bi-Sb-Te ternary phase diagram\n2. Download top 10 candidate structures from Materials Project\n3. Perform DFT calculations for transport properties\n4. Synthesize optimal compositions via ball milling and spark plasma sintering\n5. Characterize ZT via standard four-probe method".to_string()
        } else if last_message.contains("plan") || last_message.contains("Plan") || last_message.contains("revise") {
            // Return a valid JSON plan with steps that don't require external tools
            r#"{"rationale":"This plan systematically explores the Bi-Sb-Te ternary space using computational screening followed by experimental validation to discover high-ZT thermoelectric materials.","steps":[{"step":1,"task":"Search Materials Project for thermoelectric materials in Bi-Sb-Te system","tool":"materials_project","inputs":{"formula":"Bi,Sb,Te"},"depends_on":[]},{"step":2,"task":"Analyze retrieved structures and filter by known thermoelectric performance","tool":"","inputs":{},"depends_on":[1]},{"step":3,"task":"Screen compositions using CGCNN for predicted ZT values","tool":"cgcnn","inputs":{"num_structures":50},"depends_on":[2]},{"step":4,"task":"Select top 5 candidates for DFT validation","tool":"","inputs":{},"depends_on":[3]},{"step":5,"task":"Generate synthesis protocol for optimal composition","tool":"","inputs":{},"depends_on":[4]}],"other_tasks":["DFT calculation of electronic structure","TEM analysis of grain boundaries","High-temperature ZT measurement up to 500K"]}"#.to_string()
        } else if last_message.contains("approved") || last_message.contains("APPROVED") {
            "APPROVED - The plan is well-structured with clear steps and appropriate tools.".to_string()
        } else if last_message.contains("critic") || last_message.contains("Critic") {
            "APPROVED - The hypothesis is scientifically sound with clear validation strategy.".to_string()
        } else if last_message.contains("report") || last_message.contains("Report") || last_message.contains("introduction") {
            "# Research Report: Nanostructured Bi-Sb-Te Thermoelectric Materials\n\n## Introduction\n\nThermoelectric materials enable direct conversion between heat and electricity, offering a promising route for waste heat recovery in automotive, industrial, and consumer electronics applications. The efficiency of thermoelectric conversion is governed by the dimensionless figure of merit ZT = (S^2 sigma / kappa) T.\n\nAmong thermoelectric materials, the Bi-Sb-Te alloy system has long been the benchmark for near-room-temperature applications, with commercial ZT values around 1.0-1.2.\n\n## Methods\n\nWe employed a multi-agent computational screening approach:\n\n1. **Materials Project Database**: Retrieval of Bi-Sb-Te crystal structures\n2. **CGCNN Screening**: Machine learning-based prediction of thermoelectric properties\n3. **DFT Calculations**: First-principles validation of top candidates\n4. **Synthesis**: Mechanical alloying followed by spark plasma sintering\n5. **Characterization**: Standard four-probe measurement of thermoelectric properties\n\n## Results\n\nOur screening identified nanostructured Bi1.8Sb0.2Te3 with grain sizes of 50-200nm as a promising target, with predicted ZT > 1.8 at 300K.\n\nKey findings:\n- Grain boundary scattering reduces lattice thermal conductivity from ~1.7 W/mK to ~0.6 W/mK\n- Energy filtering at grain boundaries enhances Seebeck coefficient by 15-20%\n- Optimal carrier concentration of ~5x10^19 cm^-3 achieved through Sb alloying\n\n## Conclusion\n\nNanostructured Bi-Sb-Te alloys represent a promising pathway to high-ZT thermoelectric materials. Further optimization could enable ZT values exceeding 2.0.\n\n## References\n\n[1] Rowe, D.M. (1995) CRC Handbook of Thermoelectrics\n[2] Dresselhaus et al. (2007) Adv. Mater. 19, 1043-1053\n[3] Materials Project Database, materialsproject.org".to_string()
        } else {
            "Task completed successfully.".to_string()
        };

        Ok(LlmResponse::NonStream(NonStreamResponse {
            content,
            usage: LlmUsage::default(),
            model: model.to_string(),
            finish_reason: "stop".to_string(),
        }))
    }

    async fn stream_complete(
        &self,
        messages: Vec<Message>,
        model: &str,
        temperature: f32,
        max_tokens: u32,
    ) -> Result<LlmResponse, LlmError> {
        self.complete(messages, model, temperature, max_tokens).await
    }

    fn provider_name(&self) -> &'static str {
        "mock-thermoelectric"
    }
}

/// Mock Materials Project tool
struct MockMaterialsProjectTool;

#[async_trait]
impl MaterialTool for MockMaterialsProjectTool {
    fn name(&self) -> &str {
        "materials_project"
    }

    fn description(&self) -> &str {
        "Mock Materials Project tool for retrieving crystal structures"
    }

    async fn execute(&self, params: ToolParams) -> Result<ToolOutput, ToolError> {
        let formula = params.get_str("formula").unwrap_or("*");
        Ok(ToolOutput::success(serde_json::json!({
            "query": formula,
            "structures_found": 15,
            "top_candidates": [
                {"formula": "Bi2Te3", "zt_predicted": 1.1, "structure_id": "mp-559934"},
                {"formula": "Bi0.5Sb1.5Te3", "zt_predicted": 1.3, "structure_id": "mp-1229134"},
                {"formula": "Bi1.8Sb0.2Te3", "zt_predicted": 1.5, "structure_id": "mp-2336721"}
            ]
        })))
    }
}

/// Mock CGCNN tool
struct MockCgcnnTool;

#[async_trait]
impl MaterialTool for MockCgcnnTool {
    fn name(&self) -> &str {
        "cgcnn"
    }

    fn description(&self) -> &str {
        "Mock CGCNN tool for predicting thermoelectric properties"
    }

    async fn execute(&self, params: ToolParams) -> Result<ToolOutput, ToolError> {
        let num = params.get_i64("num_structures").unwrap_or(10) as usize;
        Ok(ToolOutput::success(serde_json::json!({
            "screened": num,
            "predictions": [
                {"formula": "Bi1.8Sb0.2Te3", "zt_predicted": 1.82, "confidence": 0.87},
                {"formula": "Bi1.6Sb0.4Te3", "zt_predicted": 1.71, "confidence": 0.82},
                {"formula": "Bi2Te2.8Se0.2", "zt_predicted": 1.65, "confidence": 0.79}
            ],
            "recommended": "Bi1.8Sb0.2Te3"
        })))
    }
}

fn create_research_crew() -> SparksCrew {
    let mock_llm = Arc::new(MockLlm);
    SparksCrew::new(mock_llm)
        .add_agent(Box::new(HypothesisAgent::new("Scientist1")))
        .add_agent(Box::new(HypothesisCriticAgent::new("Scientist2")))
        .add_agent(Box::new(PlannerAgent::new("Planner")))
        .add_agent(Box::new(PlanCriticAgent::new("PlanReviewer")))
        .add_agent(Box::new(ExecutorAgent::new("Assistant")))
        .add_agent(Box::new(ReportWriterAgent::new("ReportWriter")))
        .add_tool(Arc::new(MockMaterialsProjectTool))
        .add_tool(Arc::new(MockCgcnnTool))
        .with_max_iterations(2)
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("╔══════════════════════════════════════════════════════════════╗");
    println!("║  Thermoelectric Materials Research - Multi-Agent Workflow    ║");
    println!("╚══════════════════════════════════════════════════════════════╝");
    println!();

    let mut crew = create_research_crew();

    let query = "Find high-performance thermoelectric materials for waste heat recovery at 300K";

    println!("Research Query: {}", query);
    println!();
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!();

    // Phase 1: Ideation
    println!("📌 Phase 1: Ideation - Generating hypothesis...");
    match crew.run_ideation(query).await {
        Ok(hypothesis) => {
            println!("✅ Hypothesis generated!");
            println!();
            let preview = if hypothesis.len() > 400 {
                format!("{}...", &hypothesis[..400])
            } else {
                hypothesis.clone()
            };
            for line in preview.lines().take(10) {
                println!("   │ {}", line);
            }
        }
        Err(e) => {
            println!("❌ Ideation failed: {:?}", e);
        }
    }
    println!();

    // Phase 2: Planning
    println!("📌 Phase 2: Planning - Creating research plan...");
    match crew.run_planning().await {
        Ok(plan) => {
            println!("✅ Plan created!");
            println!();
            println!("   Rationale: {}", plan.rationale);
            println!();
            println!("   Steps:");
            for step in &plan.steps {
                let tool_str = if step.tool.is_empty() { "—" } else { &step.tool };
                println!("   {}. {} [tool: {}]", step.step, step.task, tool_str);
            }
        }
        Err(e) => {
            println!("❌ Planning failed: {:?}", e);
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
                println!();
                for sr in &result.step_results {
                    let status = if sr.success { "✅" } else { "❌" };
                    let tool_name = p.steps.iter()
                        .find(|s| s.step == sr.step)
                        .map(|s| s.tool.as_str())
                        .unwrap_or("");
                    if !tool_name.is_empty() {
                        if let Some(output) = &sr.output {
                            println!("   {} Step {}: {} → {}", status, sr.step, tool_name,
                                serde_json::to_string_pretty(output).unwrap_or_default().chars().take(60).collect::<String>());
                        } else {
                            println!("   {} Step {}: {} → completed", status, sr.step, tool_name);
                        }
                    } else {
                        println!("   {} Step {}: executed", status, sr.step);
                    }
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
            println!("{}", report.full_document);
        }
        Err(e) => {
            println!("❌ Reporting failed: {:?}", e);
        }
    }

    println!();
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("✅ Research workflow completed!");
    println!();

    Ok(())
}
