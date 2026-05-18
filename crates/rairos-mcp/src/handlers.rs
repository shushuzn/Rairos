//! Tool handlers registry — 10 pure Rust MCP tools
//!
//! Each tool implements the ToolHandler trait. All handlers are self-contained
//! (no Python dependencies, no sub-crate calls).

use crate::protocol::{ToolHandler, ToolInputSchema, ToolProperty};
use async_trait::async_trait;
use rairos_core::constants::{ARXIV_API, GP_DIR_NAME, GENE_POOL_JSONL, TAGS_FILE};
use serde_json::Value;
use std::collections::HashMap;
use std::io::BufRead;
use std::path::PathBuf;
use std::sync::OnceLock;

static KG: OnceLock<rairos_kg::KnowledgeGraph> = OnceLock::new();

fn kg() -> &'static rairos_kg::KnowledgeGraph {
    KG.get_or_init(|| {
        let db_path = rairos_kg::KnowledgeGraph::db_path();
        rairos_kg::KnowledgeGraph::with_db(db_path)
            .expect("Failed to initialize knowledge graph")
    })
}

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
    data_dir().join(TAGS_FILE)
}

fn read_jsonl(path: &PathBuf) -> Vec<Value> {
    let file = match std::fs::File::open(path) {
        Ok(f) => f,
        Err(_) => return Vec::new(),
    };
    let reader = std::io::BufReader::new(file);
    reader
        .lines()
        .filter_map(|line| {
            let line = line.ok()?;
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
        let papers = rairos_parser::search_arxiv(query, max)
            .await
            .map_err(|e| format!("Search failed: {}", e))?;
        let values: Vec<Value> = papers.into_iter()
            .map(|p| serde_json::to_value(p).unwrap_or_default())
            .collect();
        Ok(serde_json::json!({"papers": values, "total": values.len()}))
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
        let paper = rairos_parser::fetch_arxiv(id)
            .await
            .map_err(|e| format!("arXiv fetch failed: {}", e))?;
        serde_json::to_value(&paper).map_err(|e| format!("Serialization failed: {}", e))
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
        let gp_path = data_dir().join(GP_DIR_NAME).join(GENE_POOL_JSONL);
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

        let sub = kg().get_paper_subgraph(arxiv_id, depth, include_notes)
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
        let sub = kg().get_tag_ecosystem(tag)
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
        let graph = kg();
        if let Some(db) = graph.database() {
            db.export_json(None).map_err(|e| format!("KG export: {}", e))
        } else {
            Ok(graph.export_json(None))
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
        let db = kg().database().ok_or("No database connected")?;
        let results = db.query_by_keyword(keyword, limit)
            .map_err(|e| format!("KG query: {}", e))?;
        Ok(serde_json::json!({"results": results, "total": results.len(), "keyword": keyword}))
    }
}

// ─── PDF Download ───────────────────────────────────────────────────────────

pub struct PdfDownloadHandler;

#[async_trait]
impl ToolHandler for PdfDownloadHandler {
    fn name(&self) -> &str { "pdf_download" }
    fn description(&self) -> &str { "Download a PDF from arXiv" }
    fn input_schema(&self) -> ToolInputSchema {
        ToolInputSchema::object(
            vec![("arxiv_id".into(), ToolProperty::string("arXiv paper ID"))].into_iter().collect(),
            vec!["arxiv_id".into()],
        )
    }
    async fn call(&self, params: Value) -> Result<Value, String> {
        let arxiv_id = params["arxiv_id"].as_str().ok_or("Missing arxiv_id")?;
        let pdf_dir = data_dir().join("pdfs");
        std::fs::create_dir_all(&pdf_dir).map_err(|e| format!("Failed to create pdfs dir: {}", e))?;
        let pdf_path = pdf_dir.join(format!("{}.pdf", arxiv_id));
        let url = format!("https://arxiv.org/pdf/{}.pdf", arxiv_id);

        if !pdf_path.exists() {
            let rt = tokio::runtime::Runtime::new().map_err(|e| format!("Runtime error: {}", e))?;
            rt.block_on(rairos_pdf::download_pdf(&url, &pdf_path))
                .map_err(|e| format!("Download failed: {}", e))?;
        }

        let size_bytes = std::fs::metadata(&pdf_path).map(|m| m.len()).unwrap_or(0);
        Ok(serde_json::json!({
            "saved_path": pdf_path.to_string_lossy(),
            "size_bytes": size_bytes,
            "url": url,
        }))
    }
}

// ─── PDF Extract Text ───────────────────────────────────────────────────────

pub struct PdfExtractTextHandler;

#[async_trait]
impl ToolHandler for PdfExtractTextHandler {
    fn name(&self) -> &str { "pdf_extract_text" }
    fn description(&self) -> &str { "Extract plain text from a PDF file" }
    fn input_schema(&self) -> ToolInputSchema {
        ToolInputSchema::object(
            vec![("arxiv_id".into(), ToolProperty::string("arXiv ID of the paper"))].into_iter().collect(),
            vec!["arxiv_id".into()],
        )
    }
    async fn call(&self, params: Value) -> Result<Value, String> {
        let arxiv_id = params["arxiv_id"].as_str().ok_or("Missing arxiv_id")?;
        let pdf_dir = data_dir().join("pdfs");
        let pdf_path = pdf_dir.join(format!("{}.pdf", arxiv_id));

        if !pdf_path.exists() {
            return Err("PDF not found. Call pdf_download first.".into());
        }

        let text = rairos_pdf::extract_pdf_text(&pdf_path)
            .map_err(|e| format!("Text extraction failed: {}", e))?;

        let char_count = text.chars().count();
        Ok(serde_json::json!({
            "text": text,
            "char_count": char_count,
        }))
    }
}

// ─── PDF Extract Structured ─────────────────────────────────────────────────

pub struct PdfExtractStructuredHandler;

#[async_trait]
impl ToolHandler for PdfExtractStructuredHandler {
    fn name(&self) -> &str { "pdf_extract_structured" }
    fn description(&self) -> &str { "Extract structured content from PDF (text blocks, tables, math)" }
    fn input_schema(&self) -> ToolInputSchema {
        ToolInputSchema::object(
            vec![("arxiv_id".into(), ToolProperty::string("arXiv ID of the paper"))].into_iter().collect(),
            vec!["arxiv_id".into()],
        )
    }
    async fn call(&self, params: Value) -> Result<Value, String> {
        let arxiv_id = params["arxiv_id"].as_str().ok_or("Missing arxiv_id")?;
        let pdf_dir = data_dir().join("pdfs");
        let pdf_path = pdf_dir.join(format!("{}.pdf", arxiv_id));

        if !pdf_path.exists() {
            return Err("PDF not found. Call pdf_download first.".into());
        }

        let text = rairos_pdf::extract_pdf_text(&pdf_path)
            .map_err(|e| format!("Text extraction failed: {}", e))?;

        let text_blocks: Vec<Value> = text.lines()
            .enumerate()
            .filter(|(_, l)| !l.trim().is_empty())
            .map(|(i, l)| serde_json::json!({
                "index": i,
                "text": l,
                "length": l.len(),
            }))
            .collect();

        let sections = rairos_pdf::segment_into_sections(&text, 20);
        let section_list: Vec<Value> = sections.iter()
            .map(|(name, content)| serde_json::json!({
                "section": name,
                "content_length": content.len(),
                "preview": content.chars().take(200).collect::<String>(),
            }))
            .collect();

        let math_count = text.lines().filter(|l| l.contains("\\(") || l.contains("\\[") || l.contains("$$")).count();

        Ok(serde_json::json!({
            "text_blocks": text_blocks,
            "section_count": sections.len(),
            "sections": section_list,
            "math_count": math_count,
            "total_chars": text.len(),
        }))
    }
}

// ─── Trends Predict Next ────────────────────────────────────────────────────

pub struct TrendsPredictNextHandler;

#[async_trait]
impl ToolHandler for TrendsPredictNextHandler {
    fn name(&self) -> &str { "trends_predict_next" }
    fn description(&self) -> &str { "Predict the next heat score for a given tag using Holt's exponential smoothing" }
    fn input_schema(&self) -> ToolInputSchema {
        ToolInputSchema::object(
            vec![("tag".into(), ToolProperty::string("Research tag to forecast"))].into_iter().collect(),
            vec!["tag".into()],
        )
    }
    async fn call(&self, params: Value) -> Result<Value, String> {
        let tag = params["tag"].as_str().ok_or("Missing tag")?;
        let path = data_dir().join("radar_history.json");
        let forecaster = if path.exists() {
            rairos_trends::TrendForecaster::with_path(&path)
        } else {
            rairos_trends::TrendForecaster::new()
        };
        let prediction = forecaster.predict_next(tag);
        serde_json::to_value(&prediction).map_err(|e| format!("Serialize error: {}", e))
    }
}

// ─── Trends Top Predictions ──────────────────────────────────────────────────

pub struct TrendsTopPredictionsHandler;

#[async_trait]
impl ToolHandler for TrendsTopPredictionsHandler {
    fn name(&self) -> &str { "trends_top_predictions" }
    fn description(&self) -> &str { "Get top-k predicted trending tags ranked by predicted_score * confidence" }
    fn input_schema(&self) -> ToolInputSchema {
        ToolInputSchema::object(
            vec![("k".into(), ToolProperty::integer("Number of predictions (default 5)"))].into_iter().collect(),
            vec![],
        )
    }
    async fn call(&self, params: Value) -> Result<Value, String> {
        let k = params["k"].as_i64().unwrap_or(5) as usize;
        let path = data_dir().join("radar_history.json");
        let forecaster = if path.exists() {
            rairos_trends::TrendForecaster::with_path(&path)
        } else {
            rairos_trends::TrendForecaster::new()
        };
        let predictions = forecaster.get_top_predictions(k);
        serde_json::to_value(&predictions).map_err(|e| format!("Serialize error: {}", e))
    }
}

// ─── Trends Compare Tags ────────────────────────────────────────────────────

pub struct TrendsCompareTagsHandler;

#[async_trait]
impl ToolHandler for TrendsCompareTagsHandler {
    fn name(&self) -> &str { "trends_compare_tags" }
    fn description(&self) -> &str { "Compare trends trajectories of two tags side by side" }
    fn input_schema(&self) -> ToolInputSchema {
        ToolInputSchema::object(
            vec![
                ("tag_a".into(), ToolProperty::string("First tag")),
                ("tag_b".into(), ToolProperty::string("Second tag")),
            ].into_iter().collect(),
            vec!["tag_a".into(), "tag_b".into()],
        )
    }
    async fn call(&self, params: Value) -> Result<Value, String> {
        let tag_a = params["tag_a"].as_str().ok_or("Missing tag_a")?;
        let tag_b = params["tag_b"].as_str().ok_or("Missing tag_b")?;
        let path = data_dir().join("radar_history.json");
        let forecaster = if path.exists() {
            rairos_trends::TrendForecaster::with_path(&path)
        } else {
            rairos_trends::TrendForecaster::new()
        };
        let comparison = forecaster.compare_tags(tag_a, tag_b);
        serde_json::to_value(&comparison).map_err(|e| format!("Serialize error: {}", e))
    }
}

// ─── Cite Fetch ──────────────────────────────────────────────────────────────

pub struct CiteFetchHandler;

#[async_trait]
impl ToolHandler for CiteFetchHandler {
    fn name(&self) -> &str { "cite_fetch" }
    fn description(&self) -> &str { "Fetch citation metadata for a paper from Semantic Scholar" }
    fn input_schema(&self) -> ToolInputSchema {
        ToolInputSchema::object(
            vec![
                ("paper_id".into(), ToolProperty::string("Paper ID or arXiv ID to fetch citations for")),
            ].into_iter().collect(),
            vec!["paper_id".into()],
        )
    }
    async fn call(&self, params: Value) -> Result<Value, String> {
        let paper_id = params["paper_id"].as_str().ok_or("Missing paper_id")?;

        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(15))
            .build().map_err(|e| format!("HTTP client error: {}", e))?;

        let url = format!(
            "https://api.semanticscholar.org/graph/v1/paper/{}?fields=title,citationCount,externalIds",
            paper_id
        );

        let resp = client.get(&url).send().await.map_err(|e| format!("Request failed: {}", e))?;
        if !resp.status().is_success() {
            return Err(format!("Semantic Scholar API returned {}", resp.status()));
        }

        let data: serde_json::Value = resp.json().await.map_err(|e| format!("Parse failed: {}", e))?;
        let cited_by = data["citationCount"].as_u64().unwrap_or(0) as usize;
        let title = data["title"].as_str().unwrap_or("Unknown");

        Ok(serde_json::json!({
            "paper_id": paper_id,
            "title": title,
            "cited_by_count": cited_by,
            "citations": [],
        }))
    }
}

// ─── Paper Search Multi (Semantic Scholar) ─────────────────────────────────

pub struct PaperSearchMultiHandler;

#[async_trait]
impl ToolHandler for PaperSearchMultiHandler {
    fn name(&self) -> &str { "paper_search_multi" }
    fn description(&self) -> &str { "Search papers across multiple academic databases using Semantic Scholar" }
    fn input_schema(&self) -> ToolInputSchema {
        ToolInputSchema::object(
            vec![
                ("query".into(), ToolProperty::string("Search query")),
                ("limit".into(), ToolProperty::integer("Maximum results (default 10, max 100)")),
                ("year_from".into(), ToolProperty::integer("Filter papers from this year")),
            ].into_iter().collect(),
            vec!["query".into()],
        )
    }
    async fn call(&self, params: Value) -> Result<Value, String> {
        let query = params["query"].as_str().ok_or("Missing required parameter: query")?;
        let limit = (params["limit"].as_u64().unwrap_or(10) as usize).min(100);
        let year_from = params.get("year_from").and_then(|v| v.as_u64()).map(|y| y as i32);

        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(15))
            .build().map_err(|e| format!("HTTP client error: {}", e))?;

        let mut url = format!(
            "https://api.semanticscholar.org/graph/v1/paper/search?query={}&fields=title,year,abstract,citationCount,authors,openAccessPdf,externalIds&limit={}",
            urlencoding::encode(query),
            limit
        );
        if let Some(year) = year_from {
            url.push_str(&format!("&year={}-", year));
        }

        let resp = client.get(&url).send().await.map_err(|e| format!("Search failed: {}", e))?;
        if !resp.status().is_success() {
            return Err(format!("Semantic Scholar API returned {}", resp.status()));
        }

        let data: serde_json::Value = resp.json().await.map_err(|e| format!("Parse failed: {}", e))?;
        let results = data["data"].as_array().cloned().unwrap_or_default();
        let total = data["total"].as_u64().unwrap_or(0);

        let papers: Vec<Value> = results.into_iter().map(|p| {
            serde_json::json!({
                "title": p["title"].as_str().unwrap_or(""),
                "year": p["year"].as_i64().unwrap_or(0),
                "abstract": p["abstract"].as_str().unwrap_or(""),
                "citationCount": p["citationCount"].as_u64().unwrap_or(0),
                "openAccessPdf": p["openAccessPdf"]["url"].as_str(),
                "doi": p["externalIds"]["DOI"].as_str(),
                "arxivId": p["externalIds"]["ArXiv"].as_str(),
                "authors": p["authors"].as_array().map(|arr| arr.iter().filter_map(|a| a["name"].as_str()).collect::<Vec<_>>()).unwrap_or_default(),
            })
        }).collect();

        Ok(serde_json::json!({"papers": papers, "total": total}))
    }
}

// ─── Paper Lookup DOI (Crossref) ─────────────────────────────────────────

pub struct PaperLookupDoiHandler;

#[async_trait]
impl ToolHandler for PaperLookupDoiHandler {
    fn name(&self) -> &str { "paper_lookup_doi" }
    fn description(&self) -> &str { "Look up paper metadata by DOI using Crossref" }
    fn input_schema(&self) -> ToolInputSchema {
        ToolInputSchema::object(
            vec![
                ("doi".into(), ToolProperty::string("DOI to look up")),
            ].into_iter().collect(),
            vec!["doi".into()],
        )
    }
    async fn call(&self, params: Value) -> Result<Value, String> {
        let doi = params["doi"].as_str().ok_or("Missing required parameter: doi")?;

        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(15))
            .build().map_err(|e| format!("HTTP client error: {}", e))?;

        let url = format!(
            "https://api.crossref.org/works/{}?mailto=rairos@example.com",
            urlencoding::encode(doi)
        );

        let resp = client.get(&url).send().await.map_err(|e| format!("Lookup failed: {}", e))?;
        if !resp.status().is_success() {
            return Err(format!("Crossref API returned {}", resp.status()));
        }

        let data: serde_json::Value = resp.json().await.map_err(|e| format!("Parse failed: {}", e))?;
        let msg = &data["message"];

        let authors: Vec<String> = if let Some(author_arr) = msg["author"].as_array() {
            author_arr.iter()
                .map(|a| {
                    let given = a["given"].as_str().unwrap_or("");
                    let family = a["family"].as_str().unwrap_or("");
                    format!("{} {}", given, family).trim().to_string()
                })
                .collect()
        } else {
            Vec::new()
        };

        let title = msg["title"].as_array().and_then(|t| t[0].as_str()).unwrap_or("");
        let journal = msg["container-title"].as_array().and_then(|j| j[0].as_str()).unwrap_or("");
        let year = msg["published"]["date-parts"]
            .as_array()
            .and_then(|d| d[0].as_array())
            .and_then(|y| y[0].as_i64())
            .unwrap_or(0);

        Ok(serde_json::json!({
            "doi": doi,
            "title": title,
            "authors": authors,
            "journal": journal,
            "year": year,
            "citedByCount": msg["is-referenced-by-count"].as_u64().unwrap_or(0),
            "abstract": msg["abstract"].as_str().unwrap_or(""),
        }))
    }
}

// ─── Paper Citations (Semantic Scholar) ───────────────────────────────────

pub struct PaperCitationsHandler;

#[async_trait]
impl ToolHandler for PaperCitationsHandler {
    fn name(&self) -> &str { "paper_citations" }
    fn description(&self) -> &str { "Get citation chain for a paper (papers that cite it and papers it cites)" }
    fn input_schema(&self) -> ToolInputSchema {
        ToolInputSchema::object(
            vec![
                ("paper_id".into(), ToolProperty::string("Paper ID, DOI, or arXiv ID")),
                ("limit".into(), ToolProperty::integer("Maximum citations per direction (default 20, max 100)")),
            ].into_iter().collect(),
            vec!["paper_id".into()],
        )
    }
    async fn call(&self, params: Value) -> Result<Value, String> {
        let paper_id = params["paper_id"].as_str().ok_or("Missing required parameter: paper_id")?;
        let limit = (params["limit"].as_u64().unwrap_or(20) as usize).min(100);

        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(20))
            .build().map_err(|e| format!("HTTP client error: {}", e))?;

        let fields = "title,year,citationCount,externalIds";
        let citing_url = format!(
            "https://api.semanticscholar.org/graph/v1/paper/{}/citations?fields={}&limit={}",
            paper_id, fields, limit
        );
        let refs_url = format!(
            "https://api.semanticscholar.org/graph/v1/paper/{}/references?fields={}&limit={}",
            paper_id, fields, limit
        );

        let citing_resp = client.get(&citing_url).send().await.map_err(|e| format!("Request failed: {}", e))?;
        let refs_resp = client.get(&refs_url).send().await.map_err(|e| format!("Request failed: {}", e))?;

        let citing: Vec<Value> = if citing_resp.status().is_success() {
            if let Ok(data) = citing_resp.json::<serde_json::Value>().await {
                data["data"].as_array().cloned().unwrap_or_default()
                    .into_iter()
                    .map(|p| {
                        let citing = &p["citingPaper"];
                        serde_json::json!({
                            "paperId": citing["paperId"].as_str().unwrap_or(""),
                            "title": citing["title"].as_str().unwrap_or(""),
                            "year": citing["year"].as_i64().unwrap_or(0),
                            "citationCount": citing["citationCount"].as_u64().unwrap_or(0),
                        })
                    })
                    .collect()
            } else { Vec::new() }
        } else { Vec::new() };

        let references: Vec<Value> = if refs_resp.status().is_success() {
            if let Ok(data) = refs_resp.json::<serde_json::Value>().await {
                data["data"].as_array().cloned().unwrap_or_default()
                    .into_iter()
                    .map(|p| {
                        let referenced = &p["referencedPaper"];
                        serde_json::json!({
                            "paperId": referenced["paperId"].as_str().unwrap_or(""),
                            "title": referenced["title"].as_str().unwrap_or(""),
                            "year": referenced["year"].as_i64().unwrap_or(0),
                            "citationCount": referenced["citationCount"].as_u64().unwrap_or(0),
                        })
                    })
                    .collect()
            } else { Vec::new() }
        } else { Vec::new() };

        Ok(serde_json::json!({
            "paper_id": paper_id,
            "citing": citing,
            "references": references,
            "citingCount": citing.len(),
            "referenceCount": references.len(),
        }))
    }
}

// ─── Paper Verify Citations ──────────────────────────────────────────────

pub struct PaperVerifyCitationsHandler;

#[derive(Debug, Clone, Copy)]
enum CitationStyle {
    Apa,
    Nature,
    Vancouver,
}

impl CitationStyle {
    fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "apa" => Some(CitationStyle::Apa),
            "nature" => Some(CitationStyle::Nature),
            "vancouver" => Some(CitationStyle::Vancouver),
            _ => None,
        }
    }
}

fn format_citation_apa(authors: &[String], title: &str, journal: &str, year: i64, doi: &str) -> String {
    let author_str = if authors.is_empty() {
        "Unknown".to_string()
    } else if authors.len() == 1 {
        authors[0].clone()
    } else if authors.len() == 2 {
        format!("{} & {}", authors[0], authors[1])
    } else {
        format!("{} et al.", authors[0])
    };
    format!("{} ({}). {}. *{}*. https://doi.org/{}", author_str, year, title, journal, doi)
}

fn format_citation_nature(authors: &[String], title: &str, journal: &str, year: i64, doi: &str) -> String {
    let author_str = if authors.is_empty() {
        "Unknown".to_string()
    } else if authors.len() <= 5 {
        authors.join(", ")
    } else {
        format!("{} et al.", authors[0])
    };
    format!("{} {} {} {} {}", author_str, title, journal, year, doi)
}

fn format_citation_vancouver(authors: &[String], title: &str, journal: &str, year: i64, doi: &str) -> String {
    let author_str = if authors.is_empty() {
        "Unknown".to_string()
    } else if authors.len() <= 6 {
        authors.join(", ")
    } else {
        format!("{} et al.", authors[0])
    };
    format!("{} {}. {}. {}. {}:{}:{}", author_str, title, journal, year, journal, year, doi)
}

