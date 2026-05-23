//! Bridge module between rairos-orchestrator and rairos-cortex-pro.
//!
//! This module provides integration for:
//! - Delegating deep research tasks to cortex-pro's crew
//! - Using MCTS planner for intelligent tool selection
//! - Emitting events for cross-module coordination
//! - Sharing observability metrics between modules

use rairos_cortex_pro::{
    Agent, AgentConfig, AgentOutput, AgentRole, CrewBuilder, CrewResult,
    CortexProError, ResearchState, MctsPlanner, Tool, ToolCategory,
    EventEmitter, Event, event_types,
};
use rairos_core::ResearchGap;
use rairos_observability::get_metrics;
use async_trait::async_trait;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing;

/// Configuration for the cortex bridge
#[derive(Debug, Clone)]
pub struct CortexBridgeConfig {
    /// Max iterations in research conversation
    pub max_iterations: usize,
    /// Whether to enable MCTS-based tool planning
    pub enable_mcts: bool,
}

impl Default for CortexBridgeConfig {
    fn default() -> Self {
        Self {
            max_iterations: 10,
            enable_mcts: true,
        }
    }
}

/// A concrete Agent implementation for the orchestrator bridge.
struct BridgeAgent {
    config: AgentConfig,
}

#[async_trait]
impl Agent for BridgeAgent {
    fn config(&self) -> &AgentConfig {
        &self.config
    }

    async fn execute(
        &self,
        _state: &ResearchState,
    ) -> Result<AgentOutput, CortexProError> {
        Ok(AgentOutput {
            role: self.config.role,
            agent_name: self.config.name.clone(),
            content: format!("{} executed", self.config.name),
            confidence: 0.5,
            references: vec![],
            errors: vec![],
            execution_time_ms: 0,
        })
    }
}

/// Bridge orchestrator ↔ cortex-pro for multi-agent research
pub struct CortexBridge {
    config: CortexBridgeConfig,
    mcts_planner: Option<MctsPlanner>,
}

impl CortexBridge {
    /// Create a new cortex bridge
    pub fn new(config: CortexBridgeConfig) -> Self {
        let metrics = get_metrics();
        metrics.inc("cortex_bridge", "bridge_created", 1.0);

        let mcts_planner = if config.enable_mcts {
            Some(Self::init_mcts_planner())
        } else {
            None
        };

        Self {
            config,
            mcts_planner,
        }
    }

    /// Initialize the MCTS planner with default tools
    fn init_mcts_planner() -> MctsPlanner {
        let planner = MctsPlanner::new();

        // Register default research tools
        planner.register_tool(Tool {
            name: "literature_search".to_string(),
            description: "Search academic literature for relevant papers".to_string(),
            category: ToolCategory::Literature,
            input_schema: std::collections::HashMap::new(),
            estimated_cost: 0.3,
            description_lower: Some("search academic literature for relevant papers".to_string()),
        });
        planner.register_tool(Tool {
            name: "data_analysis".to_string(),
            description: "Analyze research data and identify patterns".to_string(),
            category: ToolCategory::Analysis,
            input_schema: std::collections::HashMap::new(),
            estimated_cost: 0.5,
            description_lower: Some("analyze research data and identify patterns".to_string()),
        });
        planner.register_tool(Tool {
            name: "gap_analysis".to_string(),
            description: "Identify research gaps and opportunities".to_string(),
            category: ToolCategory::Analysis,
            input_schema: std::collections::HashMap::new(),
            estimated_cost: 0.4,
            description_lower: Some("identify research gaps and opportunities".to_string()),
        });
        planner.register_tool(Tool {
            name: "report_generation".to_string(),
            description: "Generate comprehensive research reports".to_string(),
            category: ToolCategory::Visualization,
            input_schema: std::collections::HashMap::new(),
            estimated_cost: 0.6,
            description_lower: Some("generate comprehensive research reports".to_string()),
        });

        get_metrics().inc("cortex_bridge", "mcts_tools_registered", 4.0);
        planner
    }

    /// Initialize the research crew
    pub async fn init(&self) -> Result<(), String> {
        get_metrics().inc("cortex_bridge", "bridge_ready", 1.0);

        // Emit initialization event
        EventEmitter::default().emit(Event::new(
            event_types::ORCHESTRATOR_CYCLE_START,
            serde_json::json!({"action": "init", "mcts": self.config.enable_mcts}),
        )).await;

        Ok(())
    }

    /// Select the best tool using MCTS planner
    pub fn select_tool(&self, query: &str, context: &str) -> Option<String> {
        let planner = self.mcts_planner.as_ref()?;
        let selection = planner.select_tools(query, context);

        let metrics = get_metrics();
        if !selection.tool_name.is_empty() {
            metrics.inc("cortex_bridge", "tool_selected", 1.0);
            Some(selection.tool_name)
        } else {
            metrics.inc("cortex_bridge", "tool_selection_empty", 1.0);
            None
        }
    }

