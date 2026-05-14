//! LLM-backed tool handlers — MCP tools whose logic lives in rairos-llm.
//!
//! Each handler shares a tool name with the Python MCP tool it replaces.

use crate::protocol::{ToolHandler, ToolInputSchema, ToolProperty};
use async_trait::async_trait;
use rand::Rng;
use rairos_core::Database;
use rairos_experiment_tracker::ExperimentTracker;
use rairos_gene_pool_watcher::GenePoolWatcher;
use rairos_insight_evolution::EvolutionEngine;
use rairos_insight_storage::CapsuleStorage;
use rairos_llm::{impact, replication, LlmClient, OpenAiClient, AnthropicClient};
use serde_json::Value;
use std::collections::HashMap;

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

// ─── Gene Pool Decay ────────────────────────────────────────────────────────

fn compute_impact_score(
    capsule: &rairos_crossover::CapsuleGene,
    lambda_: f64,
) -> f64 {
    let age_days = chrono::DateTime::parse_from_rfc3339(&capsule.created_at)
        .map(|dt| {
            let now = chrono::Utc::now();
            let dur = now.signed_duration_since(dt.with_timezone(&chrono::Utc));
            dur.num_days() as f64
        })
        .unwrap_or(0.0)
        .max(0.0);

    let recency = (-lambda_ * age_days).exp();
    let quality = capsule.outcome_success_score;
    let feedback_boost = (capsule.feedback_count as f64).ln_1p() * 0.1;
    let credibility = capsule.credibility_score;

    (quality * 0.5 + credibility * 0.3 + feedback_boost * 0.2) * recency
}

pub struct GenePoolDecayHandler;

#[async_trait]
impl ToolHandler for GenePoolDecayHandler {
    fn name(&self) -> &str { "gene_pool_decay" }
    fn description(&self) -> &str { "Time-weighted impact scoring and auto-archive for Gene Pool capsules" }
    fn input_schema(&self) -> ToolInputSchema {
        ToolInputSchema::object(
            vec![
                ("action".into(), ToolProperty::string("Action: status, rank, or archived (default: status)")),
                ("min_impact".into(), ToolProperty::string("Minimum impact threshold (default: 0.1)")),
                ("lambda_".into(), ToolProperty::string("Decay rate lambda (default: 0.01)")),
            ].into_iter().collect(),
            vec![],
        )
    }
    async fn call(&self, params: Value) -> Result<Value, String> {
        let action = params.get("action").and_then(|v| v.as_str()).unwrap_or("status");
        let min_impact = params.get("min_impact").and_then(|v| v.as_f64()).unwrap_or(0.1);
        let lambda_ = params.get("lambda_").and_then(|v| v.as_f64()).unwrap_or(0.01);

        let data_dir = gene_pool_data_dir();
        let storage = CapsuleStorage::new(&data_dir)
            .map_err(|e| format!("Failed to open gene pool storage: {}", e))?;
        let capsules = storage.load_all_capsules()
            .map_err(|e| format!("Failed to load capsules: {}", e))?;

        match action {
            "rank" => {
                let mut scored: Vec<_> = capsules.iter().map(|c| {
                    (c, compute_impact_score(c, lambda_))
                }).collect();
                scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
                let ranked: Vec<Value> = scored.iter().enumerate().map(|(i, (c, s))| {
                    serde_json::json!({
                        "rank": i + 1,
                        "capsule_id": c.capsule_id,
                        "impact_score": (s * 1000.0).round() / 1000.0,
                        "title": c.action_gap_title,
                        "topic": c.trigger_topic,
                        "success_score": c.outcome_success_score,
                    })
                }).collect();
                Ok(serde_json::json!({ "ranked": ranked, "total": scored.len() }))
            }
            "archived" => {
                let archived: Vec<Value> = capsules.iter().filter(|c| c.status == "archived").map(|c| {
                    serde_json::json!({
                        "capsule_id": c.capsule_id,
                        "title": c.action_gap_title,
                        "archived_at": c.created_at,
                    })
                }).collect();
                Ok(serde_json::json!({ "archived": archived, "total": archived.len() }))
            }
            _ => {
                // status action: score all active capsules with time-weighted impact
                let active: Vec<&rairos_crossover::CapsuleGene> = capsules.iter().filter(|c| c.status == "active").collect();
                let mut scored: Vec<_> = active.iter().map(|c| {
                    let impact = compute_impact_score(c, lambda_);
                    (c, impact)
                }).collect();
                scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

                let top: Vec<Value> = scored.iter().take(10).map(|(c, s)| {
                    serde_json::json!({
                        "capsule_id": c.capsule_id,
                        "impact_score": (s * 1000.0).round() / 1000.0,
                        "success_score": c.outcome_success_score,
                        "feedback_count": c.feedback_count,
                        "credibility_score": c.credibility_score,
                        "age_days": chrono::DateTime::parse_from_rfc3339(&c.created_at)
                            .map(|dt| chrono::Utc::now().signed_duration_since(dt.with_timezone(&chrono::Utc)).num_days())
                            .unwrap_or(0),
                    })
                }).collect();
                let bottom: Vec<Value> = scored.iter().rev().take(5).map(|(c, s)| {
                    serde_json::json!({
                        "capsule_id": c.capsule_id,
                        "impact_score": (s * 1000.0).round() / 1000.0,
                        "age_days": chrono::DateTime::parse_from_rfc3339(&c.created_at)
                            .map(|dt| chrono::Utc::now().signed_duration_since(dt.with_timezone(&chrono::Utc)).num_days())
                            .unwrap_or(0),
                    })
                }).collect();

                let total_scored = scored.len();
                let below_threshold = scored.iter().filter(|(_, s)| *s < min_impact).count();

                Ok(serde_json::json!({
                    "total_scored": total_scored,
                    "below_threshold": below_threshold,
                    "min_impact": min_impact,
                    "top_capsules": top,
                    "bottom_capsules": bottom,
                }))
            }
        }
    }
}

