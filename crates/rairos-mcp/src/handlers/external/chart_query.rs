use crate::handlers::helpers::kg;
use crate::protocol::{ToolHandler, ToolInputSchema, ToolProperty};
use async_trait::async_trait;
use serde_json::Value;

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

        let db = kg().await.database().ok_or("KG database not available")?;

        let paper_node = db.get_node_by_entity("paper", paper_id)
            .await.map_err(|e| format!("KG query error: {}", e))?
            .ok_or_else(|| format!("Paper not found: {}", paper_id))?;

        let fig_edges = db.get_edges_by_node(&paper_node.id, "out", Some("has_figure"))
            .await.map_err(|e| format!("KG edge query: {}", e))?;
        let tbl_edges = db.get_edges_by_node(&paper_node.id, "out", Some("has_table"))
            .await.map_err(|e| format!("KG edge query: {}", e))?;

        let mut figures = Vec::new();
        for edge in &fig_edges {
            if let Ok(Some(node)) = db.get_node(&edge.target).await {
                let props = &node.properties;
                figures.push(serde_json::json!({
                    "label": node.label,
                    "page": props.get("page").and_then(|v| v.as_u64()).unwrap_or(0) + 1,
                    "description": props.get("description").and_then(|v| v.as_str()).unwrap_or(""),
                }));
            }
        }

        let mut tables = Vec::new();
        for edge in &tbl_edges {
            if let Ok(Some(node)) = db.get_node(&edge.target).await {
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
                let fig_label_lower = fig_label.to_lowercase();
                let fig = figures.into_iter().find(|f| {
                    f.get("label").and_then(|v| v.as_str()).is_some_and(|l| {
                        l.to_lowercase().contains(&fig_label_lower)
                    })
                });
                match fig {
                    Some(f) => {
                        let fig_node = db.get_node_by_entity("figure", fig_label)
                            .await.map_err(|e| format!("KG query: {}", e))?;
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
                let tbl_label_lower = tbl_label.to_lowercase();
                let tbl = tables.into_iter().find(|t| {
                    t.get("label").and_then(|v| v.as_str()).is_some_and(|l| {
                        l.to_lowercase().contains(&tbl_label_lower)
                    })
                });
                match tbl {
                    Some(t) => {
                        let tbl_node = db.get_node_by_entity("table", tbl_label)
                            .await.map_err(|e| format!("KG query: {}", e))?;
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
