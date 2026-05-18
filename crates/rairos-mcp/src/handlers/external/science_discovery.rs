use crate::handlers::helpers::data_dir;
use crate::protocol::{ToolHandler, ToolInputSchema, ToolProperty};
use async_trait::async_trait;
use serde_json::Value;

pub struct PaperScienceDiscoveryHandler;

#[async_trait]
impl ToolHandler for PaperScienceDiscoveryHandler {
    fn name(&self) -> &str { "paper_science_discovery" }
    fn description(&self) -> &str { "Discover scientific AI models and datasets from HuggingFace for a research topic" }
    fn input_schema(&self) -> ToolInputSchema {
        ToolInputSchema::object(
            vec![
                ("query".into(), ToolProperty::string("Research topic or scientific domain (e.g., 'protein language model', 'molecular dynamics')")),
                ("resource_type".into(), ToolProperty::string("Type: model, dataset, or all (default: all)")),
            ].into_iter().collect(),
            vec!["query".into()],
        )
    }
    async fn call(&self, params: Value) -> Result<Value, String> {
        let query = params["query"].as_str().ok_or("Missing query")?;
        let resource_type = params.get("resource_type").and_then(|v| v.as_str()).unwrap_or("all");

        let client = crate::handlers::helpers::http_client_default()?;

        let query_encoded = urlencoding::encode(query);

        let mut results = serde_json::json!({
            "query": query,
            "models": [],
            "datasets": [],
        });

        if resource_type == "all" || resource_type == "model" {
            let models_url = format!(
                "https://huggingface.co/api/models?search={}&sort=downloads&direction=-1&limit=10",
                query_encoded
            );
            if let Ok(resp) = client.get(&models_url).send().await {
                if resp.status().is_success() {
                    if let Ok(data) = resp.json::<serde_json::Value>().await {
                        if let Some(arr) = data.as_array() {
                            let models: Vec<Value> = arr.iter()
                                .take(10)
                                .map(|m| {
                                    serde_json::json!({
                                        "id": m["id"],
                                        "downloads": m["downloads"],
                                        "likes": m["likes"],
                                        "tags": m["tags"].as_array().map(|t| t.iter().filter_map(|v| v.as_str()).take(5).collect::<Vec<_>>()).unwrap_or_default(),
                                        "pipeline_tag": m["pipeline_tag"],
                                    })
                                })
                                .collect();
                            results["models"] = serde_json::json!(models);
                        }
                    }
                }
            }
        }

        if resource_type == "all" || resource_type == "dataset" {
            let datasets_url = format!(
                "https://huggingface.co/api/datasets?search={}&sort=downloads&direction=-1&limit=10",
                query_encoded
            );
            if let Ok(resp) = client.get(&datasets_url).send().await {
                if resp.status().is_success() {
                    if let Ok(data) = resp.json::<serde_json::Value>().await {
                        if let Some(arr) = data.as_array() {
                            let datasets: Vec<Value> = arr.iter()
                                .take(10)
                                .map(|d| {
                                    serde_json::json!({
                                        "id": d["id"],
                                        "downloads": d["downloads"],
                                        "likes": d["likes"],
                                        "tags": d["tags"].as_array().map(|t| t.iter().filter_map(|v| v.as_str()).take(5).collect::<Vec<_>>()).unwrap_or_default(),
                                    })
                                })
                                .collect();
                            results["datasets"] = serde_json::json!(datasets);
                        }
                    }
                }
            }
        }

        let model_count = results["models"].as_array().map(|a| a.len()).unwrap_or(0);
        let dataset_count = results["datasets"].as_array().map(|a| a.len()).unwrap_or(0);

        Ok(serde_json::json!({
            "query": query,
            "models_count": model_count,
            "datasets_count": dataset_count,
            "models": results["models"],
            "datasets": results["datasets"],
        }))
    }
}