// ─── Crossover ──────────────────────────────────────────────────────────────

pub struct CrossoverHandler;

#[async_trait]
impl ToolHandler for CrossoverHandler {
    fn name(&self) -> &str { "crossover" }
    fn description(&self) -> &str { "Run CapsuleGene genetic algorithm: select parents, crossover, mutate, encode V3" }
    fn input_schema(&self) -> ToolInputSchema {
        ToolInputSchema::object(
            vec![
                ("action".into(), ToolProperty::string("Action: evolve, rank_v3, mutate, or best (default: evolve)")),
                ("offspring_count".into(), ToolProperty::integer("Number of offspring to produce (default: 5)")),
                ("capsule_id".into(), ToolProperty::string("Capsule ID for mutate/lineage actions (optional)")),
            ].into_iter().collect(),
            vec![],
        )
    }
    async fn call(&self, params: Value) -> Result<Value, String> {
        let action = params.get("action").and_then(|v| v.as_str()).unwrap_or("evolve");
        let offspring_count = params.get("offspring_count").and_then(|v| v.as_u64()).unwrap_or(5) as usize;
        let capsule_id = params.get("capsule_id").and_then(|v| v.as_str());

        let data_dir = gene_pool_data_dir();
        let storage = CapsuleStorage::new(&data_dir)
            .map_err(|e| format!("Failed to open gene pool storage: {}", e))?;

        match action {
            "rank_v3" => {
                let capsules = storage.load_all_capsules()
                    .map_err(|e| format!("Failed to load capsules: {}", e))?;
                let v3: Vec<serde_json::Value> = capsules.iter()
                    .filter(|c| c.evolved_generation >= 1 && c.status == "active")
                    .map(|c| {
                        let fitness = rairos_crossover::compute_fitness(c);
                        serde_json::json!({
                            "capsule_id": c.capsule_id,
                            "title": c.action_gap_title,
                            "evolved_generation": c.evolved_generation,
                            "fitness": (fitness * 1000.0).round() / 1000.0,
                            "success_score": c.outcome_success_score,
                        })
                    })
                    .collect();
                Ok(serde_json::json!({ "v3_capsules": v3, "total_v3": v3.len() }))
            }
            "mutate" => {
                let cid = capsule_id.ok_or("capsule_id required for mutate action")?;
                let all = storage.load_all_capsules()
                    .map_err(|e| format!("Failed to load capsules: {}", e))?;
                let pos = all.iter().position(|c| c.capsule_id == cid)
                    .ok_or_else(|| format!("Capsule {} not found", cid))?;
                let mut capsule = all[pos].clone();
                let mutated_arch = rairos_crossover::mutate_archetype(capsule.archetype.clone());
                capsule.archetype = mutated_arch;
                storage.save_capsules(&[capsule.clone()])
                    .map_err(|e| format!("Failed to save mutated capsule: {}", e))?;
                Ok(serde_json::json!({
                    "mutated": {
                        "capsule_id": capsule.capsule_id,
                        "title": capsule.action_gap_title,
                        "status": "mutated"
                    }
                }))
            }
            "best" => {
                let capsules = storage.load_all_capsules()
                    .map_err(|e| format!("Failed to load capsules: {}", e))?;
                let mut active: Vec<_> = capsules.iter()
                    .filter(|c| c.status == "active" && c.credibility_badge != "low")
                    .collect();
                active.sort_by(|a, b| {
                    rairos_crossover::compute_fitness(b)
                        .partial_cmp(&rairos_crossover::compute_fitness(a))
                        .unwrap_or(std::cmp::Ordering::Equal)
                });
                let candidates: Vec<Value> = active.iter().take(offspring_count).map(|c| {
                    serde_json::json!({
                        "capsule_id": c.capsule_id,
                        "title": c.action_gap_title,
                        "fitness": (rairos_crossover::compute_fitness(c) * 1000.0).round() / 1000.0,
                        "success_score": c.outcome_success_score,
                        "credibility_badge": c.credibility_badge,
                    })
                }).collect();
                Ok(serde_json::json!({ "candidates": candidates, "total": candidates.len() }))
            }
            _ => {
                // evolve action: select top parents, crossover, mutate, save offspring
                let all = storage.load_all_capsules()
                    .map_err(|e| format!("Failed to load capsules: {}", e))?;
                let active: Vec<rairos_crossover::CapsuleGene> = all.into_iter()
                    .filter(|c| c.status == "active")
                    .collect();

                if active.len() < 2 {
                    return Ok(serde_json::json!({
                        "error": "Need at least 2 active capsules for crossover",
                        "active_count": active.len(),
                    }));
                }

                let mut rng = rand::thread_rng();
                let count = offspring_count.min(active.len() / 2);
                let mut offspring = Vec::new();
                let mut parents_used = Vec::new();

                for _ in 0..count {
                    let idx_a = rng.gen_range(0..active.len());
                    let idx_b = rng.gen_range(0..active.len());
                    if idx_a == idx_b { continue; }

                    let parent_a = &active[idx_a];
                    let parent_b = &active[idx_b];

                    let cross_result = rairos_crossover::crossover(parent_a, parent_b);
                    let mutated_arch = rairos_crossover::mutate_archetype(cross_result.archetype);

                    let child = rairos_crossover::CapsuleGene {
                        capsule_id: uuid::Uuid::new_v4().to_string()[..12].to_string(),
                        created_at: chrono::Utc::now().to_rfc3339(),
                        trigger_topic: format!("{} & {}", parent_a.trigger_topic, parent_b.trigger_topic),
                        trigger_gap_type: parent_a.trigger_gap_type.clone(),
                        trigger_keywords: {
                            let mut kws = parent_a.trigger_keywords.clone();
                            kws.extend(parent_b.trigger_keywords.clone());
                            kws.sort();
                            kws.dedup();
                            kws.truncate(15);
                            kws
                        },
                        action_gap_type: parent_a.action_gap_type.clone(),
                        action_gap_title: format!("Crossover: {} x {}", parent_a.action_gap_title, parent_b.action_gap_title),
                        outcome_success_score: cross_result.parent_fitness_a.max(cross_result.parent_fitness_b).min(1.0),
                        feedback_count: 0,
                        evolved_generation: cross_result.parent_generations,
                        archetype: mutated_arch,
                        status: "active".to_string(),
                        low_score_streak: 0,
                        credibility_score: 0.5,
                        trendslop: false,
                        trendslop_reason: String::new(),
                        credibility_badge: "medium".to_string(),
                        source_arxiv_category: parent_a.source_arxiv_category.clone(),
                    };

                    parents_used.push((parent_a.capsule_id.clone(), parent_b.capsule_id.clone()));
                    offspring.push(child);
                }

                // Save offspring to storage
                storage.save_capsules(&offspring)
                    .map_err(|e| format!("Failed to save offspring: {}", e))?;

                let offspring_json: Vec<Value> = offspring.iter().map(|c| {
                    serde_json::json!({
                        "capsule_id": c.capsule_id,
                        "title": c.action_gap_title,
                        "evolved_generation": c.evolved_generation,
                    })
                }).collect();

                Ok(serde_json::json!({
                    "offspring": offspring_json,
                    "total_new": offspring.len(),
                    "parents_used": parents_used.iter().map(|(a, b)| serde_json::json!({"parent_a": a, "parent_b": b})).collect::<Vec<Value>>(),
                }))
            }
        }
    }
}

