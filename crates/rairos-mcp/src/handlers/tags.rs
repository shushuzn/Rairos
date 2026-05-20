use crate::handlers::helpers::{append_jsonl, read_jsonl_async, tags_path, write_jsonl};
use crate::protocol::{ToolHandler, ToolInputSchema, ToolProperty};
use async_trait::async_trait;
use serde_json::Value;
use std::collections::HashMap;

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
        let entry = serde_json::json!({"arxiv_id": arxiv_id, "tag": tag, "created_at": crate::handlers::helpers::chrono_now()});
        append_jsonl(&tags_path(), &entry).await?;
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
        let entries = read_jsonl_async(&tags_path()).await;
        let before = entries.len();
        let filtered: Vec<Value> = entries.into_iter().filter(|e| {
            !(e["arxiv_id"].as_str() == Some(arxiv_id) && e["tag"].as_str() == Some(tag))
        }).collect();
        let removed = before - filtered.len();
        write_jsonl(&tags_path(), &filtered).await?;
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
        let entries = read_jsonl_async(&tags_path()).await;
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
