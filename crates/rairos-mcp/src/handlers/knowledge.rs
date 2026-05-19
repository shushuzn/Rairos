use crate::handlers::helpers::kg;
use crate::protocol::{ToolHandler, ToolInputSchema, ToolProperty};
use async_trait::async_trait;
use serde_json::Value;
use std::collections::HashMap;

pub struct CitationGraphHandler;

#[async_trait]
impl ToolHandler for CitationGraphHandler {
    fn name(&self) -> &str { "citation_graph" }
    fn description(&self) -> &str { "Get citation relationships between papers from arXiv data" }
    fn input_schema(&self) -> ToolInputSchema {
        ToolInputSchema::object(
            vec![
                ("arxiv_id".into(), ToolProperty::string("arXiv ID to get citations for")),
            ].into_iter().collect(),
            vec!["arxiv_id".into()],
        )
    }
    async fn call(&self, params: Value) -> Result<Value, String> {
        let arxiv_id = params["arxiv_id"].as_str().ok_or("Missing arxiv_id")?;
        let paper = rairos_parser::fetch_arxiv(arxiv_id)
            .await
            .map_err(|e| format!("arXiv fetch failed: {}", e))?;
        Ok(serde_json::json!({
            "paper": paper,
            "citations": [],
            "note": "Full citation graph requires Semantic Scholar API integration"
        }))
    }
}

pub struct KgPaperSubgraphHandler;

#[async_trait]
impl ToolHandler for KgPaperSubgraphHandler {
    fn name(&self) -> &str { "kg_paper_subgraph" }
    fn description(&self) -> &str { "Get the knowledge subgraph around a paper (nodes + edges up to depth)" }
    fn input_schema(&self) -> ToolInputSchema {
        ToolInputSchema::object(
            vec![
                ("arxiv_id".into(), ToolProperty::string("arXiv ID of the center paper")),
                ("depth".into(), ToolProperty::integer("Traversal depth (default 1, max 3)")),
                ("include_notes".into(), ToolProperty::string("Include note nodes (default true)")),
            ].into_iter().collect(),
            vec!["arxiv_id".into()],
        )
    }
    async fn call(&self, params: Value) -> Result<Value, String> {
        let arxiv_id = params["arxiv_id"].as_str().ok_or("Missing arxiv_id")?;
        let depth = params["depth"].as_u64().unwrap_or(1).min(3) as u32;
        let include_notes = params["include_notes"].as_str().map(|s| s != "false").unwrap_or(true);
        let sub = kg().get_paper_subgraph(arxiv_id, depth, include_notes)
            .map_err(|e| format!("Subgraph query: {}", e))?;
        Ok(serde_json::to_value(sub).unwrap_or_default())
    }
}

pub struct KgTagGraphHandler;

#[async_trait]
impl ToolHandler for KgTagGraphHandler {
    fn name(&self) -> &str { "kg_tag_graph" }
    fn description(&self) -> &str { "Get the knowledge graph for a tag — papers and notes connected by same_tag edges" }
    fn input_schema(&self) -> ToolInputSchema {
        ToolInputSchema::object(
            vec![
                ("tag".into(), ToolProperty::string("Tag name to query")),
            ].into_iter().collect(),
            vec!["tag".into()],
        )
    }
    async fn call(&self, params: Value) -> Result<Value, String> {
        let tag = params["tag"].as_str().ok_or("Missing tag")?;
        let sub = kg().get_tag_ecosystem(tag)
            .map_err(|e| format!("Tag ecosystem: {}", e))?;
        Ok(serde_json::to_value(sub).unwrap_or_default())
    }
}

pub struct KgFullGraphHandler;

#[async_trait]
impl ToolHandler for KgFullGraphHandler {
    fn name(&self) -> &str { "kg_full_graph" }
    fn description(&self) -> &str { "Export the entire knowledge graph as JSON (nodes + edges)" }
    fn input_schema(&self) -> ToolInputSchema {
        ToolInputSchema::object(HashMap::new(), vec![])
    }
    async fn call(&self, _params: Value) -> Result<Value, String> {
        let graph = kg();
        if let Some(db) = graph.database() {
            db.export_json(None).map_err(|e| format!("KG export: {}", e))
        } else {
            Ok(graph.export_json(None))
        }
    }
}

pub struct KgQueryHandler;

#[async_trait]
impl ToolHandler for KgQueryHandler {
    fn name(&self) -> &str { "kg_query" }
    fn description(&self) -> &str { "Query the knowledge graph by keyword — searches node labels and entity IDs" }
    fn input_schema(&self) -> ToolInputSchema {
        ToolInputSchema::object(
            vec![
                ("keyword".into(), ToolProperty::string("Keyword to search for in node labels/IDs")),
                ("limit".into(), ToolProperty::integer("Maximum results (default 20)")),
            ].into_iter().collect(),
            vec!["keyword".into()],
        )
    }
    async fn call(&self, params: Value) -> Result<Value, String> {
        let keyword = params["keyword"].as_str().ok_or("Missing keyword")?;
        let limit = params["limit"].as_u64().unwrap_or(20).min(100) as usize;
        let db = kg().database().ok_or("No database connected")?;
        let results = db.query_by_keyword(keyword, limit)
            .map_err(|e| format!("KG query: {}", e))?;
        Ok(serde_json::json!({"results": results, "total": results.len(), "keyword": keyword}))
    }
}
