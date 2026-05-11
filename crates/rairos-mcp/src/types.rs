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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_request() {
        let json = r#"{"jsonrpc":"2.0","method":"tools/list","id":1}"#;
        let req: JsonRpcRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.jsonrpc, "2.0");
        assert_eq!(req.method, "tools/list");
        assert_eq!(req.id, 1);
    }

    #[test]
    fn test_parse_request_with_params() {
        let json = r#"{"jsonrpc":"2.0","method":"tools/call","params":{"name":"test"},"id":"abc"}"#;
        let req: JsonRpcRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.method, "tools/call");
        assert!(req.params.is_some());
    }

    #[test]
    fn test_error_codes() {
        assert_eq!(ERR_PARSE_ERROR, -32700);
        assert_eq!(ERR_INVALID_REQUEST, -32600);
        assert_eq!(ERR_METHOD_NOT_FOUND, -32601);
        assert_eq!(ERR_INVALID_PARAMS, -32602);
        assert_eq!(ERR_INTERNAL_ERROR, -32603);
    }

    #[test]
    fn test_error_creation() {
        let err = JsonRpcError::new(ERR_METHOD_NOT_FOUND, "Method not found", serde_json::json!(1));
        assert_eq!(err.error.code, -32601);
        assert_eq!(err.error.message, "Method not found");
    }

    #[test]
    fn test_success_response() {
        let resp = JsonRpcSuccess {
            jsonrpc: "2.0".to_string(),
            result: serde_json::json!({"tools": []}),
            id: serde_json::json!(1),
        };
        let json = serde_json::to_string(&resp).unwrap();
        assert!(json.contains("2.0"));
        assert!(json.contains("result"));
    }
}
