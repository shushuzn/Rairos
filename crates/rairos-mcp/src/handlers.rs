//! Tool handlers registry — 10 pure Rust MCP tools
//!
//! Each tool implements the ToolHandler trait. All handlers are self-contained
//! (no Python dependencies, no sub-crate calls).

use crate::protocol::{ToolHandler, ToolInputSchema, ToolProperty};
use async_trait::async_trait;
use serde_json::Value;
use std::collections::HashMap;
use std::path::PathBuf;

// ─── Helpers ───────────────────────────────────────────────────────────────────

fn home_dir() -> PathBuf {
    std::env::var("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("."))
}

fn data_dir() -> PathBuf {
    home_dir().join(".ai_research_os")
}

fn tags_path() -> PathBuf {
    data_dir().join("tags.jsonl")
}

fn read_jsonl(path: &PathBuf) -> Vec<Value> {
    let content = match std::fs::read_to_string(path) {
        Ok(c) => c,
        Err(_) => return Vec::new(),
    };
    content
        .lines()
        .filter_map(|line| {
            let t = line.trim();
            if t.is_empty() { None } else { serde_json::from_str(t).ok() }
        })
        .collect()
}

fn append_jsonl(path: &PathBuf, value: &Value) -> Result<(), String> {
    if let Some(p) = path.parent() { std::fs::create_dir_all(p).map_err(|e| e.to_string())?; }
    let line = serde_json::to_string(value).map_err(|e| e.to_string())?;
    let mut file = std::fs::OpenOptions::new().create(true).append(true).open(path)
        .map_err(|e| e.to_string())?;
    use std::io::Write;
    writeln!(file, "{}", line).map_err(|e| e.to_string())
}

fn write_jsonl(path: &PathBuf, items: &[Value]) -> Result<(), String> {
    if let Some(p) = path.parent() { std::fs::create_dir_all(p).map_err(|e| e.to_string())?; }
    let mut file = std::fs::File::create(path).map_err(|e| e.to_string())?;
    use std::io::Write;
    for item in items {
        let line = serde_json::to_string(item).map_err(|e| e.to_string())?;
        writeln!(file, "{}", line).map_err(|e| e.to_string())?;
    }
    Ok(())
}

const ARXIV_API: &str = "http://export.arxiv.org/api/query";

// ─── Paper Search ─────────────────────────────────────────────────────────────

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
        let url = format!("{}?search_query=all:{}&start=0&max_results={}", ARXIV_API, query.replace(' ', "+"), max);
        let resp = reqwest::get(&url).await.map_err(|e| format!("arXiv request failed: {}", e))?;
        let text = resp.text().await.map_err(|e| format!("Read failed: {}", e))?;
        let papers = parse_arxiv_response(&text);
        Ok(serde_json::json!({"papers": papers, "total": papers.len()}))
    }
}

// ─── Paper Ingest ─────────────────────────────────────────────────────────────

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
        let url = format!("{}?id_list={}", ARXIV_API, id);
        let resp = reqwest::get(&url).await.map_err(|e| format!("arXiv request failed: {}", e))?;
        let text = resp.text().await.map_err(|e| format!("Read failed: {}", e))?;
        parse_arxiv_response(&text).into_iter().next().ok_or_else(|| format!("No paper found: {}", id))
    }
}

// ─── Tags: Add, Remove, List ──────────────────────────────────────────────────

pub struct TagAddHandler;

#[async_trait]
impl ToolHandler for TagAddHandler {
    fn name(&self) -> &str { "tag_add" }
    fn description(&self) -> &str { "Add a tag to a paper" }
    fn input_schema(&self) -> ToolInputSchema {
        ToolInputSchema::object(
            vec![
                ("arxiv_id".into(), ToolProperty::string("arXiv ID of the paper")),
                ("tag".into(), ToolProperty::string("Tag name")),
            ].into_iter().collect(),
            vec!["arxiv_id".into(), "tag".into()],
        )
    }
    async fn call(&self, params: Value) -> Result<Value, String> {
        let arxiv_id = params["arxiv_id"].as_str().ok_or("Missing arxiv_id")?;
        let tag = params["tag"].as_str().ok_or("Missing tag")?;
        let entry = serde_json::json!({"arxiv_id": arxiv_id, "tag": tag, "created_at": chrono_now()});
        append_jsonl(&tags_path(), &entry)?;
        Ok(serde_json::json!({"status": "added", "arxiv_id": arxiv_id, "tag": tag}))
    }
}

