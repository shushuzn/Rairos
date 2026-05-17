//! Rairos CodeGraph — Pre-indexed code knowledge graph for Claude Code
//!
//! Provides MCP tools for fast code exploration without expensive file scanning.

pub mod graph;
pub mod mcp;
pub mod indexer;

pub use graph::{CodeGraph, Node, Edge, SearchResult, CallResult, GraphStats};
pub use mcp::McpServer;
pub use indexer::Indexer;

pub mod protocol {
    pub use crate::mcp::{Tool, ToolInputSchema, ToolProperty};
}