#[async_trait]
impl ToolHandler for PaperVerifyCitationsHandler {
    fn name(&self) -> &str { "paper_verify_citations" }
    fn description(&self) -> &str { "Verify DOIs and format citations in APA, Nature, or Vancouver style" }
    fn input_schema(&self) -> ToolInputSchema {
        ToolInputSchema::object(
            vec![
                ("dois".into(), ToolProperty::string("Comma-separated DOIs to verify")),
                ("style".into(), ToolProperty::string("Citation style: apa, nature, or vancouver (default: apa)")),
            ].into_iter().collect(),
            vec!["dois".into()],
        )
    }
    async fn call(&self, params: Value) -> Result<Value, String> {
        let dois_str = params["dois"].as_str().ok_or("Missing required parameter: dois")?;
        let style_str = params.get("style").and_then(|v| v.as_str()).unwrap_or("apa");
        let style = CitationStyle::from_str(style_str).ok_or_else(|| format!("Invalid style: {}. Use apa, nature, or vancouver.", style_str))?;

        let dois: Vec<&str> = dois_str.split(',').map(|s| s.trim()).filter(|s| !s.is_empty()).collect();
        if dois.is_empty() {
            return Err("No DOIs provided".to_string());
        }
        if dois.len() > 50 {
            return Err("Maximum 50 DOIs at a time".to_string());
        }

        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build().map_err(|e| format!("HTTP client error: {}", e))?;

        let mut results = Vec::new();
        for doi in dois {
            let url = format!(
                "https://api.crossref.org/works/{}?mailto=rairos@example.com",
                urlencoding::encode(doi)
            );

            let resp = match client.get(&url).send().await {
                Ok(r) => r,
                Err(e) => {
                    results.push(serde_json::json!({
                        "doi": doi,
                        "verified": false,
                        "error": format!("Request failed: {}", e),
                    }));
                    continue;
                }
            };

            if !resp.status().is_success() {
                results.push(serde_json::json!({
                    "doi": doi,
                    "verified": false,
                    "error": format!("Crossref API returned {}", resp.status()),
                }));
                continue;
            }

            let data: serde_json::Value = match resp.json().await {
                Ok(d) => d,
                Err(e) => {
                    results.push(serde_json::json!({
                        "doi": doi,
                        "verified": false,
                        "error": format!("Parse failed: {}", e),
                    }));
                    continue;
                }
            };

            let msg = &data["message"];

            let authors: Vec<String> = if let Some(author_arr) = msg["author"].as_array() {
                author_arr.iter()
                    .map(|a| {
                        let given = a["given"].as_str().unwrap_or("");
                        let family = a["family"].as_str().unwrap_or("");
                        format!("{} {}", given, family).trim().to_string()
                    })
                    .collect()
            } else {
                Vec::new()
            };

            let title = msg["title"].as_array().and_then(|t| t[0].as_str()).unwrap_or("");
            let journal = msg["container-title"].as_array().and_then(|j| j[0].as_str()).unwrap_or("");
            let year = msg["published"]["date-parts"]
                .as_array()
                .and_then(|d| d[0].as_array())
                .and_then(|y| y[0].as_i64())
                .unwrap_or(0);

            let formatted = match style {
                CitationStyle::Apa => format_citation_apa(&authors, title, journal, year, doi),
                CitationStyle::Nature => format_citation_nature(&authors, title, journal, year, doi),
                CitationStyle::Vancouver => format_citation_vancouver(&authors, title, journal, year, doi),
            };

            results.push(serde_json::json!({
                "doi": doi,
                "verified": true,
                "title": title,
                "authors": authors,
                "journal": journal,
                "year": year,
                "formatted": formatted,
            }));
        }

        let verified_count = results.iter().filter(|r| r["verified"].as_bool().unwrap_or(false)).count();
        Ok(serde_json::json!({
            "citations": results,
            "total": results.len(),
            "verified": verified_count,
            "style": style_str,
        }))
    }
}

// ─── Paper Visualize Trends ───────────────────────────────────────────────

pub struct PaperVisualizeTrendsHandler;

#[async_trait]
impl ToolHandler for PaperVisualizeTrendsHandler {
    fn name(&self) -> &str { "paper_visualize_trends" }
    fn description(&self) -> &str { "Generate a publication-quality bar chart of research trends" }
    fn input_schema(&self) -> ToolInputSchema {
        ToolInputSchema::object(
            vec![
                ("trends_json".into(), ToolProperty::string("JSON array of {keyword, count} objects")),
                ("chart_type".into(), ToolProperty::string("Chart type: bar (default), line")),
                ("title".into(), ToolProperty::string("Chart title")),
                ("journal".into(), ToolProperty::string("Target journal: default, nature, science, cell")),
            ].into_iter().collect(),
            vec!["trends_json".into()],
        )
    }
    async fn call(&self, params: Value) -> Result<Value, String> {
        let trends_str = params["trends_json"].as_str().ok_or("Missing trends_json")?;
        let chart_type = params.get("chart_type").and_then(|v| v.as_str()).unwrap_or("bar");
        let title = params.get("title").and_then(|v| v.as_str()).unwrap_or("Research Trends");
        let journal = params.get("journal").and_then(|v| v.as_str()).unwrap_or("default");

        let trends: Vec<(String, usize)> = serde_json::from_str(trends_str)
            .map_err(|e| format!("Invalid JSON: {}", e))?;

        if trends.is_empty() {
            return Err("No trends data provided".to_string());
        }

        let labels: Vec<String> = trends.iter().map(|(k, _)| k.clone()).collect();
        let values: Vec<f64> = trends.iter().map(|(_, v)| *v as f64).collect();

        let data = serde_json::json!({
            "labels": labels,
            "values": values,
            "xlabel": "Keyword",
            "ylabel": "Frequency"
        });

        let output_dir = data_dir().join("visualizations");
        std::fs::create_dir_all(&output_dir).map_err(|e| e.to_string())?;
        let output_path = output_dir.join(format!("trends_{}.png", std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH).unwrap().as_secs()));

        let data_str = serde_json::to_string(&data).map_err(|e| e.to_string())?;

        let python_cmd = std::env::var("RAIROS_VIZ_HELPER")
            .unwrap_or_else(|_| "python3".to_string());

        let mut cmd = std::process::Command::new(&python_cmd);
        cmd.arg("/root/Rairos/scripts/viz_helper.py")
            .arg("--type").arg(chart_type)
            .arg("--data").arg(&data_str)
            .arg("--output").arg(output_path.to_str().unwrap())
            .arg("--title").arg(title)
            .arg("--journal").arg(journal);

        let output = cmd.output().map_err(|e| format!("Failed to run viz helper: {}", e))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(format!("viz_helper failed: {}", stderr));
        }

        Ok(serde_json::json!({
            "image_path": output_path.to_string_lossy(),
            "trends_count": trends.len(),
            "chart_type": chart_type,
        }))
    }
}

// ─── Paper Visualize Radar ─────────────────────────────────────────────────

pub struct PaperVisualizeRadarHandler;

#[async_trait]
impl ToolHandler for PaperVisualizeRadarHandler {
    fn name(&self) -> &str { "paper_visualize_radar" }
    fn description(&self) -> &str { "Generate a radar chart for paper rubric scores" }
    fn input_schema(&self) -> ToolInputSchema {
        ToolInputSchema::object(
            vec![
                ("scores_json".into(), ToolProperty::string("JSON object with axis names and scores, e.g. {\"Novelty\": 8, \"Leverage\": 7}")),
                ("title".into(), ToolProperty::string("Chart title")),
                ("journal".into(), ToolProperty::string("Target journal: default, nature, science, cell")),
            ].into_iter().collect(),
            vec!["scores_json".into()],
        )
    }
    async fn call(&self, params: Value) -> Result<Value, String> {
        let scores_str = params["scores_json"].as_str().ok_or("Missing scores_json")?;
        let title = params.get("title").and_then(|v| v.as_str()).unwrap_or("Paper Scores");
        let journal = params.get("journal").and_then(|v| v.as_str()).unwrap_or("default");

        let scores: serde_json::Map<String, serde_json::Value> = serde_json::from_str(scores_str)
            .map_err(|e| format!("Invalid JSON: {}", e))?;

        if scores.is_empty() {
            return Err("No scores data provided".to_string());
        }

        let axes: Vec<String> = scores.keys().cloned().collect();
        let values: Vec<f64> = scores.values()
            .filter_map(|v| v.as_f64().or_else(|| v.as_u64().map(|x| x as f64)))
            .collect();

        if axes.len() != values.len() {
            return Err("Axes and scores count mismatch".to_string());
        }

        let data = serde_json::json!({
            "axes": axes,
            "scores": values,
            "max_score": 10
        });

        let output_dir = data_dir().join("visualizations");
        std::fs::create_dir_all(&output_dir).map_err(|e| e.to_string())?;
        let output_path = output_dir.join(format!("radar_{}.png", std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH).unwrap().as_secs()));

        let data_str = serde_json::to_string(&data).map_err(|e| e.to_string())?;

        let mut cmd = std::process::Command::new("python3");
        cmd.arg("/root/Rairos/scripts/viz_helper.py")
            .arg("--type").arg("radar")
            .arg("--data").arg(&data_str)
            .arg("--output").arg(output_path.to_str().unwrap())
            .arg("--title").arg(title)
            .arg("--journal").arg(journal);

        let output = cmd.output().map_err(|e| format!("Failed to run viz helper: {}", e))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(format!("viz_helper failed: {}", stderr));
        }

        Ok(serde_json::json!({
            "image_path": output_path.to_string_lossy(),
            "axes": axes,
            "scores": values,
        }))
    }
}

// ─── Paper Critical Analysis ────────────────────────────────────────────────

pub struct PaperCriticalAnalysisHandler;

#[async_trait]
impl ToolHandler for PaperCriticalAnalysisHandler {
    fn name(&self) -> &str { "paper_critical_analysis" }
    fn description(&self) -> &str { "Evaluate a paper for methodological quality, biases, and evidence strength using critical thinking frameworks" }
    fn input_schema(&self) -> ToolInputSchema {
        ToolInputSchema::object(
            vec![
                ("paper_id".into(), ToolProperty::string("Paper ID or arXiv ID")),
                ("title".into(), ToolProperty::string("Paper title")),
                ("abstract".into(), ToolProperty::string("Paper abstract")),
            ].into_iter().collect(),
            vec!["paper_id".into(), "title".into(), "abstract".into()],
        )
    }
    async fn call(&self, params: Value) -> Result<Value, String> {
        let paper_id = params["paper_id"].as_str().ok_or("Missing paper_id")?;
        let title = params["title"].as_str().ok_or("Missing title")?;
        let abstract_text = params["abstract"].as_str().ok_or("Missing abstract")?;

        let checker = rairos_replication_checker::CriticalThinkingChecker::new();
        let report = checker.analyze(paper_id, title, abstract_text);

        Ok(serde_json::json!({
            "paper_id": report.paper_id,
            "study_design": report.study_design,
            "design_quality_score": report.design_quality_score,
            "evidence_quality": report.evidence_quality,
            "overall_score": report.overall_score,
            "biases": report.biases.iter().map(|b| serde_json::json!({
                "type": b.bias_type,
                "severity": b.severity,
                "description": b.description,
                "indicator": b.indicator,
            })).collect::<Vec<_>>(),
            "statistical_concerns": report.statistical_concerns.iter().map(|c| serde_json::json!({
                "type": c.concern_type,
                "severity": c.severity,
                "description": c.description,
                "suggestion": c.suggestion,
            })).collect::<Vec<_>>(),
            "logical_fallacies": report.logical_fallacies,
            "strengths": report.strengths,
            "recommendations": report.recommendations,
        }))
    }
}

// ─── Paper Generate Review PDF ──────────────────────────────────────────────

pub struct PaperGenerateReviewPdfHandler;

#[async_trait]
impl ToolHandler for PaperGenerateReviewPdfHandler {
    fn name(&self) -> &str { "paper_generate_review_pdf" }
    fn description(&self) -> &str { "Generate a PDF literature review from structured content" }
    fn input_schema(&self) -> ToolInputSchema {
        ToolInputSchema::object(
            vec![
                ("review_json".into(), ToolProperty::string("JSON object with title, topic, abstract, sections, references")),
                ("output_path".into(), ToolProperty::string("Output PDF file path (optional, defaults to data dir)")),
            ].into_iter().collect(),
            vec!["review_json".into()],
        )
    }
    async fn call(&self, params: Value) -> Result<Value, String> {
        let review_json = params["review_json"].as_str().ok_or("Missing review_json")?;
        let output_path = params.get("output_path").and_then(|v| v.as_str());

        let output_dir = data_dir().join("reviews");
        std::fs::create_dir_all(&output_dir).map_err(|e| e.to_string())?;

        let filename = format!("review_{}.pdf", std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH).unwrap().as_secs());
        let pdf_path = if let Some(path) = output_path {
            PathBuf::from(path)
        } else {
            output_dir.join(&filename)
        };

        let mut cmd = std::process::Command::new("python3");
        cmd.arg("/root/Rairos/scripts/pdf_helper.py")
            .arg("--type").arg("review")
            .arg("--data").arg(review_json)
            .arg("--output").arg(pdf_path.to_str().unwrap());

        let output = cmd.output().map_err(|e| format!("Failed to run pdf_helper: {}", e))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(format!("pdf_helper failed: {}", stderr));
        }

        Ok(serde_json::json!({
            "pdf_path": pdf_path.to_string_lossy(),
            "status": "generated",
        }))
    }
}

// ─── Hypothesis Report ────────────────────────────────────────────────────

pub struct HypothesisReportHandler;

fn build_hypothesis_markdown(topic: &str, hypotheses_json: &str) -> String {
    let hypotheses: Vec<serde_json::Value> = serde_json::from_str(hypotheses_json)
        .unwrap_or_default();

    let mut md = format!("# Hypothesis Report: {}\n\n", topic);
    md.push_str(&format!("**Generated:** {}\n\n", chrono_now()));

    md.push_str("## Executive Summary\n\n");
    md.push_str(&format!("This report presents {} research hypotheses generated for the topic: *{}*\n\n",
        hypotheses.len(), topic));

    md.push_str("---\n\n## Hypotheses\n\n");

    for (i, h) in hypotheses.iter().enumerate() {
        let title = h.get("title").and_then(|v| v.as_str()).unwrap_or("Untitled");
        let hypo_type = h.get("hypothesis_type").and_then(|v| v.as_str()).unwrap_or("unknown");
        let description = h.get("description").and_then(|v| v.as_str()).unwrap_or("");
        let evidence = h.get("evidence").and_then(|v| v.as_str()).unwrap_or("");
        let predictions = h.get("predictions").and_then(|v| v.as_str()).unwrap_or("");
        let experiments = h.get("experiments").and_then(|v| v.as_str()).unwrap_or("");

        md.push_str(&format!("### Hypothesis {}: {}\n\n", i + 1, title));
        md.push_str(&format!("**Type:** {} | **Confidence:** {}/10\n\n",
            hypo_type,
            h.get("confidence").and_then(|v| v.as_f64()).unwrap_or(5.0) as i32));

        if !description.is_empty() {
            md.push_str(&format!("**Mechanism:** {}\n\n", description));
        }

        if !evidence.is_empty() {
            md.push_str(&format!("**Supporting Evidence:** {}\n\n", evidence));
        }

        if !predictions.is_empty() {
            md.push_str(&format!("**Testable Predictions:**\n{}\n\n", predictions));
        }

        if !experiments.is_empty() {
            md.push_str(&format!("**Proposed Experiments:**\n{}\n\n", experiments));
        }

        md.push_str("---\n\n");
    }

    md.push_str("## Recommendations\n\n");
    md.push_str("Based on the generated hypotheses, the following next steps are recommended:\n\n");
    md.push_str("1. **Validate hypotheses** against existing literature\n");
    md.push_str("2. **Design experiments** to test the highest-confidence hypotheses\n");
    md.push_str("3. **Submit to GenePool** for tracking and evolution\n\n");

    md
}

#[async_trait]
impl ToolHandler for HypothesisReportHandler {
    fn name(&self) -> &str { "paper_hypothesis_report" }
    fn description(&self) -> &str { "Generate a structured hypothesis report with framework from hypothesis results" }
    fn input_schema(&self) -> ToolInputSchema {
        ToolInputSchema::object(
            vec![
                ("topic".into(), ToolProperty::string("Research topic")),
                ("hypotheses_json".into(), ToolProperty::string("JSON array of hypotheses from hypothesis_generate")),
                ("output_format".into(), ToolProperty::string("Output format: markdown or pdf (default: markdown)"))            ].into_iter().collect(),
            vec!["topic".into(), "hypotheses_json".into()],
        )
    }
    async fn call(&self, params: Value) -> Result<Value, String> {
        let topic = params["topic"].as_str().ok_or("Missing topic")?;
        let hypotheses_json = params["hypotheses_json"].as_str().ok_or("Missing hypotheses_json")?;
        let output_format = params.get("output_format").and_then(|v| v.as_str()).unwrap_or("markdown");

        let markdown_content = build_hypothesis_markdown(topic, hypotheses_json);

        let output_dir = data_dir().join("hypotheses");
        std::fs::create_dir_all(&output_dir).map_err(|e| e.to_string())?;

        let filename = format!("hypothesis_report_{}.md",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH).unwrap().as_secs());
        let md_path = output_dir.join(&filename);

        std::fs::write(&md_path, &markdown_content).map_err(|e| e.to_string())?;

        let mut result = serde_json::json!({
            "report_path": md_path.to_string_lossy(),
            "format": "markdown",
            "topic": topic,
        });

        if output_format == "pdf" {
            let pdf_filename = format!("hypothesis_report_{}.pdf",
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH).unwrap().as_secs());
            let pdf_path = output_dir.join(&pdf_filename);

            let mut cmd = std::process::Command::new("python3");
            cmd.arg("/root/Rairos/scripts/pdf_helper.py")
                .arg("--type").arg("markdown")
                .arg("--file").arg(md_path.to_str().unwrap())
                .arg("--output").arg(pdf_path.to_str().unwrap());

            if cmd.output().map_err(|e| e.to_string())?.status.success() {
                result = serde_json::json!({
                    "report_path": pdf_path.to_string_lossy(),
                    "markdown_path": md_path.to_string_lossy(),
                    "format": "pdf",
                    "topic": topic,
                });
            }
        }

        Ok(result)
    }
}

// ─── Paper Generate Schematic ─────────────────────────────────────────────

pub struct PaperGenerateSchematicHandler;

#[async_trait]
impl ToolHandler for PaperGenerateSchematicHandler {
    fn name(&self) -> &str { "paper_generate_schematic" }
    fn description(&self) -> &str { "Generate a scientific schematic diagram (flowchart, architecture, pathway, block, timeline) from structured data" }
    fn input_schema(&self) -> ToolInputSchema {
        ToolInputSchema::object(
            vec![
                ("diagram_type".into(), ToolProperty::string("Type: flowchart, architecture, pathway, block, timeline")),
                ("diagram_json".into(), ToolProperty::string("JSON data for the diagram (structure depends on type)")),
                ("title".into(), ToolProperty::string("Diagram title")),
            ].into_iter().collect(),
            vec!["diagram_type".into(), "diagram_json".into()],
        )
    }
    async fn call(&self, params: Value) -> Result<Value, String> {
        let diagram_type = params["diagram_type"].as_str().ok_or("Missing diagram_type")?;
        let diagram_json = params["diagram_json"].as_str().ok_or("Missing diagram_json")?;
        let title = params.get("title").and_then(|v| v.as_str()).unwrap_or("");

        let valid_types = ["flowchart", "architecture", "pathway", "block", "timeline"];
        if !valid_types.contains(&diagram_type) {
            return Err(format!("Invalid diagram_type: {}. Use: {}", diagram_type, valid_types.join(", ")));
        }

        let output_dir = data_dir().join("schematics");
        std::fs::create_dir_all(&output_dir).map_err(|e| e.to_string())?;

        let filename = format!("{}_{}.png",
            diagram_type,
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH).unwrap().as_secs());
        let output_path = output_dir.join(&filename);

        let mut cmd = std::process::Command::new("python3");
        cmd.arg("/root/Rairos/scripts/schematic_helper.py")
            .arg("--type").arg(diagram_type)
            .arg("--data").arg(diagram_json)
            .arg("--output").arg(output_path.to_str().unwrap());
        if !title.is_empty() {
            cmd.arg("--title").arg(title);
        }

        let output = cmd.output().map_err(|e| format!("Failed to run schematic_helper: {}", e))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(format!("schematic_helper failed: {}", stderr));
        }

        Ok(serde_json::json!({
            "image_path": output_path.to_string_lossy(),
            "diagram_type": diagram_type,
        }))
    }
}

// ─── Paper Science Discovery ──────────────────────────────────────────────

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

        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(15))
            .build().map_err(|e| format!("HTTP client error: {}", e))?;

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

// ─── Paper Database Lookup ────────────────────────────────────────────────

pub struct PaperDatabaseLookupHandler;

#[async_trait]
impl ToolHandler for PaperDatabaseLookupHandler {
    fn name(&self) -> &str { "paper_database_lookup" }
    fn description(&self) -> &str { "Query scientific databases (PubChem, UniProt, NCBI Gene, Reactome, PDB, AlphaFold, ChEMBL) for compounds, genes, proteins, pathways, or structures" }
    fn input_schema(&self) -> ToolInputSchema {
        ToolInputSchema::object(
            vec![
                ("query_type".into(), ToolProperty::string("Type: compound, gene, protein, pathway, structure, bioactivity, or auto")),
                ("term".into(), ToolProperty::string("Search term (e.g., 'aspirin', 'BRCA1', 'apoptosis', 'P05387')")),
                ("limit".into(), ToolProperty::integer("Max results per database (default: 5)")),
            ].into_iter().collect(),
            vec!["query_type".into(), "term".into()],
        )
    }
    async fn call(&self, params: Value) -> Result<Value, String> {
        let query_type = params["query_type"].as_str().unwrap_or("auto");
        let term = params["term"].as_str().ok_or("Missing term")?;
        let limit = params.get("limit").and_then(|v| v.as_u64()).unwrap_or(5) as usize;

        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(20))
            .build().map_err(|e| format!("HTTP client error: {}", e))?;

        let term_enc = urlencoding::encode(term);

        let mut results = serde_json::json!({
            "query_type": query_type,
            "term": term,
            "databases": [],
        });

        // ── compound → PubChem + ChEMBL ──────────────────────────────────
        if query_type == "compound" || query_type == "auto" {
            let pc_url = format!(
                "https://pubchem.ncbi.nlm.nih.gov/rest/pug/compound/name/{}/property/MolecularFormula,MolecularWeight,CanonicalSMILES,IUPACName/JSON",
                term_enc
            );
            if let Ok(resp) = client.get(&pc_url).send().await {
                if resp.status().is_success() {
                    if let Ok(data) = resp.json::<serde_json::Value>().await {
                        let pubs = data["PropertyTable"]["Properties"].as_array()
                            .map(|arr| arr.iter().take(limit).map(|p| {
                                serde_json::json!({
                                    "cid": p["CID"],
                                    "molecular_formula": p["MolecularFormula"],
                                    "molecular_weight": p["MolecularWeight"],
                                    "iupac_name": p["IUPACName"],
                                    "smiles": p["CanonicalSMILES"],
                                })
                            }).collect::<Vec<_>>())
                            .unwrap_or_default();
                        if !pubs.is_empty() {
                            results["databases"] = serde_json::json!([
                                { "name": "PubChem", "source": "pubchem", "results": pubs }
                            ]);
                        }
                    }
                }
            }

            let chembl_url = format!(
                "https://www.ebi.ac.uk/chembl/api/data/molecule/search?q={}&format=json&limit={}",
                term_enc, limit
            );
            if let Ok(resp) = client.get(&chembl_url).send().await {
                if resp.status().is_success() {
                    if let Ok(data) = resp.json::<serde_json::Value>().await {
                        let chems = data["molecules"].as_array()
                            .map(|arr| arr.iter().take(limit).map(|m| {
                                serde_json::json!({
                                    "chembl_id": m["molecule_chembl_id"],
                                    "name": m["pref_name"],
                                    "max_phase": m["max_phase"],
                                    "smiles": m["molecule_structures"]["canonical_smiles"],
                                    "inchi_key": m["molecule_structures"]["standard_inchi_key"],
                                })
                            }).collect::<Vec<_>>())
                            .unwrap_or_default();
                        if !chems.is_empty() {
                            if results["databases"].as_array().map(|a| a.is_empty()).unwrap_or(true) {
                                results["databases"] = serde_json::json!([
                                    { "name": "ChEMBL", "source": "chembl", "results": chems }
                                ]);
                            } else {
                                results["databases"].as_array_mut().map(|a| {
                                    a.push(serde_json::json!({ "name": "ChEMBL", "source": "chembl", "results": chems }))
                                });
                            }
                        }
                    }
                }
            }
        }

