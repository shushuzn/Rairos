//! Hybrid retrieval combining vector similarity with knowledge graph context.

use crate::GraphRagConfig;

/// A retrieval result combining vector and graph data
#[derive(Debug, Clone)]
pub struct HybridSearchResult {
    /// Entity ID (e.g., paper arxiv_id)
    pub entity_id: String,
    /// Retrieval score (combined vector + graph)
    pub score: f32,
    /// Vector similarity score
    pub vector_score: f32,
    /// Graph proximity score
    pub graph_score: f32,
    /// Node type
    pub node_type: String,
    /// Display label/title
    pub label: String,
    /// Additional metadata
    pub metadata: serde_json::Value,
    /// Related entities from graph
    pub related_entities: Vec<RelatedEntity>,
}

/// A related entity from the knowledge graph
#[derive(Debug, Clone)]
pub struct RelatedEntity {
    pub entity_id: String,
    pub relation: String,
    pub label: String,
}

/// Hybrid retriever that combines vector search with graph queries
pub struct HybridRetriever {
    /// Weight for vector similarity (0.0 to 1.0)
    vector_weight: f32,
    /// Weight for graph proximity (0.0 to 1.0)
    graph_weight: f32,
}

impl HybridRetriever {
    pub fn new(vector_weight: f32, graph_weight: f32) -> Self {
        Self {
            vector_weight,
            graph_weight,
        }
    }

    pub fn with_config(config: &GraphRagConfig) -> Self {
        Self {
            vector_weight: config.vector_weight,
            graph_weight: config.graph_weight,
        }
    }

    /// Combine vector and graph scores into a single score
    pub fn combine_scores(&self, vector_score: f32, graph_score: f32) -> f32 {
        let total_weight = self.vector_weight + self.graph_weight;
        if total_weight == 0.0 {
            return vector_score;
        }
        (self.vector_weight * vector_score + self.graph_weight * graph_score) / total_weight
    }

    /// Boost score based on graph connections
    ///
    /// Papers that are highly cited or connect different communities
    /// get a boost in the final score.
    pub fn apply_graph_boost(&self, score: f32, citation_count: usize, community_bridges: usize) -> f32 {
        let citation_boost = 1.0 + (citation_count as f32 * 0.01).min(0.5);
        let bridge_boost = 1.0 + (community_bridges as f32 * 0.05).min(0.3);
        score * citation_boost * bridge_boost
    }
}

/// Configuration for retrieval
#[derive(Debug, Clone)]
pub struct RetrievalConfig {
    /// Number of results to retrieve from vector store
    pub vector_top_k: usize,
    /// Number of results to retrieve from KG
    pub graph_top_k: usize,
    /// Maximum related entities per result
    pub max_related: usize,
    /// Minimum score threshold
    pub min_score: f32,
}

impl Default for RetrievalConfig {
    fn default() -> Self {
        Self {
            vector_top_k: 10,
            graph_top_k: 10,
            max_related: 5,
            min_score: 0.0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_combine_scores_normal() {
        let retriever = HybridRetriever::new(0.7, 0.3);
        // 0.7 * 0.8 + 0.3 * 0.6 = 0.56 + 0.18 = 0.74
        let combined = retriever.combine_scores(0.8, 0.6);
        assert!((combined - 0.74).abs() < 0.001);
    }

    #[test]
    fn test_combine_scores_equal_weights() {
        let retriever = HybridRetriever::new(0.5, 0.5);
        // (0.5 * 1.0 + 0.5 * 0.0) / 1.0 = 0.5
        let combined = retriever.combine_scores(1.0, 0.0);
        assert!((combined - 0.5).abs() < 0.001);
    }

    #[test]
    fn test_combine_scores_zero_total_weight() {
        let retriever = HybridRetriever::new(0.0, 0.0);
        // Should return vector_score when total weight is 0
        let combined = retriever.combine_scores(0.8, 0.6);
        assert!((combined - 0.8).abs() < 0.001);
    }

    #[test]
    fn test_combine_scores_vector_only() {
        let retriever = HybridRetriever::new(1.0, 0.0);
        let combined = retriever.combine_scores(0.9, 0.3);
        assert!((combined - 0.9).abs() < 0.001);
    }

    #[test]
    fn test_combine_scores_graph_only() {
        let retriever = HybridRetriever::new(0.0, 1.0);
        let combined = retriever.combine_scores(0.2, 0.8);
        assert!((combined - 0.8).abs() < 0.001);
    }

    #[test]
    fn test_apply_graph_boost_no_citations() {
        let retriever = HybridRetriever::new(0.5, 0.5);
        let boosted = retriever.apply_graph_boost(1.0, 0, 0);
        // citation_boost = 1.0 + min(0 * 0.01, 0.5) = 1.0
        // bridge_boost = 1.0 + min(0 * 0.05, 0.3) = 1.0
        // result = 1.0 * 1.0 * 1.0 = 1.0
        assert!((boosted - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_apply_graph_boost_high_citations() {
        let retriever = HybridRetriever::new(0.5, 0.5);
        let boosted = retriever.apply_graph_boost(1.0, 100, 0);
        // citation_boost = 1.0 + min(100 * 0.01, 0.5) = 1.5
        // result = 1.0 * 1.5 * 1.0 = 1.5
        assert!((boosted - 1.5).abs() < 0.001);
    }

    #[test]
    fn test_apply_graph_boost_bridges() {
        let retriever = HybridRetriever::new(0.5, 0.5);
        let boosted = retriever.apply_graph_boost(1.0, 0, 10);
        // bridge_boost = 1.0 + min(10 * 0.05, 0.3) = 1.0 + 0.3 = 1.3 (capped at 0.3)
        // result = 1.0 * 1.0 * 1.3 = 1.3
        assert!((boosted - 1.3).abs() < 0.001);
    }

    #[test]
    fn test_apply_graph_boost_both() {
        let retriever = HybridRetriever::new(0.5, 0.5);
        // High citations (capped at 0.5 boost) and high bridges (capped at 0.3 boost)
        let boosted = retriever.apply_graph_boost(1.0, 100, 10);
        // citation_boost = 1.5 (capped)
        // bridge_boost = 1.3 (capped)
        // result = 1.0 * 1.5 * 1.3 = 1.95
        assert!((boosted - 1.95).abs() < 0.001);
    }

    #[test]
    fn test_apply_graph_boost_with_base_score() {
        let retriever = HybridRetriever::new(0.5, 0.5);
        let boosted = retriever.apply_graph_boost(2.0, 50, 4);
        // citation_boost = 1.0 + min(50 * 0.01, 0.5) = 1.5
        // bridge_boost = 1.0 + min(4 * 0.05, 0.3) = 1.2
        // result = 2.0 * 1.5 * 1.2 = 3.6
        assert!((boosted - 3.6).abs() < 0.001);
    }

    #[test]
    fn test_retrieval_config_default() {
        let config = RetrievalConfig::default();
        assert_eq!(config.vector_top_k, 10);
        assert_eq!(config.graph_top_k, 10);
        assert_eq!(config.max_related, 5);
        assert_eq!(config.min_score, 0.0);
    }
}