// ─── Research Memory (4-in-1 handler) ─────────────────────────────────────

fn parse_stance_type(s: &str) -> Result<rairos_research_memory::StanceType, String> {
    match s.to_lowercase().as_str() {
        "supported" => Ok(rairos_research_memory::StanceType::Supported),
        "rejected" => Ok(rairos_research_memory::StanceType::Rejected),
        "deferred" => Ok(rairos_research_memory::StanceType::Deferred),
        "qualified" => Ok(rairos_research_memory::StanceType::Qualified),
        _ => Err(format!("Invalid stance type: '{}' — expected supported/rejected/deferred/qualified", s)),
    }
}

fn research_memory_add_stance_impl(memory: &mut rairos_research_memory::ResearchMemory, params: &Value) -> Result<Value, String> {
    let topic = params.get("topic").and_then(|v| v.as_str()).ok_or("Missing topic")?;
    let claim = params.get("claim").and_then(|v| v.as_str()).ok_or("Missing claim")?;
    let stance_str = params.get("stance").and_then(|v| v.as_str()).ok_or("Missing stance")?;
    let stance = parse_stance_type(stance_str)?;
    let evidence_refs: Vec<String> = params.get("evidence_refs")
        .and_then(|v| serde_json::from_value(v.clone()).ok())
        .unwrap_or_default();
    let reasoning = params.get("reasoning").and_then(|v| v.as_str()).unwrap_or("");
    let confidence = params.get("confidence").and_then(|v| v.as_f64()).unwrap_or(0.5);
    let tags: Vec<String> = params.get("tags")
        .and_then(|v| serde_json::from_value(v.clone()).ok())
        .unwrap_or_default();
    let notes = params.get("notes").and_then(|v| v.as_str()).unwrap_or("");

    let s = memory.add_stance(topic, claim, stance, evidence_refs, reasoning, confidence, tags, notes);
    Ok(serde_json::json!({
        "stance_id": s.stance_id,
        "topic": s.topic,
        "claim": s.claim,
        "stance": s.stance.to_string(),
        "confidence": s.confidence,
        "message": "Stance recorded",
    }))
}

fn research_memory_list_stances_impl(memory: &rairos_research_memory::ResearchMemory, params: &Value) -> Result<Value, String> {
    let topic = params.get("topic").and_then(|v| v.as_str());
    let stances = memory.get_stances(topic, None);
    Ok(serde_json::json!({ "stances": stances, "total": stances.len() }))
}

fn research_memory_check_paper_impl(memory: &mut rairos_research_memory::ResearchMemory, params: &Value) -> Result<Value, String> {
    let arxiv_id = params.get("arxiv_id").and_then(|v| v.as_str()).ok_or("Missing arxiv_id")?;
    let title = params.get("title").and_then(|v| v.as_str()).unwrap_or("");
    let claim = params.get("claim").and_then(|v| v.as_str()).unwrap_or("");
    let mut paper = std::collections::HashMap::new();
    paper.insert("arxiv_id".to_string(), arxiv_id.to_string());
    paper.insert("title".to_string(), title.to_string());
    paper.insert("claim".to_string(), claim.to_string());
    let anomalies = memory.check_paper_against_stances(&paper, false, None, None, None);
    Ok(serde_json::json!({ "anomalies": anomalies, "total": anomalies.len() }))
}

fn research_memory_anomalies_impl(memory: &rairos_research_memory::ResearchMemory, params: &Value) -> Result<Value, String> {
    let limit = params.get("limit").and_then(|v| v.as_u64()).unwrap_or(20) as usize;
    let anomalies = memory.get_recent_anomalies(limit);
    Ok(serde_json::json!({ "anomalies": anomalies, "total": anomalies.len() }))
}

macro_rules! make_research_memory_handler {
    ($name:ident, $tool_name:expr, $desc:expr, $impl_fn:ident) => {
        pub struct $name;
        #[async_trait]
        impl ToolHandler for $name {
            fn name(&self) -> &str { $tool_name }
            fn description(&self) -> &str { $desc }
            fn input_schema(&self) -> ToolInputSchema {
                ToolInputSchema::object(
                    vec![
                        ("topic".into(), ToolProperty::string("Research topic")),
                        ("claim".into(), ToolProperty::string("The claim being evaluated")),
                        ("stance".into(), ToolProperty::string("Stance: supported/rejected/deferred/qualified")),
                        ("evidence_refs".into(), ToolProperty::string("JSON array of evidence references")),
                        ("reasoning".into(), ToolProperty::string("Reasoning behind the stance")),
                        ("confidence".into(), ToolProperty::string("Confidence score 0.0-1.0")),
                        ("arxiv_id".into(), ToolProperty::string("arXiv ID for check_paper")),
                        ("limit".into(), ToolProperty::integer("Max results (default 20)")),
                    ].into_iter().collect(),
                    vec![],
                )
            }
            async fn call(&self, params: Value) -> Result<Value, String> {
                let mut memory = rairos_research_memory::ResearchMemory::new();
                $impl_fn(&mut memory, &params)
            }
        }
    };
}

