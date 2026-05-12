//! MCP protocol — JSON-RPC 2.0 server implementation

use crate::types::*;
use async_trait::async_trait;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Tool input schema
#[derive(Debug, Clone, serde::Serialize)]
pub struct ToolInputSchema {
    #[serde(rename = "type")]
    pub ty: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub properties: Option<HashMap<String, Value>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub required: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub items: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enum_values: Option<Vec<String>>,
}

impl ToolInputSchema {
    #[allow(dead_code)]
    fn object(properties: HashMap<String, ToolProperty>, required: Vec<String>) -> Self {
        let props: HashMap<String, Value> = properties
            .into_iter()
            .map(|(k, v)| {
                let mut m = serde_json::Map::new();
                m.insert("type".into(), serde_json::json!(v.type_));
                if let Some(d) = v.description {
                    m.insert("description".into(), serde_json::json!(d));
                }
                if let Some(e) = v.enum_values {
                    m.insert("enum".into(), serde_json::json!(e));
                }
                (k, Value::Object(m))
            })
            .collect();
        Self {
            ty: "object".into(),
            properties: Some(props),
            required: Some(required),
            items: None,
            description: None,
            enum_values: None,
        }
    }

    #[allow(dead_code)]
    fn string_enum(values: Vec<String>) -> Self {
        Self {
            ty: "string".into(),
            properties: None,
            required: None,
            items: None,
            description: None,
            enum_values: Some(values),
        }
    }

    #[allow(dead_code)]
    fn integer() -> Self {
        Self {
            ty: "integer".into(),
            properties: None,
            required: None,
            items: None,
            description: None,
            enum_values: None,
        }
    }

    #[allow(dead_code)]
    fn number() -> Self {
        Self {
            ty: "number".into(),
            properties: None,
            required: None,
            items: None,
            description: None,
            enum_values: None,
        }
    }

    #[allow(dead_code)]
    fn boolean() -> Self {
        Self {
            ty: "boolean".into(),
            properties: None,
            required: None,
            items: None,
            description: None,
            enum_values: None,
        }
    }

    #[allow(dead_code)]
    fn array(item_ty: ToolInputSchema) -> Self {
        Self {
            ty: "array".into(),
            properties: None,
            required: None,
            items: Some(serde_json::to_value(&item_ty).unwrap_or(Value::Null)),
            description: None,
            enum_values: None,
        }
    }
}

pub struct ToolProperty {
    pub type_: String,
    pub description: Option<String>,
    pub enum_values: Option<Vec<String>>,
}

/// Tool definition
#[derive(Debug, Clone, serde::Serialize)]
pub struct Tool {
    pub name: String,
    pub description: String,
    pub input_schema: ToolInputSchema,
}

/// Tool handler trait — each tool implements this
#[async_trait]
pub trait ToolHandler: Send + Sync {
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    fn input_schema(&self) -> ToolInputSchema;
    async fn call(&self, params: Value) -> Result<Value, String>;
}

// =============================================================================
// Protocol server
// =============================================================================

pub struct McpServer {
    tools: Arc<RwLock<HashMap<String, Box<dyn ToolHandler>>>>,
}

impl Default for McpServer {
    fn default() -> Self {
        Self::new()
    }
}

