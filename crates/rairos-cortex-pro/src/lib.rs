//! # rairos-cortex-pro
//!
//! Production multi-agent collaboration framework with LangGraph-style orchestration.
//!
//! ## Overview
//!
//! This crate provides advanced multi-agent capabilities for research automation,
//! building on the experimental `rairos-cortex` with production-ready features.
//!
//! ## Key Concepts
//!
//! - **Agent**: An autonomous entity that can perform specialized tasks
//! - **Crew**: A team of agents working together on a shared objective
//! - **Pipeline**: A directed workflow graph (LangGraph-style)
//! - **State**: Shared context passed through the pipeline
//!
//! ## Architecture
//!
//! ```text
//! ResearchCrew
//! ├── ResearcherAgent    → Paper search, extraction, indexing
//! ├── GapAnalyzerAgent   → Research gap detection
//! ├── CitationGraphAgent → Citation network analysis
//! ├── VectorIndexerAgent → Vector storage and retrieval
//! └── ReportWriterAgent  → Synthesis and report generation
//! ```
//!
//! ## SparksMatter Integration
//!
//! With the `tools` feature enabled, this crate supports SparksMatter-style
//! multi-agent workflows for materials discovery:
//!
//! ```text
//! Ideation → Planning → Execution (with tools) → Reporting
//! ```
//!
//! ## Example
//!
//! ```ignore
//! use rairos_cortex_pro::{ResearchCrew, CrewConfig, ResearchState};
//!
//! let crew = ResearchCrew::new(CrewConfig::default());
//! let result = crew.run("machine learning for materials discovery").await?;
//! println!("Report: {}", result.report);
//! ```

pub mod agent;
pub mod crew;
pub mod pipeline;
pub mod state;
pub mod error;

#[cfg(feature = "tools")]
pub mod sparks_crew;

#[cfg(feature = "tools")]
pub mod sparks_agents;

#[cfg(feature = "tools")]
pub mod tools;

/// Memory module for persistent context across agent interactions
#[cfg(feature = "tools")]
pub mod memory;

/// LLM-based agents using real LLM calls
#[cfg(feature = "tools")]
pub mod llm_agents;

/// MCTS planner for tool selection (ToolTree-style)
#[cfg(feature = "tools")]
pub mod mcts_planner;

/// SSE streaming for real-time agent progress
#[cfg(feature = "tools")]
pub mod streaming_sse;

/// Hierarchical agent delegation (manager → sub-team)
#[cfg(feature = "tools")]
pub mod hierarchical;

/// Benchmark module for agent evaluation (GAIA, AgentBench style)
#[cfg(feature = "tools")]
pub mod benchmark;

/// A2A protocol for agent-to-agent communication
#[cfg(feature = "tools")]
pub mod a2a_protocol;

/// Active Learning for materials discovery (LEAP-style)
#[cfg(feature = "tools")]
pub mod active_learning;

/// Context compression for token optimization (ECoRAG, ACC-RAG style)
#[cfg(feature = "tools")]
pub mod context_compression;

/// DAG workflow executor for parallel task execution (ROMA, AdaptOrch style)
#[cfg(feature = "tools")]
pub mod dag_executor;

/// Experience replay for self-improving agents (R³, ERL, LEAFE style)
#[cfg(feature = "tools")]
pub mod experience_replay;

/// Multi-agent consensus for agent agreement (EMS, AgentAuditor style)
#[cfg(feature = "tools")]
pub mod multi_agent_consensus;

/// Self-correction and reflection (CRITIC, Self-Refiner, Reflexion style)
#[cfg(feature = "tools")]
pub mod self_correction;

/// Integrations with other Rairos crates
pub mod integrations;

#[cfg(feature = "api")]
pub mod api;

pub use agent::{Agent, AgentConfig, AgentOutput, AgentRole};
pub use crew::{ResearchCrew, CrewConfig, CrewResult};
pub use pipeline::{Pipeline, PipelineNode, PipelineEdge};
pub use state::{ResearchState, Phase, ResearchContext, CrewContext};
pub use error::CortexProError;

#[cfg(feature = "tools")]
pub use sparks_crew::{SparksCrew, Plan, PlanStep, ExecutionResult, ResearchReport};

#[cfg(feature = "tools")]
pub use memory::{MemoryBank, IdeationEntry, IdeationStatus, ExperimentationEntry, MemoryStats, MemoryEntryBuilder, ExperimentationEntryBuilder};

#[cfg(feature = "tools")]
pub use llm_agents::{LlmHypothesisAgent, LlmHypothesisCriticAgent, LlmPlannerAgent, LlmPlanCriticAgent, LlmReportWriterAgent};

#[cfg(feature = "tools")]
pub use mcts_planner::{MctsPlanner, Tool, ToolCategory, ToolSelection};

#[cfg(feature = "tools")]
pub use streaming_sse::{SseBroadcaster, SseEvent, SseTimer};

#[cfg(feature = "tools")]
pub use hierarchical::{DelegationManager, DelegatedTask, HierarchicalConfig, AgentLevel, SubTeam, TeamStatus, DelegationStats};

#[cfg(feature = "tools")]
pub use benchmark::{BenchmarkMetrics, BenchmarkReport, BenchmarkTask, TaskResult, MilestoneResult, gaia_benchmark_tasks, collaboration_tasks, run_benchmark};

#[cfg(feature = "tools")]
pub use a2a_protocol::{A2AProtocol, A2AMessage, A2ATask, AgentCard, AgentCapability, A2AMessageType};

#[cfg(feature = "tools")]
pub use active_learning::{ActiveLearningManager, MaterialCandidate, EvaluationResult, AcquisitionFunction, ActiveLearningStats, HypothesisGenerator};

#[cfg(feature = "tools")]
pub use context_compression::{ContextCompressor, CompressedContext, CompressionRatio, TokenBudget, BudgetEntry};

#[cfg(feature = "tools")]
pub use dag_executor::{TaskDag, DagNode, DagExecutor, DagExecutionResult, NodeResult, dag_from_plan_steps};

#[cfg(feature = "tools")]
pub use experience_replay::{ExperienceReplay, Experience, TrajectoryStep, ExperienceOutcome, SelfReflector, ConsolidationResult};

#[cfg(feature = "tools")]
pub use multi_agent_consensus::{MultiAgentConsensus, ConsensusResult, Vote, VotingMechanism, EvidenceAuditor, AuditResult, Evidence, ConsensusAgent};

#[cfg(feature = "tools")]
pub use self_correction::{SelfCorrector, ReflexionAgent, ExternalValidator, EvaluationResult, Correction, Reflection, CorrectionType, ValidationResult, ValidationIssue, ValidationRule};