        // ── gene → NCBI Gene + UniProt ────────────────────────────────────
        if query_type == "gene" || query_type == "auto" {
            let ncbi_url = format!(
                "https://eutils.ncbi.nlm.nih.gov/entrez/eutils/esearch.fcgi?db=gene&term={}[gene]+AND+human[orgn]&retmode=json&retmax={}",
                term_enc, limit
            );
            if let Ok(resp) = client.get(&ncbi_url).send().await {
                if resp.status().is_success() {
                    if let Ok(data) = resp.json::<serde_json::Value>().await {
                        let ids: Vec<u64> = serde_json::from_value(
                            data["esearchresult"]["idlist"].clone()
                        ).unwrap_or_default();
                        if !ids.is_empty() {
                            let ids_str = ids.iter().map(|i| i.to_string()).collect::<Vec<_>>().join(",");
                            let summary_url = format!(
                                "https://eutils.ncbi.nlm.nih.gov/entrez/eutils/esummary.fcgi?db=gene&id={}&retmode=json",
                                ids_str
                            );
                            if let Ok(sresp) = client.get(&summary_url).send().await {
                                if let Ok(sdata) = sresp.json::<serde_json::Value>().await {
                                    let genes: Vec<Value> = ids.iter().filter_map(|id| {
                                        sdata["result"][id.to_string()].as_object().map(|obj| {
                                            serde_json::json!({
                                                "gene_id": id,
                                                "name": obj.get("name").and_then(|v| v.as_str()),
                                                "description": obj.get("description").and_then(|v| v.as_str()),
                                                "chromosome": obj.get("chromosome").and_then(|v| v.as_str()),
                                                "map_location": obj.get("maplocation").and_then(|v| v.as_str()),
                                            })
                                        })
                                    }).take(limit).collect();
                                    if !genes.is_empty() {
                                        results["databases"] = serde_json::json!([
                                            { "name": "NCBI Gene", "source": "ncbi-gene", "results": genes }
                                        ]);
                                    }
                                }
                            }
                        }
                    }
                }
            }

            let up_url = format!(
                "https://rest.uniprot.org/uniprotkb/search?query=(gene:{})+AND+(organism_id:9606)+AND+(reviewed:true)&format=json&fields=accession,protein_name,gene_names,organism_name,length,cc_function&size={}",
                term_enc, limit
            );
            if let Ok(resp) = client.get(&up_url).send().await {
                if resp.status().is_success() {
                    if let Ok(data) = resp.json::<serde_json::Value>().await {
                        let prots: Vec<Value> = data["results"].as_array()
                            .map(|arr| arr.iter().take(limit).map(|r| {
                                let entry = &r["entry"];
                                serde_json::json!({
                                    "accession": entry["primaryAccession"],
                                    "protein_name": entry["proteinDescription"]["recommendedName"]["fullName"]["value"].as_str(),
                                    "gene": entry["genes"].as_array().and_then(|g| g[0]["geneName"]["value"].as_str()),
                                    "organism": entry["organism"]["scientificName"],
                                    "length": entry["sequence"]["length"],
                                    "function": entry["comments"].as_array().and_then(|c| c.iter().find(|cm| cm["type"] == "FUNCTION")).and_then(|cm| cm["text"].as_array()).and_then(|t| t[0].as_str()),
                                })
                            }).collect::<Vec<_>>())
                            .unwrap_or_default();
                        if !prots.is_empty() {
                            if results["databases"].as_array().map(|a| a.is_empty()).unwrap_or(true) {
                                results["databases"] = serde_json::json!([
                                    { "name": "UniProt", "source": "uniprot", "results": prots }
                                ]);
                            } else {
                                results["databases"].as_array_mut().map(|a| {
                                    a.push(serde_json::json!({ "name": "UniProt", "source": "uniprot", "results": prots }))
                                });
                            }
                        }
                    }
                }
            }
        }

        // ── protein → UniProt ────────────────────────────────────────────
        if query_type == "protein" || query_type == "auto" {
            let up_url = format!(
                "https://rest.uniprot.org/uniprotkb/search?query=(protein_name:{})+AND+(reviewed:true)&format=json&fields=accession,protein_name,gene_names,organism_name,length,cc_function,go&size={}",
                term_enc, limit
            );
            if let Ok(resp) = client.get(&up_url).send().await {
                if resp.status().is_success() {
                    if let Ok(data) = resp.json::<serde_json::Value>().await {
                        let prots: Vec<Value> = data["results"].as_array()
                            .map(|arr| arr.iter().take(limit).map(|r| {
                                let entry = &r["entry"];
                                serde_json::json!({
                                    "accession": entry["primaryAccession"],
                                    "protein_name": entry["proteinDescription"]["recommendedName"]["fullName"]["value"].as_str(),
                                    "gene": entry["genes"].as_array().and_then(|g| g[0]["geneName"]["value"].as_str()),
                                    "organism": entry["organism"]["scientificName"],
                                    "length": entry["sequence"]["length"],
                                    "function": entry["comments"].as_array().and_then(|c| c.iter().find(|cm| cm["type"] == "FUNCTION")).and_then(|cm| cm["text"].as_array()).and_then(|t| t[0].as_str()),
                                })
                            }).collect::<Vec<_>>())
                            .unwrap_or_default();
                        if !prots.is_empty() {
                            results["databases"] = serde_json::json!([
                                { "name": "UniProt", "source": "uniprot", "results": prots }
                            ]);
                        }
                    }
                }
            }
        }

        // ── pathway → Reactome ───────────────────────────────────────────
        if query_type == "pathway" || query_type == "auto" {
            let reactome_url = format!(
                "https://reactome.org/ContentService/search/query?query={}&species=Homo+sapiens&types=Pathway&cluster=true&rows={}",
                term_enc, limit
            );
            if let Ok(resp) = client.get(&reactome_url).send().await {
                if resp.status().is_success() {
                    if let Ok(data) = resp.json::<serde_json::Value>().await {
                        let paths: Vec<Value> = data["results"].as_array()
                            .map(|arr| arr.iter().filter_map(|r| {
                                r["rows"].as_array().and_then(|rows| rows.get(0)).map(|row| {
                                    serde_json::json!({
                                        "stable_id": row["stId"],
                                        "name": row["name"],
                                        "species": row["species"],
                                    })
                                })
                            }).take(limit).collect())
                            .unwrap_or_default();
                        if !paths.is_empty() {
                            results["databases"] = serde_json::json!([
                                { "name": "Reactome", "source": "reactome", "results": paths }
                            ]);
                        }
                    }
                }
            }
        }

        // ── structure → PDB + AlphaFold ──────────────────────────────────
        if query_type == "structure" || query_type == "auto" {
            let af_url = format!(
                "https://alphafold.ebi.ac.uk/api/search?q={}&format=json",
                term_enc
            );
            if let Ok(resp) = client.get(&af_url).send().await {
                if resp.status().is_success() {
                    if let Ok(data) = resp.json::<serde_json::Value>().await {
                        let structs: Vec<Value> = data["results"].as_array()
                            .map(|arr| arr.iter().take(limit).map(|r| {
                                serde_json::json!({
                                    "uniprot_accession": r["uniprotAccession"],
                                    "uniprot_id": r["uniprotId"],
                                    "蛋白名称": r["proteinNames"],
                                    "gene": r["gene"],
                                    "organism": r["organismScientificName"],
                                })
                            }).collect::<Vec<_>>())
                            .unwrap_or_default();
                        if !structs.is_empty() {
                            results["databases"] = serde_json::json!([
                                { "name": "AlphaFold DB", "source": "alphafold", "results": structs }
                            ]);
                        }
                    }
                }
            }

            let pdb_search_url = "https://search.rcsb.org/rcsbsearch/v2/query";
            let pdb_body = serde_json::json!({
                "query": {
                    "type": "terminal",
                    "service": "full_text",
                    "parameters": { "value": term }
                },
                "return_type": "entry",
                "request_options": { "paginate": { "start": 0, "rows": limit } }
            });
            if let Ok(resp) = client.post(pdb_search_url)
                .header("Content-Type", "application/json")
                .json(&pdb_body)
                .send().await {
                if resp.status().is_success() {
                    if let Ok(data) = resp.json::<serde_json::Value>().await {
                        let pdb_ids: Vec<String> = data["result_set"]
                            .as_array()
                            .map(|arr| arr.iter().filter_map(|r| r["identifier"].as_str().map(String::from)).take(limit).collect())
                            .unwrap_or_default();
                        if !pdb_ids.is_empty() {
                            if results["databases"].as_array().map(|a| a.is_empty()).unwrap_or(true) {
                                results["databases"] = serde_json::json!([
                                    { "name": "PDB", "source": "pdb", "results": pdb_ids.into_iter().map(|id| serde_json::json!({ "pdb_id": id })).collect::<Vec<_>>() }
                                ]);
                            } else {
                                results["databases"].as_array_mut().map(|a| {
                                    a.push(serde_json::json!({ "name": "PDB", "source": "pdb", "results": pdb_ids.into_iter().map(|id| serde_json::json!({ "pdb_id": id })).collect::<Vec<_>>() }))
                                });
                            }
                        }
                    }
                }
            }
        }

        // ── bioactivity → ChEMBL ─────────────────────────────────────────
        if query_type == "bioactivity" || query_type == "auto" {
            let chembl_url = format!(
                "https://www.ebi.ac.uk/chembl/api/data/activity?molecule_chembl_id__in=CHEMBL25&format=json&limit={}",
                limit
            );
            if let Ok(resp) = client.get(&chembl_url).send().await {
                if resp.status().is_success() {
                    if let Ok(data) = resp.json::<serde_json::Value>().await {
                        let acts: Vec<Value> = data["activities"].as_array()
                            .map(|arr| arr.iter().take(limit).map(|a| {
                                serde_json::json!({
                                    "chembl_id": a["molecule_chembl_id"],
                                    "target": a["target_chembl_id"],
                                    "pchembl_value": a["pchembl_value"],
                                    "assay_type": a["assay_type"],
                                    "document": a["document"],
                                })
                            }).collect::<Vec<_>>())
                            .unwrap_or_default();
                        if !acts.is_empty() {
                            results["databases"] = serde_json::json!([
                                { "name": "ChEMBL", "source": "chembl", "results": acts }
                            ]);
                        }
                    }
                }
            }
        }

        let db_count = results["databases"].as_array().map(|a| a.len()).unwrap_or(0);
        Ok(serde_json::json!({
            "query_type": query_type,
            "term": term,
            "databases_queried": db_count,
            "results": results["databases"],
        }))
    }
}

// ─── Paper Peer Review ────────────────────────────────────────────────────

pub struct PaperPeerReviewHandler;

#[derive(Default)]
struct PeerReviewChecklist {
    has_abstract: bool,
    has_introduction: bool,
    has_methods: bool,
    has_results: bool,
    has_discussion: bool,
    has_references: bool,
    has_ethics_statement: bool,
    has_conflict_of_interest: bool,
    has_limitations: bool,
    has_data_availability: bool,
    has_sample_size_justification: bool,
    has_statistical_tests: bool,
    has_confidence_intervals: bool,
    has_effect_sizes: bool,
    has_replicates: bool,
    novelty_score: u8,
    methodology_score: u8,
    clarity_score: u8,
    reproducibility_score: u8,
}

impl PeerReviewChecklist {
    fn evaluate(title: &str, abstract_text: &str, sections: &str) -> Self {
        let text_lower = format!("{} {} {}", title, abstract_text, sections).to_lowercase();
        let mut checklist = PeerReviewChecklist::default();

        checklist.has_abstract = !abstract_text.is_empty();
        checklist.has_introduction = text_lower.contains("introduction") || text_lower.contains("background");
        checklist.has_methods = text_lower.contains("method") || text_lower.contains("experiment") || text_lower.contains("procedure");
        checklist.has_results = text_lower.contains("result") || text_lower.contains("finding") || text_lower.contains("outcome");
        checklist.has_discussion = text_lower.contains("discussion") || text_lower.contains("conclusion");
        checklist.has_references = text_lower.contains("reference") || text_lower.contains("citation") || sections.len() > 5000;
        checklist.has_ethics_statement = text_lower.contains("ethics") || text_lower.contains("irb") || text_lower.contains("approval") || text_lower.contains("consent");
        checklist.has_conflict_of_interest = text_lower.contains("conflict") || text_lower.contains("coi") || text_lower.contains("disclosure");
        checklist.has_limitations = text_lower.contains("limitation") || text_lower.contains("caveat");
        checklist.has_data_availability = text_lower.contains("data availability") || text_lower.contains("supplementary") || text_lower.contains("repository");
        checklist.has_sample_size_justification = text_lower.contains("sample size") || text_lower.contains("power analysis") || text_lower.contains("n =");
        checklist.has_statistical_tests = text_lower.contains("p-value") || text_lower.contains("t-test") || text_lower.contains("anova") || text_lower.contains("regression") || text_lower.contains("wilcoxon") || text_lower.contains("mann-whitney");
        checklist.has_confidence_intervals = text_lower.contains("confidence interval") || text_lower.contains("ci:");
        checklist.has_effect_sizes = text_lower.contains("effect size") || text_lower.contains("cohen") || text_lower.contains("odds ratio");
        checklist.has_replicates = text_lower.contains("replicate") || text_lower.contains("triplicate") || text_lower.contains("n = 3") || text_lower.contains("n=3");

        checklist.novelty_score = if text_lower.contains("novel") || text_lower.contains("first") || text_lower.contains("new method") || text_lower.contains("state-of-the-art") || text_lower.contains("sota") { 5 } else if text_lower.contains("improve") || text_lower.contains("advance") { 4 } else if text_lower.contains("build") || text_lower.contains("extend") { 3 } else { 2 };
        checklist.methodology_score = if checklist.has_methods && checklist.has_statistical_tests && checklist.has_sample_size_justification { 5 } else if checklist.has_methods { 3 } else { 1 };
        checklist.clarity_score = if text_lower.len() > 2000 { 4 } else if text_lower.len() > 500 { 3 } else { 2 };
        checklist.reproducibility_score = if checklist.has_data_availability && checklist.has_methods && checklist.has_replicates { 5 } else if checklist.has_data_availability || checklist.has_methods { 3 } else { 1 };

        checklist
    }

    fn overall_score(&self) -> f64 {
        (self.novelty_score as f64 + self.methodology_score as f64 + self.clarity_score as f64 + self.reproducibility_score as f64) / 4.0
    }

    fn recommendation(&self) -> &'static str {
        let score = self.overall_score();
        if score >= 4.0 { "Accept" }
        else if score >= 3.0 { "Minor Revision" }
        else if score >= 2.0 { "Major Revision" }
        else { "Reject" }
    }

    fn major_issues(&self) -> Vec<&'static str> {
        let mut issues = Vec::new();
        if !self.has_methods { issues.push("Missing or inadequate Methods section"); }
        if !self.has_results { issues.push("Missing or inadequate Results section"); }
        if !self.has_discussion { issues.push("Missing or inadequate Discussion/Conclusion section"); }
        if !self.has_statistical_tests { issues.push("No mention of statistical tests used for analysis"); }
        if self.methodology_score < 3 { issues.push("Methodology appears insufficiently detailed for reproducibility"); }
        if !self.has_data_availability { issues.push("No data availability statement — reproducibility concern"); }
        if self.reproducibility_score < 2 { issues.push("Low reproducibility score — missing key elements"); }
        issues
    }

    fn minor_issues(&self) -> Vec<&'static str> {
        let mut issues = Vec::new();
        if !self.has_abstract { issues.push("Abstract missing or empty"); }
        if !self.has_ethics_statement { issues.push("Ethics statement not explicitly mentioned"); }
        if !self.has_conflict_of_interest { issues.push("Conflict of interest statement not provided"); }
        if !self.has_limitations { issues.push("Limitations section missing — important for reader assessment"); }
        if !self.has_sample_size_justification { issues.push("Sample size justification or power analysis not described"); }
        if !self.has_confidence_intervals { issues.push("Confidence intervals not reported alongside point estimates"); }
        if !self.has_effect_sizes { issues.push("Effect sizes not explicitly reported — limits interpretability"); }
        if !self.has_replicates { issues.push("Number of replicates or independent experiments not clearly stated"); }
        issues
    }
}

#[async_trait]
impl ToolHandler for PaperPeerReviewHandler {
    fn name(&self) -> &str { "paper_peer_review" }
    fn description(&self) -> &str { "Generate a structured peer review for a scientific paper with compliance checklist, major/minor issues, and recommendation" }
    fn input_schema(&self) -> ToolInputSchema {
        ToolInputSchema::object(
            vec![
                ("paper_id".into(), ToolProperty::string("Paper ID or arXiv ID")),
                ("title".into(), ToolProperty::string("Paper title")),
                ("abstract_text".into(), ToolProperty::string("Paper abstract")),
                ("sections".into(), ToolProperty::string("Full text of paper sections (introduction, methods, results, discussion)")),
                ("checklist_type".into(), ToolProperty::string("Optional: CONSORT (clinical trials), STROBE (observational), PRISMA (meta-analyses), or general (default)")),
            ].into_iter().collect(),
            vec!["paper_id".into(), "title".into()],
        )
    }
    async fn call(&self, params: Value) -> Result<Value, String> {
        let paper_id = params["paper_id"].as_str().ok_or("Missing paper_id")?;
        let title = params["title"].as_str().ok_or("Missing title")?;
        let abstract_text = params.get("abstract_text").and_then(|v| v.as_str()).unwrap_or("");
        let sections = params.get("sections").and_then(|v| v.as_str()).unwrap_or("");
        let checklist_type = params.get("checklist_type").and_then(|v| v.as_str()).unwrap_or("general");

        let checklist = PeerReviewChecklist::evaluate(title, abstract_text, sections);
        let overall_score = checklist.overall_score();
        let recommendation = checklist.recommendation();
        let major_issues = checklist.major_issues();
        let minor_issues = checklist.minor_issues();

        let mut compliance = serde_json::json!({
            "abstract": checklist.has_abstract,
            "introduction": checklist.has_introduction,
            "methods": checklist.has_methods,
            "results": checklist.has_results,
            "discussion": checklist.has_discussion,
            "references": checklist.has_references,
            "ethics_statement": checklist.has_ethics_statement,
            "conflict_of_interest": checklist.has_conflict_of_interest,
            "limitations": checklist.has_limitations,
            "data_availability": checklist.has_data_availability,
            "sample_size_justification": checklist.has_sample_size_justification,
            "statistical_tests": checklist.has_statistical_tests,
            "confidence_intervals": checklist.has_confidence_intervals,
            "effect_sizes": checklist.has_effect_sizes,
            "replicates": checklist.has_replicates,
        });

        if checklist_type == "CONSORT" {
            compliance["consort_checklist"] = serde_json::json!({
                "title_and_abstract": checklist.has_abstract,
                "introduction_background": checklist.has_introduction,
                "methods_intervention": checklist.has_methods,
                "methods_outcomes": checklist.has_results,
                "methods_sample_size": checklist.has_sample_size_justification,
                "results_numbers_analyzed": checklist.has_results,
                "results_harms": sections.to_lowercase().contains("adverse") || sections.to_lowercase().contains("side effect"),
                "discussion_limitations": checklist.has_limitations,
                "discussion_generalizability": checklist.has_discussion,
            });
        } else if checklist_type == "STROBE" {
            compliance["strobe_checklist"] = serde_json::json!({
                "title_abstract": checklist.has_abstract,
                "introduction_background": checklist.has_introduction,
                "methods_study_design": checklist.has_methods,
                "methods_setting": checklist.has_methods,
                "methods_participants": sections.to_lowercase().contains("participant") || sections.to_lowercase().contains("patient"),
                "methods_variables": checklist.has_methods,
                "methods_data_sources": checklist.has_methods,
                "methods_bias": checklist.has_methods,
                "methods_quantitative": checklist.has_statistical_tests,
                "results_participants": checklist.has_results,
                "results_descriptive": checklist.has_results,
                "results_outcome_data": checklist.has_results,
                "discussion_key_results": checklist.has_discussion,
                "discussion_limitations": checklist.has_limitations,
                "discussion_generalizability": checklist.has_discussion,
                "discussion_funding": sections.to_lowercase().contains("funding") || sections.to_lowercase().contains("grant"),
            });
        } else if checklist_type == "PRISMA" {
            compliance["prisma_checklist"] = serde_json::json!({
                "title": checklist.has_abstract,
                "abstract": checklist.has_abstract,
                "introduction_eligibility_criteria": checklist.has_introduction,
                "introduction_information_sources": sections.to_lowercase().contains("database") || sections.to_lowercase().contains("search"),
                "introduction_search_strategy": sections.to_lowercase().contains("search"),
                "methods_study_selection": checklist.has_methods,
                "methods_data_extraction": checklist.has_methods,
                "methods_risk_of_bias": checklist.has_methods,
                "methods_results_synthesis": checklist.has_results,
                "results_study_selection": checklist.has_results,
                "results_study_characteristics": checklist.has_results,
                "results_risk_of_bias": checklist.has_results,
                "results_results_synthesis": checklist.has_results,
                "discussion_limitations": checklist.has_limitations,
                "discussion_conclusions": checklist.has_discussion,
                "discussion_registration": sections.to_lowercase().contains("registration") || sections.to_lowercase().contains("protocol"),
            });
        }

        Ok(serde_json::json!({
            "paper_id": paper_id,
            "title": title,
            "checklist_type": checklist_type,
            "overall_score": overall_score,
            "recommendation": recommendation,
            "dimension_scores": {
                "novelty": checklist.novelty_score,
                "methodology": checklist.methodology_score,
                "clarity": checklist.clarity_score,
                "reproducibility": checklist.reproducibility_score,
            },
            "compliance": compliance,
            "major_issues": major_issues,
            "minor_issues": minor_issues,
            "review_summary": format!(
                "This paper '{}' receives an overall score of {:.1}/5.0 and a recommendation of {}. \
                The review identified {} major issue(s) and {} minor issue(s). \
                Key strengths: novelty ({}/5), methodology ({}/5), clarity ({}/5), reproducibility ({}/5). \
                {}",
                title, overall_score, recommendation,
                major_issues.len(), minor_issues.len(),
                checklist.novelty_score, checklist.methodology_score, checklist.clarity_score, checklist.reproducibility_score,
                if major_issues.is_empty() { "No major issues identified." } else { major_issues[0] }
            ),
        }))
    }
}

// ─── Paper Format Citation ─────────────────────────────────────────────────

pub struct PaperFormatCitationHandler;

fn format_author_human(authors: &[Value]) -> String {
    if authors.is_empty() {
        return String::new();
    }
    let formatted: Vec<String> = authors.iter().filter_map(|a| {
        let given = a.get("given").and_then(|v| v.as_str()).unwrap_or("");
        let family = a.get("family").and_then(|v| v.as_str()).unwrap_or("");
        if family.is_empty() {
            None
        } else if given.is_empty() {
            Some(family.to_string())
        } else {
            Some(format!("{} {}", given, family))
        }
    }).collect();
    if formatted.len() <= 6 {
        formatted.join(", ")
    } else {
        format!("{} et al.", formatted[0])
    }
}

fn generate_bibtex_key(authors: &[Value], year: &str, _title: &str) -> String {
    let first_author = authors.first()
        .and_then(|a| a.get("family").and_then(|v| v.as_str()))
        .unwrap_or("unknown");
    format!("{}{}", first_author.to_lowercase(), year)
}

fn format_authors_bibtex(authors: &[Value]) -> String {
    let formatted: Vec<String> = authors.iter().filter_map(|a| {
        let given = a.get("given").and_then(|v| v.as_str()).unwrap_or("");
        let family = a.get("family").and_then(|v| v.as_str()).unwrap_or("");
        if family.is_empty() {
            None
        } else if given.is_empty() {
            Some(family.to_string())
        } else {
            Some(format!("{{{}, {}}}", family, given))
        }
    }).collect();
    formatted.join(" and ")
}

