//! Thermoelectric Materials Research - Mock LLM Demo
//!
//! This example demonstrates the SparksMatter multi-agent workflow for
//! thermoelectric materials discovery using mock LLM responses.
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

/// Mock LLM that returns predefined responses
#[derive(Clone)]
struct MockLlm;

#[async_trait]
impl LlmClient for MockLlm {
    async fn complete(
        &self,
        messages: Vec<Message>,
        model: &str,
        temperature: f32,
        max_tokens: u32,
    ) -> Result<LlmResponse, LlmError> {
        let last_message = messages.last()
            .map(|m| m.content.as_str())
            .unwrap_or("");

        let content = if last_message.contains("hypothesis") || last_message.contains("Hypothesis") {
            "Hypothesis: Based on the research query, we propose that nanostructured Bi2-xSbxTe3 alloys with grain boundary engineering will achieve a thermoelectric figure of merit (ZT) > 1.8 at 300K through synergistic optimization of electrical and thermal transport properties.\n\nScientific Rationale:\n1. Nanostructuring introduces coherent grain boundaries that scatter mid-wavelength phonons, reducing lattice thermal conductivity by 50-70% without significantly degrading carrier mobility\n2. Sb alloying at Bi sites optimizes carrier concentration and enhances band degeneracy, improving power factor\n3. The nanostructured architecture maintains high Seebeck coefficient through energy filtering effect at grain boundaries\n\nPotential Impact:\n- Waste heat recovery in automotive and industrial applications\n- Compact, maintenance-free power generators for remote sensing\n- Wearable thermoelectric harvesters for IoT devices\n\nValidation:\n1. Use CGCNN to screen 50 compositions across the Bi-Sb-Te ternary phase diagram\n2. Download top 10 candidate structures from Materials Project\n3. Perform DFT calculations for transport properties\n4. Synthesize optimal compositions via ball milling and spark plasma sintering\n5. Characterize ZT via standard four-probe method".to_string()
        } else if last_message.contains("plan") || last_message.contains("Plan") || last_message.contains("revise") {
            // Return a valid JSON plan
            r#"{"rationale":"This plan systematically explores the Bi-Sb-Te ternary space using computational screening followed by experimental validation.","steps":[{"step":1,"task":"Screen Bi-Sb-Te compositions using CGCNN machine learning model","tool":"cgcnn_regression","inputs":{"property":"thermoelectric_zt","num_structures":50},"depends_on":[]},{"step":2,"task":"Download crystal structures of top candidates from Materials Project","tool":"download_structures_from_mp","inputs":{"formula":"Bi*,Sb*,Te*"},"depends_on":[1]},{"step":3,"task":"Perform DFT validation of top 5 compositions","tool":"","inputs":{},"depends_on":[2]},{"step":4,"task":"Synthesize optimal composition via mechanical alloying","tool":"","inputs":{},"depends_on":[3]},{"step":5,"task":"Measure thermoelectric properties (ZT, PF, kappa)","tool":"","inputs":{},"depends_on":[4]}],"other_tasks":["TEM analysis of grain boundary structure","Thermal stability testing up to 500K"]}"#.to_string()
        } else if last_message.contains("approved") || last_message.contains("APPROVED") {
            "APPROVED - The plan is well-structured with clear steps and appropriate tools.".to_string()
        } else if last_message.contains("critic") || last_message.contains("Critic") {
            "APPROVED - The hypothesis is scientifically sound with clear validation strategy.".to_string()
        } else if last_message.contains("report") || last_message.contains("Report") || last_message.contains("introduction") {
            "# Research Report: Nanostructured Bi-Sb-Te Thermoelectric Materials\n\n## Introduction\n\nThermoelectric materials enable direct conversion between heat and electricity, offering a promising route for waste heat recovery in automotive, industrial, and consumer electronics applications. The efficiency of thermoelectric conversion is governed by the dimensionless figure of merit ZT = (S^2 sigma / kappa) T, where S is the Seebeck coefficient, sigma is electrical conductivity, kappa is thermal conductivity, and T is absolute temperature.\n\nAmong thermoelectric materials, the Bi-Sb-Te alloy system has long been the benchmark for near-room-temperature applications, with commercial ZT values around 1.0-1.2. However, recent advances in nanostructuring and band engineering have opened pathways to significantly higher performance.\n\n## Methods\n\nWe employed a multi-agent computational screening approach:\n\n1. **CGCNN Screening**: Machine learning-based prediction of thermoelectric properties across 50 compositions in the Bi-Sb-Te ternary system\n2. **Materials Project Database**: Retrieval of experimental crystal structures for validation\n3. **DFT Calculations**: First-principles calculation of electronic structure and transport properties\n4. **Synthesis**: Mechanical alloying followed by spark plasma sintering\n5. **Characterization**: Standard four-probe measurement of thermoelectric properties\n\n## Results\n\nOur screening identified nanostructured Bi1.8Sb0.2Te3 with grain sizes of 50-200nm as a promising target, with predicted ZT > 1.8 at 300K.\n\nKey findings:\n- Grain boundary scattering reduces lattice thermal conductivity from ~1.7 W/mK (bulk) to ~0.6 W/mK\n- Energy filtering at grain boundaries enhances Seebeck coefficient by 15-20%\n- Optimal carrier concentration of ~5x10^19 cm^-3 achieved through Sb alloying\n\n## Conclusion\n\nNanostructured Bi-Sb-Te alloys represent a promising pathway to high-ZT thermoelectric materials for waste heat recovery. Further optimization of grain size distribution and carrier concentration tuning could enable ZT values exceeding 2.0.\n\n## References\n\n[1] Rowe, D.M. (1995) CRC Handbook of Thermoelectrics\n[2] Dresselhaus et al. (2007) New Directions for Low-Dimensional Thermoelectric Materials\n[3] Materials Project Database, materialsproject.org".to_string()
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

fn create_research_crew() -> SparksCrew {
    let mock_llm = Arc::new(MockLlm);
    SparksCrew::new(mock_llm)
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
    println!("║  Thermoelectric Materials Research - Multi-Agent Workflow    ║");
    println!("╚══════════════════════════════════════════════════════════════╝");
    println!();

    let mut crew = create_research_crew();

    // Research query
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
            println!("   ┌─────────────────────────────────────────────────────────┐");
            let preview = if hypothesis.len() > 500 {
                format!("{}...", &hypothesis[..500])
            } else {
                hypothesis.clone()
            };
            for line in preview.lines().take(12) {
                println!("   │ {}", line);
            }
            if hypothesis.len() > 500 {
                println!("   │ ...");
            }
            println!("   └─────────────────────────────────────────────────────────┘");
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
            if !plan.other_tasks.is_empty() {
                println!();
                println!("   Other tasks: {:?}", plan.other_tasks);
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
                    println!("   {} Step {}: completed", status, sr.step);
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
