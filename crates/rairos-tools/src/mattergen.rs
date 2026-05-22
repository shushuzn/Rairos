//! MatterGen crystal generation tool.
//!
//! MatterGen is a diffusion-based Model for generating crystal structures
//! with target properties. This tool wraps a model server for property-based
//! crystal generation.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use crate::error::ToolError;
use crate::tool_trait::{MaterialTool, ToolParams, ToolOutput};

/// Generator mode for MatterGen.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum GenerationMode {
    /// Unconditioned generation (random valid crystals)
    Unconditioned,
    /// Conditioned on target property value
    Conditioned {
        property: String,
        target_value: f64,
    },
}

/// MatterGen crystal generation tool.
pub struct MatterGenGenerator {
    /// Model server endpoint
    model_endpoint: String,
    /// Default number of generations
    default_num: usize,
    /// Timeout for generation requests
    timeout_secs: u64,
}

impl MatterGenGenerator {
    /// Create a new MatterGen generator.
    pub fn new(model_endpoint: impl Into<String>) -> Self {
        Self {
            model_endpoint: model_endpoint.into(),
            default_num: 10,
            timeout_secs: 600,
        }
    }

    /// Set default number of structures to generate.
    pub fn with_default_num(mut self, num: usize) -> Self {
        self.default_num = num;
        self
    }

    /// Generate crystals unconditionally.
    pub async fn generate_unconditioned(&self, num: usize) -> Result<ToolOutput, ToolError> {
        let client = reqwest::Client::new();
        let response = client
            .post(format!("{}/generate/unconditioned", self.model_endpoint))
            .json(&serde_json::json!({
                "num_generations": num
            }))
            .timeout(std::time::Duration::from_secs(self.timeout_secs))
            .send()
            .await?;

        if !response.status().is_success() {
            return Ok(ToolOutput::failure(format!(
                "MatterGen error: {}",
                response.status()
            )));
        }

        let data: serde_json::Value = response.json().await?;
        Ok(ToolOutput::success(data))
    }

    /// Generate crystals conditioned on a target property.
    pub async fn generate_conditioned(
        &self,
        property: &str,
        target_value: f64,
        num: usize,
    ) -> Result<ToolOutput, ToolError> {
        let client = reqwest::Client::new();
        let response = client
            .post(format!("{}/generate/conditioned", self.model_endpoint))
            .json(&serde_json::json!({
                "property": property,
                "target_value": target_value,
                "num_generations": num
            }))
            .timeout(std::time::Duration::from_secs(self.timeout_secs))
            .send()
            .await?;

        if !response.status().is_success() {
            return Ok(ToolOutput::failure(format!(
                "MatterGen error: {}",
                response.status()
            )));
        }

        let data: serde_json::Value = response.json().await?;
        Ok(ToolOutput::success(data))
    }
}

#[async_trait]
impl MaterialTool for MatterGenGenerator {
    fn name(&self) -> &str {
        "generate_crystal_conditioned"
    }

    fn description(&self) -> &str {
        r#"Generate crystal structures using MatterGen diffusion model.

Modes:
1. Unconditioned: Generate random valid crystals
2. Conditioned: Generate crystals with target property value

Inputs:
  - mode: "unconditioned" or "conditioned"
  - property: Property to condition on (e.g., "bandgap", "formation_energy") (required for conditioned)
  - target_value: Target property value (required for conditioned)
  - num_generations: Number of structures to generate (default: 10)

Output:
  - structures: Array of generated CIF structures
  - properties: Predicted properties for each structure
  - scores: Generation confidence scores
"#
    }

    async fn execute(&self, params: ToolParams) -> Result<ToolOutput, ToolError> {
        if let Err(e) = self.validate_inputs(&params) {
            return Ok(ToolOutput::failure(e));
        }

        let mode = params
            .get_str("mode")
            .unwrap_or("unconditioned");

        let num = params
            .get_i64("num_generations")
            .unwrap_or(self.default_num as i64) as usize;

        match mode {
            "unconditioned" => self.generate_unconditioned(num).await,
            "conditioned" => {
                let property = params
                    .get_str("property")
                    .ok_or_else(|| ToolError::InvalidInput("property is required for conditioned mode".to_string()))?;

                let target_value = params
                    .get_f64("target_value")
                    .ok_or_else(|| ToolError::InvalidInput("target_value is required for conditioned mode".to_string()))?;

                self.generate_conditioned(property, target_value, num).await
            }
            _ => Ok(ToolOutput::failure(format!(
                "Unknown mode: {}. Use 'unconditioned' or 'conditioned'.",
                mode
            ))),
        }
    }

    fn validate_inputs(&self, params: &ToolParams) -> Result<(), String> {
        let mode = params.get_str("mode").unwrap_or("unconditioned");

        if mode == "conditioned" {
            if !params.inputs.contains_key("property") {
                return Err("property is required for conditioned mode".to_string());
            }
            if !params.inputs.contains_key("target_value") {
                return Err("target_value is required for conditioned mode".to_string());
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn test_validate_inputs_unconditioned() {
        let tool = MatterGenGenerator::new("http://localhost:8080");
        let inputs = HashMap::new();
        let params = ToolParams::new("mattergen", inputs);

        assert!(tool.validate_inputs(&params).is_ok());
    }

    #[test]
    fn test_validate_inputs_conditioned_missing_property() {
        let tool = MatterGenGenerator::new("http://localhost:8080");
        let mut inputs = HashMap::new();
        inputs.insert("mode".to_string(), serde_json::json!("conditioned"));
        inputs.insert("target_value".to_string(), serde_json::json!(1.5));
        let params = ToolParams::new("mattergen", inputs);

        assert!(tool.validate_inputs(&params).is_err());
    }

    #[test]
    fn test_validate_inputs_conditioned_valid() {
        let tool = MatterGenGenerator::new("http://localhost:8080");
        let mut inputs = HashMap::new();
        inputs.insert("mode".to_string(), serde_json::json!("conditioned"));
        inputs.insert("property".to_string(), serde_json::json!("bandgap"));
        inputs.insert("target_value".to_string(), serde_json::json!(1.5));
        let params = ToolParams::new("mattergen", inputs);

        assert!(tool.validate_inputs(&params).is_ok());
    }
}
