//! rairos-cortex - Cortex AI agent runtime integration for Rairos
//!
//! This crate provides:
//! - Rairos-specific Tool implementations for Cortex
//! - Integration with rairos-deep-research components
//! - GenePool adapter for Cortex memory
//!
//! **Status**: EXPERIMENTAL - API may change

// Re-export tools for convenience
pub mod tools;
pub use tools::*;

// ============================================================================
// TODO: Memory adapter (GenePool -> Cortex memory)
// ============================================================================
//
// Future integration points:
// - GenePool as Cortex Agent memory
// - Session management for research sessions
// - Checkpointing and recovery

// ============================================================================
// TODO: Agent integration (DeepResearchAgent -> Cortex)
// ============================================================================
//
// Future integration points:
// - Wrap DeepResearchAgent as Cortex agent
// - Use Cortex Crew for multi-researcher orchestration
// - LangGraph-style workflow for complex pipelines
