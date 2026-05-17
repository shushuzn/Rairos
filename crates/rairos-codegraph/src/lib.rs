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

pub type CodeGraphResult<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;

pub trait CodeGraphBackend: Send + Sync {
    fn search(&self, query: &str, limit: usize) -> CodeGraphResult<Vec<SearchResult>>;
    fn get_node(&self, node_id: i64) -> CodeGraphResult<Option<Node>>;
    fn get_callers(&self, node_id: i64, depth: usize) -> CodeGraphResult<Vec<CallResult>>;
    fn get_callees(&self, node_id: i64, depth: usize) -> CodeGraphResult<Vec<CallResult>>;
    fn files(&self) -> CodeGraphResult<Vec<String>>;
    fn stats(&self) -> CodeGraphResult<GraphStats>;
    fn add_node(&self, node: &Node) -> CodeGraphResult<i64>;
    fn add_edge(&self, from_node: i64, to_node: i64, edge_type: &str) -> CodeGraphResult<i64>;
    fn clear(&self) -> CodeGraphResult<()>;
}

impl CodeGraphBackend for CodeGraph {
    fn search(&self, query: &str, limit: usize) -> CodeGraphResult<Vec<SearchResult>> {
        Ok(CodeGraph::search(self, query, limit)?)
    }
    fn get_node(&self, node_id: i64) -> CodeGraphResult<Option<Node>> {
        Ok(CodeGraph::get_node(self, node_id)?)
    }
    fn get_callers(&self, node_id: i64, depth: usize) -> CodeGraphResult<Vec<CallResult>> {
        Ok(CodeGraph::get_callers(self, node_id, depth)?)
    }
    fn get_callees(&self, node_id: i64, depth: usize) -> CodeGraphResult<Vec<CallResult>> {
        Ok(CodeGraph::get_callees(self, node_id, depth)?)
    }
    fn files(&self) -> CodeGraphResult<Vec<String>> {
        Ok(CodeGraph::files(self)?)
    }
    fn stats(&self) -> CodeGraphResult<GraphStats> {
        Ok(CodeGraph::stats(self)?)
    }
    fn add_node(&self, node: &Node) -> CodeGraphResult<i64> {
        Ok(CodeGraph::add_node(self, node)?)
    }
    fn add_edge(&self, from_node: i64, to_node: i64, edge_type: &str) -> CodeGraphResult<i64> {
        Ok(CodeGraph::add_edge(self, from_node, to_node, edge_type)?)
    }
    fn clear(&self) -> CodeGraphResult<()> {
        Ok(CodeGraph::clear(self)?)
    }
}