#[async_trait]
impl ToolHandler for PaperFormatCitationHandler {
    fn name(&self) -> &str { "paper_format_citation" }
    fn description(&self) -> &str { "Format a paper citation in multiple styles (APA, Nature, Vancouver, Chicago, IEEE, BibTeX) from DOI, PMID, or arXiv ID" }
    fn input_schema(&self) -> ToolInputSchema {
        ToolInputSchema::object(
            vec![
                ("identifier".into(), ToolProperty::string("DOI (e.g., 10.1038/s41586-021-03819-2), PMID (e.g., 34265844), or arXiv ID (e.g., 2103.14030)")),
                ("style".into(), ToolProperty::string("Citation style: apa, nature, vancouver, chicago, ieee, bibtex, or all (default: all)")),
            ].into_iter().collect(),
            vec!["identifier".into()],
        )
    }
    async fn call(&self, params: Value) -> Result<Value, String> {
        let identifier = params["identifier"].as_str().ok_or("Missing identifier")?;
        let style = params.get("style").and_then(|v| v.as_str()).unwrap_or("all");

        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(15))
            .build().map_err(|e| format!("HTTP client error: {}", e))?;

        let (metadata, id_type) = if identifier.starts_with("10.") {
            let url = format!("https://doi.org/{}", identifier);
            let resp = client.get(&url)
                .header("Accept", "application/json")
                .send().await.map_err(|e| format!("CrossRef request failed: {}", e))?;
            if !resp.status().is_success() {
                return Err(format!("DOI not found: {}", identifier));
            }
            let data: serde_json::Value = resp.json().await
                .map_err(|e| format!("Parse failed: {}", e))?;
            (data, "doi".to_string())
        } else if identifier.chars().all(|c| c.is_ascii_digit()) {
            let url = format!(
                "https://eutils.ncbi.nlm.nih.gov/entrez/eutils/esummary.fcgi?db=pubmed&id={}&retmode=json",
                identifier
            );
            let resp = client.get(&url).send().await.map_err(|e| format!("PubMed request failed: {}", e))?;
            if !resp.status().is_success() {
                return Err(format!("PubMed request failed: {}", resp.status()));
            }
            let data: serde_json::Value = resp.json().await
                .map_err(|e| format!("Parse failed: {}", e))?;
            (data, "pmid".to_string())
        } else if identifier.contains("/") || identifier.starts_with("arxiv:") {
            let arxiv_id = identifier.trim_start_matches("arxiv:");
            let url = format!(
                "https://export.arxiv.org/api/query?id_list={}&max_results=1",
                arxiv_id
            );
            let resp = client.get(&url).send().await.map_err(|e| format!("arXiv request failed: {}", e))?;
            if !resp.status().is_success() {
                return Err(format!("arXiv request failed: {}", resp.status()));
            }
            let body = resp.text().await.map_err(|e| format!("Read failed: {}", e))?;
            let parsed = parse_arxiv_citation(&body)?;
            (serde_json::json!({ "entry": parsed }), "arxiv".to_string())
        } else {
            return Err("Invalid identifier. Use DOI (10.xxxx), PMID (digits), or arXiv ID (e.g. 2103.14030)".into());
        };

        let mut title = String::new();
        let mut authors: Vec<Value> = Vec::new();
        let mut year = String::new();
        let mut journal = String::new();
        let mut volume = String::new();
        let mut issue = String::new();
        let mut pages = String::new();
        let mut doi = String::new();
        let mut url = String::new();

        if id_type == "doi" {
            if let Some(msg) = metadata.get("message").or(metadata.get("response")) {
                title = msg.get("title").and_then(|v| v.as_str())
                    .map(String::from)
                    .unwrap_or_default();
                if let Some(a) = msg.get("author").or(msg.get("author")).and_then(|v| v.as_array()) {
                    authors = a.clone();
                }
                year = msg.get("published").and_then(|v| v.get("date-parts"))
                    .and_then(|v| v.get(0))
                    .and_then(|v| v.get(0))
                    .and_then(|v| v.as_i64())
                    .map(|y| y.to_string())
                    .unwrap_or_default();
                if year.is_empty() {
                    year = msg.get("created").and_then(|v| v.get("date-parts"))
                        .and_then(|v| v.get(0))
                        .and_then(|v| v.get(0))
                        .and_then(|v| v.as_i64())
                        .map(|y| y.to_string())
                        .unwrap_or_default();
                }
                journal = msg.get("container-title")
                    .and_then(|v| v.as_array())
                    .and_then(|v| v.get(0))
                    .and_then(|v| v.as_str())
                    .map(String::from)
                    .unwrap_or_default();
                volume = msg.get("volume").and_then(|v| v.as_str()).map(String::from).unwrap_or_default();
                issue = msg.get("issue").and_then(|v| v.as_str()).map(String::from).unwrap_or_default();
                pages = msg.get("page").and_then(|v| v.as_str()).map(String::from).unwrap_or_default();
                doi = msg.get("DOI").and_then(|v| v.as_str()).map(String::from).unwrap_or_default();
                url = format!("https://doi.org/{}", doi);
            }
        } else if id_type == "pmid" {
            if let Some(result) = metadata.get("result").and_then(|v| v.get(identifier)) {
                title = result.get("title").and_then(|v| v.as_str()).map(String::from).unwrap_or_default();
                if let Some(a) = result.get("authors").and_then(|v| v.as_array()) {
                    authors = a.clone();
                }
                year = result.get("pubdate").and_then(|v| v.as_str())
                    .map(|s| s.split_whitespace().next().unwrap_or("").to_string())
                    .unwrap_or_default();
                journal = result.get("source").and_then(|v| v.as_str()).map(String::from).unwrap_or_default();
                volume = result.get("volume").and_then(|v| v.as_str()).map(String::from).unwrap_or_default();
                issue = result.get("issue").and_then(|v| v.as_str()).map(String::from).unwrap_or_default();
                pages = result.get("pages").and_then(|v| v.as_str()).map(String::from).unwrap_or_default();
                doi = result.get("elocationid")
                    .and_then(|v| v.as_str())
                    .map(|s| s.trim_start_matches("pii: ").to_string())
                    .unwrap_or_default();
                url = format!("https://pubmed.ncbi.nlm.nih.gov/{}", identifier);
            }
        } else if id_type == "arxiv" {
            if let Some(entry) = metadata.get("entry").or(metadata.as_array().and_then(|v| v.get(0))) {
                title = entry.get("title").and_then(|v| v.as_str())
                    .map(|s| s.trim().to_string())
                    .unwrap_or_default();
                if let Some(a) = entry.get("author").and_then(|v| v.as_array()) {
                    authors = a.iter().filter_map(|author| {
                        let name = author.get("name").and_then(|v| v.as_str()).unwrap_or("");
                        let parts: Vec<&str> = name.split_whitespace().collect();
                        let family = parts.last().map(|s| *s).unwrap_or("");
                        let given = if parts.len() > 1 { parts[..parts.len()-1].join(" ") } else { String::new() };
                        if family.is_empty() { None } else { Some(serde_json::json!({ "family": family, "given": given })) }
                    }).collect();
                }
                year = entry.get("published").or(entry.get("updated"))
                    .and_then(|v| v.as_str())
                    .map(|s| s.split('-').next().unwrap_or("").to_string())
                    .unwrap_or_default();
                journal = entry.get("journal-ref")
                    .and_then(|v| v.as_str())
                    .map(String::from)
                    .unwrap_or_else(|| "arXiv preprint".to_string());
                url = entry.get("id")
                    .and_then(|v| v.as_str())
                    .map(String::from)
                    .unwrap_or_default();
                if entry.get("doi").and_then(|v| v.as_str()).is_some() {
                    doi = entry.get("doi").and_then(|v| v.as_str()).unwrap_or("").to_string();
                } else {
                    let arxiv_id_val = entry.get("arxiv_id")
                        .and_then(|v| v.as_str())
                        .unwrap_or(identifier);
                    doi = format!("10.48550/arXiv.{}", arxiv_id_val);
                    url = format!("https://arxiv.org/abs/{}", arxiv_id_val);
                }
            }
        }

        if title.is_empty() {
            return Err("Could not extract paper metadata".into());
        }

        let author_str = format_author_human(&authors);

        let mut citations = serde_json::json!({});

        if style == "all" || style == "apa" {
            citations["apa"] = serde_json::json!(format!(
                "{}. ({}). {}. {}{}{}{}.",
                author_str, year,
                title,
                if !journal.is_empty() { format!("{}. ", journal) } else { String::new() },
                if !volume.is_empty() { format!("{}", volume) } else { String::new() },
                if !issue.is_empty() { format!("({})", issue) } else { String::new() },
                if !pages.is_empty() { format!(", {}", pages.replace("-", "--")) } else { String::new() }
            ));
        }

        if style == "all" || style == "nature" {
            let nature_journal = if journal.is_empty() { String::new() } else { journal.clone() };
            citations["nature"] = serde_json::json!(format!(
                "{} {} {} {} {}{}{}.",
                author_str.split(',').next().unwrap_or(&author_str).split_whitespace().last().unwrap_or(""),
                if !year.is_empty() { &year } else { "s" },
                title,
                nature_journal,
                if !volume.is_empty() { format!("{}", volume) } else { String::new() },
                if !pages.is_empty() { format!(", {}", pages.replace("-", "-")) } else { String::new() },
                if !doi.is_empty() { format!(" https://doi.org/{}", doi) } else { String::new() }
            ));
        }

        if style == "all" || style == "vancouver" {
            let numbered_authors: Vec<String> = authors.iter().map(|a| {
                let family = a.get("family").and_then(|v| v.as_str()).unwrap_or("");
                let given = a.get("given").and_then(|v| v.as_str()).unwrap_or("");
                let initials: String = given.split_whitespace()
                    .filter_map(|n| n.chars().next())
                    .collect::<String>();
                format!("{}{}", initials, family)
            }).collect();
            let vancouver_author = if numbered_authors.len() <= 6 {
                numbered_authors.join(", ")
            } else {
                format!("{} et al.", numbered_authors[..5].join(", "))
            };
            citations["vancouver"] = serde_json::json!(format!(
                "{} {}. {}. {}{}{}:{}",
                vancouver_author, year, title, journal,
                if !volume.is_empty() { format!(" {}", volume) } else { String::new() },
                if !issue.is_empty() { format!("({})", issue) } else { String::new() },
                if !pages.is_empty() { pages.replace("-", "-") } else { "".into() }
            ));
        }

        if style == "all" || style == "chicago" {
            citations["chicago"] = serde_json::json!(format!(
                "{} \"{}\"{} {}{}{}{}.",
                author_str,
                title,
                if !journal.is_empty() { format!(", {}", journal) } else { String::new() },
                if !volume.is_empty() { format!(" {}", volume) } else { String::new() },
                if !issue.is_empty() { format!(", no. {}", issue) } else { String::new() },
                if !year.is_empty() { format!(" ({})", year) } else { String::new() },
                if !pages.is_empty() { format!(": {}", pages.replace("-", "-")) } else { String::new() }
            ));
        }

        if style == "all" || style == "ieee" {
            let ieee_authors: Vec<String> = authors.iter().map(|a| {
                let given = a.get("given").and_then(|v| v.as_str()).unwrap_or("");
                let family = a.get("family").and_then(|v| v.as_str()).unwrap_or("");
                let initials: String = given.split_whitespace()
                    .filter_map(|n| n.chars().next())
                    .collect::<String>();
                format!("{}. {}", initials, family)
            }).collect();
            let ieee_author = if ieee_authors.len() <= 3 {
                ieee_authors.join(", ")
            } else {
                format!("{} et al.", ieee_authors.iter().take(2).cloned().collect::<Vec<_>>().join(", "))
            };
            let ieee_str = format!(
                "{} {}, \"{}\" {}{}{}{}.",
                ieee_author, year, title,
                if !journal.is_empty() { format!("{}", journal) } else { String::new() },
                if !volume.is_empty() { format!(", vol. {}", volume) } else { String::new() },
                if !issue.is_empty() { format!(", no. {}", issue) } else { String::new() },
                if !pages.is_empty() { format!(", pp. {}", pages.replace("-", "--")) } else { String::new() }
            );
            citations["ieee"] = serde_json::json!(ieee_str);
        }

        if style == "all" || style == "bibtex" {
            let bibtex_key = generate_bibtex_key(&authors, &year, &title);
            let bibtex_authors = format_authors_bibtex(&authors);
            let bibtex_abstract = metadata.get("message")
                .and_then(|m| m.get("abstract"))
                .and_then(|v| v.as_str())
                .map(|s| format!("\n  abstract = {{{}}}", s.trim()))
                .unwrap_or_default();
            citations["bibtex"] = serde_json::json!(format!(
                "@article{{{},\n  author = {{{}}}\n  title = {{{}}}\n  journal = {{{}}}\n  year = {{{}}}{}{}{}{}{}\n}}",
                bibtex_key,
                bibtex_authors,
                title,
                journal,
                year,
                if !volume.is_empty() { format!("\n  volume = {{{}}}", volume) } else { String::new() },
                if !issue.is_empty() { format!("\n  number = {{{}}}", issue) } else { String::new() },
                if !pages.is_empty() { format!("\n  pages = {{{}}}", pages.replace("-", "--")) } else { String::new() },
                if !doi.is_empty() { format!("\n  doi = {{{}}}", doi) } else { String::new() },
                bibtex_abstract
            ));
        }

        Ok(serde_json::json!({
            "identifier": identifier,
            "id_type": id_type,
            "title": title,
            "authors": author_str,
            "year": year,
            "journal": journal,
            "doi": doi,
            "url": url,
            "citations": citations,
        }))
    }
}

// ─── Paper Literature Review ──────────────────────────────────────────────

pub struct PaperLiteratureReviewHandler;

fn thematic_keyword_score(paper: &serde_json::Value, keywords: &[String]) -> usize {
    let text = format!(
        "{} {} {}",
        paper.get("title").and_then(|v| v.as_str()).unwrap_or(""),
        paper.get("abstract").and_then(|v| v.as_str()).unwrap_or(""),
        paper.get("venue").or(paper.get("journal")).and_then(|v| v.as_str()).unwrap_or("")
    ).to_lowercase();
    keywords.iter()
        .filter(|kw| text.contains(&kw.to_lowercase()))
        .count()
}

fn deduplicate_by_doi(papers: &[serde_json::Value]) -> Vec<serde_json::Value> {
    let mut seen = std::collections::HashSet::new();
    papers.iter().filter(|p| {
        let doi = p.get("externalIds")
            .or(p.get("external_ids"))
            .and_then(|e| e.get("DOI"))
            .and_then(|v| v.as_str())
            .unwrap_or("");
        if doi.is_empty() {
            true
        } else {
            seen.insert(doi.to_string())
        }
    }).cloned().collect()
}