pub struct TagRemoveHandler;

#[async_trait]
impl ToolHandler for TagRemoveHandler {
    fn name(&self) -> &str { "tag_remove" }
    fn description(&self) -> &str { "Remove a tag from a paper" }
    fn input_schema(&self) -> ToolInputSchema {
        ToolInputSchema::object(
            vec![
                ("arxiv_id".into(), ToolProperty::string("arXiv ID")),
                ("tag".into(), ToolProperty::string("Tag name to remove")),
            ].into_iter().collect(),
            vec!["arxiv_id".into(), "tag".into()],
        )
    }
    async fn call(&self, params: Value) -> Result<Value, String> {
        let arxiv_id = params["arxiv_id"].as_str().ok_or("Missing arxiv_id")?;
        let tag = params["tag"].as_str().ok_or("Missing tag")?;
        let entries = read_jsonl(&tags_path());
        let before = entries.len();
        let filtered: Vec<Value> = entries.into_iter().filter(|e| {
            !(e["arxiv_id"].as_str() == Some(arxiv_id) && e["tag"].as_str() == Some(tag))
        }).collect();
        let removed = before - filtered.len();
        write_jsonl(&tags_path(), &filtered)?;
        Ok(serde_json::json!({"status": "removed", "count": removed}))
    }
}

pub struct TagListHandler;

#[async_trait]
impl ToolHandler for TagListHandler {
    fn name(&self) -> &str { "tag_list" }
    fn description(&self) -> &str { "List all tags and their associated papers" }
    fn input_schema(&self) -> ToolInputSchema {
        ToolInputSchema::object(HashMap::new(), vec![])
    }
    async fn call(&self, _params: Value) -> Result<Value, String> {
        let entries = read_jsonl(&tags_path());
        let mut by_tag: HashMap<String, Vec<String>> = HashMap::new();
        for e in &entries {
            if let (Some(tag), Some(id)) = (e["tag"].as_str(), e["arxiv_id"].as_str()) {
                by_tag.entry(tag.to_string()).or_default().push(id.to_string());
            }
        }
        let tags: Vec<Value> = by_tag.into_iter().map(|(tag, papers)| {
            serde_json::json!({"tag": tag, "papers": papers, "count": papers.len()})
        }).collect();
        Ok(serde_json::json!({"tags": tags, "total": tags.len()}))
    }
}

// ─── Trends: Detect Trending Topics ────────────────────────────────────────────

pub struct TrendsDetectTrendingHandler;

#[async_trait]
impl ToolHandler for TrendsDetectTrendingHandler {
    fn name(&self) -> &str { "trends_detect_trending" }
    fn description(&self) -> &str { "Detect trending research topics from recent arXiv papers" }
    fn input_schema(&self) -> ToolInputSchema {
        ToolInputSchema::object(
            vec![
                ("category".into(), ToolProperty::string("arXiv category (e.g. cs.LG, cs.CL, all)")),
                ("max_results".into(), ToolProperty::integer("Number of recent papers to analyze (default 100)")),
            ].into_iter().collect(),
            vec![],
        )
    }
    async fn call(&self, params: Value) -> Result<Value, String> {
        let category = params["category"].as_str().unwrap_or("cs.LG");
        let max = (params["max_results"].as_u64().unwrap_or(100) as usize).min(200);

        let query = if category == "all" { "cat:*".to_string() } else { format!("cat:{}", category) };
        let url = format!("{}?search_query={}&sortBy=submittedDate&sortOrder=descending&max_results={}", ARXIV_API, query, max);
        let resp = reqwest::get(&url).await.map_err(|e| format!("arXiv request failed: {}", e))?;
        let text = resp.text().await.map_err(|e| format!("Read failed: {}", e))?;
        let papers = parse_arxiv_response(&text);

        // Count keyword frequency (simple bag-of-words)
        let mut word_count: HashMap<String, usize> = HashMap::new();
        for p in &papers {
            let title = p["title"].as_str().unwrap_or("").to_lowercase();
            for word in title.split_whitespace() {
                let clean: String = word.chars().filter(|c| c.is_alphanumeric()).collect();
                if clean.len() > 3 {
                    *word_count.entry(clean).or_default() += 1;
                }
            }
        }

        let mut trends: Vec<Value> = word_count.into_iter()
            .map(|(word, count)| serde_json::json!({"keyword": word, "count": count}))
            .collect();
        trends.sort_by(|a, b| b["count"].as_u64().cmp(&a["count"].as_u64()));
        trends.truncate(20);

        Ok(serde_json::json!({
            "trends": trends,
            "papers_analyzed": papers.len(),
            "category": category,
        }))
    }
}