make_research_memory_handler!(ResearchMemoryAddStanceHandler, "research_memory_add_stance", "Record a new research stance", research_memory_add_stance_impl);
make_research_memory_handler!(ResearchMemoryListStancesHandler, "research_memory_list_stances", "List all research stances", research_memory_list_stances_impl);
make_research_memory_handler!(ResearchMemoryCheckPaperHandler, "research_memory_check_paper", "Check a paper against prior research stances", research_memory_check_paper_impl);
make_research_memory_handler!(ResearchMemoryAnomaliesHandler, "research_memory_anomalies", "List recent research memory anomalies", research_memory_anomalies_impl);

// ─── Leaderboard ───────────────────────────────────────────────────────────

pub struct LeaderboardHandler;

#[async_trait]
impl ToolHandler for LeaderboardHandler {
    fn name(&self) -> &str { "leaderboard" }
    fn description(&self) -> &str { "Benchmark Leaderboard: ranked paper2code implementations by pass_rate + coverage" }
    fn input_schema(&self) -> ToolInputSchema {
        ToolInputSchema::object(
            vec![
                ("action".into(), ToolProperty::string("Action: status, rankings, entry (default: status)")),
                ("arxiv_id".into(), ToolProperty::string("arXiv ID for entry action")),
                ("sort_by".into(), ToolProperty::string("Sort: combined, pass_rate, coverage (default: combined)")),
                ("limit".into(), ToolProperty::integer("Max results (default: 20)")),
            ].into_iter().collect(),
            vec![],
        )
    }
    async fn call(&self, params: Value) -> Result<Value, String> {
        let action = params.get("action").and_then(|v| v.as_str()).unwrap_or("status");
        let arxiv_id = params.get("arxiv_id").and_then(|v| v.as_str());
        let sort_by = params.get("sort_by").and_then(|v| v.as_str()).unwrap_or("combined");
        let limit = params.get("limit").and_then(|v| v.as_u64()).unwrap_or(20) as usize;
        Ok(rairos_leaderboard::leaderboard_action(action, arxiv_id, sort_by, limit))
    }
}

// ─── Impact Leaderboard ────────────────────────────────────────────────────

pub struct ImpactLeaderboardHandler;

#[async_trait]
impl ToolHandler for ImpactLeaderboardHandler {
    fn name(&self) -> &str { "impact_leaderboard" }
    fn description(&self) -> &str { "Get overall impact leaderboard from local database" }
    fn input_schema(&self) -> ToolInputSchema {
        ToolInputSchema::object(
            vec![
                ("limit".into(), ToolProperty::integer("Max results (default: 20)")),
                ("year_min".into(), ToolProperty::integer("Minimum year (default: 2020)")),
            ].into_iter().collect(),
            vec![],
        )
    }
    async fn call(&self, params: Value) -> Result<Value, String> {
        let limit = params.get("limit").and_then(|v| v.as_u64()).unwrap_or(20) as usize;
        let _year_min = params.get("year_min").and_then(|v| v.as_u64()).unwrap_or(2020) as i32;
        // Use the Rust impact module from rairos-llm
        // For now, delegate to the leaderboard handler with rankings action
        Ok(rairos_leaderboard::leaderboard_action("rankings", None, "combined", limit))
    }
}

// ─── Claim Graph ───────────────────────────────────────────────────────────

pub struct ClaimGraphHandler;

#[async_trait]
impl ToolHandler for ClaimGraphHandler {
    fn name(&self) -> &str { "claim_graph" }
    fn description(&self) -> &str { "Cross-paper numerical claim tracking with contradiction detection" }
    fn input_schema(&self) -> ToolInputSchema {
        ToolInputSchema::object(
            vec![
                ("action".into(), ToolProperty::string("Action: status, add_claim, add_edge, contradictions (default: status)")),
                ("paper_id".into(), ToolProperty::string("Paper ID for add_claim")),
                ("claim_type".into(), ToolProperty::string("Claim type: accuracy, efficiency, scalability, etc.")),
                ("value".into(), ToolProperty::string("Numeric value of the claim")),
                ("source_text".into(), ToolProperty::string("Source text for the claim")),
                ("from_paper".into(), ToolProperty::string("Source paper ID for edge")),
                ("to_paper".into(), ToolProperty::string("Target paper ID for edge")),
                ("improvement_ratio".into(), ToolProperty::string("Improvement ratio for improvement edges")),
            ].into_iter().collect(),
            vec![],
        )
    }
    async fn call(&self, params: Value) -> Result<Value, String> {
        let action = params.get("action").and_then(|v| v.as_str()).unwrap_or("status");
        let paper_id = params.get("paper_id").and_then(|v| v.as_str());
        let claim_type = params.get("claim_type").and_then(|v| v.as_str());
        let value = params.get("value").and_then(|v| v.as_f64());
        let source_text = params.get("source_text").and_then(|v| v.as_str());
        let from_paper = params.get("from_paper").and_then(|v| v.as_str());
        let to_paper = params.get("to_paper").and_then(|v| v.as_str());
        let improvement_ratio = params.get("improvement_ratio").and_then(|v| v.as_f64());
        Ok(rairos_claimgraph_py::claim_graph_action(
            action, paper_id, claim_type, value, source_text, from_paper, to_paper, improvement_ratio,
        ))
    }
}

// ─── Hypothesis Generate ──────────────────────────────────────────────────

pub struct HypothesisGenerateHandler;

