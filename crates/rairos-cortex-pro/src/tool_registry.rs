//! Tool Registry Module for centralized tool management.
//!
//! Based on research from:
//! - agentkit-tools-core - Tool trait with permission system
//! - agents_sdk - ToolRegistry with schema generation
//! - tensorzero - IndexMap-based registry for LLM function definitions
//!
//! ## Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────┐
//! │              ToolRegistry                     │
//! │  ┌─────────────────────────────────────┐   │
//! │  │ tools: HashMap<String, Box<dyn Tool>>│   │
//! │  └─────────────────────────────────────┘   │
//! └─────────────────────────────────────────────┘
//! ```

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// JSON value type
pub type JsonValue = serde_json::Value;

/// Tool parameter definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolParameter {
    pub name: String,
    pub param_type: String,
    pub description: String,
    pub required: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default: Option<JsonValue>,
}

/// Tool schema for LLM function calling
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolSchema {
    pub name: String,
    pub description: String,
    pub parameters: Vec<ToolParameter>,
}

/// Result of tool execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolExecResult {
    pub success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<JsonValue>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    pub execution_time_ms: u64,
}

impl ToolExecResult {
    pub fn success(data: JsonValue, execution_time_ms: u64) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
            execution_time_ms,
        }
    }

    pub fn error(msg: String, execution_time_ms: u64) -> Self {
        Self {
            success: false,
            data: None,
            error: Some(msg),
            execution_time_ms,
        }
    }
}

/// Tool execution context
#[derive(Debug, Clone)]
pub struct ToolContext {
    pub agent_id: Option<String>,
    pub session_id: Option<String>,
    pub user_id: Option<String>,
    pub metadata: HashMap<String, String>,
}

impl Default for ToolContext {
    fn default() -> Self {
        Self {
            agent_id: None,
            session_id: None,
            user_id: None,
            metadata: HashMap::new(),
        }
    }
}

/// Dynamic tool function type
pub type DynToolFunc = Arc<
    dyn Fn(JsonValue, ToolContext) -> std::pin::Pin<Box<dyn std::future::Future<Output = ToolExecResult> + Send>>
        + Send
        + Sync,
>;

/// A registered tool
#[derive(Clone)]
pub struct RegisteredTool {
    pub name: String,
    pub description: String,
    pub parameters: Vec<ToolParameter>,
    pub func: DynToolFunc,
}

impl std::fmt::Debug for RegisteredTool {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RegisteredTool")
            .field("name", &self.name)
            .field("description", &self.description)
            .field("parameters", &self.parameters)
            .finish()
    }
}

impl RegisteredTool {
    pub fn new(
        name: String,
        description: String,
        parameters: Vec<ToolParameter>,
        func: DynToolFunc,
    ) -> Self {
        Self {
            name,
            description,
            parameters,
            func,
        }
    }

    pub fn to_schema(&self) -> ToolSchema {
        ToolSchema {
            name: self.name.clone(),
            description: self.description.clone(),
            parameters: self.parameters.clone(),
        }
    }

    pub async fn execute(&self, params: JsonValue, context: ToolContext) -> ToolExecResult {
        (self.func)(params, context).await
    }
}

/// Thread-safe tool registry
#[derive(Debug, Clone)]
pub struct ToolRegistry {
    tools: Arc<RwLock<HashMap<String, RegisteredTool>>>,
    schemas: Arc<RwLock<Vec<ToolSchema>>>,
    dirty: Arc<RwLock<bool>>,
}

impl Default for ToolRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl ToolRegistry {
    /// Create a new empty registry
    pub fn new() -> Self {
        Self {
            tools: Arc::new(RwLock::new(HashMap::new())),
            schemas: Arc::new(RwLock::new(Vec::new())),
            dirty: Arc::new(RwLock::new(false)),
        }
    }

    /// Register a tool
    pub async fn register(&self, tool: RegisteredTool) -> Result<(), ToolRegistryError> {
        let name = tool.name.clone();

        if name.is_empty() {
            return Err(ToolRegistryError::InvalidTool(
                "Tool name cannot be empty".to_string(),
            ));
        }

        {
            let tools = self.tools.read().await;
            if tools.contains_key(&name) {
                return Err(ToolRegistryError::AlreadyExists(name));
            }
        }

        {
            let mut tools = self.tools.write().await;
            tools.insert(name, tool);
        }

        {
            let mut dirty = self.dirty.write().await;
            *dirty = true;
        }

        Ok(())
    }

    /// Unregister a tool
    pub async fn unregister(&self, name: &str) -> bool {
        let removed = {
            let mut tools = self.tools.write().await;
            tools.remove(name).is_some()
        };

        if removed {
            let mut dirty = self.dirty.write().await;
            *dirty = true;
        }

        removed
    }

    /// Check if a tool exists
    pub async fn has(&self, name: &str) -> bool {
        let tools = self.tools.read().await;
        tools.contains_key(name)
    }