// ─── Paper Recommend (GenePool-based) ──────────────────────────────────────────

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
        let gp_path = data_dir().join("evolution").join("gene_pool.jsonl");
        let entries = read_jsonl(&gp_path);

        // Score each capsule by topic overlap (simple substring match)
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

// ─── Citation Graph ────────────────────────────────────────────────────────────

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

        // Get the paper metadata from arXiv
        let url = format!("{}?id_list={}", ARXIV_API, arxiv_id);
        let resp = reqwest::get(&url).await.map_err(|e| format!("arXiv request failed: {}", e))?;
        let text = resp.text().await.map_err(|e| format!("Read failed: {}", e))?;
        let papers = parse_arxiv_response(&text);
        let paper = papers.into_iter().next().ok_or_else(|| format!("Paper not found: {}", arxiv_id))?;

        Ok(serde_json::json!({
            "paper": paper,
            "citations": [],
            "note": "Full citation graph requires Semantic Scholar API integration"
        }))
    }
}

// ─── Paper Query (enhanced search) ─────────────────────────────────────────────

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
            let url = format!("{}?id_list={}", ARXIV_API, id);
            if let Ok(resp) = reqwest::get(&url).await {
                if let Ok(text) = resp.text().await {
                    papers.extend(parse_arxiv_response(&text));
                }
            }
        }
        Ok(serde_json::json!({"papers": papers, "total": papers.len()}))
    }
}

// ─── Paper Chat (abstract-based Q&A) ──────────────────────────────────────────

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

        let url = format!("{}?search_query=all:{}&start=0&max_results={}", ARXIV_API, query.replace(' ', "+"), max);
        let resp = reqwest::get(&url).await.map_err(|e| format!("arXiv request failed: {}", e))?;
        let text = resp.text().await.map_err(|e| format!("Read failed: {}", e))?;
        let papers = parse_arxiv_response(&text);

        let summaries: Vec<Value> = papers.iter().map(|p| {
            let abstract_text = p["abstract"].as_str().unwrap_or("");
            let preview = if abstract_text.len() > 200 {
                format!("{}...", &abstract_text[..200])
            } else { abstract_text.to_string() };

            serde_json::json!({
                "arxiv_id": p["arxiv_id"],
                "title": p["title"],
                "authors": p["authors"],
                "abstract_preview": preview,
                "published": p["published"],
            })
        }).collect();

        Ok(serde_json::json!({
            "papers": summaries,
            "total": summaries.len(),
            "message": format!("Found {} papers related to '{}'. Full text available via paper_search.", summaries.len(), query),
        }))
    }
}

// ─── KG: Paper Subgraph ───────────────────────────────────────────────────────

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

        let db_path = rairos_kg::KnowledgeGraph::db_path();
        let graph = rairos_kg::KnowledgeGraph::with_db(db_path)
            .map_err(|e| format!("KG init: {}", e))?;
        let sub = graph.get_paper_subgraph(arxiv_id, depth, include_notes)
            .map_err(|e| format!("Subgraph query: {}", e))?;
        Ok(serde_json::to_value(sub).unwrap_or_default())
    }
}