#[async_trait]
impl ToolHandler for HypothesisGenerateHandler {
    fn name(&self) -> &str { "hypothesis_generate" }
    fn description(&self) -> &str { "Generate testable research hypotheses from topic + gap context with experiment designs, risk assessment, and scoring" }
    fn input_schema(&self) -> ToolInputSchema {
        ToolInputSchema::object(
            vec![
                ("topic".into(), ToolProperty::string("Research topic")),
                ("gap_context".into(), ToolProperty::string("Context from gap detection (optional)")),
                ("gap_type".into(), ToolProperty::string("Type of gap (optional, auto-detected from context)")),
                ("creative".into(), ToolProperty::string("Generate creative cross-domain hypotheses (true/false, default false)")),
            ].into_iter().collect(),
            vec!["topic".into()],
        )
    }
    async fn call(&self, params: Value) -> Result<Value, String> {
        let topic = params["topic"].as_str().ok_or("Missing required parameter: topic")?;
        let gap_context = params.get("gap_context").and_then(|v| v.as_str()).unwrap_or("");
        let creative = params.get("creative").and_then(|v| v.as_str()).unwrap_or("false") == "true";

        // Try LLM-enhanced path if client available
        if let Some(client) = llm_client() {
            let gen = rairos_research::hypothesis_generator::HypothesisGenerator::new();
            let result = gen.generate_llm(
                client.as_ref(), llm_model(), topic, gap_context, creative,
            ).await;
            return Ok(serde_json::to_value(result).unwrap_or_else(|_| serde_json::json!({
                "topic": topic, "summary": "Error serializing result", "hypotheses": []
            })));
        }

        // No LLM: template-only fallback
        let gen = rairos_research::hypothesis_generator::HypothesisGenerator::new();
        let result = gen.generate(topic, gap_context, creative);
        Ok(serde_json::to_value(result).unwrap_or_else(|_| serde_json::json!({
            "topic": topic, "summary": "Error serializing result", "hypotheses": []
        })))
    }
}

// ─── Register ───────────────────────────────────────────────────────────────

pub async fn register_llm_handlers(server: &crate::McpServer) {
    tracing::debug!("registering 24 llm-backed MCP tool handlers");
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
    server.register(GenePoolDecayHandler).await;
    server.register(CrossoverHandler).await;
    server.register(ResearchMemoryAddStanceHandler).await;
    server.register(ResearchMemoryListStancesHandler).await;
    server.register(ResearchMemoryCheckPaperHandler).await;
    server.register(ResearchMemoryAnomaliesHandler).await;
    server.register(LeaderboardHandler).await;
    server.register(ImpactLeaderboardHandler).await;
    server.register(ClaimGraphHandler).await;
    server.register(TagAllHandler).await;
    server.register(ReviewListHandler).await;
    server.register(ExperimentRecordHandler).await;
    server.register(LitReviewListHandler).await;
    server.register(ReviewSimulateHandler).await;
    server.register(GenePoolWatcherHandler).await;
    server.register(ReplicationCompareHandler).await;
    server.register(RoutePlanListHandler).await;
    server.register(RoutePlanUpdateStepHandler).await;
    server.register(RoutePlanReviseHandler).await;
    server.register(ResearchRunHandler).await;
    server.register(HypothesisGenerateHandler).await;
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
            let _ = rairos_llm::route_planner::save_plan(&plan);
            return Ok(serde_json::json!(plan));
        }

        // No LLM: keyword routing fallback (semantic_router)
        let route = rairos_llm::semantic_router::route_by_keyword(hypothesis);
        Ok(serde_json::json!({"semantic_route": route, "note": "No LLM available — keyword routing only. Set OPENAI_API_KEY or ANTHROPIC_API_KEY for full plan generation."}))
    }
}

// ─── Tag All ────────────────────────────────────────────────────────────────

pub struct TagAllHandler;

#[async_trait]
impl ToolHandler for TagAllHandler {
    fn name(&self) -> &str { "tag_all" }
    fn description(&self) -> &str { "List all tags in the system from the database" }
    fn input_schema(&self) -> ToolInputSchema {
        ToolInputSchema::object(HashMap::new(), vec![])
    }
    async fn call(&self, _params: Value) -> Result<Value, String> {
        let db_path = std::env::var("RAIROS_DB").unwrap_or_else(|_| "rairos.db".to_string());
        let db = Database::open(&db_path).map_err(|e| format!("DB error: {}", e))?;
        let tags = db.list_tags().map_err(|e| format!("List tags error: {}", e))?;
        let names: Vec<String> = tags.into_iter().map(|t| t.name).collect();
        Ok(serde_json::json!({"tags": names, "count": names.len()}))
    }
}

// ─── Review List ───────────────────────────────────────────────────────────

pub struct ReviewListHandler;

#[async_trait]
impl ToolHandler for ReviewListHandler {
    fn name(&self) -> &str { "review_list" }
    fn description(&self) -> &str { "List saved simulated reviews" }
    fn input_schema(&self) -> ToolInputSchema {
        ToolInputSchema::object(HashMap::new(), vec![])
    }
    async fn call(&self, _params: Value) -> Result<Value, String> {
        let reviews = rairos_review_simulator::list_reviews(20);
        Ok(serde_json::json!({"reviews": reviews, "count": reviews.len()}))
    }
}

// ─── Experiment Record ─────────────────────────────────────────────────────

pub struct ExperimentRecordHandler;

