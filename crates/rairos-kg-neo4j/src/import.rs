//! Import tools for migrating data from SQLite rairos-kg to Neo4j.
//!
//! This module provides utilities to export data from the SQLite-based
//! `rairos-kg` and import it into the Neo4j-backed `rairos-kg-neo4j`.

use crate::client::{Neo4jKgClient, WriteStats};
use crate::error::KgError;
use std::collections::HashMap;

/// Import result summary
#[derive(Debug, Clone)]
pub struct ImportResult {
    pub nodes_created: usize,
    pub edges_created: usize,
    pub errors: Vec<String>,
}

impl ImportResult {
    pub fn new() -> Self {
        Self {
            nodes_created: 0,
            edges_created: 0,
            errors: vec![],
        }
    }

    pub fn add_error(&mut self, err: String) {
        self.errors.push(err);
    }

    pub fn merge(&mut self, stats: WriteStats) {
        self.nodes_created += stats.nodes_created as usize;
        self.edges_created += stats.relationships_created as usize;
    }
}

impl Default for ImportResult {
    fn default() -> Self {
        Self::new()
    }
}

/// Import data from a CSV export of rairos-kg
///
/// The CSV should have the following columns:
/// - entity_id, label, node_type, properties (JSON)
pub async fn import_nodes_from_csv(
    client: &Neo4jKgClient,
    csv_content: &str,
) -> Result<ImportResult, KgError> {
    let mut result = ImportResult::new();

    for (line_num, line) in csv_content.lines().enumerate() {
        if line_num == 0 {
            // Skip header
            continue;
        }

        let parts: Vec<&str> = line.split(',').collect();
        if parts.len() < 4 {
            result.add_error(format!("Line {}: Invalid format", line_num + 1));
            continue;
        }

        let entity_id = parts[0].trim_matches('"');
        let label = parts[1].trim_matches('"');
        let node_type = parts[2].trim_matches('"');
        let properties_str = parts[3].trim_matches('"');

        let properties: serde_json::Value = serde_json::from_str(properties_str)
            .unwrap_or(serde_json::json!({}));

        match node_type.to_lowercase().as_str() {
            "paper" => {
                match client.upsert_paper(entity_id, label, properties).await {
                    Ok(_) => result.nodes_created += 1,
                    Err(e) => result.add_error(format!("Paper {}: {}", entity_id, e)),
                };
            }
            "tag" => {
                match client.upsert_tag(entity_id, label).await {
                    Ok(_) => result.nodes_created += 1,
                    Err(e) => result.add_error(format!("Tag {}: {}", entity_id, e)),
                }
            }
            "author" => {
                match client.upsert_author(entity_id, label).await {
                    Ok(_) => result.nodes_created += 1,
                    Err(e) => result.add_error(format!("Author {}: {}", entity_id, e)),
                }
            }
            _ => {
                result.add_error(format!("Unknown node type: {}", node_type));
            }
        }
    }

    Ok(result)
}

/// Import edges from a CSV export
///
/// The CSV should have the following columns:
/// - source, target, edge_type, weight, properties (JSON)
pub async fn import_edges_from_csv(
    client: &Neo4jKgClient,
    csv_content: &str,
) -> Result<ImportResult, KgError> {
    let mut result = ImportResult::new();

    for (line_num, line) in csv_content.lines().enumerate() {
        if line_num == 0 {
            continue;
        }

        let parts: Vec<&str> = line.split(',').collect();
        if parts.len() < 3 {
            result.add_error(format!("Line {}: Invalid format", line_num + 1));
            continue;
        }

        let source = parts[0].trim_matches('"');
        let target = parts[1].trim_matches('"');
        let edge_type = parts[2].trim_matches('"');

        let edge_result = match edge_type.to_lowercase().as_str() {
            "cite" => client.add_citation(source, target).await,
            "derive" => client.add_author_paper(source, target).await,
            "same_tag" | "tagged_with" => client.add_paper_tag(source, target).await,
            _ => {
                result.add_error(format!("Unknown edge type: {}", edge_type));
                continue;
            }
        };

        match edge_result {
            Ok(_) => result.edges_created += 1,
            Err(e) => result.add_error(format!("Edge {} -> {}: {}", source, target, e)),
        }
    }

    Ok(result)
}

/// Import papers with their metadata (authors, tags, citations)
///
/// This is a convenience function that imports a paper and its related entities.
pub async fn import_paper_full(
    client: &Neo4jKgClient,
    entity_id: &str,
    title: &str,
    properties: serde_json::Value,
    authors: Vec<String>,
    tags: Vec<String>,
    cited_papers: Vec<String>,
) -> Result<ImportResult, KgError> {
    let mut result = ImportResult::new();

    // Create paper node
    match client.upsert_paper(entity_id, title, properties).await {
        Ok(_) => result.nodes_created += 1,
        Err(e) => {
            result.add_error(format!("Paper {}: {}", entity_id, e));
            return Ok(result);
        }
    }

    // Create author nodes and relationships
    for author_name in authors {
        let author_id = format!("author_{}", author_name.replace(' ', "_"));
        match client.upsert_author(&author_id, &author_name).await {
            Ok(_) => result.nodes_created += 1,
            Err(e) => result.add_error(format!("Author {}: {}", author_name, e)),
        }

        if client.add_author_paper(&author_id, entity_id).await.is_ok() {
            result.edges_created += 1;
        }
    }

    // Create tag nodes and relationships
    for tag_label in tags {
        let tag_id = format!("tag_{}", tag_label.replace(' ', "_"));
        match client.upsert_tag(&tag_id, &tag_label).await {
            Ok(_) => result.nodes_created += 1,
            Err(e) => result.add_error(format!("Tag {}: {}", tag_label, e)),
        }

        if client.add_paper_tag(entity_id, &tag_id).await.is_ok() {
            result.edges_created += 1;
        }
    }

    // Create citation relationships
    for cited_id in cited_papers {
        // First ensure the cited paper exists (create placeholder if needed)
        let _ = client.upsert_paper(&cited_id, &cited_id, serde_json::json!({})).await;

        if client.add_citation(entity_id, &cited_id).await.is_ok() {
            result.edges_created += 1;
        }
    }

    Ok(result)
}

