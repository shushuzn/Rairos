use crate::handlers::helpers::{data_dir, gene_pool_path, read_jsonl};
use crate::protocol::{ToolHandler, ToolInputSchema, ToolProperty};
use async_trait::async_trait;
use rairos_core::constants::{GP_DIR_NAME, GENE_POOL_JSONL};
use serde_json::Value;

pub struct PaperSearchHandler;

#[async_trait]
impl ToolHandler for PaperSearchHandler {
    fn name(&self) -> &str { "paper_search" }
    fn description(&self) -> &str { "Search for research papers on arXiv by query" }
    fn input_schema(&self) -> ToolInputSchema {
        ToolInputSchema::object(
            vec![
                ("query".into(), ToolProperty::string("Search query")),
                ("max_results".into(), ToolProperty::integer("Maximum results (default 10, max 50)")),
            ].into_iter().collect(),
            vec!["query".into()],
        )
    }
    async fn call(&self, params: Value) -> Result<Value, String> {
        let query = params["query"].as_str().ok_or("Missing required parameter: query")?;
        let max = (params["max_results"].as_u64().unwrap_or(10) as usize).min(50);
        let papers = rairos_parser::search_arxiv(query, max)
            .await
            .map_err(|e| format!("Search failed: {}", e))?;
        let values: Vec<Value> = papers.into_iter()
            .map(|p| serde_json::to_value(p).unwrap_or_default())
            .collect();
        Ok(serde_json::json!({"papers": values, "total": values.len()}))
    }
}

pub struct PaperIngestHandler;

#[async_trait]
impl ToolHandler for PaperIngestHandler {
    fn name(&self) -> &str { "paper_ingest" }
    fn description(&self) -> &str { "Fetch paper metadata from arXiv by ID" }
    fn input_schema(&self) -> ToolInputSchema {
        ToolInputSchema::object(
            vec![("arxiv_id".into(), ToolProperty::string("arXiv ID to ingest"))].into_iter().collect(),
            vec!["arxiv_id".into()],
        )
    }
    async fn call(&self, params: Value) -> Result<Value, String> {
        let id = params["arxiv_id"].as_str().ok_or("Missing arxiv_id")?;
        let paper = rairos_parser::fetch_arxiv(id)
            .await
            .map_err(|e| format!("arXiv fetch failed: {}", e))?;
        serde_json::to_value(&paper).map_err(|e| format!("Serialization failed: {}", e))
    }
}

pub struct PaperRecommendHandler;

#[async_trait]
impl ToolHandler for PaperRecommendHandler {
    fn name(&self) -> &str { "paper_recommend" }
    fn description(&self) -> &str { "Recommend research topics based on GenePool search patterns" }
    fn input_schema(&self) -> ToolInputSchema {
        ToolInputSchema::object(
            vec![
                ("topic".into(), ToolProperty::string("Research topic to find related recommendations for")),
            ].into_iter().collect(),
            vec!["topic".into()],
        )
    }
    async fn call(&self, params: Value) -> Result<Value, String> {
        let topic = params["topic"].as_str().ok_or("Missing topic")?;
        let gp_path = gene_pool_path();
        let entries = read_jsonl(&gp_path);

        let topic_lower = topic.to_lowercase();
        let mut scored: Vec<(f64, Value)> = entries.into_iter().filter_map(|e| {
            let title = e["action_gap_title"].as_str().unwrap_or("").to_lowercase();
            let trigger = e["trigger_topic"].as_str().unwrap_or("").to_lowercase();
            let status = e["status"].as_str().unwrap_or("");
            if status == "archived" { return None; }

            let mut score = 0.0;
            if title.contains(&topic_lower) || topic_lower.contains(&title) { score += 0.5; }
            if trigger.contains(&topic_lower) { score += 0.3; }
            if let Some(score_val) = e["outcome_success_score"].as_f64() {
                if score_val > 0.3 { score += score_val * 0.2; }
            }

            if score > 0.0 {
                Some((score, serde_json::json!({
                    "title": title,
                    "gap_type": e["action_gap_type"],
                    "score": score,
                    "feedback_count": e["feedback_count"],
                })))
            } else { None }
        }).collect();

        scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
        scored.truncate(10);

        let recommendations: Vec<Value> = scored.into_iter().map(|(_, v)| v).collect();
        Ok(serde_json::json!({"recommendations": recommendations, "total": recommendations.len()}))
    }
}

pub struct PaperQueryHandler;

#[async_trait]
impl ToolHandler for PaperQueryHandler {
    fn name(&self) -> &str { "paper_query" }
    fn description(&self) -> &str { "Query papers by arXiv ID with full metadata" }
    fn input_schema(&self) -> ToolInputSchema {
        ToolInputSchema::object(
            vec![
                ("arxiv_ids".into(), ToolProperty::string("Comma-separated arXiv IDs")),
            ].into_iter().collect(),
            vec!["arxiv_ids".into()],
        )
    }
    async fn call(&self, params: Value) -> Result<Value, String> {
        let ids_str = params["arxiv_ids"].as_str().ok_or("Missing arxiv_ids")?;
        let ids: Vec<&str> = ids_str.split(',').map(|s| s.trim()).collect();

        let mut papers = Vec::new();
        for id in ids {
            match rairos_parser::fetch_arxiv(id).await {
                Ok(paper) => {
                    papers.push(serde_json::to_value(&paper).unwrap_or_default());
                }
                Err(e) => {
                    return Err(format!("Failed to fetch paper {}: {}", id, e));
                }
            }
        }
        Ok(serde_json::json!({"papers": papers, "total": papers.len()}))
    }
}

pub struct PaperChatHandler;

#[async_trait]
impl ToolHandler for PaperChatHandler {
    fn name(&self) -> &str { "paper_chat" }
    fn description(&self) -> &str { "Ask a question about papers — searches arXiv and returns relevant paper information" }
    fn input_schema(&self) -> ToolInputSchema {
        ToolInputSchema::object(
            vec![
                ("query".into(), ToolProperty::string("Question about research papers")),
                ("max_results".into(), ToolProperty::integer("Number of papers to search (default 5)")),
            ].into_iter().collect(),
            vec!["query".into()],
        )
    }
    async fn call(&self, params: Value) -> Result<Value, String> {
        let query = params["query"].as_str().ok_or("Missing query")?;
        let max = (params["max_results"].as_u64().unwrap_or(5) as usize).min(20);

        let papers = rairos_parser::search_arxiv(query, max)
            .await
            .map_err(|e| format!("Search failed: {}", e))?;

        let summaries: Vec<Value> = papers.iter().map(|p| {
            let abstract_text = &p.abstract_text;
            let preview = if abstract_text.len() > 200 {
                format!("{}...", &abstract_text[..200])
            } else { abstract_text.clone() };

            serde_json::json!({
                "arxiv_id": p.arxiv_id,
                "title": p.title,
                "authors": p.authors,
                "abstract_preview": preview,
                "published": p.published,
            })
        }).collect();

        Ok(serde_json::json!({
            "papers": summaries,
            "total": summaries.len(),
            "message": format!("Found {} papers related to '{}'. Full text available via paper_search.", summaries.len(), query),
        }))
    }
}
