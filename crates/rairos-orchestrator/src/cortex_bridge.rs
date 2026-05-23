//! Bridge module between rairos-orchestrator and rairos-cortex-pro.
//!
//! This module provides integration for:
//! - Delegating deep research tasks to cortex-pro's crew
//! - Sharing observability metrics between modules
//! - Event-driven coordination via shared state
//!
//! Design: Facade pattern — orchestrator calls cortex bridge, bridge manages
//! cortex-pro internals and reports results back.

use rairos_cortex_pro::{
    Agent, AgentConfig, AgentOutput, AgentRole, CrewBuilder,
    CrewResult, CortexProError, ResearchState,
};
use rairos_core::ResearchGap;
use rairos_observability::get_metrics;
use async_trait::async_trait;
use tracing;

/// Configuration for the cortex bridge
#[derive(Debug, Clone)]
pub struct CortexBridgeConfig {
    /// Max iterations in research conversation
    pub max_iterations: usize,
}

impl Default for CortexBridgeConfig {
    fn default() -> Self {
        Self {
            max_iterations: 10,
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
}

impl CortexBridge {
    /// Create a new cortex bridge
    pub fn new(config: CortexBridgeConfig) -> Self {
        let metrics = get_metrics();
        metrics.inc("cortex_bridge", "bridge_created", 1.0);
        Self { config }
    }

    /// Initialize the research crew
    pub async fn init(&self) -> Result<(), String> {
        get_metrics().inc("cortex_bridge", "bridge_ready", 1.0);
        Ok(())
    }

    /// Build and run a research crew for the given topic
    pub async fn run_deep_research(
        &self,
        _topic: &str,
        _context: &str,
    ) -> Result<(CrewResult, Vec<ResearchGap>), String> {
        let metrics = get_metrics();
        metrics.inc("cortex_bridge", "research_started", 1.0);

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

        let result = crew.run(_topic).await.map_err(|e| {
            metrics.inc("cortex_bridge", "research_failed", 1.0);
            format!("Research failed: {}", e)
        })?;

        let gaps = Self::extract_gaps(&result);

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
                &gap_info.description
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
                    let report_trimmed = if report.len() > 200 { &report[..200] } else { report.as_str() };
                    gaps.push(ResearchGap::new(
                        report_trimmed,
                        "analysis",
                        "Research gap identified",
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
    }

    #[test]
    fn test_bridge_creation() {
        let bridge = CortexBridge::new(CortexBridgeConfig::default());
        assert!(bridge.config.max_iterations > 0);
    }
}