    /// List all tool names
    pub async fn names(&self) -> Vec<String> {
        let tools = self.tools.read().await;
        tools.keys().cloned().collect()
    }

    /// Get the number of registered tools
    pub async fn len(&self) -> usize {
        let tools = self.tools.read().await;
        tools.len()
    }

    /// Check if the registry is empty
    pub async fn is_empty(&self) -> bool {
        let tools = self.tools.read().await;
        tools.is_empty()
    }

    /// Get all tool schemas
    pub async fn schemas(&self) -> Vec<ToolSchema> {
        {
            let dirty = self.dirty.read().await;
            if !*dirty {
                let schemas = self.schemas.read().await;
                return schemas.clone();
            }
        }

        let new_schemas: Vec<ToolSchema> = {
            let tools = self.tools.read().await;
            tools.values().map(|t| t.to_schema()).collect()
        };

        {
            let mut schemas = self.schemas.write().await;
            *schemas = new_schemas.clone();
        }
        {
            let mut dirty = self.dirty.write().await;
            *dirty = false;
        }

        new_schemas
    }

    /// Execute a tool by name
    pub async fn execute(
        &self,
        name: &str,
        params: JsonValue,
        context: ToolContext,
    ) -> Result<ToolExecResult, ToolRegistryError> {
        let tool = {
            let tools = self.tools.read().await;
            tools.get(name).cloned()
        };

        let tool = tool.ok_or_else(|| ToolRegistryError::NotFound(name.to_string()))?;

        let start = std::time::Instant::now();
        let result = tool.execute(params, context).await;
        let execution_time_ms = start.elapsed().as_millis() as u64;

        if result.success {
            Ok(ToolExecResult::success(
                result.data.unwrap_or(JsonValue::Null),
                execution_time_ms,
            ))
        } else {
            Ok(ToolExecResult::error(
                result.error.unwrap_or_else(|| "Unknown error".to_string()),
                execution_time_ms,
            ))
        }
    }

    /// Clear all tools
    pub async fn clear(&self) {
        let mut tools = self.tools.write().await;
        tools.clear();
        drop(tools);

        let mut dirty = self.dirty.write().await;
        *dirty = true;
    }
}

/// Errors
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ToolRegistryError {
    NotFound(String),
    AlreadyExists(String),
    InvalidTool(String),
    ExecutionError(String),
}

impl std::fmt::Display for ToolRegistryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ToolRegistryError::NotFound(name) => write!(f, "Tool not found: {}", name),
            ToolRegistryError::AlreadyExists(name) => write!(f, "Tool already registered: {}", name),
            ToolRegistryError::InvalidTool(msg) => write!(f, "Invalid tool: {}", msg),
            ToolRegistryError::ExecutionError(msg) => write!(f, "Execution error: {}", msg),
        }
    }
}

impl std::error::Error for ToolRegistryError {}

// =============================================================================
// Tool Builder
// =============================================================================

/// Builder for creating tools
pub struct ToolBuilder {
    name: Option<String>,
    description: String,
    parameters: Vec<ToolParameter>,
}

impl ToolBuilder {
    pub fn new() -> Self {
        Self {
            name: None,
            description: String::new(),
            parameters: Vec::new(),
        }
    }

    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    pub fn description(mut self, desc: impl Into<String>) -> Self {
        self.description = desc.into();
        self
    }

    pub fn required_param(
        mut self,
        name: impl Into<String>,
        param_type: impl Into<String>,
        description: impl Into<String>,
    ) -> Self {
        self.parameters.push(ToolParameter {
            name: name.into(),
            param_type: param_type.into(),
            description: description.into(),
            required: true,
            default: None,
        });
        self
    }

    pub fn optional_param(
        mut self,
        name: impl Into<String>,
        param_type: impl Into<String>,
        description: impl Into<String>,
        default: JsonValue,
    ) -> Self {
        self.parameters.push(ToolParameter {
            name: name.into(),
            param_type: param_type.into(),
            description: description.into(),
            required: false,
            default: Some(default),
        });
        self
    }

    pub fn build<F, Fut>(self, handler: F) -> Result<RegisteredTool, ToolRegistryError>
    where
        F: Fn(JsonValue, ToolContext) -> Fut + Send + Sync + 'static,
        Fut: std::future::Future<Output = ToolExecResult> + Send + 'static,
    {
        let name = self.name.ok_or_else(|| {
            ToolRegistryError::InvalidTool("Tool name is required".to_string())
        })?;

        // Wrap handler in Arc so it can be called multiple times
        let handler = Arc::new(handler);
        let func: DynToolFunc = Arc::new(move |params, ctx| {
            let handler = handler.clone();
            Box::pin(async move { handler(params, ctx).await })
        });

        Ok(RegisteredTool::new(
            name,
            self.description,
            self.parameters,
            func,
        ))
    }
}

impl Default for ToolBuilder {
    fn default() -> Self {
        Self::new()
    }
}

// =============================================================================
// Registry Builder
// =============================================================================

