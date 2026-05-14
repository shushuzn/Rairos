//! LLM-backed tool handlers — MCP tools whose logic lives in rairos-llm.
//!
//! Each handler shares a tool name with the Python MCP tool it replaces.

use crate::protocol::{ToolHandler, ToolInputSchema, ToolProperty};
use async_trait::async_trait;
use rairos_llm::{impact, replication, LlmClient, OpenAiClient, AnthropicClient};
use serde_json::Value;

// ─── LLM client factory ────────────────────────────────────────────────────

fn llm_client() -> Option<Box<dyn LlmClient>> {
    if let Ok(key) = std::env::var("OPENAI_API_KEY") {
        Some(Box::new(OpenAiClient::new(key)))
    } else if let Ok(key) = std::env::var("ANTHROPIC_API_KEY") {
        Some(Box::new(AnthropicClient::new(key)))
    } else {
        None
    }
}

fn llm_model() -> &'static str {
    if std::env::var("ANTHROPIC_API_KEY").is_ok() { "claude-sonnet-4-20250514" } else { "gpt-4o" }
}

// ─── Briefing Generate ──────────────────────────────────────────────────────

pub struct BriefingGenerateHandler;

#[async_trait]
impl ToolHandler for BriefingGenerateHandler {
    fn name(&self) -> &str { "briefing_generate" }
    fn description(&self) -> &str { "Generate a research briefing for a paper" }
    fn input_schema(&self) -> ToolInputSchema {
        ToolInputSchema::object(
            vec![("arxiv_id".into(), ToolProperty::string("arXiv ID of the paper"))].into_iter().collect(),
            vec!["arxiv_id".into()],
        )
    }
    async fn call(&self, params: Value) -> Result<Value, String> {
        let arxiv_id = params["arxiv_id"].as_str().ok_or("Missing arxiv_id")?;
        let client = llm_client().ok_or("No LLM client available".to_string())?;
        let title = params.get("title").and_then(|v| v.as_str()).unwrap_or(arxiv_id);
        let abstract_text = params.get("abstract").and_then(|v| v.as_str()).unwrap_or("");
        let authors: Vec<String> = params.get("authors")
            .and_then(|v| v.as_array())
            .map(|a| a.iter().filter_map(|s| s.as_str().map(String::from)).collect())
            .unwrap_or_default();

        let result = rairos_llm::briefing::generate_briefing(
            client.as_ref(), llm_model(), arxiv_id, title, abstract_text, &authors,
        ).await;

        if !result.success { return Err("Briefing generation failed".into()); }
        Ok(serde_json::json!({
            "arxiv_id": result.arxiv_id, "summary": result.summary,
            "key_contributions": result.key_contributions, "methodology": result.methodology,
            "results": result.results, "relevance": result.relevance, "verdict": result.verdict,
            "markdown": result.markdown,
        }))
    }
}

// ─── LitReview Generate ─────────────────────────────────────────────────────

pub struct LitReviewGenerateHandler;

#[async_trait]
impl ToolHandler for LitReviewGenerateHandler {
    fn name(&self) -> &str { "litreview_generate" }
    fn description(&self) -> &str { "Generate a structured literature review for a topic" }
    fn input_schema(&self) -> ToolInputSchema {
        ToolInputSchema::object(
            vec![
                ("topic".into(), ToolProperty::string("Research topic")),
                ("max_papers".into(), ToolProperty::integer("Max papers (default 10)")),
            ].into_iter().collect(),
            vec!["topic".into()],
        )
    }
    async fn call(&self, params: Value) -> Result<Value, String> {
        let topic = params["topic"].as_str().ok_or("Missing topic")?;
        let client = llm_client().ok_or("No LLM client available".to_string())?;
        let papers: Vec<rairos_llm::lit_review::LitReviewPaper> = params.get("papers")
            .and_then(|v| v.as_array())
            .map(|a| a.iter().map(|p| rairos_llm::lit_review::LitReviewPaper {
                title: p["title"].as_str().unwrap_or("").into(),
                authors: p["authors"].as_array().map(|a| a.iter().filter_map(|s| s.as_str().map(String::from)).collect()).unwrap_or_default(),
                abstract_text: p["abstract"].as_str().unwrap_or("").into(),
                year: p["year"].as_i64().unwrap_or(2024) as i32,
                arxiv_id: p["arxiv_id"].as_str().map(String::from),
            }).collect())
            .unwrap_or_default();

        let result = rairos_llm::lit_review::generate_lit_review(client.as_ref(), llm_model(), topic, &papers).await;
        Ok(serde_json::json!(result))
    }
}