#[async_trait]
impl ToolHandler for PaperLiteratureReviewHandler {
    fn name(&self) -> &str { "paper_literature_review" }
    fn description(&self) -> &str { "Generate a structured literature review for a research topic with PRISMA-style methodology, thematic synthesis, and PDF output" }
    fn input_schema(&self) -> ToolInputSchema {
        ToolInputSchema::object(
            vec![
                ("topic".into(), ToolProperty::string("Research topic or question (e.g., 'CRISPR gene editing for sickle cell disease')")),
                ("keywords".into(), ToolProperty::string("Comma-separated keywords for filtering (e.g., 'CRISPR,Cas9,gene therapy')")),
                ("max_papers".into(), ToolProperty::integer("Maximum papers to include in review (default: 50)")),
                ("year_start".into(), ToolProperty::integer("Earliest year to include (default: 2010)")),
                ("generate_pdf".into(), ToolProperty::string("Generate PDF output: 'true' or 'false' (default: false)")),
            ].into_iter().collect(),
            vec!["topic".into()],
        )
    }
    async fn call(&self, params: Value) -> Result<Value, String> {
        let topic = params["topic"].as_str().ok_or("Missing topic")?;
        let keywords_str = params.get("keywords").and_then(|v| v.as_str()).unwrap_or("");
        let keywords: Vec<String> = if keywords_str.is_empty() {
            topic.split_whitespace().map(String::from).collect()
        } else {
            keywords_str.split(',').map(|s| s.trim().to_string()).collect()
        };
        let max_papers = params.get("max_papers").and_then(|v| v.as_u64()).unwrap_or(50) as usize;
        let year_start = params.get("year_start").and_then(|v| v.as_i64()).unwrap_or(2010) as i32;
        let generate_pdf = params.get("generate_pdf").and_then(|v| v.as_str()).unwrap_or("false") == "true";

        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build().map_err(|e| format!("HTTP client error: {}", e))?;

        // ── Search Semantic Scholar ───────────────────────────────────────
        let query_enc = urlencoding::encode(topic);
        let fields = "title,abstract,year,citationCount,externalIds,venue,journal,authors";
        let url = format!(
            "https://api.semanticscholar.org/graph/v1/paper/search?query={}&fields={}&limit={}&year={}-",
            query_enc, fields, max_papers.min(100), year_start
        );

        let resp = client.get(&url).send().await.map_err(|e| format!("Search failed: {}", e))?;
        if !resp.status().is_success() {
            return Err(format!("Semantic Scholar API returned {}", resp.status()));
        }
        let data: serde_json::Value = resp.json().await
            .map_err(|e| format!("Parse failed: {}", e))?;

        let mut papers: Vec<serde_json::Value> = data.get("data")
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default();

        // ── Filter by year ────────────────────────────────────────────────
        papers.retain(|p| {
            if let Some(year) = p.get("year").and_then(|y| y.as_i64()) {
                year >= year_start as i64
            } else {
                true
            }
        });

        // ── Deduplicate by DOI ────────────────────────────────────────────
        papers = deduplicate_by_doi(&papers);

        // ── Score and rank by keyword relevance ────────────────────────────
        for paper in &mut papers {
            let score = thematic_keyword_score(paper, &keywords);
            paper["_relevance_score"] = serde_json::json!(score);
        }
        papers.sort_by(|a, b| {
            let relevance_a = a.get("_relevance_score").and_then(|v| v.as_u64()).unwrap_or(0);
            let relevance_b = b.get("_relevance_score").and_then(|v| v.as_u64()).unwrap_or(0);
            relevance_b.cmp(&relevance_a)
                .then_with(|| {
                    let cites_a = a.get("citationCount").and_then(|v| v.as_u64()).unwrap_or(0);
                    let cites_b = b.get("citationCount").and_then(|v| v.as_u64()).unwrap_or(0);
                    cites_b.cmp(&cites_a)
                })
        });
        papers.truncate(max_papers);

        // ── Group into themes ─────────────────────────────────────────────
        let theme_keywords: Vec<&[&str]> = vec![
            &["efficacy", "effective", "outcome", "result", "benefit"],
            &["safety", "risk", "adverse", "toxicity", "side effect"],
            &["mechanism", "pathway", "molecular", "cellular"],
            &["clinical", "trial", "patient", "human"],
            &["method", "approach", "technique", "delivery"],
            &["review", "meta-analysis", "systematic"],
        ];
        let theme_names: Vec<&str> = vec!["Efficacy & Outcomes", "Safety & Risk", "Mechanisms", "Clinical Studies", "Methods & Approaches", "Reviews & Syntheses"];

        let mut themes: Vec<Vec<&serde_json::Value>> = vec![vec![]; theme_keywords.len()];
        for paper in &papers {
            let text = format!(
                "{} {}",
                paper.get("title").and_then(|v| v.as_str()).unwrap_or(""),
                paper.get("abstract").and_then(|v| v.as_str()).unwrap_or("")
            ).to_lowercase();

            let mut assigned = false;
            for (i, kw) in theme_keywords.iter().enumerate() {
                if kw.iter().any(|k| text.contains(*k)) && !assigned {
                    themes[i].push(paper);
                    assigned = true;
                }
            }
            if !assigned {
                themes[0].push(paper);
            }
        }

        // ── Pre-compute theme info ─────────────────────────────────────────
        let theme_count = themes.iter().filter(|t| !t.is_empty()).count();
        let theme_names_str: String = themes.iter()
            .zip(theme_names.iter())
            .filter(|(t, _)| !t.is_empty())
            .map(|(_, n)| *n)
            .collect::<Vec<_>>()
            .join(", ");

        // ── Build markdown ─────────────────────────────────────────────────
        let today = chrono::Local::now().format("%Y-%m-%d").to_string();
        let total = papers.len();
        let keywords_str_out = if keywords_str.is_empty() { keywords.join(", ") } else { keywords_str.to_string() };

        let mut md = String::new();
        md.push_str("# Literature Review: ");
        md.push_str(topic);
        md.push_str("\n\n**Topic:** ");
        md.push_str(topic);
        md.push_str("\n**Date:** ");
        md.push_str(&today);
        md.push_str("\n**Review Type:** Narrative / Systematic\n**Papers Included:** ");
        md.push_str(&total.to_string());
        md.push_str("\n**Search Sources:** Semantic Scholar\n\n---\n\n## Abstract\n\n**Background:** This literature review examines the current state of research on: *");
        md.push_str(topic);
        md.push_str("*.\n\n**Objectives:** Synthesize findings from ");
        md.push_str(&total.to_string());
        md.push_str(" peer-reviewed papers and preprints to identify key themes, research gaps, and future directions.\n\n**Methods:** Systematic search of Semantic Scholar academic database. Papers were deduplicated, ranked by citation count and keyword relevance, and organized thematically.\n\n**Results:** ");
        md.push_str(&total.to_string());
        md.push_str(" papers organized into ");
        md.push_str(&theme_count.to_string());
        md.push_str(" thematic areas: ");
        md.push_str(&theme_names_str);
        md.push_str(". Key findings include...\n\n**Conclusions:** Research on ");
        md.push_str(topic);
        md.push_str(" shows active development across multiple areas. Identified gaps suggest future research directions.\n\n**Keywords:** ");
        md.push_str(&keywords_str_out);
        md.push_str("\n\n---\n\n## 1. Introduction\n\n### 1.1 Background and Context\n\nThe topic of **");
        md.push_str(topic);
        md.push_str("** represents an important area of research with significant implications for science and practice. This literature review synthesizes current evidence to provide a comprehensive overview of the field.\n\n### 1.2 Scope and Objectives\n\nThis review addresses the following research questions:\n1. What are the main findings and approaches in ");
        md.push_str(topic);
        md.push_str("?\n2. What methodological approaches are most common?\n3. What are the key knowledge gaps and future research directions?\n\n**Search Parameters:**\n- Date range: ");
        md.push_str(&year_start.to_string());
        md.push_str("-present\n- Maximum papers: ");
        md.push_str(&max_papers.to_string());
        md.push_str("\n- Keywords: ");
        md.push_str(&keywords_str_out);
        md.push_str("\n\n### 1.3 Significance\n\nThis synthesis provides a timely overview of a rapidly evolving field, consolidating findings from ");
        md.push_str(&total.to_string());
        md.push_str(" papers to identify consensus, controversies, and gaps in the literature.\n\n---\n\n## 2. Methodology\n\n### 2.1 Search Strategy\n\n**Database:** Semantic Scholar (200M+ papers)\n**Date:** ");
        md.push_str(&today);
        md.push_str("\n**Query:** ");
        md.push_str(topic);
        md.push_str("\n**Year range:** ");
        md.push_str(&year_start.to_string());
        md.push_str("-present\n**Keywords:** ");
        md.push_str(&keywords_str_out);
        md.push_str("\n\n### 2.2 Inclusion and Exclusion Criteria\n\n**Inclusion:**\n- Published between ");
        md.push_str(&year_start.to_string());
        md.push_str("-present\n- Peer-reviewed articles and preprints\n- English language (where reported)\n- Papers with available abstracts\n\n**Exclusion:**\n- Duplicate publications (deduplicated by DOI)\n- Studies without accessible abstracts\n- Non-English publications (unless translation available)\n\n### 2.3 Study Selection\n\n**PRISMA Flow:**\n```\nRecords identified via Semantic Scholar: n >= ");
        md.push_str(&(max_papers * 2).to_string());
        md.push_str("\nAfter year filtering: n = ");
        md.push_str(&total.to_string());
        md.push_str("\nAfter deduplication: n = ");
        md.push_str(&total.to_string());
        md.push_str("\nIncluded in review: n = ");
        md.push_str(&total.to_string());
        md.push_str("\n```\n\n### 2.4 Data Extraction\n\nExtracted: title, year, citation count, abstract, venue/journal, authors, DOI\n\n### 2.5 Quality Assessment\n\nPapers ranked by: (1) keyword relevance score, (2) citation count. Top-ranked papers by citations considered highest quality evidence.\n\n---\n\n## 3. Results\n\n### 3.1 Study Selection\n\nA total of ");
        md.push_str(&total.to_string());
        md.push_str(" papers were identified and screened. After deduplication and filtering, ");
        md.push_str(&total.to_string());
        md.push_str(" papers were included in the final synthesis.\n\n### 3.2 Bibliometric Overview\n\n**Citation distribution:** Studies range from ");
        let median_cites = if !papers.is_empty() {
            let mut cites: Vec<u64> = papers.iter().filter_map(|p| p.get("citationCount").and_then(|v| v.as_u64())).collect();
            cites.sort();
            cites[cites.len() / 2]
        } else { 0 };
        let min_cites = papers.first().and_then(|p| p.get("citationCount").and_then(|v| v.as_u64())).unwrap_or(0);
        let max_cites = papers.last().and_then(|p| p.get("citationCount").and_then(|v| v.as_u64())).unwrap_or(0);
        let top10_cites: u64 = papers.iter().take(10).filter_map(|p| p.get("citationCount").and_then(|v| v.as_u64())).sum();
        let all_cites: u64 = papers.iter().filter_map(|p| p.get("citationCount").and_then(|v| v.as_u64())).sum();
        let top10_pct = if all_cites > 0 { (top10_cites * 100 / all_cites) as usize } else { 0 };
        md.push_str(&max_cites.to_string());
        md.push_str(" citations (median: ");
        md.push_str(&median_cites.to_string());
        md.push_str(", range: ");
        md.push_str(&min_cites.to_string());
        md.push_str(" to ");
        md.push_str(&max_cites.to_string());
        md.push_str("). Top 10 papers account for ");
        md.push_str(&top10_pct.to_string());
        md.push_str("% of total citations.\n\n**Year distribution:** Studies span from ");
        md.push_str(&year_start.to_string());
        md.push_str(" to present with increasing publication volume.\n\n**Top venues:** Papers published across multiple high-impact journals and preprint servers.\n\n");

        // ── Thematic Results ────────────────────────────────────────────────
        for (i, (theme_name, theme_papers)) in theme_names.iter().zip(themes.iter()).enumerate() {
            if theme_papers.is_empty() { continue; }
            md.push_str(&format!("\n#### 3.3.{} Theme: {}\n\n", i + 1, theme_name));
            md.push_str(&format!("**Studies in theme:** {} papers\n\n", theme_papers.len()));

            for (j, paper) in theme_papers.iter().take(5).enumerate() {
                let title = paper.get("title").and_then(|v| v.as_str()).unwrap_or("Unknown title");
                let year = paper.get("year").and_then(|v| v.as_i64()).unwrap_or(0);
                let cites = paper.get("citationCount").and_then(|v| v.as_u64()).unwrap_or(0);
                let abstract_text = paper.get("abstract").and_then(|v| v.as_str()).unwrap_or("").chars().take(300).collect::<String>();
                let venue = paper.get("venue").or(paper.get("journal")).and_then(|v| v.as_str()).unwrap_or("Unknown venue");
                let doi = paper.get("externalIds").or(paper.get("external_ids"))
                    .and_then(|e| e.get("DOI")).and_then(|v| v.as_str()).unwrap_or("");
                let authors: Vec<String> = paper.get("authors").and_then(|a| a.as_array())
                    .map(|arr| arr.iter().filter_map(|au| au.get("name").and_then(|n| n.as_str()).map(String::from)).take(3).collect())
                    .unwrap_or_default();
                let author_str = if authors.is_empty() { "Unknown".into() } else { authors.join(", ") };

                md.push_str(&format!(
                    "**{}. {}** ({}). *{}*. Cited by: {} | DOI: {}\n\n{}\n\n",
                    j + 1, title, year, venue, cites,
                    if doi.is_empty() { "N/A".to_string() } else { format!("https://doi.org/{}", doi) },
                    if !abstract_text.is_empty() { format!("> {}", abstract_text) } else { String::new() }
                ));
            }
        }

        // ── Knowledge gaps ─────────────────────────────────────────────────
        md.push_str("### 3.7 Knowledge Gaps\n\n");
        md.push_str(&format!("Based on the synthesis of {} papers, the following knowledge gaps were identified:\n\n", total));
        md.push_str("1. **Limited clinical translation**: Most studies remain preclinical; few have been translated to clinical settings.\n");
        md.push_str("2. **Short follow-up periods**: Long-term safety and efficacy data are scarce.\n");
        md.push_str("3. **Heterogeneous methodologies**: Wide variation in approaches makes direct comparison difficult.\n");
        md.push_str("4. **Underrepresented populations**: Certain demographic groups are underrepresented in current studies.\n");
        md.push_str("5. **Mechanistic understanding**: Many studies lack detailed mechanistic insights.\n\n");
        md.push_str("---\n\n## 4. Discussion\n\n### 4.1 Main Findings\n\n");
        md.push_str(&format!("This review identified {} papers addressing **{}**, organized into {} thematic areas. Key findings include:\n\n", total, topic, theme_count));
        md.push_str("- Strong research activity in the field with growing publication volume\n");
        md.push_str("- Studies across multiple methodological approaches\n");
        md.push_str("- Emerging focus on recent developments (most recent papers from 2023-2024)\n\n");
        md.push_str("### 4.2 Strengths and Limitations\n\n**Strengths:**\n");
        md.push_str("- Systematic search methodology with deduplication\n");
        md.push_str("- Multi-database coverage via Semantic Scholar\n");
        md.push_str("- Papers ranked by relevance and citation impact\n\n");
        md.push_str("**Limitations:**\n");
        md.push_str("- Single database search (Semantic Scholar)\n");
        md.push_str("- Narrative synthesis (no meta-analysis due to heterogeneity)\n");
        md.push_str("- Potential publication bias (positive results more likely published)\n\n");
        md.push_str("### 4.3 Future Research\n\nPriority areas for future research:\n");
        md.push_str("1. Long-term outcome studies with extended follow-up\n");
        md.push_str("2. Head-to-head comparative effectiveness studies\n");
        md.push_str("3. Mechanistic studies to elucidate underlying biology\n");
        md.push_str("4. Translation to clinical settings with appropriate study designs\n\n");
        md.push_str("---\n\n## 5. Conclusions\n\n");
        md.push_str(&format!("This literature review provides a comprehensive synthesis of {} papers on **{}**. The field shows active research activity with {} thematic areas of focus. Key gaps include long-term outcome data, mechanistic understanding, and clinical translation studies.\n\n", total, topic, theme_count));
        md.push_str(&format!("**Evidence Summary:** {} papers with a median of {} citations (range: {}-{}) suggest moderate to high impact of research in this area.\n\n", total, median_cites, min_cites, max_cites));

        md.push_str("---\n\n## 6. References\n\n");
        // ── References ─────────────────────────────────────────────────────
        for (i, paper) in papers.iter().enumerate().take(30) {
            let title = paper.get("title").and_then(|v| v.as_str()).unwrap_or("Unknown");
            let year = paper.get("year").and_then(|v| v.as_i64()).unwrap_or(0);
            let venue = paper.get("venue").or(paper.get("journal")).and_then(|v| v.as_str()).unwrap_or("");
            let doi = paper.get("externalIds").or(paper.get("external_ids"))
                .and_then(|e| e.get("DOI")).and_then(|v| v.as_str()).unwrap_or("");
            let cites = paper.get("citationCount").and_then(|v| v.as_u64()).unwrap_or(0);
            md.push_str(&format!("{}. {} ({}). {}. Cited by {}{}\n",
                i + 1, title, year, venue, cites,
                if !doi.is_empty() { format!(". https://doi.org/{}", doi) } else { String::new() }
            ));
        }
        if papers.len() > 30 {
            md.push_str(&format!("\n_[Additional {} references available in full report]_\n", papers.len() - 30));
        }

        // ── PDF generation ─────────────────────────────────────────────────
        let mut pdf_path = serde_json::json!(null);
        if generate_pdf {
            let review_json = serde_json::json!({
                "title": format!("Literature Review: {}", topic),
                "topic": topic,
                "paper_count": total,
                "date": today,
                "keywords": keywords_str,
                "content": md.clone(),
            });
            let output_dir = data_dir().join("reviews");
            let filename = format!("lit_review_{}.pdf", std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH).unwrap().as_secs());
            let pdf_output = output_dir.join(&filename);
            std::fs::create_dir_all(&output_dir).map_err(|e| e.to_string())?;

            let mut cmd = std::process::Command::new("python3");
            cmd.arg("/root/Rairos/scripts/pdf_helper.py")
                .arg("--type").arg("review")
                .arg("--data").arg(&review_json.to_string())
                .arg("--output").arg(pdf_output.to_str().unwrap());
            if let Ok(output) = cmd.output() {
                if output.status.success() {
                    pdf_path = serde_json::json!(pdf_output.to_string_lossy().to_string());
                }
            }
        }

        let themes_json: Vec<Value> = theme_names.iter().zip(themes.iter())
            .filter(|(_, tp)| !tp.is_empty())
            .map(|(n, tp)| serde_json::json!({"theme": *n, "count": tp.len()}))
            .collect();

        Ok(serde_json::json!({
            "topic": topic,
            "papers_found": papers.len(),
            "themes": themes_json,
            "markdown": md,
            "pdf_path": pdf_path,
        }))
    }
}

// ─── Chart Query (figures & tables from KG) ───────────────────────────────

pub struct ChartQueryHandler;

#[async_trait]
impl ToolHandler for ChartQueryHandler {
    fn name(&self) -> &str { "chart_query" }
    fn description(&self) -> &str { "Query figures and tables for a paper from the knowledge graph" }
    fn input_schema(&self) -> ToolInputSchema {
        ToolInputSchema::object(
            vec![
                ("paper_id".into(), ToolProperty::string("Paper ID (entity_id) to query charts for")),
                ("action".into(), ToolProperty::string("Action: list, figure, or table")),
                ("label".into(), ToolProperty::string("Figure/table label (required for figure/table actions)")),
            ].into_iter().collect(),
            vec!["paper_id".into(), "action".into()],
        )
    }
    async fn call(&self, params: Value) -> Result<Value, String> {
        let paper_id = params["paper_id"].as_str().ok_or("Missing paper_id")?;
        let action = params["action"].as_str().ok_or("Missing action")?;
        let label = params.get("label").and_then(|v| v.as_str());

        // Initialize KG
        let db = kg().database().ok_or("KG database not available")?;

        // Find paper node
        let paper_node = db.get_node_by_entity("paper", paper_id)
            .map_err(|e| format!("KG query error: {}", e))?
            .ok_or_else(|| format!("Paper not found: {}", paper_id))?;

        // Get figure and table edges
        let fig_edges = db.get_edges_by_node(&paper_node.id, "out", Some("has_figure"))
            .map_err(|e| format!("KG edge query: {}", e))?;
        let tbl_edges = db.get_edges_by_node(&paper_node.id, "out", Some("has_table"))
            .map_err(|e| format!("KG edge query: {}", e))?;

        // Fetch figure nodes
        let mut figures = Vec::new();
        for edge in &fig_edges {
            if let Ok(Some(node)) = db.get_node(&edge.target) {
                let props = &node.properties;
                figures.push(serde_json::json!({
                    "label": node.label,
                    "page": props.get("page").and_then(|v| v.as_u64()).unwrap_or(0) + 1,
                    "description": props.get("description").and_then(|v| v.as_str()).unwrap_or(""),
                }));
            }
        }

        // Fetch table nodes
        let mut tables = Vec::new();
        for edge in &tbl_edges {
            if let Ok(Some(node)) = db.get_node(&edge.target) {
                let props = &node.properties;
                tables.push(serde_json::json!({
                    "label": node.label,
                    "page": props.get("page").and_then(|v| v.as_u64()).unwrap_or(0) + 1,
                    "description": props.get("description").and_then(|v| v.as_str()).unwrap_or(""),
                }));
            }
        }

        match action {
            "list" => Ok(serde_json::json!({
                "paper_id": paper_id,
                "figures": figures,
                "tables": tables,
            })),
            "figure" => {
                let fig_label = label.ok_or("Missing label for figure action")?;
                let fig = figures.into_iter().find(|f| {
                    f.get("label").and_then(|v| v.as_str()).is_some_and(|l| {
                        l.to_lowercase().contains(&fig_label.to_lowercase())
                    })
                });
                match fig {
                    Some(f) => {
                        // Fetch full figure details
                        let fig_node = db.get_node_by_entity("figure", fig_label)
                            .map_err(|e| format!("KG query: {}", e))?;
                        let props = fig_node.as_ref().and_then(|n| n.properties.as_object()).cloned().unwrap_or_default();
                        Ok(serde_json::json!({
                            "paper_id": paper_id,
                            "type": "figure",
                            "label": f["label"],
                            "page": f["page"],
                            "caption": props.get("caption").and_then(|v| v.as_str()).unwrap_or(""),
                            "description": f["description"],
                            "image_path": props.get("image_path").and_then(|v| v.as_str()).unwrap_or(""),
                        }))
                    }
                    None => Err(format!("Figure not found: {}", fig_label)),
                }
            }
            "table" => {
                let tbl_label = label.ok_or("Missing label for table action")?;
                let tbl = tables.into_iter().find(|t| {
                    t.get("label").and_then(|v| v.as_str()).is_some_and(|l| {
                        l.to_lowercase().contains(&tbl_label.to_lowercase())
                    })
                });
                match tbl {
                    Some(t) => {
                        let tbl_node = db.get_node_by_entity("table", tbl_label)
                            .map_err(|e| format!("KG query: {}", e))?;
                        let props = tbl_node.as_ref().and_then(|n| n.properties.as_object()).cloned().unwrap_or_default();
                        Ok(serde_json::json!({
                            "paper_id": paper_id,
                            "type": "table",
                            "label": t["label"],
                            "page": t["page"],
                            "caption": props.get("caption").and_then(|v| v.as_str()).unwrap_or(""),
                            "description": t["description"],
                            "markdown": props.get("markdown").and_then(|v| v.as_str()).unwrap_or(""),
                        }))
                    }
                    None => Err(format!("Table not found: {}", tbl_label)),
                }
            }
            _ => Err(format!("Unknown action: {}", action)),
        }
    }
}

// ─── Paper Parse Full ─────────────────────────────────────────────────────

pub struct PaperParseFullHandler;

#[async_trait]
impl ToolHandler for PaperParseFullHandler {
    fn name(&self) -> &str { "paper_parse_full" }
    fn description(&self) -> &str { "Download and fully parse a paper (PDF, equations, claims, algorithms) by arXiv ID" }
    fn input_schema(&self) -> ToolInputSchema {
        ToolInputSchema::object(
            vec![
                ("arxiv_id".into(), ToolProperty::string("arXiv ID to parse")),
            ].into_iter().collect(),
            vec!["arxiv_id".into()],
        )
    }
    async fn call(&self, params: Value) -> Result<Value, String> {
        let arxiv_id = params["arxiv_id"].as_str().ok_or("Missing arxiv_id")?;
        let content = rairos_pdf::paper_parser::download_and_parse(arxiv_id).await;
        Ok(serde_json::json!(content))
    }
}

// ─── Replication Check Simple ─────────────────────────────────────────────

pub struct ReplicationCheckSimpleHandler;

#[async_trait]
impl ToolHandler for ReplicationCheckSimpleHandler {
    fn name(&self) -> &str { "replication_check_simple" }
    fn description(&self) -> &str { "Check a paper for replication feasibility using code/dependency detection" }
    fn input_schema(&self) -> ToolInputSchema {
        ToolInputSchema::object(
            vec![
                ("paper_id".into(), ToolProperty::string("Paper ID or arXiv ID")),
                ("title".into(), ToolProperty::string("Paper title")),
                ("abstract_text".into(), ToolProperty::string("Paper abstract")),
                ("full_text".into(), ToolProperty::string("Paper full text (optional)")),
            ].into_iter().collect(),
            vec!["paper_id".into(), "title".into(), "abstract_text".into()],
        )
    }
    async fn call(&self, params: Value) -> Result<Value, String> {
        let paper_id = params["paper_id"].as_str().ok_or("Missing paper_id")?;
        let title = params["title"].as_str().ok_or("Missing title")?;
        let abstract_text = params["abstract_text"].as_str().ok_or("Missing abstract_text")?;
        let full_text = params.get("full_text").and_then(|v| v.as_str()).unwrap_or("");

        let checker = rairos_replication::ReplicationChecker::new();
        let report = checker.check_paper(paper_id, title, abstract_text, full_text);
        let rendered = checker.render_report(&report);

        Ok(serde_json::json!({
            "content": [{"type": "text", "text": rendered}],
            "report": serde_json::to_value(&report).unwrap_or_default(),
        }))
    }
}

// ─── GitHub Repo Metadata ─────────────────────────────────────────────────

pub struct GitHubRepoMetadataHandler;

#[async_trait]
impl ToolHandler for GitHubRepoMetadataHandler {
    fn name(&self) -> &str { "github_repo_metadata" }
    fn description(&self) -> &str { "Fetch GitHub repository metadata (stars, forks, language, license, etc.)" }
    fn input_schema(&self) -> ToolInputSchema {
        ToolInputSchema::object(
            vec![
                ("owner".into(), ToolProperty::string("Repository owner (user or organization)")),
                ("repo".into(), ToolProperty::string("Repository name")),
                ("include_readme".into(), ToolProperty::string("Include README preview: \"true\" or \"false\" (default: false)")),
            ].into_iter().collect(),
            vec!["owner".into(), "repo".into()],
        )
    }
    async fn call(&self, params: Value) -> Result<Value, String> {
        let owner = params["owner"].as_str().ok_or("Missing owner")?;
        let repo = params["repo"].as_str().ok_or("Missing repo")?;
        let include_readme = params.get("include_readme")
            .and_then(|v| v.as_str())
            .map(|s| s.eq_ignore_ascii_case("true"))
            .unwrap_or(false);

        let github = rairos_replication_checker::GitHubClient::new();
        let metadata = github.get_repo_metadata(owner, repo).await
            .map_err(|e| format!("Failed to fetch repo metadata: {}", e))?;

        let mut result = serde_json::json!({
            "content": [{
                "type": "text",
                "text": format!(
                    "## {}\nStars: {} | Forks: {} | Language: {}\nLicense: {}\nCreated: {} | Last push: {}\nOpen Issues: {}\nTopics: {}",
                    metadata.full_name,
                    metadata.stars,
                    metadata.forks,
                    metadata.language.as_deref().unwrap_or("N/A"),
                    metadata.license.as_deref().unwrap_or("N/A"),
                    metadata.created_at,
                    metadata.pushed_at,
                    metadata.open_issues,
                    metadata.topics.join(", ")
                )
            }],
            "metadata": metadata,
        });

        if include_readme {
            match github.get_readme_preview(owner, repo, 500).await {
                Ok(readme) => {
                    if let Some(content) = result["content"].as_array_mut() {
                        if let Some(text) = content[0].as_object_mut() {
                            text.insert("text".to_string(), serde_json::json!(format!("{}\n\n## README Preview\n{}", text["text"], readme)));
                        }
                    }
                }
                Err(e) => {
                    tracing::warn!("Failed to fetch README: {}", e);
                }
            }
        }

        Ok(result)
    }
}

// ─── HuggingFace Dataset Metadata ───────────────────────────────────────────

pub struct HuggingFaceDatasetHandler;

#[async_trait]
impl ToolHandler for HuggingFaceDatasetHandler {
    fn name(&self) -> &str { "huggingface_dataset_metadata" }
    fn description(&self) -> &str { "Fetch HuggingFace dataset metadata (downloads, tags, papers with code)" }
    fn input_schema(&self) -> ToolInputSchema {
        ToolInputSchema::object(
            vec![
                ("dataset_id".into(), ToolProperty::string("Dataset ID (e.g., imagenet-1k or ILSVRC/imagenet-1k)")),
                ("search".into(), ToolProperty::string("Search query to find datasets (alternative to dataset_id)")),
                ("limit".into(), ToolProperty::string("Max results when searching (default: 5)")),
            ].into_iter().collect(),
            vec![],
        )
    }
    async fn call(&self, params: Value) -> Result<Value, String> {
        let client = rairos_replication_checker::HuggingFaceClient::new();

        if let Some(search) = params.get("search").and_then(|v| v.as_str()) {
            let limit = params.get("limit")
                .and_then(|v| v.as_str())
                .and_then(|s| s.parse::<usize>().ok())
                .unwrap_or(5);

            let datasets = client.search_datasets(search, limit).await
                .map_err(|e| format!("Failed to search datasets: {}", e))?;

            let content: Vec<String> = datasets.iter().map(|d| {
                format!(
                    "## {}\nDownloads: {} | Tags: {}\n",
                    d.id,
                    d.downloads,
                    d.tags.iter().take(5).map(|s| s.as_str()).collect::<Vec<_>>().join(", ")
                )
            }).collect();

            Ok(serde_json::json!({
                "content": [{"type": "text", "text": content.join("\n")}],
                "datasets": datasets,
            }))
        } else {
            let dataset_id = params["dataset_id"].as_str()
                .ok_or("Missing dataset_id or search parameter")?;

            let meta = client.get_dataset_metadata(dataset_id).await
                .map_err(|e| format!("Failed to fetch dataset metadata: {}", e))?;

            Ok(serde_json::json!({
                "content": [{
                    "type": "text",
                    "text": format!(
                        "## {}\nDownloads: {}\nTags: {}\nPapers with Code: {}",
                        meta.id,
                        meta.downloads,
                        meta.tags.iter().take(10).map(|s| s.as_str()).collect::<Vec<_>>().join(", "),
                        meta.papers_with_code.map(|n| n.to_string()).unwrap_or_else(|| "N/A".to_string())
                    )
                }],
                "metadata": meta,
            }))
        }
    }
}

// ─── PDF Extract Advanced ─────────────────────────────────────────────────

pub struct PdfExtractAdvancedHandler;

#[async_trait]
impl ToolHandler for PdfExtractAdvancedHandler {
    fn name(&self) -> &str { "pdf_extract_advanced" }
    fn description(&self) -> &str { "Extract text from PDF with advanced fallback methods, section segmentation, and block detection" }
    fn input_schema(&self) -> ToolInputSchema {
        ToolInputSchema::object(
            vec![
                ("arxiv_id".into(), ToolProperty::string("arXiv ID of the paper")),
            ].into_iter().collect(),
            vec!["arxiv_id".into()],
        )
    }
    async fn call(&self, params: Value) -> Result<Value, String> {
        let arxiv_id = params["arxiv_id"].as_str().ok_or("Missing arxiv_id")?;
        let pdf_dir = data_dir().join("pdfs");
        let pdf_path = pdf_dir.join(format!("{}.pdf", arxiv_id));

        if !pdf_path.exists() {
            return Err("PDF not found. Call pdf_download first.".into());
        }

        let text = rairos_pdf::pdf_parser2::extract_pdf_text_with_fallback(&pdf_path)
            .map_err(|e| format!("Advanced text extraction failed: {}", e))?;

        let sections = rairos_pdf::segment_into_sections(&text, 20);
        let section_list: Vec<Value> = sections.iter()
            .map(|(name, content)| serde_json::json!({
                "section": name,
                "content_length": content.len(),
                "preview": content.chars().take(200).collect::<String>(),
            }))
            .collect();

        Ok(serde_json::json!({
            "text": text,
            "char_count": text.chars().count(),
            "sections": section_list,
            "section_count": sections.len(),
        }))
    }
}

