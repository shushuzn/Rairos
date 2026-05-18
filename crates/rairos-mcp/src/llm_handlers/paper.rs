use crate::protocol::{ToolHandler, ToolInputSchema, ToolProperty};
use crate::llm_handlers::helpers::llm_client;
use async_trait::async_trait;
use serde_json::Value;
use std::collections::HashMap;

pub struct PaperCompareHandler;

#[async_trait]
impl ToolHandler for PaperCompareHandler {
    fn name(&self) -> &str { "paper_compare" }
    fn description(&self) -> &str { "Compare multiple papers side-by-side by methods, datasets, metrics" }
    fn input_schema(&self) -> ToolInputSchema {
        ToolInputSchema::object(
            vec![
                ("papers".into(), ToolProperty::string("JSON array of {paper_id, title, year, authors, abstract_text, method, dataset}")),
                ("aspects".into(), ToolProperty::string("Optional comma-separated aspects: methods,datasets,metrics,authors,year,abstract")),
            ].into_iter().collect(),
            vec!["papers".into()],
        )
    }
    async fn call(&self, params: Value) -> Result<Value, String> {
        let raw = params["papers"].as_str().ok_or("Missing papers (JSON string)")?;
        let papers: Vec<rairos_llm::paper_comparison::ComparisonColumn> = serde_json::from_str(raw)
            .map_err(|e| format!("Invalid paper JSON: {}", e))?;

        let aspects_str = params.get("aspects").and_then(|v| v.as_str()).unwrap_or("methods,datasets,metrics,authors");
        let aspects: Vec<&str> = aspects_str.split(',').map(|s| s.trim()).collect();

        let rows = rairos_llm::paper_comparison::compare(&aspects, &papers);
        Ok(serde_json::json!({"columns": papers, "aspect_rows": rows}))
    }
}

pub struct PaperAnalyzeMcpHandler;

#[async_trait]
impl ToolHandler for PaperAnalyzeMcpHandler {
    fn name(&self) -> &str { "paper_analyze" }
    fn description(&self) -> &str { "Analyze a paper: extract claims, methods, contributions, limitations" }
    fn input_schema(&self) -> ToolInputSchema {
        ToolInputSchema::object(
            vec![
                ("paper_id".into(), ToolProperty::string("Paper ID")),
                ("title".into(), ToolProperty::string("Paper title")),
                ("abstract_text".into(), ToolProperty::string("Paper abstract")),
                ("sections".into(), ToolProperty::string("Optional JSON array of section dicts")),
            ].into_iter().collect(),
            vec!["paper_id".into(), "title".into()],
        )
    }
    async fn call(&self, params: Value) -> Result<Value, String> {
        let paper_id = params["paper_id"].as_str().ok_or("Missing paper_id")?;
        let title = params["title"].as_str().ok_or("Missing title")?;
        let abstract_text = params.get("abstract_text").and_then(|v| v.as_str()).unwrap_or("");
        let authors = params.get("authors").and_then(|v| v.as_str()).unwrap_or("");

        if let Some(client) = llm_client() {
            let body = params.get("body").and_then(|v| v.as_str()).unwrap_or("");
            let result = rairos_llm::paper_analyzer::analyze_paper(
                client.as_ref(), crate::llm_handlers::helpers::llm_model(), title, abstract_text, authors, body,
            ).await;
            Ok(serde_json::json!(result))
        } else {
            Ok(serde_json::json!({
                "paper_id": paper_id,
                "title": title,
                "note": "No LLM available — set OPENAI_API_KEY or ANTHROPIC_API_KEY for full analysis",
                "keywords_found": rairos_llm::paper_comparison::extract_methods(title, abstract_text, ""),
            }))
        }
    }
}