#[async_trait]
impl ToolHandler for ExperimentRecordHandler {
    fn name(&self) -> &str { "experiment_record" }
    fn description(&self) -> &str { "Record an experiment result for a hypothesis" }
    fn input_schema(&self) -> ToolInputSchema {
        ToolInputSchema::object(
            vec![
                ("hypothesis_id".into(), ToolProperty::string("ID of the hypothesis")),
                ("name".into(), ToolProperty::string("Name of the experiment")),
                ("result".into(), ToolProperty::string("Result: validated, rejected, failed, running, or completed")),
                ("metrics".into(), ToolProperty::string("Optional JSON object of metrics")),
            ].into_iter().collect(),
            vec!["hypothesis_id".into(), "name".into(), "result".into()],
        )
    }
    async fn call(&self, params: Value) -> Result<Value, String> {
        let hypothesis_id = params["hypothesis_id"].as_str().ok_or("Missing hypothesis_id")?;
        let name = params["name"].as_str().ok_or("Missing name")?;
        let result = params["result"].as_str().ok_or("Missing result")?;

        let tracker = ExperimentTracker::new(None);
        let exp = tracker.run(name, "", "", hypothesis_id, None, None);

        let metrics: Option<serde_json::Value> = params.get("metrics")
            .and_then(|v| {
                if v.is_string() { serde_json::from_str(v.as_str()?).ok() } else { Some(v.clone()) }
            });

        match result.to_lowercase().as_str() {
            "rejected" | "failed" => {
                tracker.fail(&exp.id, result);
            }
            _ => {
                let mut results = HashMap::new();
                results.insert("verdict".to_string(), serde_json::json!(result));
                if let Some(m) = metrics {
                    results.insert("metrics".to_string(), m);
                }
                tracker.complete(&exp.id, Some(results));
            }
        }

        Ok(serde_json::json!({
            "experiment_id": exp.id,
            "hypothesis_id": hypothesis_id,
            "status": if matches!(result.to_lowercase().as_str(), "rejected" | "failed") { "failed" } else { "completed" },
            "message": format!("Experiment recorded: {} -> {}", name, result),
        }))
    }
}

// ─── LitReview List ────────────────────────────────────────────────────────

pub struct LitReviewListHandler;

#[async_trait]
impl ToolHandler for LitReviewListHandler {
    fn name(&self) -> &str { "litreview_list" }
    fn description(&self) -> &str { "List all saved literature reviews" }
    fn input_schema(&self) -> ToolInputSchema {
        ToolInputSchema::object(HashMap::new(), vec![])
    }
    async fn call(&self, _params: Value) -> Result<Value, String> {
        let cwd = std::env::current_dir().map_err(|e| format!("CWD error: {}", e))?;
        let reviews_dir = cwd.join("data").join("litreviews");
        let mut reviews = Vec::new();

        if reviews_dir.exists() {
            let mut entries: Vec<_> = std::fs::read_dir(&reviews_dir)
                .map_err(|e| format!("Read dir error: {}", e))?
                .filter_map(|e| e.ok())
                .filter(|e| {
                    let name = e.file_name();
                    let n = name.to_string_lossy();
                    n.starts_with("litreview_") && n.ends_with(".md")
                })
                .collect();
            entries.sort_by(|a, b| {
                let a_m = a.metadata().ok().and_then(|m| m.modified().ok());
                let b_m = b.metadata().ok().and_then(|m| m.modified().ok());
                b_m.cmp(&a_m)
            });
            for entry in entries.iter().take(20) {
                let content = std::fs::read_to_string(entry.path()).unwrap_or_default();
                let lines: Vec<&str> = content.lines().collect();
                let title = lines.first()
                    .map(|l| l.trim_start_matches("# ").trim().to_string())
                    .unwrap_or_else(|| {
                        entry.file_name().to_string_lossy().replace(".md", "")
                    });
                let mut date = String::new();
                for line in lines.iter().skip(1).take(4) {
                    if let Some(pos) = line.find("Generated:") {
                        date = line[pos + 10..].trim().to_string();
                        break;
                    }
                }
                let size = entry.metadata().ok().map(|m| m.len()).unwrap_or(0);
                reviews.push(serde_json::json!({
                    "filename": entry.file_name().to_string_lossy(),
                    "topic": title,
                    "date": date,
                    "size_bytes": size,
                }));
            }
        }

        Ok(serde_json::json!({"reviews": reviews, "count": reviews.len()}))
    }
}

// ─── Review Simulate ───────────────────────────────────────────────────────

pub struct ReviewSimulateHandler;

#[async_trait]
impl ToolHandler for ReviewSimulateHandler {
    fn name(&self) -> &str { "review_simulate" }
    fn description(&self) -> &str { "Simulate adversarial peer reviewers on a paper" }
    fn input_schema(&self) -> ToolInputSchema {
        ToolInputSchema::object(
            vec![
                ("arxiv_id".into(), ToolProperty::string("arXiv ID of the paper to review")),
                ("persona".into(), ToolProperty::string("Reviewer persona (e.g. 'methodologist', 'all' for consensus)")),
            ].into_iter().collect(),
            vec!["arxiv_id".into()],
        )
    }
    async fn call(&self, params: Value) -> Result<Value, String> {
        let arxiv_id = params["arxiv_id"].as_str().ok_or("Missing arxiv_id")?;
        let persona = params.get("persona").and_then(|v| v.as_str()).unwrap_or("all");

        // Fetch paper from DB
        let db_path = std::env::var("RAIROS_DB").unwrap_or_else(|_| "rairos.db".to_string());
        let db = Database::open(&db_path).map_err(|e| format!("DB error: {}", e))?;
        let paper = db.get_paper_by_arxiv(arxiv_id)
            .map_err(|e| format!("DB query error: {}", e))?
            .ok_or_else(|| format!("Paper not found: {}", arxiv_id))?;

        let full_text = format!("{}\n\n{}", paper.title, paper.abstract_text);

        // Run review simulator (uses env vars for LLM credentials via resolve_credentials)
        let simulator = rairos_review_simulator::ReviewSimulator::new();
        let review = if persona != "all" {
            let personas = rairos_review_simulator::default_personas();
            let selected = personas.into_iter().find(|p| {
                p.name.to_lowercase().starts_with(&persona.to_lowercase())
            }).ok_or_else(|| format!("Unknown persona: {}", persona))?;
            simulator.review(&full_text, Some(&paper.title), Some(&selected), None, None, None).await
                .map_err(|e| format!("Review error: {}", e))?
        } else {
            simulator.review(&full_text, Some(&paper.title), None, None, None, None).await
                .map_err(|e| format!("Review error: {}", e))?
        };

        // Save review
        rairos_review_simulator::save_review(&review);

        Ok(serde_json::json!({
            "review_id": review.review_id,
            "persona": review.persona,
            "overall_score": review.overall_score,
            "summary": review.summary,
            "strengths": review.strengths,
            "weaknesses": review.weaknesses,
            "recommendation": review.recommendation,
            "annotation_count": review.annotations.len(),
        }))
    }
}

