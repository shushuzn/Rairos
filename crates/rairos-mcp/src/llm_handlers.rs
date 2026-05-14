//! LLM-backed tool handlers — MCP tools whose logic lives in rairos-llm.
//!
//! Each handler shares a tool name with the Python MCP tool it replaces.

use crate::protocol::{ToolHandler, ToolInputSchema, ToolProperty};
use async_trait::async_trait;
use rairos_insight_evolution::EvolutionEngine;
use rairos_insight_storage::CapsuleStorage;
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
            vec![("paper_id".into(), ToolProperty::string("Paper ID"))].into_iter().collect(),
            vec!["paper_id".into()],
        )
    }
    async fn call(&self, params: Value) -> Result<Value, String> {
        let paper_id = params.get("paper_id").and_then(|v| v.as_str())
            .or_else(|| params.get("arxiv_id").and_then(|v| v.as_str()))
            .ok_or("Missing paper_id or arxiv_id")?;
        let title = params.get("title").and_then(|v| v.as_str()).unwrap_or(paper_id);
        let citations = params.get("citation_count").and_then(|v| v.as_u64()).unwrap_or(0) as u32;
        let year = params.get("year").and_then(|v| v.as_i64()).unwrap_or(2024) as i32;
        let score = impact::score_paper(paper_id, title, citations, year, 2026);
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
        let arxiv_id = params.get("arxiv_id").and_then(|v| v.as_str())
            .or_else(|| params.get("paper_id").and_then(|v| v.as_str()))
            .ok_or("Missing arxiv_id or paper_id")?;
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

// ─── Paper Compare ─────────────────────────────────────────────────────────

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

// ─── Paper Analyze (MCP wrapper) ───────────────────────────────────────────

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
                client.as_ref(), llm_model(), title, abstract_text, authors, body,
            ).await;
            Ok(serde_json::json!(result))
        } else {
            // Keyword-based analysis fallback
            Ok(serde_json::json!({
                "paper_id": paper_id,
                "title": title,
                "note": "No LLM available — set OPENAI_API_KEY or ANTHROPIC_API_KEY for full analysis",
                "keywords_found": rairos_llm::paper_comparison::extract_methods(title, abstract_text, ""),
            }))
        }
    }
}

// ─── Gene Pool data directory helper ────────────────────────────────────────

fn gene_pool_data_dir() -> std::path::PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join(".ai_research_os")
        .join("evolution")
}

// ─── Gap Submit ─────────────────────────────────────────────────────────────

pub struct GapSubmitHandler;

#[async_trait]
impl ToolHandler for GapSubmitHandler {
    fn name(&self) -> &str { "gap_submit" }
    fn description(&self) -> &str { "Submit a new research gap directly to the Gene Pool as a CapsuleGene" }
    fn input_schema(&self) -> ToolInputSchema {
        ToolInputSchema::object(
            vec![
                ("topic".into(), ToolProperty::string("Research topic")),
                ("gap_type".into(), ToolProperty::string("Type of research gap")),
                ("title".into(), ToolProperty::string("Gap title")),
                ("description".into(), ToolProperty::string("Gap description (optional)")),
                ("success_score".into(), ToolProperty::string("Success score 0.0-1.0 (default 0.8)")),
            ].into_iter().collect(),
            vec!["topic".into(), "gap_type".into(), "title".into()],
        )
    }
    async fn call(&self, params: Value) -> Result<Value, String> {
        let topic = params["topic"].as_str().ok_or("Missing required parameter: topic")?;
        let gap_type = params["gap_type"].as_str().ok_or("Missing required parameter: gap_type")?;
        let title = params["title"].as_str().ok_or("Missing required parameter: title")?;
        let description = params.get("description").and_then(|v| v.as_str()).unwrap_or("");
        let success_score = params.get("success_score").and_then(|v| v.as_f64()).unwrap_or(0.8);

        let data_dir = gene_pool_data_dir();
        let storage = CapsuleStorage::new(&data_dir)
            .map_err(|e| format!("Failed to open gene pool storage: {}", e))?;

        let capsule = storage.encode_capsule(
            topic, gap_type, title, description, success_score,
            "active", "", "", None, None, &data_dir,
        ).map_err(|e| format!("encode_capsule failed: {}", e))?;

        Ok(serde_json::json!({
            "capsule_id": capsule.capsule_id,
            "topic": topic,
            "gap_type": gap_type,
            "title": title,
            "status": capsule.status,
            "message": format!("Gap '{}' submitted to Gene Pool successfully", title),
        }))
    }
}

// ─── Gap Evolve ─────────────────────────────────────────────────────────────

pub struct GapEvolveHandler;

