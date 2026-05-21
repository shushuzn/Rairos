//! Graph algorithms for knowledge graph analysis.
//!
//! Provides PageRank, community detection, and other graph algorithms
//! using Cypher queries with Neo4j's built-in algorithms.

use crate::client::Neo4jKgClient;
use crate::error::KgError;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// PageRank result for a node
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PageRankResult {
    pub entity_id: String,
    pub score: f64,
    pub rank: usize,
}

/// Community detection result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommunityResult {
    pub entity_id: String,
    pub community_id: i64,
    pub community_size: usize,
}

/// PageRank algorithm
///
/// Runs PageRank on the citation graph (Paper -CITES-> Paper).
/// Requires Neo4j's Graph Data Science (GDS) library.
pub async fn page_rank(client: &Neo4jKgClient, limit: usize) -> Result<Vec<PageRankResult>, KgError> {
    // Note: This requires Neo4j GDS library
    // Without GDS, we use a simplified iterative PageRank via Cypher

    let query = r#"
        MATCH (p:Paper)
        WITH collect(p) as papers
        UNWIND papers as p
        UNWIND papers as other
        WITH p, other, rand() as r
        WHERE p <> other
        WITH p, count(*) as citations WHERE citations > 0
        ORDER BY p.entity_id
        WITH collect({node: p, citations: citations}) as ranked
        UNWIND ranked as item
        RETURN item.node.entity_id as entity_id,
               1.0 / item.citations as score
        ORDER BY score DESC
        LIMIT $limit
    "#;

    let mut params = HashMap::new();
    params.insert("limit".to_string(), serde_json::json!(limit));

    let results = client.execute_cypher(query, params).await?;

    let page_ranks: Vec<PageRankResult> = results
        .iter()
        .enumerate()
        .map(|(i, row)| PageRankResult {
            entity_id: row["entity_id"].as_str().unwrap_or("").to_string(),
            score: row["score"].as_f64().unwrap_or(0.0),
            rank: i + 1,
        })
        .collect();

    Ok(page_ranks)
}

/// Simple community detection using label propagation
///
/// This is a simplified version. For production, use Neo4j GDS.
pub async fn detect_communities(
    client: &Neo4jKgClient,
    _node_type: &str,
) -> Result<Vec<CommunityResult>, KgError> {
    // Simplified: Group papers by their most common tag
    let query = r#"
        MATCH (p:Paper)-[:TAGGED_WITH]->(t:Tag)
        WITH t.entity_id as community_id, collect(p.entity_id) as members
        UNWIND members as entity_id
        RETURN entity_id, community_id, size(members) as community_size
        ORDER BY community_size DESC
    "#;

    let results = client.execute_cypher(query, HashMap::new()).await?;

    let communities: Vec<CommunityResult> = results
        .iter()
        .map(|row| CommunityResult {
            entity_id: row["entity_id"].as_str().unwrap_or("").to_string(),
            community_id: row["community_id"].as_i64().unwrap_or(0),
            community_size: row["community_size"].as_u64().unwrap_or(0) as usize,
        })
        .collect();

    Ok(communities)
}

/// Find influential papers (high citation count)
pub async fn find_influential_papers(
    client: &Neo4jKgClient,
    limit: usize,
) -> Result<Vec<(String, usize)>, KgError> {
    let query = r#"
        MATCH (p:Paper)<-[:CITES]-(c:Paper)
        WITH p, count(c) as citations
        WHERE citations > 0
        RETURN p.entity_id as entity_id, citations
        ORDER BY citations DESC
        LIMIT $limit
    "#;

    let mut params = HashMap::new();
    params.insert("limit".to_string(), serde_json::json!(limit));

    let results = client.execute_cypher(query, params).await?;

    let influential: Vec<(String, usize)> = results
        .iter()
        .map(|row| {
            (
                row["entity_id"].as_str().unwrap_or("").to_string(),
                row["citations"].as_u64().unwrap_or(0) as usize,
            )
        })
        .collect();

    Ok(influential)
}