// ─── Gene Pool Watcher ─────────────────────────────────────────────────────

pub struct GenePoolWatcherHandler;

#[async_trait]
impl ToolHandler for GenePoolWatcherHandler {
    fn name(&self) -> &str { "gene_pool_watcher" }
    fn description(&self) -> &str { "Manage GenePoolWatcher: check diversity gaps and auto-subscribe" }
    fn input_schema(&self) -> ToolInputSchema {
        ToolInputSchema::object(
            vec![
                ("action".into(), ToolProperty::string("Action: status, start, stop, or trigger_now")),
                ("interval_minutes".into(), ToolProperty::integer("Check interval in minutes")),
                ("min_diversity_score".into(), ToolProperty::string("Minimum diversity score threshold (0-100)")),
            ].into_iter().collect(),
            vec![],
        )
    }
    async fn call(&self, params: Value) -> Result<Value, String> {
        let action = params.get("action").and_then(|v| v.as_str()).unwrap_or("status");
        let interval = params.get("interval_minutes").and_then(|v| v.as_u64()).unwrap_or(60);
        let min_score = params.get("min_diversity_score").and_then(|v| v.as_f64()).unwrap_or(50.0);

        match action {
            "start" => {
                let mut watcher = GenePoolWatcher::new(interval, min_score, true);
                let result = watcher.watch();
                Ok(serde_json::json!({
                    "status": "started",
                    "message": format!("GenePoolWatcher started. Will check diversity every {}min.", interval),
                    "diversity_score": result.diversity_score,
                    "underrepresented_families": result.underrepresented_families,
                }))
            }
            "trigger_now" => {
                let mut watcher = GenePoolWatcher::new(interval, min_score, true);
                let result = watcher.watch();
                Ok(serde_json::json!({
                    "status": "checked",
                    "diversity_score": result.diversity_score,
                    "total_capsules": result.total_capsules,
                    "underrepresented_families": result.underrepresented_families,
                    "gap_subscriptions_added": result.gap_subscriptions_added,
                    "gap_subscriptions_removed": result.gap_subscriptions_removed,
                    "triggered": result.triggered,
                }))
            }
            _ => {
                let watcher = GenePoolWatcher::new(interval, min_score, false);
                let state = watcher.get_state();
                Ok(serde_json::json!({
                    "status": "ok",
                    "diversity_score": state.diversity_score,
                    "underrepresented_families": state.underrepresented_families,
                    "gap_subscriptions": state.gap_subscriptions.iter().map(|gs| {
                        serde_json::json!({
                            "family": gs.family,
                            "enabled": gs.enabled,
                            "keywords": gs.keywords,
                        })
                    }).collect::<Vec<_>>(),
                }))
            }
        }
    }
}

// ─── Replication Compare ──────────────────────────────────────────────────

pub struct ReplicationCompareHandler;

#[async_trait]
impl ToolHandler for ReplicationCompareHandler {
    fn name(&self) -> &str { "replication_compare" }
    fn description(&self) -> &str { "Compare reproducibility of two papers" }
    fn input_schema(&self) -> ToolInputSchema {
        ToolInputSchema::object(
            vec![
                ("arxiv_id_1".into(), ToolProperty::string("arXiv ID of the first paper")),
                ("arxiv_id_2".into(), ToolProperty::string("arXiv ID of the second paper")),
            ].into_iter().collect(),
            vec!["arxiv_id_1".into(), "arxiv_id_2".into()],
        )
    }
    async fn call(&self, params: Value) -> Result<Value, String> {
        let arxiv_id_1 = params["arxiv_id_1"].as_str().ok_or("Missing arxiv_id_1")?;
        let arxiv_id_2 = params["arxiv_id_2"].as_str().ok_or("Missing arxiv_id_2")?;

        let checker = rairos_replication_checker::ReplicationChecker::new();

        let report1 = checker.check_paper(arxiv_id_1, arxiv_id_1, "", "");
        let report2 = checker.check_paper(arxiv_id_2, arxiv_id_2, "", "");

        let easier_id = if report1.difficulty_score < report2.difficulty_score {
            report1.paper_id.clone()
        } else {
            report2.paper_id.clone()
        };

        Ok(serde_json::json!({
            "paper_1": report1,
            "paper_2": report2,
            "easier_to_reproduce": easier_id,
            "comparison": {
                "difficulty_diff": (report1.difficulty_score - report2.difficulty_score).abs() as f64,
            },
        }))
    }
}

// ─── Route Plan List ──────────────────────────────────────────────────────

pub struct RoutePlanListHandler;

#[async_trait]
impl ToolHandler for RoutePlanListHandler {
    fn name(&self) -> &str { "routeplan_list" }
    fn description(&self) -> &str { "List all research plans" }
    fn input_schema(&self) -> ToolInputSchema {
        ToolInputSchema::object(HashMap::new(), vec![])
    }
    async fn call(&self, _params: Value) -> Result<Value, String> {
        let plans = rairos_llm::route_planner::list_plans(20);
        let list: Vec<Value> = plans.iter().map(|p| {
            let progress = p.get_progress();
            serde_json::json!({
                "plan_id": p.plan_id,
                "hypothesis": p.hypothesis.chars().take(80).collect::<String>(),
                "goal": p.goal.chars().take(80).collect::<String>(),
                "status": p.status,
                "step_count": p.steps.len(),
                "progress": progress.progress_pct,
                "revision_count": p.revision_count,
                "created_at": p.created_at,
                "updated_at": p.updated_at,
            })
        }).collect();
        Ok(serde_json::json!({"plans": list, "count": list.len()}))
    }
}

// ─── Route Plan Update Step ────────────────────────────────────────────────