#[async_trait]
impl ToolHandler for GapEvolveHandler {
    fn name(&self) -> &str { "gap_evolve" }
    fn description(&self) -> &str { "Run Gene Pool evolution cycle for a topic — audit, propose, evaluate, apply" }
    fn input_schema(&self) -> ToolInputSchema {
        ToolInputSchema::object(
            vec![
                ("topic".into(), ToolProperty::string("Research topic to evolve for")),
                ("gap_type".into(), ToolProperty::string("Optional gap type filter")),
            ].into_iter().collect(),
            vec!["topic".into()],
        )
    }
    async fn call(&self, params: Value) -> Result<Value, String> {
        let topic = params["topic"].as_str().ok_or("Missing required parameter: topic")?;
        let gap_type = params.get("gap_type").and_then(|v| v.as_str());

        let data_dir = gene_pool_data_dir();
        let storage = CapsuleStorage::new(&data_dir)
            .map_err(|e| format!("Failed to open gene pool storage: {}", e))?;

        let capsules = storage.load_all_capsules()
            .map_err(|e| format!("Failed to load capsules: {}", e))?;

        if capsules.is_empty() {
            return Ok(serde_json::json!({
                "topic": topic,
                "gap_type": gap_type,
                "audit": { "total": 0, "avg_quality": 0.0, "candidates": 0, "to_retire": 0 },
                "proposed": 0,
                "evaluated": 0,
                "result": { "added": 0, "retired": 0, "total_capsules": 0, "avg_quality": 0.0 },
                "note": "Gene pool is empty — submit gaps first with gap_submit",
            }));
        }

        let mut engine = EvolutionEngine::new(capsules);
        let result = engine.evolve(topic, gap_type);

        // Persist the evolved gene pool back to storage
        let evolved = engine.get_capsules().to_vec();
        storage.save_capsules(&evolved)
            .map_err(|e| format!("Failed to persist evolved gene pool: {}", e))?;

        let audit = result.get("audit").and_then(|v| v.as_object()).cloned().unwrap_or_default();
        let result_data = result.get("result").and_then(|v| v.as_object()).cloned().unwrap_or_default();

        Ok(serde_json::json!({
            "topic": topic,
            "gap_type": gap_type,
            "audit": audit,
            "proposed": result.get("proposed").and_then(|v| v.as_u64()).unwrap_or(0),
            "evaluations": result.get("evaluations").and_then(|v| v.as_u64()).unwrap_or(0),
            "result": result_data,
        }))
    }
}

// ─── Register ───────────────────────────────────────────────────────────────

pub async fn register_llm_handlers(server: &crate::McpServer) {
    tracing::debug!("registering 13 llm-backed MCP tool handlers");
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
    server.register(RouteQueryHandler).await;
    server.register(TrustScorerComputeHandler).await;
    server.register(PaperCompareHandler).await;
    server.register(PaperAnalyzeMcpHandler).await;
    server.register(GapSubmitHandler).await;
    server.register(GapEvolveHandler).await;
}

// ─── Trust Scorer Compute ──────────────────────────────────────────────────

pub struct TrustScorerComputeHandler;

#[async_trait]
impl ToolHandler for TrustScorerComputeHandler {
    fn name(&self) -> &str { "trust_scorer_compute" }
    fn description(&self) -> &str { "Compute per-category trust scores from capsule quality data" }
    fn input_schema(&self) -> ToolInputSchema {
        ToolInputSchema::object(
            vec![("scores".into(), ToolProperty::string("JSON array of {category: string, score: number} objects"))].into_iter().collect(),
            vec!["scores".into()],
        )
    }
    async fn call(&self, params: Value) -> Result<Value, String> {
        // Accept scores as a JSON string or as a direct JSON array
        let entries: Vec<(String, f64)> = if let Some(arr) = params["scores"].as_array() {
            arr.iter().filter_map(|v| {
                let cat = v["category"].as_str()?;
                let score = v["score"].as_f64()?;
                Some((cat.to_string(), score))
            }).collect()
        } else if let Some(s) = params["scores"].as_str() {
            serde_json::from_str(s).map_err(|e| format!("Invalid scores JSON: {}", e))?
        } else {
            return Err("Missing scores: provide JSON array or JSON string".into());
        };
        let refs: Vec<(&str, f64)> = entries.iter().map(|(c, s)| (c.as_str(), *s)).collect();
        let result = rairos_llm::trust_scorer::compute_trust(&refs);
        Ok(serde_json::json!(result))
    }
}

// ─── Route Query (semantic router) ──────────────────────────────────────────

pub struct RouteQueryHandler;

#[async_trait]
impl ToolHandler for RouteQueryHandler {
    fn name(&self) -> &str { "routeplan_create" }
    fn description(&self) -> &str { "Create a research route plan from a hypothesis (LLM-backed, keyword fallback)" }
    fn input_schema(&self) -> ToolInputSchema {
        ToolInputSchema::object(
            vec![
                ("hypothesis".into(), ToolProperty::string("Research hypothesis to investigate")),
                ("goal".into(), ToolProperty::string("What the plan should determine")),
                ("known_papers".into(), ToolProperty::string("JSON array of {arxiv_id, title} (optional)")),
            ].into_iter().collect(),
            vec!["hypothesis".into()],
        )
    }
    async fn call(&self, params: Value) -> Result<Value, String> {
        let hypothesis = params["hypothesis"].as_str().ok_or("Missing hypothesis")?;
        let goal = params.get("goal").and_then(|v| v.as_str()).unwrap_or("Test the hypothesis");

        // Try LLM path if available
        if let Some(client) = llm_client() {
            let known_papers: Vec<String> = params.get("known_papers")
                .and_then(|v| v.as_array())
                .map(|a| a.iter().filter_map(|p| {
                    let title = p.get("title").and_then(|t| t.as_str()).unwrap_or("");
                    let arxiv_id = p.get("arxiv_id").and_then(|i| i.as_str()).unwrap_or("");
                    Some(format!("{} ({})", title, arxiv_id))
                }).collect())
                .unwrap_or_default();

            let plan = rairos_llm::route_planner::create_plan(
                client.as_ref(), llm_model(), hypothesis, goal, &known_papers,
            ).await;

            return Ok(serde_json::json!(plan));
        }

        // No LLM: keyword routing fallback (semantic_router)
        let route = rairos_llm::semantic_router::route_by_keyword(hypothesis);
        Ok(serde_json::json!({"semantic_route": route, "note": "No LLM available — keyword routing only. Set OPENAI_API_KEY or ANTHROPIC_API_KEY for full plan generation."}))
    }
}