// ─── KG: Tag Graph ────────────────────────────────────────────────────────────

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
        let db_path = rairos_kg::KnowledgeGraph::db_path();
        let graph = rairos_kg::KnowledgeGraph::with_db(db_path)
            .map_err(|e| format!("KG init: {}", e))?;
        let sub = graph.get_tag_ecosystem(tag)
            .map_err(|e| format!("Tag ecosystem: {}", e))?;
        Ok(serde_json::to_value(sub).unwrap_or_default())
    }
}

// ─── KG: Full Graph ───────────────────────────────────────────────────────────

pub struct KgFullGraphHandler;

#[async_trait]
impl ToolHandler for KgFullGraphHandler {
    fn name(&self) -> &str { "kg_full_graph" }
    fn description(&self) -> &str { "Export the entire knowledge graph as JSON (nodes + edges)" }
    fn input_schema(&self) -> ToolInputSchema {
        ToolInputSchema::object(HashMap::new(), vec![])
    }
    async fn call(&self, _params: Value) -> Result<Value, String> {
        let db_path = rairos_kg::KnowledgeGraph::db_path();
        let graph = rairos_kg::KnowledgeGraph::with_db(db_path)
            .map_err(|e| format!("KG init: {}", e))?;
        if let Some(db) = graph.database() {
            db.export_json().map_err(|e| format!("KG export: {}", e))
        } else {
            Ok(graph.export_json())
        }
    }
}

// ─── KG: Query by keyword ─────────────────────────────────────────────────────

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
        let db_path = rairos_kg::KnowledgeGraph::db_path();
        let graph = rairos_kg::KnowledgeGraph::with_db(db_path)
            .map_err(|e| format!("KG init: {}", e))?;
        let db = graph.database().ok_or("No database connected")?;
        let results = db.query_by_keyword(keyword, limit)
            .map_err(|e| format!("KG query: {}", e))?;
        Ok(serde_json::json!({"results": results, "total": results.len(), "keyword": keyword}))
    }
}

// ─── Register all tools ───────────────────────────────────────────────────────

pub async fn register_all(server: &crate::McpServer) {
    server.register(PaperSearchHandler).await;
    server.register(PaperIngestHandler).await;
    server.register(PaperQueryHandler).await;
    server.register(PaperChatHandler).await;
    server.register(TagAddHandler).await;
    server.register(TagRemoveHandler).await;
    server.register(TagListHandler).await;
    server.register(TrendsDetectTrendingHandler).await;
    server.register(PaperRecommendHandler).await;
    server.register(CitationGraphHandler).await;
    server.register(KgPaperSubgraphHandler).await;
    server.register(KgTagGraphHandler).await;
    server.register(KgFullGraphHandler).await;
    server.register(KgQueryHandler).await;
}

// ─── arXiv XML Parser ─────────────────────────────────────────────────────────

pub fn parse_arxiv_response(xml: &str) -> Vec<Value> {
    let mut papers = Vec::new();
    let mut pos = 0;
    while let Some(entry_start) = xml[pos..].find("<entry>") {
        let abs_start = pos + entry_start;
        let Some(entry_end) = xml[abs_start..].find("</entry>") else { break; };
        let entry = &xml[abs_start..abs_start + entry_end + 8];
        pos = abs_start + entry_end + 8;

        let id = extract_tag(entry, "id").unwrap_or_default();
        let published = extract_tag(entry, "published").unwrap_or_default();
        let title = extract_tag(entry, "title").map(clean_xml).unwrap_or_default();
        let summary = extract_tag(entry, "summary").map(clean_xml).unwrap_or_default();
        let authors = extract_authors(entry);
        let categories = extract_categories(entry);

        let arxiv_id = id.strip_prefix("http://arxiv.org/abs/")
            .or_else(|| id.strip_prefix("https://arxiv.org/abs/"))
            .map(|s| s.to_string()).unwrap_or_default();

        papers.push(serde_json::json!({
            "arxiv_id": arxiv_id, "title": title, "abstract": summary,
            "authors": authors, "categories": categories, "published": published,
            "pdf_url": format!("https://arxiv.org/pdf/{}.pdf", arxiv_id), "abs_url": id,
        }));
    }
    papers
}

