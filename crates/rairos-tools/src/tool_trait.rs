//! MaterialTool trait definition.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use crate::error::ToolError;

/// Parameters for tool execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolParams {
    /// Tool name (set automatically)
    pub name: String,
    /// Tool input parameters
    pub inputs: HashMap<String, serde_json::Value>,
}

impl ToolParams {
    /// Create new tool params with inputs.
    pub fn new(name: impl Into<String>, inputs: HashMap<String, serde_json::Value>) -> Self {
        Self {
            name: name.into(),
            inputs,
        }
    }

    /// Get a string input value.
    pub fn get_str(&self, key: &str) -> Option<&str> {
        self.inputs.get(key)?.as_str()
    }

    /// Get a f64 input value.
    pub fn get_f64(&self, key: &str) -> Option<f64> {
        self.inputs.get(key)?.as_f64()
    }

    /// Get a i64 input value.
    pub fn get_i64(&self, key: &str) -> Option<i64> {
        self.inputs.get(key)?.as_i64()
    }

    /// Get a bool input value.
    pub fn get_bool(&self, key: &str) -> Option<bool> {
        self.inputs.get(key)?.as_bool()
    }
}

/// Output from tool execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolOutput {
    /// Whether the tool executed successfully.
    pub success: bool,
    /// Tool result data.
    pub result: serde_json::Value,
    /// Error message if failed.
    pub error: Option<String>,
}

impl ToolOutput {
    /// Create a successful output.
    pub fn success(result: impl Into<serde_json::Value>) -> Self {
        Self {
            success: true,
            result: result.into(),
            error: None,
        }
    }

    /// Create a failed output.
    pub fn failure(error: impl Into<String>) -> Self {
        Self {
            success: false,
            result: serde_json::Value::Null,
            error: Some(error.into()),
        }
    }
}

/// Unified interface for material science tools.
///
/// This trait defines the contract for all tools used in the
/// SparksMatter-style multi-agent workflow.
#[async_trait]
pub trait MaterialTool: Send + Sync {
    /// Returns the tool's unique identifier.
    fn name(&self) -> &str;

    /// Returns a description of the tool for LLM usage.
    ///
    /// Should describe:
    /// - What the tool does
    /// - Required inputs
    /// - Expected outputs
    fn description(&self) -> &str;

    /// Execute the tool with the given parameters.
    async fn execute(&self, params: ToolParams) -> Result<ToolOutput, ToolError>;

    /// Validate input parameters before execution.
    ///
    /// Returns `Ok(())` if valid, or error message if invalid.
    fn validate_inputs(&self, params: &ToolParams) -> Result<(), String> {
        let _ = params;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tool_params_get_str() {
        let mut inputs = HashMap::new();
        inputs.insert("formula".to_string(), serde_json::json!("Bi2Te3"));
        let params = ToolParams::new("test", inputs);

        assert_eq!(params.get_str("formula"), Some("Bi2Te3"));
        assert_eq!(params.get_str("missing"), None);
    }

    #[test]
    fn test_tool_params_get_f64() {
        let mut inputs = HashMap::new();
        inputs.insert("temperature".to_string(), serde_json::json!(300.0));
        let params = ToolParams::new("test", inputs);

        assert_eq!(params.get_f64("temperature"), Some(300.0));
    }

    #[test]
    fn test_tool_output_success() {
        let output = ToolOutput::success(serde_json::json!({"bandgap": 0.5}));
        assert!(output.success);
        assert!(output.error.is_none());
    }

    #[test]
    fn test_tool_output_failure() {
        let output: ToolOutput = ToolOutput::failure("Something went wrong");
        assert!(!output.success);
        assert_eq!(output.error, Some("Something went wrong".to_string()));
    }
}