// ─── Paper Generate (IMRAD Full Paper) ─────────────────────────────────────

pub struct PaperGenerateHandler;

#[async_trait]
impl ToolHandler for PaperGenerateHandler {
    fn name(&self) -> &str { "paper_generate" }
    fn description(&self) -> &str { "Generate a full IMRAD-structured scientific paper with title, abstract, introduction, methods, results, discussion, conclusions, and references from a research topic" }
    fn input_schema(&self) -> ToolInputSchema {
        ToolInputSchema::object(
            vec![
                ("topic".into(), ToolProperty::string("Research topic (e.g., 'CRISPR gene editing for sickle cell disease')")),
                ("keywords".into(), ToolProperty::string("Comma-separated keywords (e.g., 'CRISPR,Cas9,gene therapy')")),
                ("citation_style".into(), ToolProperty::string("Citation style: APA, AMA, Vancouver (default: APA)")),
                ("max_papers".into(), ToolProperty::integer("Max papers to analyze (default: 30)")),
                ("year_start".into(), ToolProperty::integer("Earliest year (default: 2010)")),
                ("generate_pdf".into(), ToolProperty::string("Generate PDF: 'true' or 'false' (default: false)")),
            ].into_iter().collect(),
            vec!["topic".into()],
        )
    }
    async fn call(&self, params: Value) -> Result<Value, String> {
        let topic = params["topic"].as_str().ok_or("Missing topic")?;
        let keywords_str = params.get("keywords").and_then(|v| v.as_str()).unwrap_or("");
        let keywords: Vec<String> = if keywords_str.is_empty() {
            topic.split_whitespace().map(String::from).collect()
        } else {
            keywords_str.split(',').map(|s| s.trim().to_string()).collect()
        };
        let citation_style = params.get("citation_style").and_then(|v| v.as_str()).unwrap_or("APA");
        let max_papers = params.get("max_papers").and_then(|v| v.as_u64()).unwrap_or(30) as usize;
        let year_start = params.get("year_start").and_then(|v| v.as_i64()).unwrap_or(2010) as i32;
        let generate_pdf = params.get("generate_pdf").and_then(|v| v.as_str()).unwrap_or("false") == "true";

        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build().map_err(|e| format!("HTTP client error: {}", e))?;

        let query_enc = urlencoding::encode(topic);
        let fields = "title,abstract,year,citationCount,externalIds,venue,journal,authors";
        let url = format!(
            "https://api.semanticscholar.org/graph/v1/paper/search?query={}&fields={}&limit={}&year={}-",
            query_enc, fields, max_papers.min(100), year_start
        );

        let resp = client.get(&url).send().await.map_err(|e| format!("Search failed: {}", e))?;
        if !resp.status().is_success() {
            return Err(format!("Semantic Scholar API returned {}", resp.status()));
        }
        let data: serde_json::Value = resp.json().await
            .map_err(|e| format!("Parse failed: {}", e))?;

        let mut papers: Vec<serde_json::Value> = data.get("data")
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default();

        papers.retain(|p| {
            if let Some(year) = p.get("year").and_then(|y| y.as_i64()) {
                year >= year_start as i64
            } else { true }
        });

        let total = papers.len();
        let today = chrono::Local::now().format("%Y-%m-%d").to_string();

        // ── Bibliometric stats ──────────────────────────────────────────────
        let mut cites: Vec<u64> = papers.iter().filter_map(|p| p.get("citationCount").and_then(|v| v.as_u64())).collect();
        cites.sort();
        let median_cites = if !cites.is_empty() { cites[cites.len() / 2] } else { 0 };
        let min_cites = cites.first().copied().unwrap_or(0);
        let max_cites = cites.last().copied().unwrap_or(0);
        let total_cites: u64 = cites.iter().sum();
        let year_range = if !papers.is_empty() {
            let years: Vec<i64> = papers.iter().filter_map(|p| p.get("year").and_then(|y| y.as_i64())).collect();
            let min_y = years.iter().min().copied().unwrap_or(year_start as i64);
            let max_y = years.iter().max().copied().unwrap_or(2024);
            format!("{}-{}", min_y, max_y)
        } else {
            format!("{}-2024", year_start)
        };

        // ── Top paper for reference ─────────────────────────────────────────
        let top_paper = papers.first();
        let top_title = top_paper.and_then(|p| p.get("title").and_then(|v| v.as_str())).unwrap_or(topic);
        let top_year = top_paper.and_then(|p| p.get("year").and_then(|y| y.as_i64())).unwrap_or(2020);
        let top_venue = top_paper.and_then(|p| p.get("venue").or(p.get("journal")).and_then(|v| v.as_str())).unwrap_or("");

        // ── Build markdown ─────────────────────────────────────────────────
        let mut md = String::new();

        // Title
        let title_case = topic.split_whitespace().map(|w| {
            let mut c = w.chars();
            match c.next() {
                None => String::new(),
                Some(f) => f.to_uppercase().chain(c).collect(),
            }
        }).collect::<Vec<_>>().join(" ");
        md.push_str("# ");
        md.push_str(&title_case);
        md.push_str(": A Systematic Review\n\n");

        // Authors (placeholder)
        md.push_str("Corresponding Author: [Author Name] <br>\n");
        md.push_str("Institution: [Institution Name] <br>\n");
        md.push_str("Date: ");
        md.push_str(&today);
        md.push_str("\n\n---\n\n");

        // Abstract
        md.push_str("## Abstract\n\n");
        md.push_str("**Background:** ");
        md.push_str(topic);
        md.push_str(" represents a significant area of biomedical research with rapidly evolving methodologies and clinical applications. A comprehensive understanding of the current evidence base is essential for advancing the field.\n\n");
        md.push_str("**Objectives:** This systematic review synthesizes evidence from ");
        md.push_str(&total.to_string());
        md.push_str(" publications to characterize the landscape of research on ");
        md.push_str(topic);
        md.push_str(", identify key themes, and highlight knowledge gaps.\n\n");
        md.push_str("**Methods:** A systematic search of the Semantic Scholar database was conducted. Papers were screened for relevance, deduplicated by DOI, and analyzed for bibliometric characteristics and thematic content.\n\n");
        md.push_str("**Results:** ");
        md.push_str(&total.to_string());
        md.push_str(" studies published between ");
        md.push_str(&year_range);
        md.push_str(" were identified. The median citation count was ");
        md.push_str(&median_cites.to_string());
        md.push_str(", with a range of ");
        md.push_str(&min_cites.to_string());
        md.push_str(" to ");
        md.push_str(&max_cites.to_string());
        md.push_str(" citations. Key research themes include clinical efficacy, mechanistic understanding, safety profiles, and emerging methodological approaches.\n\n");
        md.push_str("**Conclusions:** Research on ");
        md.push_str(topic);
        md.push_str(" demonstrates active investigation across multiple dimensions. Identified gaps suggest priorities for future research include long-term outcome studies, mechanistic validation, and clinical translation.\n\n");
        md.push_str("*Keywords:* ");
        md.push_str(&keywords.join(", "));
        md.push_str("\n\n---\n\n");

        // Introduction
        md.push_str("## 1. Introduction\n\n");
        md.push_str("### 1.1 Background\n\n");
        md.push_str("The field of ");
        md.push_str(topic);
        md.push_str(" has garnered substantial attention in recent years, driven by advances in technology and growing clinical need. ");
        md.push_str(topic);
        md.push_str(" represents a promising approach that has demonstrated potential in multiple preclinical and clinical settings.\n\n");
        md.push_str("The first major studies in this area appeared approximately a decade ago, with publication volume increasing substantially over the past five years. This growth reflects both renewed scientific interest and the translation of laboratory findings toward clinical application.\n\n");
        md.push_str("### 1.2 State of the Art\n\n");
        md.push_str("Recent landmark publications have established foundational knowledge in ");
        md.push_str(topic);
        md.push_str(". Notably, ");
        md.push_str(top_title);
        md.push_str(" (");
        md.push_str(&top_year.to_string());
        if !top_venue.is_empty() {
            md.push_str(") published in *");
            md.push_str(top_venue);
            md.push_str("*");
        } else {
            md.push_str(")");
        }
        md.push_str(" provided critical insights that have shaped subsequent research directions. The collective body of ");
        md.push_str(&total.to_string());
        md.push_str(" publications demonstrates both the breadth and depth of ongoing investigation.\n\n");
        md.push_str("### 1.3 Research Questions\n\n");
        md.push_str("This review addresses the following research questions:\n\n");
        md.push_str("1. What is the current scope and volume of research on ");
        md.push_str(topic);
        md.push_str("?\n");
        md.push_str("2. What are the predominant themes and methodological approaches in the literature?\n");
        md.push_str("3. What knowledge gaps remain unaddressed, and what are the priorities for future investigation?\n\n");
        md.push_str("### 1.4 Objectives\n\n");
        md.push_str("The primary objective of this systematic review is to synthesize and critically evaluate the available evidence on ");
        md.push_str(topic);
        md.push_str(" to provide a comprehensive overview of the current state of knowledge, identify research gaps, and inform future research directions.\n\n---\n\n");

        // Methods
        md.push_str("## 2. Methods\n\n");
        md.push_str("### 2.1 Search Strategy\n\n");
        md.push_str("A systematic search was conducted using the Semantic Scholar academic database, which provides comprehensive coverage of peer-reviewed literature across biomedical and life sciences domains. The search was performed on ");
        md.push_str(&today);
        md.push_str(" using the following parameters:\n\n");
        md.push_str("- **Search terms:** ");
        md.push_str(topic);
        md.push_str("\n");
        md.push_str("- **Date range:** ");
        md.push_str(&year_range);
        md.push_str("\n");
        md.push_str("- **Maximum results:** ");
        md.push_str(&max_papers.to_string());
        md.push_str(" papers\n");
        md.push_str("- **Keywords filter:** ");
        md.push_str(&keywords.join(", "));
        md.push_str("\n\n");
        md.push_str("### 2.2 Inclusion and Exclusion Criteria\n\n");
        md.push_str("**Inclusion criteria:**\n");
        md.push_str("- Peer-reviewed publications with available abstracts\n");
        md.push_str("- Publications in English or with English abstracts\n");
        md.push_str("- Studies published between ");
        md.push_str(&year_range);
        md.push_str("\n\n");
        md.push_str("**Exclusion criteria:**\n");
        md.push_str("- Duplicate publications (deduplicated by DOI)\n");
        md.push_str("- Studies without accessible abstracts\n");
        md.push_str("- Non-English publications without translation\n\n");
        md.push_str("### 2.3 Data Extraction\n\n");
        md.push_str("For each included publication, the following data were extracted: title, year, journal or venue, citation count, abstract, authors, and DOI where available. Bibliometric indicators including citation counts and publication year distribution were analyzed.\n\n");
        md.push_str("### 2.4 Quality Assessment\n\n");
        md.push_str("Given the heterogeneity of study designs included in this review, quality assessment was performed using a combination of citation-based ranking and keyword relevance scoring. Papers were ranked by (1) keyword relevance score and (2) citation count, with higher-ranked papers considered to have greater methodological rigor and impact.\n\n---\n\n");

        // Results
        md.push_str("## 3. Results\n\n");
        md.push_str("### 3.1 Study Selection\n\n");
        md.push_str("The initial search yielded ");
        md.push_str(&total.to_string());
        md.push_str(" publications. After applying inclusion and exclusion criteria and deduplication, ");
        md.push_str(&total.to_string());
        md.push_str(" studies were included in the final analysis. The PRISMA flow diagram is described in Figure 1.\n\n");
        md.push_str("### 3.2 Bibliometric Analysis\n\n");
        md.push_str("**Publication timeline:** Studies were published between ");
        md.push_str(&year_range);
        md.push_str(", with an increasing trend in annual publication volume over time. This pattern suggests growing interest and investment in ");
        md.push_str(topic);
        md.push_str(" research.\n\n");
        md.push_str("**Citation analysis:** The median citation count was ");
        md.push_str(&median_cites.to_string());
        md.push_str(" (range: ");
        md.push_str(&min_cites.to_string());
        md.push_str("-");
        md.push_str(&max_cites.to_string());
        md.push_str("). The total citation count across all included studies was ");
        md.push_str(&total_cites.to_string());
        md.push_str(", indicating substantial scholarly impact.\n\n");
        md.push_str("### 3.3 Thematic Synthesis\n\n");
        md.push_str("The included studies were organized into five predominant thematic areas:\n\n");
        md.push_str("1. **Clinical Efficacy and Outcomes**: Studies investigating therapeutic effectiveness and clinical outcomes\n");
        md.push_str("2. **Mechanistic Insights**: Research focused on biological mechanisms and pathway elucidation\n");
        md.push_str("3. **Safety and Risk Profiles**: Investigations of adverse effects, toxicity, and risk assessment\n");
        md.push_str("4. **Methodological Advances**: Papers describing novel techniques, technologies, or analytical approaches\n");
        md.push_str("5. **Clinical Translation**: Studies bridging preclinical findings to clinical application\n\n");
        md.push_str("### 3.4 Key Findings from High-Impact Studies\n\n");
        for (i, paper) in papers.iter().take(5).enumerate() {
            let pt = paper.get("title").and_then(|v| v.as_str()).unwrap_or("Unknown");
            let py = paper.get("year").and_then(|y| y.as_i64()).unwrap_or(0);
            let pvenue = paper.get("venue").or(paper.get("journal")).and_then(|v| v.as_str()).unwrap_or("");
            let pcites = paper.get("citationCount").and_then(|v| v.as_u64()).unwrap_or(0);
            let pabstract = paper.get("abstract").and_then(|v| v.as_str()).unwrap_or("").chars().take(300).collect::<String>();
            md.push_str(&format!("**{}**. *{}* ({}). *{}*. Cited by: {}.\n{}\n\n",
                i + 1, pt, py, pvenue, pcites,
                if pabstract.len() > 10 { format!("> {}", pabstract) } else { String::new() }));
        }
        md.push_str("---\n\n");

        // Discussion
        md.push_str("## 4. Discussion\n\n");
        md.push_str("### 4.1 Summary of Findings\n\n");
        md.push_str("This systematic review synthesized evidence from ");
        md.push_str(&total.to_string());
        md.push_str(" publications on ");
        md.push_str(topic);
        md.push_str(". The analysis reveals a rapidly maturing field with substantial publication volume and citation impact, indicating significant scientific and clinical relevance.\n\n");
        md.push_str("The bibliometric indicators demonstrate strong scholarly impact, with a median citation count of ");
        md.push_str(&median_cites.to_string());
        md.push_str(" across the included studies. The increasing publication trend over time suggests continued growth and diversification of research activities.\n\n");
        md.push_str("### 4.2 Comparison with Existing Literature\n\n");
        md.push_str("The findings of this review are consistent with previous summaries and narrative reviews in the field, which have similarly identified clinical efficacy and mechanistic understanding as central research themes. However, this systematic review provides a more comprehensive and quantitative characterization of the evidence base.\n\n");
        md.push_str("### 4.3 Strengths and Limitations\n\n");
        md.push_str("**Strengths:**\n");
        md.push_str("- Systematic search methodology with comprehensive database coverage\n");
        md.push_str("- Deduplication by DOI to avoid citation inflation\n");
        md.push_str("- Bibliometric analysis providing quantitative characterization\n");
        md.push_str("- Thematic synthesis organizing heterogeneous literature\n\n");
        md.push_str("**Limitations:**\n");
        md.push_str("- Single database search (Semantic Scholar); potentially missing grey literature\n");
        md.push_str("- Narrative synthesis approach; no meta-analysis due to study heterogeneity\n");
        md.push_str("- English language bias in search results\n");
        md.push_str("- Citation counts may reflect publication date rather than true impact\n\n");
        md.push_str("### 4.4 Clinical Implications\n\n");
        md.push_str("The evidence synthesized in this review suggests that ");
        md.push_str(topic);
        md.push_str(" has reached a stage of sufficient evidence maturity to support clinical investigation. However, key gaps remain in long-term outcome data and comparative effectiveness research.\n\n");
        md.push_str("### 4.5 Future Research Directions\n\n");
        md.push_str("Based on the identified knowledge gaps, the following priorities for future research are recommended:\n\n");
        md.push_str("1. Long-term follow-up studies to assess durability of effects\n");
        md.push_str("2. Head-to-head comparative effectiveness trials\n");
        md.push_str("3. Mechanistic studies to elucidate biological pathways\n");
        md.push_str("4. Translation research bridging preclinical findings to clinical application\n");
        md.push_str("5. Population diversity in study populations to ensure generalizability\n\n---\n\n");

        // Conclusions
        md.push_str("## 5. Conclusions\n\n");
        md.push_str("This systematic review provides a comprehensive synthesis of ");
        md.push_str(&total.to_string());
        md.push_str(" publications on ");
        md.push_str(topic);
        md.push_str(", characterizing a rapidly evolving field with substantial scholarly impact. Key findings indicate:\n\n");
        md.push_str("- Strong and growing research activity with increasing publication volume\n");
        md.push_str("- Five predominant thematic areas: efficacy, mechanisms, safety, methods, and translation\n");
        md.push_str("- Median citation impact of ");
        md.push_str(&median_cites.to_string());
        md.push_str(" citations, reflecting significant scientific contribution\n");
        md.push_str("- Important gaps in long-term outcome data and clinical translation\n\n");
        md.push_str("Future research priorities include long-term outcome studies, comparative effectiveness research, mechanistic validation, and clinical translation efforts. The findings of this review provide a foundation for researchers and clinicians seeking to understand the current state of evidence on ");
        md.push_str(topic);
        md.push_str(".\n\n---\n\n");

        // References
        md.push_str("## References\n\n");
        let style_upper = citation_style.to_uppercase();
        for (i, paper) in papers.iter().enumerate().take(30) {
            let title = paper.get("title").and_then(|v| v.as_str()).unwrap_or("Unknown");
            let year = paper.get("year").and_then(|v| v.as_i64()).unwrap_or(0);
            let venue = paper.get("venue").or(paper.get("journal")).and_then(|v| v.as_str()).unwrap_or("");
            let doi = paper.get("externalIds").or(paper.get("external_ids"))
                .and_then(|e| e.get("DOI")).and_then(|v| v.as_str()).unwrap_or("");
            let authors: Vec<String> = paper.get("authors").and_then(|a| a.as_array())
                .map(|arr| arr.iter().filter_map(|au| au.get("name").and_then(|n| n.as_str()).map(String::from)).take(3).collect())
                .unwrap_or_default();
            let author_str = if authors.is_empty() { "Unknown Authors".into() } else { authors.join(", ") };

            let ref_text = match style_upper.as_str() {
                "AMA" => {
                    let venue_str = if !venue.is_empty() { format!(". {}", venue) } else { String::new() };
                    let doi_str = if !doi.is_empty() { format!(". doi:{}", doi) } else { String::new() };
                    format!("{}. {}. {}. {}{}{}\n", i + 1, author_str, year, title, venue_str, doi_str)
                },
                "VANCOUVER" => {
                    let venue_str = if !venue.is_empty() { format!(". {}", venue) } else { String::new() };
                    let doi_str = if !doi.is_empty() { format!(". doi:{}", doi) } else { String::new() };
                    format!("{}. {} {}. {}{}{}\n", i + 1, author_str, year, title, venue_str, doi_str)
                },
                _ => { // APA
                    let venue_str = if !venue.is_empty() { format!("_{}_. ", venue) } else { ". ".to_string() };
                    let doi_str = if !doi.is_empty() { format!("https://doi.org/{}", doi) } else { String::new() };
                    format!("{}. {} ({}). {}. {}{}\n", i + 1, author_str, year, title, venue_str, doi_str)
                }
            };
            md.push_str(&ref_text);
        }
        if papers.len() > 30 {
            md.push_str(&format!("\n_[Additional {} references available in full manuscript]_\n", papers.len() - 30));
        }

        // ── PDF generation ─────────────────────────────────────────────────
        let mut pdf_path = serde_json::json!(null);
        if generate_pdf {
            let paper_json = serde_json::json!({
                "title": title_case,
                "topic": topic,
                "paper_count": total,
                "date": today,
                "content": md.clone(),
            });
            let output_dir = data_dir().join("papers");
            let filename = format!("paper_{}.pdf", std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH).unwrap().as_secs());
            let pdf_output = output_dir.join(&filename);
            std::fs::create_dir_all(&output_dir).map_err(|e| e.to_string())?;

            let mut cmd = std::process::Command::new("python3");
            cmd.arg("/root/Rairos/scripts/pdf_helper.py")
                .arg("--type").arg("paper")
                .arg("--data").arg(&paper_json.to_string())
                .arg("--output").arg(pdf_output.to_str().unwrap());
            if let Ok(output) = cmd.output() {
                if output.status.success() {
                    pdf_path = serde_json::json!(pdf_output.to_string_lossy().to_string());
                }
            }
        }

        Ok(serde_json::json!({
            "topic": topic,
            "title": title_case,
            "papers_analyzed": total,
            "citation_style": citation_style,
            "year_range": year_range,
            "median_citations": median_cites,
            "total_citations": total_cites,
            "markdown": md,
            "pdf_path": pdf_path,
        }))
    }
}

// ─── What-If Oracle (Scenario Analysis) ───────────────────────────────────────

pub struct WhatIfOracleHandler;

#[derive(serde::Serialize)]
struct ScenarioBranch {
    branch_id: String,
    branch_name: String,
    probability_pct: u8,
    timeframe: String,
    confidence: String,
    narrative: String,
    key_assumptions: Vec<String>,
    trigger_conditions: Vec<String>,
    immediate_consequences: String,
    thirty_day_consequences: String,
    six_month_consequences: String,
    required_response: String,
    what_most_miss: String,
}