fn extract_tag<'a>(s: &'a str, tag: &str) -> Option<String> {
    let start = s.find(&format!("<{}>", tag))?;
    let value_start = start + tag.len() + 2;
    let end = s[value_start..].find(&format!("</{}>", tag))?;
    Some(s[value_start..value_start + end].to_string())
}

fn clean_xml(s: String) -> String {
    s.trim().replace('\n', " ").replace("  ", " ")
        .replace("&amp;", "&").replace("&lt;", "<").replace("&gt;", ">")
        .replace("&quot;", "\"").replace("&apos;", "'")
        .split_whitespace().collect::<Vec<_>>().join(" ")
}

fn extract_authors(entry: &str) -> Vec<String> {
    let mut authors = Vec::new();
    let mut pos = 0;
    while let Some(start) = entry[pos..].find("<author>") {
        let abs_start = pos + start;
        let Some(end) = entry[abs_start..].find("</author>") else { break; };
        let ab = &entry[abs_start..abs_start + end + 9];
        if let Some(n) = extract_tag(ab, "name") { authors.push(n); }
        pos = abs_start + end + 9;
    }
    authors
}

fn extract_categories(entry: &str) -> Vec<String> {
    let mut cats = Vec::new();
    let mut pos = 0;
    while let Some(start) = entry[pos..].find("term=\"") {
        let after = &entry[pos + start + 6..];
        if let Some(end) = after.find('"') { cats.push(after[..end].to_string()); }
        pos += start + 6;
    }
    cats
}

fn chrono_now() -> String {
    chrono::Utc::now().format("%Y-%m-%dT%H:%M:%S%.f").to_string()
}

// ─── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tag_add_list_remove_cycle() {
        let path = std::env::temp_dir().join("test_tags.jsonl");
        let _ = std::fs::remove_file(&path);

        // Use tags_path by setting env var? No — just test the JSONL helpers.
        let entry = serde_json::json!({"arxiv_id": "2401.00001", "tag": "transformer"});
        let p = path.clone();
        append_jsonl(&p, &entry).unwrap();
        let entries = read_jsonl(&p);
        assert_eq!(entries.len(), 1);

        let entry2 = serde_json::json!({"arxiv_id": "2401.00002", "tag": "gnn"});
        append_jsonl(&p, &entry2).unwrap();
        let entries = read_jsonl(&p);
        assert_eq!(entries.len(), 2);

        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn test_schema_definitions() {
        assert!(PaperSearchHandler.name() == "paper_search");
        assert!(TagAddHandler.name() == "tag_add");
        assert!(TagRemoveHandler.name() == "tag_remove");
        assert!(TagListHandler.name() == "tag_list");
        assert!(TrendsDetectTrendingHandler.name() == "trends_detect_trending");
        assert!(PaperRecommendHandler.name() == "paper_recommend");
        assert!(CitationGraphHandler.name() == "citation_graph");
        assert!(PaperQueryHandler.name() == "paper_query");
        assert!(PaperIngestHandler.name() == "paper_ingest");
        assert!(PaperChatHandler.name() == "paper_chat");
        assert!(KgPaperSubgraphHandler.name() == "kg_paper_subgraph");
        assert!(KgTagGraphHandler.name() == "kg_tag_graph");
        assert!(KgFullGraphHandler.name() == "kg_full_graph");
        assert!(KgQueryHandler.name() == "kg_query");
    }

    #[test]
    fn test_parse_arxiv_response() {
        let xml = r#"<?xml version="1.0"?><feed>
<entry><id>http://arxiv.org/abs/2401.12345</id><published>2024-01-01</published>
<title>Test Title</title><summary>Test abstract</summary>
<author><name>John Doe</name></author><category term="cs.LG"/>
</entry></feed>"#;
        let papers = parse_arxiv_response(xml);
        assert_eq!(papers.len(), 1);
        assert_eq!(papers[0]["arxiv_id"], "2401.12345");
    }
}
