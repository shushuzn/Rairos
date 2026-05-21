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

/// Integrations with other Rairos crates
pub mod integrations;

pub use agent::{Agent, AgentConfig, AgentOutput, AgentRole};
pub use crew::{ResearchCrew, CrewConfig, CrewResult};
pub use pipeline::{Pipeline, PipelineNode, PipelineEdge};
pub use state::{ResearchState, Phase, ResearchContext, CrewContext};
pub use error::CortexProError;

#[cfg(feature = "tools")]
pub use sparks_crew::{SparksCrew, Plan, PlanStep, ExecutionResult, ResearchReport};