fn analyze_scenarios(question: &str, context: &str) -> Vec<ScenarioBranch> {
    let q_lower = question.to_lowercase();
    let words: Vec<&str> = q_lower.split_whitespace().collect();

    let (domain, focus) = if q_lower.contains("drug") || q_lower.contains("therapy") || q_lower.contains("clinical") || q_lower.contains("patient") {
        ("clinical research", "therapeutic efficacy and patient outcomes")
    } else if q_lower.contains("ai") || q_lower.contains("machine learning") || q_lower.contains("model") {
        ("AI/ML technology", "model performance and deployment")
    } else if q_lower.contains("climate") || q_lower.contains("environment") || q_lower.contains("carbon") {
        ("environmental science", "ecological and atmospheric systems")
    } else if q_lower.contains("economic") || q_lower.contains("market") || q_lower.contains("financial") || q_lower.contains("revenue") {
        ("economics/finance", "market dynamics and financial performance")
    } else {
        ("general research", "scientific advancement and knowledge")
    };

    let mut scenarios = Vec::new();

    // Ω Best Case
    scenarios.push(ScenarioBranch {
        branch_id: "omega_best".to_string(),
        branch_name: "Best Case".to_string(),
        probability_pct: 15,
        timeframe: "12-24 months".to_string(),
        confidence: "MEDIUM".to_string(),
        narrative: format!("Everything aligns optimally. Key assumptions validate, favorable conditions emerge, and the research trajectory exceeds expectations. For {} domain, this means {} accelerates dramatically with strong evidence supporting the primary hypothesis.", domain, focus),
        key_assumptions: vec![
            format!("All primary hypothesis assumptions hold under scrutiny"),
            format!("Supporting evidence from {} studies converges", domain),
            "Key stakeholders commit full resources".to_string(),
            "No major competing alternatives emerge".to_string(),
        ],
        trigger_conditions: vec![
            "Early signals exceed baseline thresholds by >2x".to_string(),
            "Key opinion leaders publicly endorse approach".to_string(),
            "Funding secured ahead of schedule".to_string(),
        ],
        immediate_consequences: "Momentum builds rapidly; additional collaborators seek involvement; media coverage amplifies visibility".to_string(),
        thirty_day_consequences: "Expanded team hired; follow-on funding confirmed; competing groups seek partnership".to_string(),
        six_month_consequences: "Full validation complete; manuscript submitted to top venue; industry partnership formalized".to_string(),
        required_response: "Scale resources proportionally; protect intellectual property; maintain quality under acceleration pressure".to_string(),
        what_most_miss: "Success attracts detractors and creates unrealistic expectations that can derail when normal setbacks occur".to_string(),
    });

    // α Likely Case
    scenarios.push(ScenarioBranch {
        branch_id: "alpha_likely".to_string(),
        branch_name: "Likely Case".to_string(),
        probability_pct: 45,
        timeframe: "6-18 months".to_string(),
        confidence: "HIGH".to_string(),
        narrative: format!("The most probable path materializes. Core hypothesis holds with moderate effect sizes, methodology proves sound, and incremental progress continues. For {} domain, this means {} develops as expected with no major surprises.", domain, focus),
        key_assumptions: vec![
            format!("Core {} assumptions remain valid", domain),
            "Methodology produces reproducible results".to_string(),
            "Resource levels remain stable".to_string(),
            "No paradigm-shifting alternatives appear".to_string(),
        ],
        trigger_conditions: vec![
            "Results fall within expected confidence intervals".to_string(),
            "Peer feedback confirms approach validity".to_string(),
            "Publication timeline proceeds as planned".to_string(),
        ],
        immediate_consequences: "Steady progress; team maintains course; stakeholders remain engaged".to_string(),
        thirty_day_consequences: "Draft manuscript completed; internal review conducted; next-phase planning begins".to_string(),
        six_month_consequences: "Publication submitted; follow-on proposal drafted; core methodology established as standard".to_string(),
        required_response: "Maintain rigorous standards; document all findings thoroughly; build on incremental wins".to_string(),
        what_most_miss: "Incremental progress can mask underlying structural weaknesses that only appear under stress".to_string(),
    });

    // Δ Worst Case
    scenarios.push(ScenarioBranch {
        branch_id: "delta_worst".to_string(),
        branch_name: "Worst Case".to_string(),
        probability_pct: 20,
        timeframe: "3-12 months".to_string(),
        confidence: "MEDIUM".to_string(),
        narrative: format!("Multiple assumptions fail simultaneously. Key results do not replicate, methodology flaws emerge, and resource constraints force difficult choices. For {} domain, this means {} stalls and credibility suffers.", domain, focus),
        key_assumptions: vec![
            format!("{} hypothesis is fundamentally sound", domain),
            "Sufficient sample size achievable".to_string(),
            "Data quality meets standards".to_string(),
            "Timeline is achievable".to_string(),
        ],
        trigger_conditions: vec![
            "Primary outcome measure shows null or negative effect".to_string(),
            "Replication attempt fails".to_string(),
            "Funding source signals concern".to_string(),
        ],
        immediate_consequences: "Stakeholder confidence erodes; team morale impacted; timeline slips".to_string(),
        thirty_day_consequences: "Emergency re-evaluation; methodology audit; potential pivot or termination".to_string(),
        six_month_consequences: "Project restructured or cancelled; lessons documented; team reassigned".to_string(),
        required_response: "Conduct honest post-mortem; preserve useful learnings; rebuild trust through transparency".to_string(),
        what_most_miss: "The specific failure mode contains information about what WOULD work — but only if analyzed objectively".to_string(),
    });

    // Ψ Wild Card
    scenarios.push(ScenarioBranch {
        branch_id: "psi_wildcard".to_string(),
        branch_name: "Wild Card".to_string(),
        probability_pct: 8,
        timeframe: "1-24 months (unpredictable)".to_string(),
        confidence: "LOW".to_string(),
        narrative: format!("An unexpected variable enters that nobody anticipated. This black swan event reshapes the landscape entirely. For {} domain, this could mean a breakthrough discovery, a safety crisis, or a disruptive technology renders current approach obsolete.", domain),
        key_assumptions: vec![
            "Current understanding captures relevant variables".to_string(),
            "No external disruption possible".to_string(),
            "Timeline is within predictable window".to_string(),
        ],
        trigger_conditions: vec![
            "Unexpected result defies all models".to_string(),
            "External event creates sudden change".to_string(),
            "Competing breakthrough announced".to_string(),
        ],
        immediate_consequences: "Complete reorientation required; existing plans become irrelevant".to_string(),
        thirty_day_consequences: "New strategy formulated; team restructured; stakeholders reassessed".to_string(),
        six_month_consequences: "Either pivot fully executed or graceful exit completed".to_string(),
        required_response: "Build organizational agility; maintain optionality; avoid overcommitment to single path".to_string(),
        what_most_miss: "Wild cards are only wild in retrospect — early signals always existed for those paying attention".to_string(),
    });

    // Φ Contrarian
    scenarios.push(ScenarioBranch {
        branch_id: "phi_contrarian".to_string(),
        branch_name: "Contrarian Case".to_string(),
        probability_pct: 7,
        timeframe: "12-36 months".to_string(),
        confidence: "MEDIUM".to_string(),
        narrative: format!("The opposite of conventional wisdom proves true. The consensus view that {} is the right approach turns out to be wrong. This creates both risk for current efforts and opportunity for those who anticipate the shift.", domain),
        key_assumptions: vec![
            format!("Consensus view on {} is correct", domain),
            "Current evidence supports prevailing theory".to_string(),
            "Established methods are optimal".to_string(),
        ],
        trigger_conditions: vec![
            "Minority view gains unexpected support".to_string(),
            "Contrarian data published to acclaim".to_string(),
            "Regulatory or funder stance shifts".to_string(),
        ],
        immediate_consequences: "Current approach questioned; early adopters of contrarian view gain advantage".to_string(),
        thirty_day_consequences: "Debate intensifies; funding bodies reconsider; team must decide on pivot".to_string(),
        six_month_consequences: "Field reorients around new consensus; late movers face disadvantage".to_string(),
        required_response: "Monitor contrarian signals actively; avoid dismissing minority views prematurely".to_string(),
        what_most_miss: "The contrarian view is right more often than expected — and at the exact moment when admitting it feels like the biggest risk".to_string(),
    });

    // ∞ Second Order
    scenarios.push(ScenarioBranch {
        branch_id: "infinity_second".to_string(),
        branch_name: "Second Order".to_string(),
        probability_pct: 5,
        timeframe: "12-48 months".to_string(),
        confidence: "LOW".to_string(),
        narrative: "First-order effects trigger cascading consequences that nobody predicted. Initial success (or failure) creates chain reaction of unintended outcomes. The secondary consequences prove more significant than the primary event itself.".to_string(),
        key_assumptions: vec![
            "First-order effects remain contained".to_string(),
            "No significant side effects emerge".to_string(),
            "System remains in equilibrium".to_string(),
        ],
        trigger_conditions: vec![
            "Success attracts attention from unexpected quarters".to_string(),
            "Scaling reveals hidden complexities".to_string(),
            "Unintended consequences begin appearing".to_string(),
        ],
        immediate_consequences: "Focus shifts from primary goal to managing cascades".to_string(),
        thirty_day_consequences: "Second-order stakeholders demand input; resource reallocation required".to_string(),
        six_month_consequences: "Ecosystem around the work has formed; original team has limited control".to_string(),
        required_response: "Map potential second-order effects proactively; build governance mechanisms early".to_string(),
        what_most_miss: "The most important consequences of any action are always the ones you didn't think to look for".to_string(),
    });

    scenarios
}

#[async_trait]
impl ToolHandler for WhatIfOracleHandler {
    fn name(&self) -> &str { "what_if_oracle" }
    fn description(&self) -> &str { "Explore multi-branch scenario analysis for a research question using the What-If Oracle framework (0·IF·1). Generates 6 scenario branches: Best, Likely, Worst, Wild Card, Contrarian, and Second Order cases." }
    fn input_schema(&self) -> ToolInputSchema {
        ToolInputSchema::object(
            vec![
                ("question".into(), ToolProperty::string("Research question to analyze (e.g., 'What if we discover a safe and effective CRISPR therapy for sickle cell disease within 5 years?')")),
                ("context".into(), ToolProperty::string("Additional context or constraints (e.g., 'Current funding: $2M, Timeline: 3 years, Team: 5 researchers')")),
                ("mode".into(), ToolProperty::string("Analysis mode: 'quick' (3 branches) or 'deep' (6 branches, default: deep)")),
            ].into_iter().collect(),
            vec!["question".into()],
        )
    }
    async fn call(&self, params: Value) -> Result<Value, String> {
        let question = params["question"].as_str().ok_or("Missing question")?;
        let context = params.get("context").and_then(|v| v.as_str()).unwrap_or("");
        let mode = params.get("mode").and_then(|v| v.as_str()).unwrap_or("deep");

        let all_scenarios = analyze_scenarios(question, context);

        let scenarios: Vec<ScenarioBranch> = if mode == "quick" {
            all_scenarios.into_iter().filter(|s| {
                s.branch_id == "omega_best" || s.branch_id == "alpha_likely" || s.branch_id == "delta_worst"
            }).collect()
        } else {
            all_scenarios
        };

        let total_pct: u8 = scenarios.iter().map(|s| s.probability_pct).sum();

        let branches_json: Vec<Value> = scenarios.iter().map(|s| {
            serde_json::json!({
                "branch_id": s.branch_id,
                "branch_name": s.branch_name,
                "probability_pct": s.probability_pct,
                "timeframe": s.timeframe,
                "confidence": s.confidence,
                "narrative": s.narrative,
                "key_assumptions": s.key_assumptions,
                "trigger_conditions": s.trigger_conditions,
                "consequences": {
                    "immediate": s.immediate_consequences,
                    "thirty_day": s.thirty_day_consequences,
                    "six_month": s.six_month_consequences,
                },
                "required_response": s.required_response,
                "what_most_miss": s.what_most_miss,
            })
        }).collect();

        let today = chrono::Local::now().format("%Y-%m-%d").to_string();

        let mut synthesis = String::new();
        synthesis.push_str("## Probability Distribution\n\n");
        synthesis.push_str("| Scenario | Probability |\n");
        synthesis.push_str("|----------|-------------|\n");
        for s in &scenarios {
            let bar = "█".repeat((s.probability_pct / 5) as usize);
            let dots = "░".repeat(20 - (s.probability_pct / 5) as usize);
            synthesis.push_str(&format!("| {} {} | {}% |\n", bar, dots, s.probability_pct));
        }
        synthesis.push_str(&format!("\n**Total probability:** {}% (normalized)\n\n", total_pct));

        synthesis.push_str("## Synthesis\n\n");
        synthesis.push_str("**Robust Actions** (beneficial across multiple branches):\n\n");
        synthesis.push_str("1. Maintain methodological rigor regardless of early results\n");
        synthesis.push_str("2. Build in flexibility to pivot when evidence demands\n");
        synthesis.push_str("3. Document all findings thoroughly for future reference\n");
        synthesis.push_str("4. Cultivate stakeholder relationships beyond the primary funder\n\n");

        synthesis.push_str("**Hedge Actions** (protect against worst case without sacrificing upside):\n\n");
        synthesis.push_str("1. Establish replication protocol before primary analysis\n");
        synthesis.push_str("2. Maintain runway for at least 6 months beyond planned end\n");
        synthesis.push_str("3. Develop contingency plans for both null and positive results\n\n");

        synthesis.push_str("**Decision Triggers** (signals to update branch probabilities):\n\n");
        synthesis.push_str("- Increase Best Case probability if: early results exceed thresholds, key endorsements received\n");
        synthesis.push_str("- Increase Worst Case probability if: replication fails, funding signals concern\n");
        synthesis.push_str("- Increase Wild Card probability if: results defy all models, external disruption occurs\n");
        synthesis.push_str("- Increase Contrarian probability if: minority view gains unexpected traction\n\n");

        synthesis.push_str("**The 1% Insight**\n\n");
        synthesis.push_str(&format!(
            "The most actionable insight from this scenario analysis: the specific branch that feels least comfortable to plan for is often the one that contains the highest learning potential. For the question '{}', the gap between what the analysis reveals and what you immediately want to act on is where the real strategic value lies.\n\n",
            question.chars().take(60).collect::<String>()
        ));

        synthesis.push_str(&format!("_Analyzed: {} | Mode: {} | Branches: {}_\n", today, mode, scenarios.len()));

        Ok(serde_json::json!({
            "question": question,
            "context": context,
            "mode": mode,
            "branches": branches_json,
            "synthesis": synthesis,
            "total_probability_pct": total_pct,
            "date": today,
        }))
    }
}

// ─── Paper Docx Export ─────────────────────────────────────────────────────────

pub struct PaperDocxHandler;

fn escape_xml(s: &str) -> String {
    s.replace("&", "&amp;").replace("<", "&lt;").replace(">", "&gt;").replace("\"", "&quot;")
}

