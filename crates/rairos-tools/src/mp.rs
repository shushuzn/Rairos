//! Materials Project API tool.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use crate::error::ToolError;
use crate::tool_trait::{MaterialTool, ToolParams, ToolOutput};

/// Tool for retrieving crystal structures from Materials Project.
pub struct MaterialsProjectTool {
    api_key: String,
    base_url: String,
}

impl MaterialsProjectTool {
    /// Create a new Materials Project tool.
    pub fn new(api_key: impl Into<String>) -> Self {
        Self {
            api_key: api_key.into(),
            base_url: "https://materialsproject.org/rest/v2".to_string(),
        }
    }

    /// Search for materials by formula.
    pub async fn search_by_formula(&self, formula: &str) -> Result<ToolOutput, ToolError> {
        let url = format!("{}/materials/{}/vasp", self.base_url, formula);
        let client = reqwest::Client::new();
        let response = client
            .get(&url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Accept", "application/json")
            .send()
            .await?;

        if !response.status().is_success() {
            return Ok(ToolOutput::failure(format!(
                "MP API error: {}",
                response.status()
            )));
        }

        let data: serde_json::Value = response.json().await?;
        Ok(ToolOutput::success(data))
    }

    /// Get structure by materials ID.
    pub async fn get_structure(&self, material_id: &str) -> Result<ToolOutput, ToolError> {
        let url = format!("{}/materials/{}/cif", self.base_url, material_id);
        let client = reqwest::Client::new();
        let response = client
            .get(&url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .send()
            .await?;

        if !response.status().is_success() {
            return Ok(ToolOutput::failure(format!(
                "MP API error: {}",
                response.status()
            )));
        }

        let cif: String = response.text().await?;
        Ok(ToolOutput::success(serde_json::json!({ "cif": cif })))
    }
}

#[async_trait]
impl MaterialTool for MaterialsProjectTool {
    fn name(&self) -> &str {
        "download_structures_from_mp"
    }

    fn description(&self) -> &str {
        r#"Download crystal structures from Materials Project.

Inputs:
  - formula: Chemical formula (e.g., "Bi2Te3", "Fe2O3")
  - properties: List of properties to retrieve (optional, default: ["structure"])

Output:
  - materials: Array of materials with requested properties
  - Each material contains: material_id, formula, structure (CIF), properties
"#
    }

    async fn execute(&self, params: ToolParams) -> Result<ToolOutput, ToolError> {
        if let Err(e) = self.validate_inputs(&params) {
            return Ok(ToolOutput::failure(e));
        }

        let formula = params
            .get_str("formula")
            .ok_or_else(|| ToolError::InvalidInput("formula is required".to_string()))?;

        self.search_by_formula(formula).await
    }

    fn validate_inputs(&self, params: &ToolParams) -> Result<(), String> {
        if !params.inputs.contains_key("formula") {
            return Err("formula is required".to_string());
        }

        let formula = params.get_str("formula").unwrap();
        if formula.is_empty() {
            return Err("formula cannot be empty".to_string());
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_inputs() {
        let tool = MaterialsProjectTool::new("test-key");
        let mut inputs = HashMap::new();
        inputs.insert("formula".to_string(), serde_json::json!("Bi2Te3"));
        let params = ToolParams::new("mp", inputs);

        assert!(tool.validate_inputs(&params).is_ok());
    }

    #[test]
    fn test_validate_inputs_missing() {
        let tool = MaterialsProjectTool::new("test-key");
        let inputs = HashMap::new();
        let params = ToolParams::new("mp", inputs);

        assert!(tool.validate_inputs(&params).is_err());
    }
}
