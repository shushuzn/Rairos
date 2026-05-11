//! rairos-mcp-jin10 — Jin10 Financial Data MCP Client.
//!
//! Ported from `llm/tool/mcp_jin10.py`.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

const MCP_URL: &str = "https://mcp.jin10.com/mcp";
const MCP_VERSION: &str = "2025-11-25";

#[derive(Debug, thiserror::Error)]
pub enum MCPError {
    #[error("JSON-RPC error [{code}]: {message}")]
    JsonRpc { code: i32, message: String },
    #[error("HTTP error: {0}")]
    Http(String),
    #[error("Invalid JSON-RPC response: {0}")]
    InvalidResponse(String),
    #[error("No matching response in SSE for method '{0}'")]
    SseNoMatch(String),
}

impl From<serde_json::Error> for MCPError {
    fn from(e: serde_json::Error) -> Self {
        MCPError::InvalidResponse(e.to_string())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcRequest {
    pub jsonrpc: &'static str,
    pub id: i64,
    pub method: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<Value>,
}

impl JsonRpcRequest {
    pub fn new(method: &str, id: i64) -> Self {
        Self {
            jsonrpc: "2.0",
            id,
            method: method.to_string(),
            params: None,
        }
    }

    pub fn with_params(mut self, params: Value) -> Self {
        self.params = Some(params);
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcResponse {
    pub jsonrpc: String,
    pub id: i64,
    #[serde(default)]
    pub result: Option<Value>,
    #[serde(default)]
    pub error: Option<JsonRpcError>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcError {
    pub code: i32,
    pub message: String,
}

pub struct Jin10Client {
    url: String,
    token: String,
    headers: HashMap<String, String>,
    session_id: Option<String>,
    initialized: bool,
    tools: HashMap<String, Value>,
    resources: HashMap<String, Value>,
    client: reqwest::Client,
}

impl Jin10Client {
    pub fn new(url: Option<&str>, token: Option<&str>) -> Self {
        let url = url.unwrap_or(MCP_URL).to_string();
        let token = token.unwrap_or("").to_string();

        let mut headers = HashMap::new();
        headers.insert("Content-Type".to_string(), "application/json".to_string());
        if !token.is_empty() {
            headers.insert("Authorization".to_string(), format!("Bearer {}", token));
        }

        Self {
            url,
            token,
            headers,
            session_id: None,
            initialized: false,
            tools: HashMap::new(),
            resources: HashMap::new(),
            client: reqwest::Client::new(),
        }
    }

    pub fn url(&self) -> &str {
        &self.url
    }

    pub fn is_initialized(&self) -> bool {
        self.initialized
    }

    async fn call(&mut self, method: &str, params: Option<Value>, id: i64) -> Result<Value, MCPError> {
        let mut request = JsonRpcRequest::new(method, id);
        if let Some(p) = params {
            request = request.with_params(p);
        }

        let mut req_builder = self
            .client
            .post(&self.url)
            .header("Content-Type", "application/json");

        if let Some(ref session_id) = self.session_id {
            req_builder = req_builder.header("Mcp-Session-Id", session_id);
        }

        for (k, v) in &self.headers {
            if k != "Content-Type" {
                req_builder = req_builder.header(k.as_str(), v.as_str());
            }
        }

        let response = req_builder
            .json(&request)
            .send()
            .await
            .map_err(|e| MCPError::Http(e.to_string()))?;

        let status = response.status();
        if !status.is_success() {
            return Err(MCPError::Http(format!(
                "HTTP {}: {}",
                status.as_u16(),
                status.canonical_reason().unwrap_or("Unknown")
            )));
        }

        let content_type = response
            .headers()
            .get("Content-Type")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");

        if content_type.contains("event-stream") {
            let headers_map: HashMap<String, String> = response
                .headers()
                .iter()
                .filter_map(|(k, v)| {
                    v.to_str().ok().map(|s| (k.to_string(), s.to_string()))
                })
                .collect();

            if let Some(sid) = headers_map.get("Mcp-Session-Id") {
                self.session_id = Some(sid.clone());
            }

            let body = response
                .text()
                .await
                .map_err(|e| MCPError::Http(e.to_string()))?;

            for line in body.lines() {
                let line = line.trim();
                if let Some(data_str) = line.strip_prefix("data: ") {
                    if let Ok(data) = serde_json::from_str::<Value>(data_str) {
                        if let Some(err) = data.get("error") {
                            let code = err
                                .get("code")
                                .and_then(|v| v.as_i64())
                                .unwrap_or(0) as i32;
                            let message = err
                                .get("message")
                                .and_then(|v| v.as_str())
                                .unwrap_or("Unknown error")
                                .to_string();
                            return Err(MCPError::JsonRpc { code, message });
                        }
                        let response_id = data.get("id").and_then(|v| v.as_i64()).unwrap_or(0);
                        if response_id == id {
                            return Ok(data.get("result").cloned().unwrap_or(Value::Null));
                        }
                    }
                }
            }

            return Err(MCPError::SseNoMatch(method.to_string()));
        }

        let data: Value = response
            .json()
            .await
            .map_err(|e| MCPError::InvalidResponse(e.to_string()))?;

        if let Some(err) = data.get("error") {
            let code = err.get("code").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
            let message = err
                .get("message")
                .and_then(|v| v.as_str())
                .unwrap_or("Unknown error")
                .to_string();
            return Err(MCPError::JsonRpc { code, message });
        }

        Ok(data.get("result").cloned().unwrap_or(Value::Null))
    }

    pub async fn initialize(&mut self) -> Result<Value, MCPError> {
        let params = serde_json::json!({
            "protocolVersion": MCP_VERSION,
            "capabilities": {},
            "clientInfo": {
                "name": "rairos",
                "version": "1.0"
            }
        });

        let result = self.call("initialize", Some(params), 1).await?;
        self.initialized = true;
        Ok(result)
    }

    pub async fn tools_list(&mut self) -> Result<Vec<Value>, MCPError> {
        let result = self.call("tools/list", None, 2).await?;
        let tools = result
            .get("tools")
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default();

        self.tools = tools
            .iter()
            .filter_map(|t| {
                t.get("name")
                    .and_then(|n| n.as_str())
                    .map(|name| (name.to_string(), t.clone()))
            })
            .collect();

        Ok(tools)
    }

    pub async fn resources_list(&mut self) -> Result<Vec<Value>, MCPError> {
        let result = self.call("resources/list", None, 3).await?;
        let resources = result
            .get("resources")
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default();

        self.resources = resources
            .iter()
            .filter_map(|t| {
                t.get("name")
                    .and_then(|n| n.as_str())
                    .map(|name| (name.to_string(), t.clone()))
            })
            .collect();

        Ok(resources)
    }

    pub async fn call_tool(
        &mut self,
        name: &str,
        arguments: HashMap<String, Value>,
    ) -> Result<Value, MCPError> {
        if !self.initialized {
            self.initialize().await?;
            self.tools_list().await?;
        }

        let params = serde_json::json!({
            "name": name,
            "arguments": arguments
        });

        self.call("tools/call", Some(params), 4).await
    }

    pub async fn read_resource(&mut self, uri: &str) -> Result<Value, MCPError> {
        if !self.initialized {
            self.initialize().await?;
        }

        let params = serde_json::json!({ "uri": uri });
        self.call("resources/read", Some(params), 5).await
    }

    pub async fn ensure_init(&mut self) -> Result<(), MCPError> {
        if !self.initialized {
            self.initialize().await?;
            self.tools_list().await?;
        }
        Ok(())
    }

    fn extract_content(result: &Value) -> Value {
        result
            .get("structuredContent")
            .or(result.get("content"))
            .cloned()
            .unwrap_or(Value::Null)
    }

    pub async fn get_quote(&mut self, code: &str) -> Result<Value, MCPError> {
        self.ensure_init().await?;
        let args = serde_json::json!({ "code": code });
        let result = self.call_tool("get_quote", serde_json::from_value(args)?).await?;
        Ok(Self::extract_content(&result))
    }

    pub async fn get_kline(
        &mut self,
        code: &str,
        time: i32,
        count: i32,
    ) -> Result<Value, MCPError> {
        self.ensure_init().await?;
        let args = serde_json::json!({
            "code": code,
            "time": time,
            "count": count
        });
        let result = self.call_tool("get_kline", serde_json::from_value(args)?).await?;
        Ok(Self::extract_content(&result))
    }

    pub async fn list_flash(&mut self, cursor: Option<&str>) -> Result<Value, MCPError> {
        self.ensure_init().await?;
        let mut args = serde_json::json!({});
        if let Some(c) = cursor {
            args["cursor"] = serde_json::json!(c);
        }
        let result = self.call_tool("list_flash", serde_json::from_value(args)?).await?;
        Ok(Self::extract_content(&result))
    }

    pub async fn search_flash(&mut self, keyword: &str) -> Result<Value, MCPError> {
        self.ensure_init().await?;
        let args = serde_json::json!({ "keyword": keyword });
        let result = self.call_tool("search_flash", serde_json::from_value(args)?).await?;
        Ok(Self::extract_content(&result))
    }

    pub async fn list_news(&mut self, cursor: Option<&str>) -> Result<Value, MCPError> {
        self.ensure_init().await?;
        let mut args = serde_json::json!({});
        if let Some(c) = cursor {
            args["cursor"] = serde_json::json!(c);
        }
        let result = self.call_tool("list_news", serde_json::from_value(args)?).await?;
        Ok(Self::extract_content(&result))
    }

    pub async fn search_news(&mut self, keyword: &str, cursor: Option<&str>) -> Result<Value, MCPError> {
        self.ensure_init().await?;
        let mut args = serde_json::json!({ "keyword": keyword });
        if let Some(c) = cursor {
            args["cursor"] = serde_json::json!(c);
        }
        let result = self.call_tool("search_news", serde_json::from_value(args)?).await?;
        Ok(Self::extract_content(&result))
    }

    pub async fn get_news(&mut self, id: &str) -> Result<Value, MCPError> {
        self.ensure_init().await?;
        let args = serde_json::json!({ "id": id });
        let result = self.call_tool("get_news", serde_json::from_value(args)?).await?;
        Ok(Self::extract_content(&result))
    }

    pub async fn list_calendar(&mut self) -> Result<Value, MCPError> {
        self.ensure_init().await?;
        let args = serde_json::json!({});
        let result = self.call_tool("list_calendar", serde_json::from_value(args)?).await?;
        Ok(Self::extract_content(&result))
    }

    pub async fn list_symbols(&mut self) -> Result<Vec<Value>, MCPError> {
        let result = self.read_resource("quote://codes").await?;
        let contents = result.get("contents").and_then(|v| v.as_array()).cloned();

        if let Some(contents) = contents {
            if let Some(first) = contents.first() {
                let text = first.get("text").and_then(|v| v.as_str()).unwrap_or("{}");
                if let Ok(parsed) = serde_json::from_str::<Value>(text) {
                    return Ok(parsed.get("data").and_then(|v| v.as_array()).cloned().unwrap_or_default());
                }
            }
        }

        Ok(Vec::new())
    }

    pub fn get_tools(&self) -> &HashMap<String, Value> {
        &self.tools
    }

    pub fn get_resources(&self) -> &HashMap<String, Value> {
        &self.resources
    }
}

impl Default for Jin10Client {
    fn default() -> Self {
        Self::new(None, None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_json_rpc_request_new() {
        let req = JsonRpcRequest::new("test_method", 123);
        assert_eq!(req.jsonrpc, "2.0");
        assert_eq!(req.id, 123);
        assert_eq!(req.method, "test_method");
        assert!(req.params.is_none());
    }

    #[test]
    fn test_json_rpc_request_with_params() {
        let req = JsonRpcRequest::new("test", 1).with_params(serde_json::json!({"key": "value"}));
        assert!(req.params.is_some());
        let params = req.params.unwrap();
        assert_eq!(params["key"], "value");
    }

    #[test]
    fn test_jin10_client_default() {
        let client = Jin10Client::default();
        assert_eq!(client.url, MCP_URL);
        assert!(!client.is_initialized());
    }

    #[test]
    fn test_jin10_client_custom_url_token() {
        let client = Jin10Client::new(Some("http://custom.url"), Some("token123"));
        assert_eq!(client.url, "http://custom.url");
        assert!(!client.is_initialized());
    }

    #[test]
    fn test_jin10_client_url_getter() {
        let client = Jin10Client::new(Some("http://test.url"), None);
        assert_eq!(client.url(), "http://test.url");
    }

    #[tokio::test]
    async fn test_jin10_client_not_initialized_by_default() {
        let client = Jin10Client::default();
        assert!(!client.is_initialized());
    }

    #[test]
    fn test_mcp_error_display() {
        let err = MCPError::JsonRpc { code: -32600, message: "Invalid Request".to_string() };
        assert!(err.to_string().contains("Invalid Request"));

        let err = MCPError::Http("Connection refused".to_string());
        assert!(err.to_string().contains("Connection refused"));
    }

    #[test]
    fn test_jin10_client_tools_initially_empty() {
        let client = Jin10Client::default();
        assert!(client.get_tools().is_empty());
    }

    #[test]
    fn test_jin10_client_resources_initially_empty() {
        let client = Jin10Client::default();
        assert!(client.get_resources().is_empty());
    }
}
