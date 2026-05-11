//! Rairos MCP — JSON-RPC 2.0 MCP protocol server
//!
//! Exposes Rairos tools via the Model Context Protocol (JSON-RPC 2.0).

pub mod protocol;
pub mod types;

pub use protocol::{McpServer, Tool, ToolHandler, ToolInputSchema, ToolProperty};
pub use types::{
    JsonRpcError, JsonRpcErrorDetail, JsonRpcRequest, JsonRpcResponse, JsonRpcSuccess,
};