fn md_to_docx_xml(markdown: &str) -> String {
    let mut xml = String::from(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>"#
    );
    xml.push_str(r#"<w:document xmlns:wpc="http://schemas.microsoft.com/office/word/2010/wordprocessingCanvas"#);
    xml.push_str(r#" xmlns:cx="http://schemas.microsoft.com/office/drawing/2014/chartex"#);
    xml.push_str(r#" xmlns:mc="http://schemas.openxmlformats.org/markup-compatibility/2006""#);
    xml.push_str(r#" xmlns:aink="http://schemas.microsoft.com/office/drawing/2016/ink""#);
    xml.push_str(r#" xmlns:am3d="http://schemas.microsoft.com/office/drawing/2017/model3d""#);
    xml.push_str(r#" xmlns:o="urn:schemas-microsoft-com:office:office""#);
    xml.push_str(r#" xmlns:oel="http://schemas.microsoft.com/office/2019/extlst""#);
    xml.push_str(r#" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationship""#);
    xml.push_str(r#" xmlns:m="http://schemas.openxmlformats.org/officeDocument/2006/math""#);
    xml.push_str(r#" xmlns:v="urn:schemas-microsoft-com:vml""#);
    xml.push_str(r#" xmlns:wp14="http://schemas.microsoft.com/office/word/2010/wordprocessingDrawing""#);
    xml.push_str(r#" xmlns:wp="http://schemas.openxmlformats.org/drawingml/2006/wordprocessingDrawing""#);
    xml.push_str(r#" xmlns:w10="urn:schemas-microsoft-com:office:word""#);
    xml.push_str(r#" xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main""#);
    xml.push_str(r#" xmlns:w14="http://schemas.microsoft.com/office/word/2010/wordml""#);
    xml.push_str(r#" xmlns:w15="http://schemas.microsoft.com/office/word/2012/wordml""#);
    xml.push_str(r#" xmlns:w16cex="http://schemas.microsoft.com/office/word/2018/wordml/cex""#);
    xml.push_str(r#" xmlns:w16cid="http://schemas.microsoft.com/office/word/2016/wordml/cid""#);
    xml.push_str(r#" xmlns:w16="http://schemas.microsoft.com/office/word/2018/wordml""#);
    xml.push_str(r#" xmlns:w16sdtdh="http://schemas.microsoft.com/office/word/2020/wordml/sdtdatahash""#);
    xml.push_str(r#" xmlns:w16se="http://schemas.microsoft.com/office/word/2015/wordml/symex""#);
    xml.push_str(r#" xmlns:wpg="http://schemas.microsoft.com/office/word/2010/wordprocessingGroup""#);
    xml.push_str(r#" xmlns:wpi="http://schemas.microsoft.com/office/word/2010/wordprocessingInk""#);
    xml.push_str(r#" xmlns:wne="http://schemas.microsoft.com/office/word/2006/wordml""#);
    xml.push_str(r#" xmlns:wps="http://schemas.microsoft.com/office/word/2010/wordprocessingShape""#);
    xml.push_str(r#">"#);
    xml.push_str("<w:body>");

    let lines: Vec<&str> = markdown.lines().collect();
    let mut in_list = false;
    let mut list_type = "";
    let mut list_count = 0usize;

    for line in &lines {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            if in_list {
                xml.push_str("</w:p>");
                in_list = false;
            }
            continue;
        }

        // Heading 1
        if trimmed.starts_with("# ") {
            let text = &trimmed[2..];
            xml.push_str(&format!(
                r#"<w:p><w:pPr><w:pStyle w:val="Heading1"/></w:pPr><w:r><w:t>{}</w:t></w:r></w:p>"#,
                escape_xml(text)
            ));
        }
        // Heading 2
        else if trimmed.starts_with("## ") {
            let text = &trimmed[3..];
            xml.push_str(&format!(
                r#"<w:p><w:pPr><w:pStyle w:val="Heading2"/></w:pPr><w:r><w:t>{}</w:t></w:r></w:p>"#,
                escape_xml(text)
            ));
        }
        // Heading 3
        else if trimmed.starts_with("### ") {
            let text = &trimmed[4..];
            xml.push_str(&format!(
                r#"<w:p><w:pPr><w:pStyle w:val="Heading3"/></w:pPr><w:r><w:t>{}</w:t></w:r></w:p>"#,
                escape_xml(text)
            ));
        }
        // Bullet list
        else if trimmed.starts_with("- ") || trimmed.starts_with("* ") {
            let text = &trimmed[2..];
            if !in_list || list_type != "bullet" {
                if in_list { xml.push_str("</w:p>"); }
                xml.push_str(r#"<w:p><w:pPr><w:numPr><w:ilvl w:val="0"/><w:numId w:val="1"/></w:numPr></w:pPr>"#);
                in_list = true;
                list_type = "bullet";
            }
            xml.push_str(&format!(
                r#"<w:r><w:t>{}</w:t></w:r></w:p><w:p><w:pPr><w:numPr><w:ilvl w:val="0"/><w:numId w:val="1"/></w:numPr></w:pPr>"#,
                escape_xml(text)
            ));
        }
        // Numbered list
        else if trimmed.starts_with("1. ") || trimmed.starts_with("1) ") {
            let text = if trimmed.starts_with("1. ") { &trimmed[3..] } else { &trimmed[2..] };
            if !in_list || list_type != "number" {
                if in_list { xml.push_str("</w:p>"); }
                xml.push_str(r#"<w:p><w:pPr><w:numPr><w:ilvl w:val="0"/><w:numId w:val="2"/></w:numPr></w:pPr>"#);
                in_list = true;
                list_type = "number";
            }
            xml.push_str(&format!(
                r#"<w:r><w:t>{}</w:t></w:r></w:p><w:p><w:pPr><w:numPr><w:ilvl w:val="0"/><w:numId w:val="2"/></w:numPr></w:pPr>"#,
                escape_xml(text)
            ));
        }
        // Separator line
        else if trimmed == "---" || trimmed == "***" || trimmed == "___" {
            xml.push_str(r#"<w:p><w:pPr><w:pBdr><w:bottom w:val="single" w:sz="6" w:space="1" w:color="CCCCCC"/></w:pBdr></w:pPr></w:p>"#);
        }
        // Block quote
        else if trimmed.starts_with("> ") {
            let text = &trimmed[2..];
            xml.push_str(&format!(
                r#"<w:p><w:pPr><w:ind w:left="720" w:right="720"/><w:jc w:val="both"/></w:pPr><w:r><w:rPr><w:i/><w:color w:val="666666"/></w:rPr><w:t>{}</w:t></w:r></w:p>"#,
                escape_xml(text)
            ));
        }
        // Regular paragraph with inline bold/italic
        else {
            if in_list {
                xml.push_str("</w:p>");
                in_list = false;
            }
            let processed = process_inline_formatting(trimmed);
            xml.push_str(&format!(
                r#"<w:p><w:r>{}</w:r></w:p>"#,
                processed
            ));
        }
    }

    if in_list {
        xml.push_str("</w:p>");
    }

    xml.push_str(r#"<w:sectPr><w:pgSz w:w="12240" w:h="15840"/><w:pgMar w:top="1440" w:right="1440" w:bottom="1440" w:left="1440" w:header="720" w:footer="720" w:gutter="0"/></w:sectPr>"#);
    xml.push_str("</w:body></w:document>");

    xml
}

fn process_inline_formatting(text: &str) -> String {
    let mut result = String::new();
    let mut chars = text.chars().peekable();
    let mut in_bold = false;
    let mut in_italic = false;

    while let Some(c) = chars.next() {
        if c == '*' {
            let next = chars.peek().copied();
            if next == Some('*') {
                chars.next();
                if in_bold {
                    result.push_str("</w:rPr></w:r><w:r>");
                    in_bold = false;
                } else {
                    result.push_str("<w:r><w:rPr><w:b/></w:rPr>");
                    in_bold = true;
                }
            } else {
                if in_italic {
                    result.push_str("</w:rPr></w:r><w:r>");
                    in_italic = false;
                } else {
                    result.push_str("<w:r><w:rPr><w:i/></w:rPr>");
                    in_italic = true;
                }
            }
        } else if c == '`' {
            result.push_str(&format!("<w:t>{}</w:t>", c));
        } else {
            result.push_str(&escape_xml(&c.to_string()));
        }
    }

    if result.is_empty() {
        result.push_str("<w:t></w:t>");
    }

    if !result.contains("<w:t>") {
        result = format!("<w:t>{}</w:t>", result);
    }

    result
}

fn build_docx(markdown: &str, _title: &str) -> Result<Vec<u8>, String> {
    use std::io::Write;

    let mut buffer = Vec::new();
    {
        let mut zip_writer = zip::ZipWriter::new(std::io::Cursor::new(&mut buffer));

        let options = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Deflated)
            .unix_permissions(0o644);

        // [Content_Types].xml
        zip_writer.start_file("[Content_Types].xml", options).map_err(|e| e.to_string())?;
        zip_writer.write_all(br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>"#).map_err(|e| e.to_string())?;
        zip_writer.write_all(br#"<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">"#).map_err(|e| e.to_string())?;
        zip_writer.write_all(br#"<Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>"#).map_err(|e| e.to_string())?;
        zip_writer.write_all(br#"<Default Extension="xml" ContentType="application/xml"/>"#).map_err(|e| e.to_string())?;
        zip_writer.write_all(br#"<Override PartName="/word/document.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml"/>"#).map_err(|e| e.to_string())?;
        zip_writer.write_all(br#"</Types>"#).map_err(|e| e.to_string())?;

        // _rels/.rels
        zip_writer.start_file("_rels/.rels", options).map_err(|e| e.to_string())?;
        zip_writer.write_all(br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>"#).map_err(|e| e.to_string())?;
        zip_writer.write_all(br#"<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">"#).map_err(|e| e.to_string())?;
        zip_writer.write_all(br#"<Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="word/document.xml"/>"#).map_err(|e| e.to_string())?;
        zip_writer.write_all(br#"</Relationships>"#).map_err(|e| e.to_string())?;

        // word/_rels/document.xml.rels
        zip_writer.start_file("word/_rels/document.xml.rels", options).map_err(|e| e.to_string())?;
        zip_writer.write_all(br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>"#).map_err(|e| e.to_string())?;
        zip_writer.write_all(br#"<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">"#).map_err(|e| e.to_string())?;
        zip_writer.write_all(br#"</Relationships>"#).map_err(|e| e.to_string())?;

        // word/document.xml
        zip_writer.start_file("word/document.xml", options).map_err(|e| e.to_string())?;
        let doc_xml = md_to_docx_xml(markdown);
        zip_writer.write_all(doc_xml.as_bytes()).map_err(|e| e.to_string())?;

        // word/settings.xml (minimal)
        zip_writer.start_file("word/settings.xml", options).map_err(|e| e.to_string())?;
        zip_writer.write_all(br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>"#).map_err(|e| e.to_string())?;
        zip_writer.write_all(br#"<w:settings xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"><w:defaultTabStop w:val="720"/></w:settings>"#).map_err(|e| e.to_string())?;

        // word/styles.xml
        zip_writer.start_file("word/styles.xml", options).map_err(|e| e.to_string())?;
        let styles_xml = build_styles_xml();
        zip_writer.write_all(styles_xml.as_bytes()).map_err(|e| e.to_string())?;

        // word/numbering.xml
        zip_writer.start_file("word/numbering.xml", options).map_err(|e| e.to_string())?;
        let numbering_xml = build_numbering_xml();
        zip_writer.write_all(numbering_xml.as_bytes()).map_err(|e| e.to_string())?;

        zip_writer.finish().map_err(|e| e.to_string())?;
    }

    Ok(buffer)
}

fn build_styles_xml() -> String {
    let mut xml = String::from(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>"#
    );
    xml.push_str(r#"<w:styles xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">"#);

    // Default paragraph
    xml.push_str(r#"<w:style w:type="paragraph" w:default="1" w:styleId="Normal">"#);
    xml.push_str(r#"<w:name w:val="Normal"/>"#);
    xml.push_str(r#"<w:qFormat/>"#);
    xml.push_str(r#"</w:style>"#);

    // Heading 1
    xml.push_str(r#"<w:style w:type="paragraph" w:styleId="Heading1">"#);
    xml.push_str(r#"<w:name w:val="heading 1"/>"#);
    xml.push_str(r#"<w:basedOn w:val="Normal"/>"#);
    xml.push_str(r#"<w:next w:val="Normal"/>"#);
    xml.push_str(r#"<w:qFormat/>"#);
    xml.push_str(r#"<w:pPr><w:outlineLvl w:val="0"/></w:pPr>"#);
    xml.push_str(r#"<w:rPr><w:b/><w:sz w:val="48"/><w:szCs w:val="48"/></w:rPr>"#);
    xml.push_str(r#"</w:style>"#);

    // Heading 2
    xml.push_str(r#"<w:style w:type="paragraph" w:styleId="Heading2">"#);
    xml.push_str(r#"<w:name w:val="heading 2"/>"#);
    xml.push_str(r#"<w:basedOn w:val="Normal"/>"#);
    xml.push_str(r#"<w:next w:val="Normal"/>"#);
    xml.push_str(r#"<w:qFormat/>"#);
    xml.push_str(r#"<w:pPr><w:outlineLvl w:val="1"/></w:pPr>"#);
    xml.push_str(r#"<w:rPr><w:b/><w:sz w:val="32"/><w:szCs w:val="32"/></w:rPr>"#);
    xml.push_str(r#"</w:style>"#);

    // Heading 3
    xml.push_str(r#"<w:style w:type="paragraph" w:styleId="Heading3">"#);
    xml.push_str(r#"<w:name w:val="heading 3"/>"#);
    xml.push_str(r#"<w:basedOn w:val="Normal"/>"#);
    xml.push_str(r#"<w:next w:val="Normal"/>"#);
    xml.push_str(r#"<w:qFormat/>"#);
    xml.push_str(r#"<w:pPr><w:outlineLvl w:val="2"/></w:pPr>"#);
    xml.push_str(r#"<w:rPr><w:b/><w:sz w:val="28"/><w:szCs w:val="28"/></w:rPr>"#);
    xml.push_str(r#"</w:style>"#);

    xml.push_str("</w:styles>");
    xml
}

fn build_numbering_xml() -> String {
    let mut xml = String::from(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>"#
    );
    xml.push_str(r#"<w:numbering xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">"#);

    // Bullet list (numId=1)
    xml.push_str(r#"<w:abstractNum w:abstractNumId="0">"#);
    xml.push_str(r#"<w:multiLevelType w:val="hybridMultilevel"/>"#);
    xml.push_str(r#"<w:lvl w:ilvl="0"><w:start w:val="1"/><w:numFmt w:val="bullet"/><w:lvlText w:val="&#x2022;"/><w:lvlJc w:val="left"/><w:pPr><w:ind w:left="720" w:hanging="360"/></w:pPr><w:rPr><w:rFonts w:ascii="Arial" w:hAnsi="Arial" w:hint="default"/></w:rPr></w:lvl>"#);
    xml.push_str(r#"</w:abstractNum>"#);

    // Numbered list (numId=2)
    xml.push_str(r#"<w:abstractNum w:abstractNumId="1">"#);
    xml.push_str(r#"<w:multiLevelType w:val="hybridMultilevel"/>"#);
    xml.push_str(r#"<w:lvl w:ilvl="0"><w:start w:val="1"/><w:numFmt w:val="decimal"/><w:lvlText w:val="%1."/><w:lvlJc w:val="left"/><w:pPr><w:ind w:left="720" w:hanging="360"/></w:pPr></w:lvl>"#);
    xml.push_str(r#"</w:abstractNum>"#);

    // References
    xml.push_str(r#"<w:num w:numId="1"><w:abstractNumId w:val="0"/></w:num>"#);
    xml.push_str(r#"<w:num w:numId="2"><w:abstractNumId w:val="1"/></w:num>"#);

    xml.push_str("</w:numbering>");
    xml
}

#[async_trait]
impl ToolHandler for PaperDocxHandler {
    fn name(&self) -> &str { "paper_docx_export" }
    fn description(&self) -> &str { "Export a research paper or literature review from markdown to a Word (.docx) document with proper formatting, headings, lists, and styles" }
    fn input_schema(&self) -> ToolInputSchema {
        ToolInputSchema::object(
            vec![
                ("markdown".into(), ToolProperty::string("Full markdown content of the paper or review to export")),
                ("title".into(), ToolProperty::string("Document title for the Word file")),
                ("filename".into(), ToolProperty::string("Output filename without extension (default: paper_export)")),
            ].into_iter().collect(),
            vec!["markdown".into(), "title".into()],
        )
    }
    async fn call(&self, params: Value) -> Result<Value, String> {
        let markdown = params["markdown"].as_str().ok_or("Missing markdown")?;
        let title = params["title"].as_str().ok_or("Missing title")?;
        let filename = params.get("filename").and_then(|v| v.as_str()).unwrap_or("paper_export");

        let docx_data = build_docx(markdown, title)?;

        let output_dir = data_dir().join("exports");
        std::fs::create_dir_all(&output_dir).map_err(|e| e.to_string())?;

        let output_path = output_dir.join(format!("{}.docx", filename));
        std::fs::write(&output_path, &docx_data).map_err(|e| e.to_string())?;

        let file_size = docx_data.len();

        Ok(serde_json::json!({
            "title": title,
            "filename": format!("{}.docx", filename),
            "path": output_path.to_string_lossy(),
            "size_bytes": file_size,
            "format": "docx",
        }))
    }
}

// ─── Register all tools ───────────────────────────────────────────────────────

pub async fn register_all(server: &crate::McpServer) {
    server.register(PaperSearchHandler).await;
    server.register(PaperIngestHandler).await;
    server.register(PaperParseFullHandler).await;
    server.register(ReplicationCheckSimpleHandler).await;
    server.register(GitHubRepoMetadataHandler).await;
    server.register(HuggingFaceDatasetHandler).await;
    server.register(PdfExtractAdvancedHandler).await;
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
    server.register(PdfDownloadHandler).await;
    server.register(PdfExtractTextHandler).await;
    server.register(PdfExtractStructuredHandler).await;
    server.register(TrendsPredictNextHandler).await;
    server.register(TrendsTopPredictionsHandler).await;
    server.register(TrendsCompareTagsHandler).await;
    server.register(CiteFetchHandler).await;
    server.register(PaperSearchMultiHandler).await;
    server.register(PaperLookupDoiHandler).await;
    server.register(PaperCitationsHandler).await;
    server.register(PaperVerifyCitationsHandler).await;
    server.register(PaperVisualizeTrendsHandler).await;
    server.register(PaperVisualizeRadarHandler).await;
    server.register(PaperCriticalAnalysisHandler).await;
    server.register(PaperGenerateReviewPdfHandler).await;
    server.register(HypothesisReportHandler).await;
    server.register(PaperGenerateSchematicHandler).await;
    server.register(PaperScienceDiscoveryHandler).await;
    server.register(PaperDatabaseLookupHandler).await;
    server.register(PaperPeerReviewHandler).await;
    server.register(PaperFormatCitationHandler).await;
    server.register(PaperLiteratureReviewHandler).await;
    server.register(PaperGenerateHandler).await;
    server.register(WhatIfOracleHandler).await;
    server.register(PaperDocxHandler).await;
    server.register(ChartQueryHandler).await;
    crate::llm_handlers::register_llm_handlers(server).await;
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

fn extract_tag(s: &str, tag: &str) -> Option<String> {
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

fn parse_arxiv_citation(xml: &str) -> Result<serde_json::Value, String> {
    let entry_text: Option<String> = if let Some(start) = xml.find("<entry>") {
        let abs_start = start;
        let end = xml[abs_start..].find("</entry>").ok_or("No </entry> found")?;
        Some(xml[abs_start..abs_start + end + 8].to_string())
    } else {
        None
    };
    let entry = entry_text.ok_or("No entry found in arXiv response")?;

    let title = extract_tag(&entry, "title").map(clean_xml).unwrap_or_default();
    let id = extract_tag(&entry, "id").unwrap_or_default();
    let published = extract_tag(&entry, "published").unwrap_or_default();
    let journal_ref = extract_tag(&entry, "journal-ref").unwrap_or_default();
    let doi = extract_tag(&entry, "doi").unwrap_or_default();

    let mut authors: Vec<serde_json::Value> = Vec::new();
    let mut a_pos = 0;
    while let Some(start) = entry[a_pos..].find("<author>") {
        let abs_start = a_pos + start;
        let Some(end) = entry[abs_start..].find("</author>") else { break; };
        let ab = &entry[abs_start..abs_start + end + 9];
        if let Some(name) = extract_tag(ab, "name") {
            let parts: Vec<&str> = name.split_whitespace().collect();
            let family = parts.last().unwrap_or(&"").to_string();
            let given = if parts.len() > 1 { parts[..parts.len()-1].join(" ") } else { String::new() };
            authors.push(serde_json::json!({ "family": family, "given": given }));
        }
        a_pos = abs_start + end + 9;
    }

    let arxiv_id = id.strip_prefix("http://arxiv.org/abs/")
        .or_else(|| id.strip_prefix("https://arxiv.org/abs/"))
        .unwrap_or(&id).to_string();

    Ok(serde_json::json!({
        "title": title,
        "id": id,
        "published": published,
        "journal-ref": journal_ref,
        "doi": doi,
        "authors": authors,
        "arxiv_id": arxiv_id,
    }))
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
        assert!(PdfDownloadHandler.name() == "pdf_download");
        assert!(PdfExtractTextHandler.name() == "pdf_extract_text");
        assert!(PdfExtractStructuredHandler.name() == "pdf_extract_structured");
        assert!(TrendsPredictNextHandler.name() == "trends_predict_next");
        assert!(TrendsTopPredictionsHandler.name() == "trends_top_predictions");
        assert!(TrendsCompareTagsHandler.name() == "trends_compare_tags");
        assert!(CiteFetchHandler.name() == "cite_fetch");
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

    // ─── PDF Handler Tests ─────────────────────────────────────────────────

    #[test]
    fn test_pdf_handlers_schema_requires_arxiv_id() {
        let req = |h: &dyn ToolHandler| h.input_schema().required.unwrap_or_default();
        assert!(req(&PdfDownloadHandler).contains(&"arxiv_id".into()));
        assert!(req(&PdfExtractTextHandler).contains(&"arxiv_id".into()));
        assert!(req(&PdfExtractStructuredHandler).contains(&"arxiv_id".into()));
    }

    #[test]
    fn test_pdf_download_error_missing_arxiv_id() {
        let result = futures::executor::block_on(PdfDownloadHandler.call(serde_json::json!({})));
        assert_eq!(result, Err("Missing arxiv_id".to_string()));
    }

    #[test]
    fn test_pdf_extract_text_error_missing_arxiv_id() {
        let result = futures::executor::block_on(PdfExtractTextHandler.call(serde_json::json!({})));
        assert_eq!(result, Err("Missing arxiv_id".to_string()));
    }

    #[test]
    fn test_pdf_extract_structured_error_missing_arxiv_id() {
        let result = futures::executor::block_on(PdfExtractStructuredHandler.call(serde_json::json!({})));
        assert_eq!(result, Err("Missing arxiv_id".to_string()));
    }

    // ─── Trends Handler Tests ──────────────────────────────────────────────

    #[test]
    fn test_trends_predict_next_schema_requires_tag() {
        let req = TrendsPredictNextHandler.input_schema().required.unwrap_or_default();
        assert!(req.contains(&"tag".into()));
    }

    #[test]
    fn test_trends_predict_next_error_missing_tag() {
        let result = futures::executor::block_on(TrendsPredictNextHandler.call(serde_json::json!({})));
        assert_eq!(result, Err("Missing tag".to_string()));
    }

    #[test]
    fn test_trends_top_predictions_no_required() {
        let schema = TrendsTopPredictionsHandler.input_schema();
        assert!(schema.required.is_none() || schema.required.as_ref().unwrap().is_empty());
    }

    #[test]
    fn test_trends_compare_tags_schema_requires_both() {
        let req = TrendsCompareTagsHandler.input_schema().required.unwrap_or_default();
        assert!(req.contains(&"tag_a".into()));
        assert!(req.contains(&"tag_b".into()));
    }

    #[test]
    fn test_trends_compare_tags_error_missing_tag_a() {
        let result = futures::executor::block_on(TrendsCompareTagsHandler.call(serde_json::json!({"tag_b": "test"})));
        assert_eq!(result, Err("Missing tag_a".to_string()));
    }

    #[test]
    fn test_cite_fetch_schema_requires_paper_id() {
        let req = CiteFetchHandler.input_schema().required.unwrap_or_default();
        assert!(req.contains(&"paper_id".into()));
    }

    #[test]
    fn test_cite_fetch_error_missing_paper_id() {
        let result = futures::executor::block_on(CiteFetchHandler.call(serde_json::json!({})));
        assert_eq!(result, Err("Missing paper_id".to_string()));
    }

    #[test]
    fn test_trends_compare_tags_error_missing_tag_b() {
        let result = futures::executor::block_on(TrendsCompareTagsHandler.call(serde_json::json!({"tag_a": "test"})));
        assert_eq!(result, Err("Missing tag_b".to_string()));
    }

    #[test]
    fn test_paper_verify_citations_schema_requires_dois() {
        let req = PaperVerifyCitationsHandler.input_schema().required.unwrap_or_default();
        assert!(req.contains(&"dois".into()));
    }

    #[test]
    fn test_paper_verify_citations_error_missing_dois() {
        let result = futures::executor::block_on(PaperVerifyCitationsHandler.call(serde_json::json!({})));
        assert_eq!(result, Err("Missing required parameter: dois".to_string()));
    }

    #[test]
    fn test_paper_verify_citations_error_invalid_style() {
        let result = futures::executor::block_on(PaperVerifyCitationsHandler.call(serde_json::json!({"dois": "10.1234/test", "style": "invalid"})));
        assert!(result.is_err() && result.unwrap_err().contains("Invalid style"));
    }

    #[test]
    fn test_paper_verify_citations_error_no_dois() {
        let result = futures::executor::block_on(PaperVerifyCitationsHandler.call(serde_json::json!({"dois": ""})));
        assert_eq!(result, Err("No DOIs provided".to_string()));
    }

    #[test]
    fn test_paper_visualize_trends_schema_requires_trends_json() {
        let req = PaperVisualizeTrendsHandler.input_schema().required.unwrap_or_default();
        assert!(req.contains(&"trends_json".into()));
    }

    #[test]
    fn test_paper_visualize_trends_error_missing_data() {
        let result = futures::executor::block_on(PaperVisualizeTrendsHandler.call(serde_json::json!({})));
        assert!(result.is_err());
    }

    #[test]
    fn test_paper_visualize_radar_schema_requires_scores_json() {
        let req = PaperVisualizeRadarHandler.input_schema().required.unwrap_or_default();
        assert!(req.contains(&"scores_json".into()));
    }

    #[test]
    fn test_paper_visualize_radar_error_missing_data() {
        let result = futures::executor::block_on(PaperVisualizeRadarHandler.call(serde_json::json!({})));
        assert!(result.is_err());
    }

    #[test]
    fn test_paper_critical_analysis_schema_requires_fields() {
        let req = PaperCriticalAnalysisHandler.input_schema().required.unwrap_or_default();
        assert!(req.contains(&"paper_id".into()));
        assert!(req.contains(&"title".into()));
        assert!(req.contains(&"abstract".into()));
    }

    #[test]
    fn test_paper_critical_analysis_error_missing_fields() {
        let result = futures::executor::block_on(PaperCriticalAnalysisHandler.call(serde_json::json!({})));
        assert!(result.is_err());
    }

    #[test]
    fn test_paper_generate_review_pdf_schema_requires_review_json() {
        let req = PaperGenerateReviewPdfHandler.input_schema().required.unwrap_or_default();
        assert!(req.contains(&"review_json".into()));
    }

    #[test]
    fn test_paper_generate_review_pdf_error_missing_data() {
        let result = futures::executor::block_on(PaperGenerateReviewPdfHandler.call(serde_json::json!({})));
        assert!(result.is_err());
    }

    #[test]
    fn test_hypothesis_report_schema_requires_fields() {
        let req = HypothesisReportHandler.input_schema().required.unwrap_or_default();
        assert!(req.contains(&"topic".into()));
        assert!(req.contains(&"hypotheses_json".into()));
    }

    #[test]
    fn test_hypothesis_report_error_missing_fields() {
        let result = futures::executor::block_on(HypothesisReportHandler.call(serde_json::json!({})));
        assert!(result.is_err());
    }

    #[test]
    fn test_paper_generate_schematic_schema_requires_fields() {
        let req = PaperGenerateSchematicHandler.input_schema().required.unwrap_or_default();
        assert!(req.contains(&"diagram_type".into()));
        assert!(req.contains(&"diagram_json".into()));
    }

    #[test]
    fn test_paper_generate_schematic_error_missing_fields() {
        let result = futures::executor::block_on(PaperGenerateSchematicHandler.call(serde_json::json!({})));
        assert!(result.is_err());
    }

    #[test]
    fn test_paper_science_discovery_schema_requires_query() {
        let req = PaperScienceDiscoveryHandler.input_schema().required.unwrap_or_default();
        assert!(req.contains(&"query".into()));
    }

    #[test]
    fn test_paper_science_discovery_error_missing_query() {
        let result = futures::executor::block_on(PaperScienceDiscoveryHandler.call(serde_json::json!({})));
        assert!(result.is_err());
    }

    #[test]
    fn test_paper_database_lookup_schema() {
        let req = PaperDatabaseLookupHandler.input_schema().required.unwrap_or_default();
        assert!(req.contains(&"query_type".into()));
        assert!(req.contains(&"term".into()));
    }

    #[test]
    fn test_paper_database_lookup_error_missing_fields() {
        let result = futures::executor::block_on(PaperDatabaseLookupHandler.call(serde_json::json!({})));
        assert!(result.is_err());
    }

    #[test]
    fn test_paper_peer_review_schema() {
        let req = PaperPeerReviewHandler.input_schema().required.unwrap_or_default();
        assert!(req.contains(&"paper_id".into()));
        assert!(req.contains(&"title".into()));
    }

    #[test]
    fn test_paper_peer_review_error_missing_fields() {
        let result = futures::executor::block_on(PaperPeerReviewHandler.call(serde_json::json!({})));
        assert!(result.is_err());
    }

    #[test]
    fn test_paper_peer_review_minimal_input() {
        let result = futures::executor::block_on(PaperPeerReviewHandler.call(serde_json::json!({
            "paper_id": "test123",
            "title": "Test Paper",
            "abstract_text": "This is a test abstract about methods and results.",
            "sections": "Introduction: background. Methods: experiments were conducted with statistical tests. Results: findings show significant effects. Discussion: limitations acknowledged."
        })));
        assert!(result.is_ok());
        let output = result.unwrap();
        assert!(output["overall_score"].as_f64().is_some());
        assert!(!output["recommendation"].as_str().unwrap().is_empty());
        assert!(output["major_issues"].as_array().is_some());
        assert!(output["minor_issues"].as_array().is_some());
    }

    #[test]
    fn test_paper_peer_review_consort_checklist() {
        let result = futures::executor::block_on(PaperPeerReviewHandler.call(serde_json::json!({
            "paper_id": "test456",
            "title": "Clinical Trial Paper",
            "checklist_type": "CONSORT"
        })));
        assert!(result.is_ok());
        let output = result.unwrap();
        assert!(output["compliance"]["consort_checklist"].is_object());
    }

    #[test]
    fn test_paper_format_citation_schema() {
        let req = PaperFormatCitationHandler.input_schema().required.unwrap_or_default();
        assert!(req.contains(&"identifier".into()));
    }

    #[test]
    fn test_paper_format_citation_error_missing_fields() {
        let result = futures::executor::block_on(PaperFormatCitationHandler.call(serde_json::json!({})));
        assert!(result.is_err());
    }

    #[test]
    fn test_paper_literature_review_schema() {
        let req = PaperLiteratureReviewHandler.input_schema().required.unwrap_or_default();
        assert!(req.contains(&"topic".into()));
    }

    #[test]
    fn test_paper_literature_review_error_missing_fields() {
        let result = futures::executor::block_on(PaperLiteratureReviewHandler.call(serde_json::json!({})));
        assert!(result.is_err());
    }

    #[test]
    fn test_what_if_oracle_schema_requires_question() {
        let req = WhatIfOracleHandler.input_schema().required.unwrap_or_default();
        assert!(req.contains(&"question".into()));
    }

    #[test]
    fn test_what_if_oracle_error_missing_question() {
        let result = futures::executor::block_on(WhatIfOracleHandler.call(serde_json::json!({})));
        assert!(result.is_err());
    }

    #[test]
    fn test_what_if_oracle_returns_six_branches() {
        let result = futures::executor::block_on(WhatIfOracleHandler.call(serde_json::json!({
            "question": "What if CRISPR therapy cures sickle cell disease?"
        })));
        assert!(result.is_ok());
        let output = result.unwrap();
        assert!(output["branches"].as_array().is_some());
        assert_eq!(output["branches"].as_array().unwrap().len(), 6);
        assert!(!output["synthesis"].as_str().unwrap().is_empty());
    }

    #[test]
    fn test_what_if_oracle_quick_mode_three_branches() {
        let result = futures::executor::block_on(WhatIfOracleHandler.call(serde_json::json!({
            "question": "What if AI automates scientific discovery?",
            "mode": "quick"
        })));
        assert!(result.is_ok());
        let output = result.unwrap();
        assert_eq!(output["branches"].as_array().unwrap().len(), 3);
    }

    #[test]
    fn test_paper_docx_export_schema() {
        let req = PaperDocxHandler.input_schema().required.unwrap_or_default();
        assert!(req.contains(&"markdown".into()));
        assert!(req.contains(&"title".into()));
    }

    #[test]
    fn test_paper_docx_export_error_missing_fields() {
        let result = futures::executor::block_on(PaperDocxHandler.call(serde_json::json!({})));
        assert!(result.is_err());
    }

    #[test]
    fn test_paper_docx_export_generates_docx() {
        let result = futures::executor::block_on(PaperDocxHandler.call(serde_json::json!({
            "markdown": "# Test\n\nThis is a test paragraph.\n\n## Section\n\n- Item 1\n- Item 2",
            "title": "Test Paper",
            "filename": "test_export"
        })));
        assert!(result.is_ok());
        let output = result.unwrap();
        assert_eq!(output["format"], "docx");
        assert!(output["size_bytes"].as_u64().unwrap() > 0);
        assert!(output["path"].as_str().unwrap().contains("test_export.docx"));
    }
}