pub struct RoutePlanUpdateStepHandler;

#[async_trait]
impl ToolHandler for RoutePlanUpdateStepHandler {
    fn name(&self) -> &str { "routeplan_update_step" }
    fn description(&self) -> &str { "Update a step status in a research plan" }
    fn input_schema(&self) -> ToolInputSchema {
        ToolInputSchema::object(
            vec![
                ("plan_id".into(), ToolProperty::string("ID of the plan")),
                ("step_id".into(), ToolProperty::string("ID of the step to update")),
                ("status".into(), ToolProperty::string("New status: pending, in_progress, completed, blocked, failed, skipped")),
                ("result".into(), ToolProperty::string("Result details (optional)")),
                ("notes".into(), ToolProperty::string("Notes (optional)")),
            ].into_iter().collect(),
            vec!["plan_id".into(), "step_id".into(), "status".into()],
        )
    }
    async fn call(&self, params: Value) -> Result<Value, String> {
        let plan_id = params["plan_id"].as_str().ok_or("Missing plan_id")?;
        let step_id = params["step_id"].as_str().ok_or("Missing step_id")?;
        let status = params["status"].as_str().ok_or("Missing status")?;
        let result = params.get("result").and_then(|v| v.as_str()).unwrap_or("");
        let notes = params.get("notes").and_then(|v| v.as_str()).unwrap_or("");

        let plan = rairos_llm::route_planner::update_step(plan_id, step_id, status, result, notes)
            .ok_or_else(|| format!("Plan {} or step {} not found", plan_id, step_id))?;

        let ready: Vec<Value> = plan.get_ready_steps().iter().map(|s| {
            serde_json::json!({"step_id": s.step_id, "description": s.description})
        }).collect();

        Ok(serde_json::json!({
            "plan_id": plan.plan_id,
            "step_id": step_id,
            "status": status,
            "progress": plan.get_progress(),
            "ready_steps": ready,
        }))
    }
}

// ─── Route Plan Revise ────────────────────────────────────────────────────

pub struct RoutePlanReviseHandler;

#[async_trait]
impl ToolHandler for RoutePlanReviseHandler {
    fn name(&self) -> &str { "routeplan_revise" }
    fn description(&self) -> &str { "Revise a plan when dead ends are hit" }
    fn input_schema(&self) -> ToolInputSchema {
        ToolInputSchema::object(
            vec![
                ("plan_id".into(), ToolProperty::string("ID of the plan to revise")),
                ("reason".into(), ToolProperty::string("Reason for the revision")),
            ].into_iter().collect(),
            vec!["plan_id".into(), "reason".into()],
        )
    }
    async fn call(&self, params: Value) -> Result<Value, String> {
        let plan_id = params["plan_id"].as_str().ok_or("Missing plan_id")?;
        let reason = params["reason"].as_str().ok_or("Missing reason")?;

        let new_plan = rairos_llm::route_planner::revise_plan(plan_id, reason)
            .ok_or_else(|| format!("Plan {} not found", plan_id))?;

        Ok(serde_json::json!({
            "new_plan_id": new_plan.plan_id,
            "old_plan_id": plan_id,
            "revision_count": new_plan.revision_count,
            "step_count": new_plan.steps.len(),
            "progress": new_plan.get_progress(),
        }))
    }
}

// ─── Research Run ─────────────────────────────────────────────────────────

pub struct ResearchRunHandler;

#[async_trait]
impl ToolHandler for ResearchRunHandler {
    fn name(&self) -> &str { "research_run" }
    fn description(&self) -> &str { "Search arXiv, save papers to DB, generate report" }
    fn input_schema(&self) -> ToolInputSchema {
        ToolInputSchema::object(
            vec![
                ("topic".into(), ToolProperty::string("Research topic to search for")),
                ("limit".into(), ToolProperty::integer("Maximum results (default 5, max 20)")),
            ].into_iter().collect(),
            vec!["topic".into()],
        )
    }
    async fn call(&self, params: Value) -> Result<Value, String> {
        let topic = params["topic"].as_str().ok_or("Missing topic")?;
        let limit = (params["limit"].as_u64().unwrap_or(5) as usize).min(20);

        // Search arXiv
        let arxiv_url = "http://export.arxiv.org/api/query";
        let url = format!("{}?search_query=all:{}&start=0&max_results={}", arxiv_url, topic.replace(' ', "+"), limit);
        let resp = reqwest::get(&url).await.map_err(|e| format!("arXiv request failed: {}", e))?;
        let text = resp.text().await.map_err(|e| format!("Read failed: {}", e))?;
        let papers = crate::handlers::parse_arxiv_response(&text);

        // Save to DB
        let db_path = std::env::var("RAIROS_DB").unwrap_or_else(|_| "rairos.db".to_string());
        let db = Database::open(&db_path).map_err(|e| format!("DB error: {}", e))?;

        let mut saved = 0;
        for p in &papers {
            let arxiv_id = p["arxiv_id"].as_str().unwrap_or("");
            if arxiv_id.is_empty() { continue; }
            let title = p["title"].as_str().unwrap_or("");
            let abstract_text = p["abstract"].as_str().unwrap_or("");
            let authors: Vec<String> = p["authors"].as_array()
                .map(|a| a.iter().filter_map(|v| v.as_str().map(String::from)).collect())
                .unwrap_or_default();
            let categories: Vec<String> = p["categories"].as_array()
                .map(|c| c.iter().filter_map(|v| v.as_str().map(String::from)).collect())
                .unwrap_or_default();

            let paper = rairos_core::Paper::with_metadata(
                Some(arxiv_id.to_string()),
                title.to_string(),
                abstract_text.to_string(),
                authors,
                categories,
                rairos_core::PaperMetadata::default(),
            );
            if db.insert_paper(&paper).is_ok() {
                saved += 1;
            }
        }

        Ok(serde_json::json!({
            "topic": topic,
            "papers_found": papers.len(),
            "papers_saved": saved,
            "status": "completed",
        }))
    }
}