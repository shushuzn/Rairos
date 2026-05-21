//! rairos-traitclaw - TraitClaw integration for Rairos
//!
//! This crate provides:
//! - Rairos-specific Tool implementations wrapping rairos components
//! - GenePool adapter for TraitClaw Memory trait
//! - RairosResearchStrategy implementing AgentStrategy
//!
//! **Status**: EXPERIMENTAL - API may change

// Re-export tools
pub mod tools;
pub use tools::*;

// ============================================================================
// TODO: Memory adapter (GenePool -> Memory trait)
// ============================================================================
//
// This module is commented out because it requires:
// - proper rairos-llm GenePool integration
// - Memory trait implementation
//
// See git history for in-progress implementation.

// ============================================================================
// TODO: Strategy (RairosResearchStrategy -> AgentStrategy)
// ============================================================================
//
// This module is commented out because it requires:
// - proper tool execution wiring
// - runtime integration
//
// See git history for in-progress implementation.