/// Batch import multiple papers
pub async fn import_papers_batch(
    client: &Neo4jKgClient,
    papers: Vec<PaperImport>,
) -> Result<ImportResult, KgError> {
    let mut result = ImportResult::new();

    for paper in papers {
        let paper_result = import_paper_full(
            client,
            &paper.entity_id,
            &paper.title,
            paper.properties,
            paper.authors,
            paper.tags,
            paper.cited_papers,
        )
        .await?;

        result.nodes_created += paper_result.nodes_created;
        result.edges_created += paper_result.edges_created;
        result.errors.extend(paper_result.errors);
    }

    Ok(result)
}

/// Paper import structure
pub struct PaperImport {
    pub entity_id: String,
    pub title: String,
    pub properties: serde_json::Value,
    pub authors: Vec<String>,
    pub tags: Vec<String>,
    pub cited_papers: Vec<String>,
}

/// Clear all data from the database
pub async fn clear_database(client: &Neo4jKgClient) -> Result<WriteStats, KgError> {
    let query = "MATCH (n) DETACH DELETE n";
    client.execute_write(query, HashMap::new()).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::client::WriteStats;

    #[test]
    fn test_import_result() {
        let mut result = ImportResult::new();
        result.nodes_created = 10;
        result.edges_created = 5;
        result.add_error("Test error".to_string());

        assert_eq!(result.nodes_created, 10);
        assert_eq!(result.edges_created, 5);
        assert_eq!(result.errors.len(), 1);
    }

    #[test]
    fn test_import_result_new() {
        let result = ImportResult::new();
        assert_eq!(result.nodes_created, 0);
        assert_eq!(result.edges_created, 0);
        assert!(result.errors.is_empty());
    }

    #[test]
    fn test_import_result_add_error() {
        let mut result = ImportResult::new();
        result.add_error("Error 1".to_string());
        result.add_error("Error 2".to_string());

        assert_eq!(result.errors.len(), 2);
        assert_eq!(result.errors[0], "Error 1");
        assert_eq!(result.errors[1], "Error 2");
    }

    #[test]
    fn test_import_result_merge() {
        let mut result = ImportResult::new();
        result.nodes_created = 10;
        result.edges_created = 5;

        let stats = WriteStats {
            nodes_created: 3,
            nodes_deleted: 1,
            relationships_created: 2,
            relationships_deleted: 0,
        };

        result.merge(stats);

        assert_eq!(result.nodes_created, 13); // 10 + 3
        assert_eq!(result.edges_created, 7); // 5 + 2
        assert!(result.errors.is_empty());
    }

    #[test]
    fn test_import_result_merge_multiple() {
        let mut result = ImportResult::new();

        let stats1 = WriteStats {
            nodes_created: 5,
            nodes_deleted: 0,
            relationships_created: 3,
            relationships_deleted: 0,
        };
        result.merge(stats1);

        let stats2 = WriteStats {
            nodes_created: 2,
            nodes_deleted: 1,
            relationships_created: 1,
            relationships_deleted: 1,
        };
        result.merge(stats2);

        assert_eq!(result.nodes_created, 7); // 5 + 2
        assert_eq!(result.edges_created, 4); // 3 + 1
    }

    #[test]
    fn test_import_result_default() {
        let result = ImportResult::default();
        assert_eq!(result.nodes_created, 0);
        assert_eq!(result.edges_created, 0);
        assert!(result.errors.is_empty());
    }

    #[test]
    fn test_import_result_debug() {
        let result = ImportResult::new();
        let debug_str = format!("{:?}", result);
        assert!(debug_str.contains("nodes_created"));
        assert!(debug_str.contains("edges_created"));
    }

    #[test]
    fn test_paper_import_structure() {
        let paper = PaperImport {
            entity_id: "1706.03762".to_string(),
            title: "Attention Is All You Need".to_string(),
            properties: serde_json::json!({"year": 2017}),
            authors: vec!["Vaswani".to_string()],
            tags: vec!["transformer".to_string()],
            cited_papers: vec!["1703.12345".to_string()],
        };

        assert_eq!(paper.entity_id, "1706.03762");
        assert_eq!(paper.title, "Attention Is All You Need");
        assert_eq!(paper.authors.len(), 1);
        assert_eq!(paper.tags.len(), 1);
        assert_eq!(paper.cited_papers.len(), 1);
    }
}
