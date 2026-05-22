//! CGCNN property prediction tool.

use async_trait::async_trait;
use crate::error::ToolError;
use crate::tool_trait::{MaterialTool, ToolParams, ToolOutput};

/// Tool for predicting material properties using CGCNN.
///
/// This tool wraps a pre-trained CGCNN model for property prediction.
/// In production, this would call a model server or use a local inference engine.
pub struct CgcnnRegressor {
    model_endpoint: String,
    timeout_secs: u64,
}

impl CgcnnRegressor {
    /// Create a new CGCNN regressor tool.
    pub fn new(model_endpoint: impl Into<String>) -> Self {
        Self {
            model_endpoint: model_endpoint.into(),
            timeout_secs: 300,
        }
    }

    /// Predict formation energy.
    pub async fn predict_formation_energy(&self, cif: &str) -> Result<f64, ToolError> {
        let client = reqwest::Client::new();
        let response = client
            .post(format!("{}/predict/formation_energy", self.model_endpoint))
            .json(&serde_json::json!({ "cif": cif }))
            .timeout(std::time::Duration::from_secs(self.timeout_secs))
            .send()
            .await?;

        let result: serde_json::Value = response.json().await?;
        result
            .get("formation_energy")
            .and_then(|v| v.as_f64())
            .ok_or_else(|| ToolError::ExecutionFailed("Invalid response".to_string()))
    }

    /// Predict elastic tensor properties.
    pub async fn predict_elastic_tensor(&self, cif: &str) -> Result<serde_json::Value, ToolError> {
        let client = reqwest::Client::new();
        let response = client
            .post(format!("{}/predict/elastic_tensor", self.model_endpoint))
            .json(&serde_json::json!({ "cif": cif }))
            .timeout(std::time::Duration::from_secs(self.timeout_secs))
            .send()
            .await?;

        let result: serde_json::Value = response.json().await?;
        Ok(result)
    }
}

#[async_trait]
impl MaterialTool for CgcnnRegressor {
    fn name(&self) -> &str {
        "cgcnn_regression"
    }

    fn description(&self) -> &str {
        r#"Predict material properties using CGCNN (Crystal Graph Convolutional Neural Networks).

Inputs:
  - structure: CIF string of the crystal structure
  - property: Property to predict ("formation_energy" | "elastic_tensor" | "bandgap")

Output:
  - For formation_energy: { "formation_energy": <float> } in eV/atom
  - For elastic_tensor: { "elastic_constants": [<3x3 matrix>], "bulk_modulus": <float>, "shear_modulus": <float> }
  - For bandgap: { "bandgap": <float> } in eV
"#
    }

    async fn execute(&self, params: ToolParams) -> Result<ToolOutput, ToolError> {
        if let Err(e) = self.validate_inputs(&params) {
            return Ok(ToolOutput::failure(e));
        }

        let cif = params
            .get_str("structure")
            .ok_or_else(|| ToolError::InvalidInput("structure is required".to_string()))?;

        let property = params
            .get_str("property")
            .unwrap_or("formation_energy");

        let result = match property {
            "formation_energy" => {
                let value = self.predict_formation_energy(cif).await?;
                serde_json::json!({ "formation_energy": value })
            }
            "elastic_tensor" => self.predict_elastic_tensor(cif).await?,
            "bandgap" => {
                // CGCNN doesn't directly predict bandgap, this is a placeholder
                serde_json::json!({ "bandgap": null, "note": "Use a different model for bandgap prediction" })
            }
            _ => {
                return Ok(ToolOutput::failure(format!(
                    "Unknown property: {}. Use: formation_energy, elastic_tensor",
                    property
                )));
            }
        };

        Ok(ToolOutput::success(result))
    }

    fn validate_inputs(&self, params: &ToolParams) -> Result<(), String> {
        if !params.inputs.contains_key("structure") {
            return Err("structure (CIF string) is required".to_string());
        }

        let cif = params.get_str("structure").unwrap();
        if cif.is_empty() {
            return Err("structure cannot be empty".to_string());
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn test_validate_inputs() {
        let tool = CgcnnRegressor::new("http://localhost:8080");
        let mut inputs = HashMap::new();
        inputs.insert(
            "structure".to_string(),
            serde_json::json!("data_test\n# mock CIF"),
        );
        let params = ToolParams::new("cgcnn", inputs);

        assert!(tool.validate_inputs(&params).is_ok());
    }

    #[test]
    fn test_validate_inputs_missing_structure() {
        let tool = CgcnnRegressor::new("http://localhost:8080");
        let inputs = HashMap::new();
        let params = ToolParams::new("cgcnn", inputs);

        assert!(tool.validate_inputs(&params).is_err());
    }
}
