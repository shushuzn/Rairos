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

pub use agent::{Agent, AgentConfig, AgentOutput, AgentRole};
pub use crew::{ResearchCrew, CrewConfig, CrewResult};
pub use pipeline::{Pipeline, PipelineNode, PipelineEdge};
pub use state::{ResearchState, Phase, ResearchContext};
pub use error::CortexProError;