// ─── Slides Generate ────────────────────────────────────────────────────────

pub struct SlidesGenerateHandler;

#[async_trait]
impl ToolHandler for SlidesGenerateHandler {
    fn name(&self) -> &str { "slides_generate" }
    fn description(&self) -> &str { "Generate a slide deck from a paper's content" }
    fn input_schema(&self) -> ToolInputSchema {
        ToolInputSchema::object(
            vec![
                ("arxiv_id".into(), ToolProperty::string("arXiv ID")),
                ("briefing_markdown".into(), ToolProperty::string("Briefing markdown (optional)")),
            ].into_iter().collect(),
            vec!["arxiv_id".into()],
        )
    }
    async fn call(&self, params: Value) -> Result<Value, String> {
        let arxiv_id = params["arxiv_id"].as_str().ok_or("Missing arxiv_id")?;
        let briefing = params.get("briefing_markdown").and_then(|v| v.as_str()).unwrap_or("");
        let client = llm_client().ok_or("No LLM client available".to_string())?;
        let result = rairos_llm::slides::generate_slides(client.as_ref(), llm_model(), arxiv_id, briefing).await;
        Ok(serde_json::json!(result))
    }
}

// ─── Gap Detect ─────────────────────────────────────────────────────────────

pub struct GapDetectHandler;

#[async_trait]
impl ToolHandler for GapDetectHandler {
    fn name(&self) -> &str { "gap_detect" }
    fn description(&self) -> &str { "Detect research gaps from the paper corpus for a topic" }
    fn input_schema(&self) -> ToolInputSchema {
        ToolInputSchema::object(
            vec![("topic".into(), ToolProperty::string("Research topic to analyze"))].into_iter().collect(),
            vec!["topic".into()],
        )
    }
    async fn call(&self, params: Value) -> Result<Value, String> {
        let topic = params["topic"].as_str().ok_or("Missing topic")?;
        let gaps = rairos_llm::gap_detector::detect_gaps_keyword(topic);
        Ok(serde_json::json!({
            "topic": topic,
            "gaps": gaps.iter().map(|g| serde_json::json!({
                "type": g.gap_type.as_str(), "description": g.description,
                "evidence_papers": g.evidence_papers, "confidence": g.confidence,
            })).collect::<Vec<_>>(),
            "total": gaps.len(),
        }))
    }
}

// ─── Citation Chain: Build ──────────────────────────────────────────────────

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

// ─── Citation Chain: Families ───────────────────────────────────────────────

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

// ─── Citation Chain: Silent ─────────────────────────────────────────────────

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

// ─── Citation Chain: Render ─────────────────────────────────────────────────

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

// ─── Impact Score Paper ─────────────────────────────────────────────────────

pub struct ImpactScorePaperHandler;

#[async_trait]
impl ToolHandler for ImpactScorePaperHandler {
    fn name(&self) -> &str { "impact_score_paper" }
    fn description(&self) -> &str { "Score a paper's impact" }
    fn input_schema(&self) -> ToolInputSchema {
        ToolInputSchema::object(
            vec![("arxiv_id".into(), ToolProperty::string("arXiv ID"))].into_iter().collect(),
            vec!["arxiv_id".into()],
        )
    }
    async fn call(&self, params: Value) -> Result<Value, String> {
        let arxiv_id = params["arxiv_id"].as_str().ok_or("Missing arxiv_id")?;
        let title = params.get("title").and_then(|v| v.as_str()).unwrap_or(arxiv_id);
        let citations = params.get("citation_count").and_then(|v| v.as_u64()).unwrap_or(0) as u32;
        let year = params.get("year").and_then(|v| v.as_i64()).unwrap_or(2024) as i32;
        let score = impact::score_paper(arxiv_id, title, citations, year, 2026);
        Ok(serde_json::json!(score))
    }
}

