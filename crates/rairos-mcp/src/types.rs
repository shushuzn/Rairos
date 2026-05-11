//! JSON-RPC 2.0 types for MCP protocol

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// JSON-RPC 2.0 request
#[derive(Debug, Clone, Deserialize)]
pub struct JsonRpcRequest {
    pub jsonrpc: String,
    pub method: String,
    #[serde(default)]
    pub params: Option<Value>,
    pub id: Value,
}

/// JSON-RPC 2.0 response
#[derive(Debug, Clone, Serialize)]
#[serde(untagged)]
pub enum JsonRpcResponse {
    Success(JsonRpcSuccess),
    Error(JsonRpcError),
}

/// Successful response
#[derive(Debug, Clone, Serialize)]
pub struct JsonRpcSuccess {
    pub jsonrpc: String,
    pub result: Value,
    pub id: Value,
}

/// Error response
#[derive(Debug, Clone, Serialize)]
pub struct JsonRpcError {
    pub jsonrpc: String,
    pub error: JsonRpcErrorDetail,
    pub id: Value,
}

/// Error detail
#[derive(Debug, Clone, Serialize)]
pub struct JsonRpcErrorDetail {
    pub code: i32,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
}

impl JsonRpcError {
    pub fn new(code: i32, message: impl Into<String>, id: Value) -> Self {
        Self {
            jsonrpc: "2.0".into(),
            error: JsonRpcErrorDetail {
                code,
                message: message.into(),
                data: None,
            },
            id,
        }
    }

    pub fn with_data(code: i32, message: impl Into<String>, data: Value, id: Value) -> Self {
        Self {
            jsonrpc: "2.0".into(),
            error: JsonRpcErrorDetail {
                code,
                message: message.into(),
                data: Some(data),
            },
            id,
        }
    }
}

// Standard error codes
pub const ERR_PARSE_ERROR: i32 = -32700;
pub const ERR_INVALID_REQUEST: i32 = -32600;
pub const ERR_METHOD_NOT_FOUND: i32 = -32601;
pub const ERR_INVALID_PARAMS: i32 = -32602;
pub const ERR_INTERNAL_ERROR: i32 = -32603;
