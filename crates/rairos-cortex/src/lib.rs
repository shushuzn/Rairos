//! rairos-cortex - Cortex AI agent runtime integration for Rairos
//!
//! This crate provides:
//! - Rairos-specific Tool implementations for Cortex
//! - Integration with rairos-deep-research components
//! - GenePool adapter for Cortex memory
//!
//! **Status**: EXPERIMENTAL - API may change
//!
//! # SQLite3 Conflict Note
//!
//! This crate uses only `cortexai-core` to avoid sqlite3 linking conflicts.
//! Full `cortexai-agents` integration (with sqlx) conflicts with `rairos-core`
//! which uses `rusqlite` - both declare `links = "sqlite3"`.
//!
//! Resolution options:
//! 1. Migrate rairos-core from rusqlite to sqlx (major change)
//! 2. Fork cortexai-agents to use rusqlite
//! 3. Use rairos-cortex as thin wrapper only (current approach)

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