// ─── Impact Rank ────────────────────────────────────────────────────────────

pub struct ImpactRankHandler;

#[async_trait]
impl ToolHandler for ImpactRankHandler {
    fn name(&self) -> &str { "impact_rank" }
    fn description(&self) -> &str { "Rank papers by impact score" }
    fn input_schema(&self) -> ToolInputSchema {
        ToolInputSchema::object(
            vec![
                ("topic".into(), ToolProperty::string("Research topic")),
                ("top_k".into(), ToolProperty::integer("Number of results (default 10)")),
            ].into_iter().collect(),
            vec!["topic".into()],
        )
    }
    async fn call(&self, params: Value) -> Result<Value, String> {
        let top_k = params.get("top_k").and_then(|v| v.as_u64()).unwrap_or(10) as usize;
        let papers: Vec<(String, String, u32, i32)> = params.get("papers")
            .and_then(|v| v.as_array())
            .map(|a| a.iter().filter_map(|p| {
                Some((
                    p["arxiv_id"].as_str()?.to_string(),
                    p["title"].as_str()?.to_string(),
                    p.get("citation_count").and_then(|c| c.as_u64()).unwrap_or(0) as u32,
                    p.get("year").and_then(|y| y.as_i64()).unwrap_or(2024) as i32,
                ))
            }).collect())
            .unwrap_or_default();
        let ranked = impact::rank_papers(&papers, 2026, top_k);
        Ok(serde_json::json!({"ranked": ranked, "total": ranked.len()}))
    }
}

// ─── Replication Check ──────────────────────────────────────────────────────

pub struct ReplicationCheckHandler;

#[async_trait]
impl ToolHandler for ReplicationCheckHandler {
    fn name(&self) -> &str { "replication_check" }
    fn description(&self) -> &str { "Check a paper's reproducibility" }
    fn input_schema(&self) -> ToolInputSchema {
        ToolInputSchema::object(
            vec![
                ("arxiv_id".into(), ToolProperty::string("arXiv ID")),
                ("include_abstract".into(), ToolProperty::string("Abstract text (optional)")),
            ].into_iter().collect(),
            vec!["arxiv_id".into()],
        )
    }
    async fn call(&self, params: Value) -> Result<Value, String> {
        let arxiv_id = params["arxiv_id"].as_str().ok_or("Missing arxiv_id")?;
        let abstract_text = params.get("include_abstract").and_then(|v| v.as_str()).unwrap_or("");
        let title = params.get("title").and_then(|v| v.as_str()).unwrap_or(arxiv_id);

        let result = if let Some(client) = llm_client() {
            replication::llm_assess_replication(client.as_ref(), llm_model(), title, abstract_text).await
        } else {
            replication::keyword_check(abstract_text)
        };
        Ok(serde_json::json!({
            "arxiv_id": arxiv_id, "score": result.score,
            "has_code": result.has_code, "has_data": result.has_data,
            "has_method": result.has_method, "has_env": result.has_env,
            "reasoning": result.reasoning,
        }))
    }
}

// ─── Register ───────────────────────────────────────────────────────────────

pub async fn register_llm_handlers(server: &crate::McpServer) {
    tracing::debug!("registering 11 llm-backed MCP tool handlers");
    server.register(BriefingGenerateHandler).await;
    server.register(LitReviewGenerateHandler).await;
    server.register(SlidesGenerateHandler).await;
    server.register(GapDetectHandler).await;
    server.register(CitationChainBuildHandler).await;
    server.register(CitationChainFamiliesHandler).await;
    server.register(CitationChainSilentHandler).await;
    server.register(CitationChainRenderHandler).await;
    server.register(ImpactScorePaperHandler).await;
    server.register(ImpactRankHandler).await;
    server.register(ReplicationCheckHandler).await;
}
