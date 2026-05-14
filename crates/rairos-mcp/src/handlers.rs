//! Tool handlers registry
//!
//! Each tool implements the ToolHandler trait. Handlers use Rust sub-crates
//! where possible and return JSON-RPC-compatible results.

use crate::protocol::{Tool, ToolHandler, ToolInputSchema, ToolProperty};
use async_trait::async_trait;
use serde_json::Value;
use std::collections::HashMap;

// ─── Paper Search ─────────────────────────────────────────────────────────────

pub struct PaperSearchHandler;

const ARXIV_API: &str = "http://export.arxiv.org/api/query";

#[async_trait]
impl ToolHandler for PaperSearchHandler {
    fn name(&self) -> &str {
        "paper_search"
    }

    fn description(&self) -> &str {
        "Search for research papers on arXiv by query"
    }

    fn input_schema(&self) -> ToolInputSchema {
        ToolInputSchema::object(
            vec![
                ("query".into(), ToolProperty::string("Search query")),
                (
                    "max_results".into(),
                    ToolProperty::integer("Maximum results to return (default 10)"),
                ),
            ]
            .into_iter()
            .collect(),
            vec!["query".into()],
        )
    }

    async fn call(&self, params: Value) -> Result<Value, String> {
        let query = params["query"]
            .as_str()
            .ok_or_else(|| "Missing required parameter: query".to_string())?;
        let max_results = params["max_results"].as_u64().unwrap_or(10).min(50) as usize;

        let url = format!(
            "{}?search_query=all:{}&start=0&max_results={}",
            ARXIV_API,
            query.replace(' ', "+"),
            max_results
        );

        let resp = reqwest::get(&url)
            .await
            .map_err(|e| format!("arXiv API request failed: {}", e))?;

        let text = resp
            .text()
            .await
            .map_err(|e| format!("Failed to read response: {}", e))?;

        let papers = parse_arxiv_response(&text);

        Ok(serde_json::json!({
            "papers": papers,
            "total": papers.len()
        }))
    }
}

// ─── Paper Ingest ─────────────────────────────────────────────────────────────

pub struct PaperIngestHandler;

#[async_trait]
impl ToolHandler for PaperIngestHandler {
    fn name(&self) -> &str {
        "paper_ingest"
    }

    fn description(&self) -> &str {
        "Ingest a paper by arXiv ID — fetch metadata from arXiv"
    }

    fn input_schema(&self) -> ToolInputSchema {
        ToolInputSchema::object(
            vec![("arxiv_id".into(), ToolProperty::string("arXiv ID to ingest"))]
                .into_iter()
                .collect(),
            vec!["arxiv_id".into()],
        )
    }

    async fn call(&self, params: Value) -> Result<Value, String> {
        let arxiv_id = params["arxiv_id"]
            .as_str()
            .ok_or_else(|| "Missing required parameter: arxiv_id".to_string())?;

        let url = format!("http://export.arxiv.org/api/query?id_list={}", arxiv_id);
        let resp = reqwest::get(&url)
            .await
            .map_err(|e| format!("arXiv API request failed: {}", e))?;

        let text = resp
            .text()
            .await
            .map_err(|e| format!("Failed to read response: {}", e))?;

        let papers = parse_arxiv_response(&text);
        let paper = papers.into_iter().next().ok_or_else(|| {
            format!("No paper found for arXiv ID: {}", arxiv_id)
        })?;

        Ok(serde_json::json!(paper))
    }
}

// ─── Tag List ─────────────────────────────────────────────────────────────────

pub struct TagListHandler;

#[async_trait]
impl ToolHandler for TagListHandler {
    fn name(&self) -> &str {
        "tag_list"
    }

    fn description(&self) -> &str {
        "List all tags. Tags are a way to organize and categorize papers in the database."
    }

    fn input_schema(&self) -> ToolInputSchema {
        ToolInputSchema::object(HashMap::new(), vec![])
    }

    async fn call(&self, _params: Value) -> Result<Value, String> {
        // Returns a placeholder until DB integration is added
        Ok(serde_json::json!({
            "tags": [],
            "note": "Tag listing requires database connection — use Python MCP server for full functionality"
        }))
    }
}