pub struct ToolRegistryBuilder {
    tools: Vec<RegisteredTool>,
}

impl Default for ToolRegistryBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl ToolRegistryBuilder {
    pub fn new() -> Self {
        Self { tools: Vec::new() }
    }

    pub fn with_tool(mut self, tool: RegisteredTool) -> Self {
        self.tools.push(tool);
        self
    }

    pub fn with_tool_func<F, Fut>(mut self, name: &str, desc: &str, handler: F) -> Self
    where
        F: Fn(JsonValue, ToolContext) -> Fut + Send + Sync + 'static,
        Fut: std::future::Future<Output = ToolExecResult> + Send + 'static,
    {
        let tool = ToolBuilder::new()
            .name(name)
            .description(desc)
            .build(handler)
            .unwrap();
        self.tools.push(tool);
        self
    }

    pub async fn build(self) -> ToolRegistry {
        let registry = ToolRegistry::new();
        for tool in self.tools {
            let _ = registry.register(tool).await;
        }
        registry
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_registry_basic() {
        let registry = ToolRegistry::new();

        let tool = ToolBuilder::new()
            .name("echo")
            .description("Echo back the input")
            .required_param("text", "string", "Text to echo")
            .build(|params: JsonValue, _ctx: ToolContext| {
                Box::pin(async move {
                    let text = params
                        .get("text")
                        .and_then(|v| v.as_str())
                        .unwrap_or("nothing");
                    ToolExecResult::success(JsonValue::String(format!("echo: {}", text)), 0)
                })
            })
            .unwrap();

        registry.register(tool).await.unwrap();
        assert!(registry.has("echo").await);
        assert_eq!(registry.len().await, 1);
    }

    #[tokio::test]
    async fn test_registry_execute() {
        let registry = ToolRegistry::new();

        let tool = ToolBuilder::new()
            .name("add")
            .description("Add two numbers")
            .required_param("a", "number", "First number")
            .required_param("b", "number", "Second number")
            .build(|params: JsonValue, _ctx: ToolContext| {
                Box::pin(async move {
                    let a = params.get("a").and_then(|v| v.as_f64()).unwrap_or(0.0);
                    let b = params.get("b").and_then(|v| v.as_f64()).unwrap_or(0.0);
                    let sum = a + b;
                    ToolExecResult::success(
                        JsonValue::Number(serde_json::Number::from_f64(sum).unwrap()),
                        0,
                    )
                })
            })
            .unwrap();

        registry.register(tool).await.unwrap();

        let result = registry
            .execute("add", serde_json::json!({"a": 5, "b": 3}), ToolContext::default())
            .await
            .unwrap();

        assert!(result.success);
        assert_eq!(result.data, Some(JsonValue::Number(serde_json::Number::from(8))));
    }

    #[tokio::test]
    async fn test_registry_schemas() {
        let registry = ToolRegistry::new();

        let tool = ToolBuilder::new()
            .name("calc")
            .description("Calculator")
            .required_param("x", "number", "First number")
            .optional_param("y", "number", "Second number", JsonValue::Number(serde_json::Number::from(0)))
            .build(|_params: JsonValue, _ctx: ToolContext| {
                Box::pin(async move { ToolExecResult::success(JsonValue::Null, 0) })
            })
            .unwrap();

        registry.register(tool).await.unwrap();

        let schemas = registry.schemas().await;
        assert_eq!(schemas.len(), 1);
        assert_eq!(schemas[0].name, "calc");
        assert_eq!(schemas[0].parameters.len(), 2);
    }

    #[tokio::test]
    async fn test_registry_builder() {
        let registry = ToolRegistryBuilder::new()
            .with_tool_func("tool1", "First tool", |_params: JsonValue, _ctx: ToolContext| {
                Box::pin(async move { ToolExecResult::success(JsonValue::Null, 0) })
            })
            .with_tool_func("tool2", "Second tool", |_params: JsonValue, _ctx: ToolContext| {
                Box::pin(async move { ToolExecResult::success(JsonValue::Null, 0) })
            })
            .build()
            .await;

        assert_eq!(registry.len().await, 2);
        assert!(registry.has("tool1").await);
        assert!(registry.has("tool2").await);
    }

    #[tokio::test]
    async fn test_unregister() {
        let registry = ToolRegistry::new();

        let tool = ToolBuilder::new()
            .name("temp")
            .description("Temporary tool")
            .build(|_params: JsonValue, _ctx: ToolContext| {
                Box::pin(async move { ToolExecResult::success(JsonValue::Null, 0) })
            })
            .unwrap();

        registry.register(tool).await.unwrap();
        assert!(registry.has("temp").await);

        let removed = registry.unregister("temp").await;
        assert!(removed);
        assert!(!registry.has("temp").await);
    }

    #[tokio::test]
    async fn test_not_found() {
        let registry = ToolRegistry::new();

        let result = registry
            .execute("nonexistent", JsonValue::Null, ToolContext::default())
            .await;

        assert!(result.is_err());
    }
}