impl McpServer {
    pub fn new() -> Self {
        Self {
            tools: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn register<H: ToolHandler + 'static>(&self, handler: H) {
        let name = handler.name().to_string();
        let mut tools = self.tools.write().await;
        tools.insert(name, Box::new(handler));
    }

    pub async fn list_tools(&self) -> Vec<Tool> {
        let tools = self.tools.read().await;
        tools
            .values()
            .map(|h| Tool {
                name: h.name().to_string(),
                description: h.description().to_string(),
                input_schema: h.input_schema(),
            })
            .collect()
    }

    pub async fn handle_request(&self, raw: &[u8]) -> Vec<u8> {
        // Parse JSON-RPC request
        let req: Result<JsonRpcRequest, _> = serde_json::from_slice(raw);

        let req = match req {
            Ok(r) => r,
            Err(e) => {
                let resp = JsonRpcResponse::Error(JsonRpcError::new(
                    ERR_PARSE_ERROR,
                    format!("Parse error: {}", e),
                    Value::Null,
                ));
                return serde_json::to_vec(&resp).unwrap_or_default();
            }
        };

        // Handle batch or single
        let responses = if serde_json::json!(req.jsonrpc) != "2.0" {
            vec![JsonRpcResponse::Error(JsonRpcError::new(
                ERR_INVALID_REQUEST,
                "Invalid JSON-RPC version",
                req.id,
            ))]
        } else {
            self.dispatch(req).await
        };

        // Handle empty batch
        if responses.is_empty() {
            return serde_json::to_vec(&JsonRpcResponse::Error(JsonRpcError::new(
                ERR_INVALID_REQUEST,
                "Empty batch",
                Value::Null,
            )))
            .unwrap_or_default();
        }

        // Single response or batch

        if responses.len() == 1 {
            serde_json::to_vec(&responses[0]).unwrap_or_default()
        } else {
            serde_json::to_vec(&responses).unwrap_or_default()
        }
    }

    async fn dispatch(&self, req: JsonRpcRequest) -> Vec<JsonRpcResponse> {
        match req.method.as_str() {
            "tools/list" => self.handle_tools_list(req.id).await,
            "tools/call" => self.handle_tools_call(req.params, req.id).await,
            "ping" => self.handle_ping(req.id).await,
            _ => vec![JsonRpcResponse::Error(JsonRpcError::new(
                ERR_METHOD_NOT_FOUND,
                format!("Method not found: {}", req.method),
                req.id,
            ))],
        }
    }

    async fn handle_tools_list(&self, id: Value) -> Vec<JsonRpcResponse> {
        let tools = self.list_tools().await;
        let result = serde_json::json!({ "tools": tools });
        vec![JsonRpcResponse::Success(JsonRpcSuccess {
            jsonrpc: "2.0".into(),
            result,
            id,
        })]
    }

    async fn handle_tools_call(&self, params: Option<Value>, id: Value) -> Vec<JsonRpcResponse> {
        let params = match params {
            Some(p) => p,
            None => {
                return vec![JsonRpcResponse::Error(JsonRpcError::new(
                    ERR_INVALID_PARAMS,
                    "Missing params",
                    id,
                ))];
            }
        };

        let name = match params.get("name").and_then(|v| v.as_str()) {
            Some(n) => n,
            None => {
                return vec![JsonRpcResponse::Error(JsonRpcError::new(
                    ERR_INVALID_PARAMS,
                    "Missing tool name",
                    id,
                ))];
            }
        };

        let arguments = params.get("arguments").cloned().unwrap_or(Value::Null);

        let tools = self.tools.read().await;
        match tools.get(name) {
            Some(handler) => match handler.call(arguments).await {
                Ok(result) => vec![JsonRpcResponse::Success(JsonRpcSuccess {
                    jsonrpc: "2.0".into(),
                    result: serde_json::json!({
                        "content": [
                            {
                                "type": "text",
                                "text": serde_json::to_string_pretty(&result).unwrap_or_else(|_| "{}".into())
                            }
                        ]
                    }),
                    id,
                })],
                Err(e) => vec![JsonRpcResponse::Error(JsonRpcError::new(
                    ERR_INTERNAL_ERROR,
                    e,
                    id,
                ))],
            },
            None => vec![JsonRpcResponse::Error(JsonRpcError::new(
                ERR_METHOD_NOT_FOUND,
                format!("Tool not found: {}", name),
                id,
            ))],
        }
    }

    async fn handle_ping(&self, id: Value) -> Vec<JsonRpcResponse> {
        vec![JsonRpcResponse::Success(JsonRpcSuccess {
            jsonrpc: "2.0".into(),
            result: serde_json::json!({}),
            id,
        })]
    }
}

// =============================================================================
// Convenience constructors for tool schemas
// =============================================================================

pub fn string_prop(desc: &str) -> ToolProperty {
    ToolProperty {
        type_: "string".into(),
        description: Some(desc.into()),
        enum_values: None,
    }
}

pub fn string_enum_prop(values: Vec<String>, desc: &str) -> ToolProperty {
    ToolProperty {
        type_: "string".into(),
        description: Some(desc.into()),
        enum_values: Some(values),
    }
}

pub fn integer_prop(desc: &str) -> ToolProperty {
    ToolProperty {
        type_: "integer".into(),
        description: Some(desc.into()),
        enum_values: None,
    }
}

pub fn number_prop(desc: &str) -> ToolProperty {
    ToolProperty {
        type_: "number".into(),
        description: Some(desc.into()),
        enum_values: None,
    }
}

pub fn array_prop(_item: ToolInputSchema, desc: &str) -> ToolProperty {
    ToolProperty {
        type_: "array".into(),
        description: Some(desc.into()),
        enum_values: None,
    }
}