// ─── Register all tools ───────────────────────────────────────────────────────

pub async fn register_all(server: &crate::McpServer) {
    server.register(PaperSearchHandler).await;
    server.register(PaperIngestHandler).await;
    server.register(TagListHandler).await;
}

// ─── arXiv XML Parser (self-contained, no XML crate) ──────────────────────────

fn parse_arxiv_response(xml: &str) -> Vec<serde_json::Value> {
    let mut papers = Vec::new();
    let mut pos = 0;

    while let Some(entry_start) = xml[pos..].find("<entry>") {
        let abs_start = pos + entry_start;
        let Some(entry_end) = xml[abs_start..].find("</entry>") else {
            break;
        };
        let entry = &xml[abs_start..abs_start + entry_end + 8];
        pos = abs_start + entry_end + 8;

        let id = extract_tag(entry, "id").unwrap_or_default();
        let published = extract_tag(entry, "published").unwrap_or_default();
        let title = extract_tag(entry, "title").map(clean_xml).unwrap_or_default();
        let summary = extract_tag(entry, "summary").map(clean_xml).unwrap_or_default();
        let authors = extract_authors(entry);
        let categories = extract_categories(entry);

        let arxiv_id = id
            .strip_prefix("http://arxiv.org/abs/")
            .or_else(|| id.strip_prefix("https://arxiv.org/abs/"))
            .map(|s| s.to_string())
            .unwrap_or_default();

        papers.push(serde_json::json!({
            "arxiv_id": arxiv_id,
            "title": title,
            "abstract": summary,
            "authors": authors,
            "categories": categories,
            "published": published,
            "pdf_url": format!("https://arxiv.org/pdf/{}.pdf", arxiv_id),
            "abs_url": id,
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
    s.trim()
        .replace('\n', " ")
        .replace("  ", " ")
        .replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&apos;", "'")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn extract_authors(entry: &str) -> Vec<String> {
    let mut authors = Vec::new();
    let mut pos = 0;
    while let Some(start) = entry[pos..].find("<author>") {
        let abs_start = pos + start;
        let Some(end) = entry[abs_start..].find("</author>") else {
            break;
        };
        let author_block = &entry[abs_start..abs_start + end + 9];
        if let Some(name) = extract_tag(author_block, "name") {
            authors.push(name);
        }
        pos = abs_start + end + 9;
    }
    authors
}

fn extract_categories(entry: &str) -> Vec<String> {
    let mut cats = Vec::new();
    let mut pos = 0;
    while let Some(start) = entry[pos..].find("term=\"") {
        let after = &entry[pos + start + 6..];
        if let Some(end) = after.find('"') {
            cats.push(after[..end].to_string());
        }
        pos += start + 6;
    }
    cats
}

// ─── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_arxiv_response() {
        let xml = r#"<?xml version="1.0"?><feed>
<entry>
  <id>http://arxiv.org/abs/2401.12345</id>
  <published>2024-01-01</published>
  <title>Test Title About Neural Networks</title>
  <summary>This is a test abstract about deep learning.</summary>
  <author><name>John Doe</name></author>
  <category term="cs.LG"/>
</entry></feed>"#;
        let papers = parse_arxiv_response(xml);
        assert_eq!(papers.len(), 1);
        assert_eq!(papers[0]["arxiv_id"].as_str(), Some("2401.12345"));
        assert_eq!(papers[0]["authors"][0].as_str(), Some("John Doe"));
    }

    #[test]
    fn test_empty_response() {
        let xml = r#"<?xml version="1.0"?><feed></feed>"#;
        let papers = parse_arxiv_response(xml);
        assert!(papers.is_empty());
    }

    #[test]
    fn test_paper_search_schema() {
        let h = PaperSearchHandler;
        assert_eq!(h.name(), "paper_search");
        let schema = h.input_schema();
        assert!(schema.properties.is_some());
        assert!(schema.required.is_some());
    }

    #[test]
    fn test_tag_list_schema() {
        let h = TagListHandler;
        assert_eq!(h.name(), "tag_list");
    }
}