    /// Build and run a research crew for the given topic
    pub async fn run_deep_research(
        &self,
        topic: &str,
        context: &str,
    ) -> Result<(CrewResult, Vec<ResearchGap>), String> {
        let metrics = get_metrics();
        metrics.inc("cortex_bridge", "research_started", 1.0);

        // Emit research start event
        EventEmitter::default().emit(Event::new(
            event_types::ORCHESTRATOR_CYCLE_START,
            serde_json::json!({"topic": topic, "context_len": context.len()}),
        )).await;

        // Use MCTS to select optimal research approach
        if let Some(selected_tool) = self.select_tool(topic, context) {
            tracing::info!("[CortexBridge] MCTS selected tool: {}", selected_tool);
            metrics.inc("cortex_bridge", "mcts_selection", 1.0);
        }

        let crew = CrewBuilder::new("orchestrator-crew")
            .with_max_iterations(self.config.max_iterations)
            .with_agent(BridgeAgent {
                config: AgentConfig {
                    name: "researcher".to_string(),
                    role: AgentRole::Researcher,
                    ..AgentConfig::default()
                },
            })
            .with_agent(BridgeAgent {
                config: AgentConfig {
                    name: "gap_analyzer".to_string(),
                    role: AgentRole::GapAnalyzer,
                    ..AgentConfig::default()
                },
            })
            .build();

        let result = crew.run(topic).await.map_err(|e| {
            metrics.inc("cortex_bridge", "research_failed", 1.0);

            // Emit failure event
            let error_msg = format!("{}", e);
            let emitter = EventEmitter::default();
            tokio::spawn(async move {
                emitter.emit(Event::new(
                    event_types::ORCHESTRATOR_CYCLE_COMPLETE,
                    serde_json::json!({"status": "failed", "error": error_msg}),
                )).await;
            });

            format!("Research failed: {}", e)
        })?;

        let gaps = Self::extract_gaps(&result);

        // Emit completion event
        EventEmitter::default().emit(Event::new(
            event_types::ORCHESTRATOR_CYCLE_COMPLETE,
            serde_json::json!({"topic": topic, "gaps_found": gaps.len(), "success": result.success}),
        )).await;

        metrics.inc("cortex_bridge", "research_completed", 1.0);
        metrics.inc("cortex_bridge", "gaps_found", gaps.len() as f64);

        Ok((result, gaps))
    }

    /// Extract ResearchGap from CrewResult's state
    fn extract_gaps(result: &CrewResult) -> Vec<ResearchGap> {
        let mut gaps = Vec::new();

        for gap_info in &result.state.gaps {
            let desc_trimmed = if gap_info.description.len() > 100 {
                &gap_info.description[..100]
            } else {
                &gap_info.description.as_str()
            };
            gaps.push(ResearchGap::new(
                desc_trimmed,
                "research_gap",
                &gap_info.description,
                "deep_research",
                &gap_info.description,
                "MEDIUM",
                "NORMAL",
            ));
        }

        if gaps.is_empty() {
            if let Some(ref report) = result.state.report {
                if !report.is_empty() {
                    let report_trimmed: &str = if report.len() > 200 {
                        &report[..200]
                    } else {
                        report
                    };
                    gaps.push(ResearchGap::new(
                        report_trimmed,
                        "analysis",
                        "Research gap identified from report",
                        "deep_research",
                        "Gap identified during multi-agent research",
                        "MEDIUM",
                        "NORMAL",
                    ));
                }
            }
        }

        gaps
    }

    /// Shutdown and clean up resources
    pub async fn shutdown(&self) {
        let metrics = get_metrics();
        metrics.inc("cortex_bridge", "shutdown", 1.0);

        EventEmitter::default().emit(Event::new(
            event_types::SYSTEM_SHUTDOWN,
            serde_json::json!({"component": "cortex_bridge"}),
        )).await;

        tracing::info!("[CortexBridge] Shut down");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bridge_config_default() {
        let config = CortexBridgeConfig::default();
        assert_eq!(config.max_iterations, 10);
        assert!(config.enable_mcts);
    }

    #[test]
    fn test_bridge_creation() {
        let bridge = CortexBridge::new(CortexBridgeConfig::default());
        assert!(bridge.config.max_iterations > 0);
    }

    #[test]
    fn test_mcts_planner_registration() {
        let planner = CortexBridge::init_mcts_planner();
        // After registration, MCTS should have tools

        // Verify by checking tool selection works
        let selection = planner.select_tools("test query", "context");
        assert!(!selection.tool_name.is_empty());
    }
}