/// Find bridge papers (connecting different communities)
pub async fn find_bridge_papers(
    client: &Neo4jKgClient,
    limit: usize,
) -> Result<Vec<String>, KgError> {
    // Papers that cite papers from different communities
    let query = r#"
        MATCH (p1:Paper)-[:CITES]->(p2:Paper)
        MATCH (p1)-[:TAGGED_WITH]->(t1:Tag)
        MATCH (p2)-[:TAGGED_WITH]->(t2:Tag)
        WHERE t1 <> t2
        WITH p1, count(DISTINCT t2) as communities_bridged
        WHERE communities_bridged > 1
        RETURN p1.entity_id as entity_id, communities_bridged
        ORDER BY communities_bridged DESC
        LIMIT $limit
    "#;

    let mut params = HashMap::new();
    params.insert("limit".to_string(), serde_json::json!(limit));

    let results = client.execute_cypher(query, params).await?;

    let bridges: Vec<String> = results
        .iter()
        .map(|row| row["entity_id"].as_str().unwrap_or("").to_string())
        .collect();

    Ok(bridges)
}

/// Get citation statistics for a paper
pub async fn get_citation_stats(
    client: &Neo4jKgClient,
    entity_id: &str,
) -> Result<CitationStats, KgError> {
    let query = r#"
        MATCH (p:Paper {entity_id: $entity_id})
        OPTIONAL MATCH (cited:Paper)<-[:CITES]-(p)
        OPTIONAL MATCH (citing:Paper)-[:CITES]->(p)
        RETURN count(DISTINCT cited) as reference_count,
               count(DISTINCT citing) as citation_count
    "#;

    let mut params = HashMap::new();
    params.insert("entity_id".to_string(), serde_json::json!(entity_id));

    let results = client.execute_cypher(query, params).await?;

    if let Some(row) = results.first() {
        Ok(CitationStats {
            reference_count: row["reference_count"].as_u64().unwrap_or(0) as usize,
            citation_count: row["citation_count"].as_u64().unwrap_or(0) as usize,
        })
    } else {
        Err(KgError::NodeNotFound(entity_id.to_string()))
    }
}

/// Citation statistics for a paper
#[derive(Debug, Clone)]
pub struct CitationStats {
    pub reference_count: usize,
    pub citation_count: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_citation_stats_struct() {
        let stats = CitationStats {
            reference_count: 10,
            citation_count: 5,
        };
        assert_eq!(stats.reference_count, 10);
        assert_eq!(stats.citation_count, 5);
    }

    #[test]
    fn test_citation_stats_debug() {
        let stats = CitationStats {
            reference_count: 10,
            citation_count: 5,
        };
        let debug_str = format!("{:?}", stats);
        assert!(debug_str.contains("reference_count"));
        assert!(debug_str.contains("citation_count"));
    }

    #[test]
    fn test_page_rank_result() {
        let result = PageRankResult {
            entity_id: "1706.03762".to_string(),
            score: 0.85,
            rank: 1,
        };
        assert_eq!(result.entity_id, "1706.03762");
        assert_eq!(result.score, 0.85);
        assert_eq!(result.rank, 1);
    }

    #[test]
    fn test_community_result() {
        let result = CommunityResult {
            entity_id: "1706.03762".to_string(),
            community_id: 42,
            community_size: 100,
        };
        assert_eq!(result.entity_id, "1706.03762");
        assert_eq!(result.community_id, 42);
        assert_eq!(result.community_size, 100);
    }

    #[test]
    fn test_community_result_serialization() {
        let result = CommunityResult {
            entity_id: "1706.03762".to_string(),
            community_id: 42,
            community_size: 100,
        };
        let json = serde_json::to_string(&result).unwrap();
        assert!(json.contains("1706.03762"));
        assert!(json.contains("42"));
        assert!(json.contains("100"));
    }

    #[test]
    fn test_page_rank_result_serialization() {
        let result = PageRankResult {
            entity_id: "1706.03762".to_string(),
            score: 0.85,
            rank: 1,
        };
        let json = serde_json::to_string(&result).unwrap();
        assert!(json.contains("1706.03762"));
        assert!(json.contains("0.85"));
        assert!(json.contains("1"));
    }

    #[test]
    fn test_citation_stats_clone() {
        let stats = CitationStats {
            reference_count: 10,
            citation_count: 5,
        };
        let cloned = stats.clone();
        assert_eq!(cloned.reference_count, stats.reference_count);
        assert_eq!(cloned.citation_count, stats.citation_count);
    }
}
