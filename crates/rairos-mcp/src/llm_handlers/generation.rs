use crate::protocol::{ToolHandler, ToolInputSchema, ToolProperty};
use crate::llm_handlers::helpers::{llm_client, llm_model};
use async_trait::async_trait;
use rairos_llm::{impact, LlmClient};
use serde_json::Value;

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
