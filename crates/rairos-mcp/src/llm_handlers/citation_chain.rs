use crate::protocol::{ToolHandler, ToolInputSchema, ToolProperty};
use async_trait::async_trait;
use serde_json::Value;

pub struct CitationChainBuildHandler;

#[async_trait]
impl ToolHandler for CitationChainBuildHandler {
    fn name(&self) -> &str { "citation_chain_build" }
    fn description(&self) -> &str { "Build a citation chain starting from a seed paper" }
    fn input_schema(&self) -> ToolInputSchema {
        ToolInputSchema::object(
            vec![
                ("seed_arxiv_id".into(), ToolProperty::string("Seed arXiv ID")),
                ("max_depth".into(), ToolProperty::integer("Max depth (default 2)")),
            ].into_iter().collect(),
            vec!["seed_arxiv_id".into()],
        )
    }
    async fn call(&self, params: Value) -> Result<Value, String> {
        let seed = params["seed_arxiv_id"].as_str().ok_or("Missing seed_arxiv_id")?;
        let depth = params.get("max_depth").and_then(|v| v.as_u64()).unwrap_or(2) as u32;
        let chain = rairos_llm::citation_chain::build_chain(seed, depth).await
            .map_err(|e| format!("Build chain failed: {}", e))?;
        Ok(serde_json::json!({
            "seed": seed, "depth": depth,
            "nodes": chain.nodes, "edges": chain.edges,
            "total": chain.nodes.len(),
        }))
    }
}

pub struct CitationChainFamiliesHandler;

#[async_trait]
impl ToolHandler for CitationChainFamiliesHandler {
    fn name(&self) -> &str { "citation_chain_families" }
    fn description(&self) -> &str { "Find citation families in a chain" }
    fn input_schema(&self) -> ToolInputSchema {
        ToolInputSchema::object(
            vec![("arxiv_id".into(), ToolProperty::string("Seed arXiv ID"))].into_iter().collect(),
            vec!["arxiv_id".into()],
        )
    }
    async fn call(&self, params: Value) -> Result<Value, String> {
        let _arxiv_id = params["arxiv_id"].as_str().ok_or("Missing arxiv_id")?;
        let nodes = params.get("nodes").and_then(|v| v.as_array())
            .map(|a| -> Vec<rairos_llm::citation_chain::CitationNode> {
                a.iter().filter_map(|n| {
                    Some(rairos_llm::citation_chain::CitationNode {
                        paper_id: n["paper_id"].as_str()?.to_string(),
                        title: n["title"].as_str()?.to_string(),
                        year: Some(n["year"].as_i64().unwrap_or(2024) as i32),
                        citations: n.get("citations").and_then(|c| c.as_array())
                            .map(|c| c.iter().filter_map(|x| x.as_str().map(String::from)).collect())
                            .unwrap_or_default(),
                        references: n.get("references").and_then(|r| r.as_array())
                            .map(|r| r.iter().filter_map(|x| x.as_str().map(String::from)).collect())
                            .unwrap_or_default(),
                    })
                }).collect()
            }).unwrap_or_default();
        let edges: Vec<(String, String, String)> = params.get("edges").and_then(|v| v.as_array())
            .map(|a| a.iter().filter_map(|e| {
                let arr = e.as_array()?;
                Some((arr[0].as_str()?.to_string(), arr[1].as_str()?.to_string(), arr.get(2).and_then(|x| x.as_str()).unwrap_or("cites").to_string()))
            }).collect())
            .unwrap_or_default();

        let chain = rairos_llm::citation_chain::CitationChain {
            root_id: _arxiv_id.to_string(), nodes, edges,
        };
        let families = rairos_llm::citation_chain::find_families(&chain);
        Ok(serde_json::json!({"families": families, "total": families.len()}))
    }
}

pub struct CitationChainSilentHandler;

#[async_trait]
impl ToolHandler for CitationChainSilentHandler {
    fn name(&self) -> &str { "citation_chain_silent" }
    fn description(&self) -> &str { "Detect silent citations" }
    fn input_schema(&self) -> ToolInputSchema {
        ToolInputSchema::object(
            vec![("arxiv_id".into(), ToolProperty::string("Seed arXiv ID"))].into_iter().collect(),
            vec!["arxiv_id".into()],
        )
    }
    async fn call(&self, params: Value) -> Result<Value, String> {
        let _arxiv_id = params["arxiv_id"].as_str().ok_or("Missing arxiv_id")?;
        let nodes = params.get("nodes").map(|_| vec![]).unwrap_or_default();
        let chain = rairos_llm::citation_chain::CitationChain {
            root_id: _arxiv_id.to_string(), nodes, edges: vec![],
        };
        let silent = rairos_llm::citation_chain::find_silent(&chain);
        Ok(serde_json::json!({"silent_citations": silent, "total": silent.len()}))
    }
}

pub struct CitationChainRenderHandler;

#[async_trait]
impl ToolHandler for CitationChainRenderHandler {
    fn name(&self) -> &str { "citation_chain_render" }
    fn description(&self) -> &str { "Render a citation chain as text" }
    fn input_schema(&self) -> ToolInputSchema {
        ToolInputSchema::object(
            vec![
                ("arxiv_id".into(), ToolProperty::string("Seed arXiv ID")),
                ("format".into(), ToolProperty::string("Output format (text)")),
            ].into_iter().collect(),
            vec!["arxiv_id".into()],
        )
    }
    async fn call(&self, params: Value) -> Result<Value, String> {
        let _arxiv_id = params["arxiv_id"].as_str().ok_or("Missing arxiv_id")?;
        let nodes = params.get("nodes").map(|_| vec![]).unwrap_or_default();
        let chain = rairos_llm::citation_chain::CitationChain {
            root_id: _arxiv_id.to_string(), nodes, edges: vec![],
        };
        let text = rairos_llm::citation_chain::render_text(&chain, 50);
        Ok(serde_json::json!({"text": text}))
    }
}
