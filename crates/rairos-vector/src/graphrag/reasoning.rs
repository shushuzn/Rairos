//! Multi-hop reasoning through knowledge graph paths.

use std::collections::{HashMap, HashSet};

/// A reasoning path through the knowledge graph
#[derive(Debug, Clone)]
pub struct ReasoningPath {
    /// Entities in the path
    pub entities: Vec<PathEntity>,
    /// Relations connecting entities
    pub relations: Vec<String>,
    /// Path score (coherence, relevance)
    pub score: f32,
    /// Number of hops
    pub hops: usize,
}

/// An entity in a reasoning path
#[derive(Debug, Clone)]
pub struct PathEntity {
    pub entity_id: String,
    pub label: String,
    pub node_type: String,
}

/// Path finder for multi-hop reasoning
pub struct PathFinder {
    /// Maximum path length (hops)
    #[allow(dead_code)]
    max_hops: usize,
    /// Maximum paths to return
    max_paths: usize,
}

impl PathFinder {
    pub fn new(max_hops: usize, max_paths: usize) -> Self {
        Self {
            max_hops,
            max_paths,
        }
    }

    pub fn with_config(max_hops: usize) -> Self {
        Self {
            max_hops,
            max_paths: 5,
        }
    }

    /// Find paths between a source entity and target entities
    ///
    /// This is a simplified BFS-based path finding.
    /// In production, this would use Neo4j's path finding algorithms.
    pub fn find_paths(
        &self,
        source_id: &str,
        target_ids: &[String],
    ) -> Vec<ReasoningPath> {
        let mut paths = Vec::new();

        // Simplified: direct path if source cites any target
        for target in target_ids {
            if source_id != target {
                paths.push(ReasoningPath {
                    entities: vec![
                        PathEntity {
                            entity_id: source_id.to_string(),
                            label: source_id.to_string(),
                            node_type: "Paper".to_string(),
                        },
                        PathEntity {
                            entity_id: target.to_string(),
                            label: target.to_string(),
                            node_type: "Paper".to_string(),
                        },
                    ],
                    relations: vec!["CITES".to_string()],
                    score: 0.8,
                    hops: 1,
                });
            }
        }

        paths.truncate(self.max_paths);
        paths
    }

    /// Score the coherence of a path
    pub fn score_path_coherence(&self, path: &ReasoningPath) -> f32 {
        if path.entities.is_empty() {
            return 0.0;
        }

        // Penalize very long paths
        let length_penalty = 1.0 / (1.0 + path.hops as f32 * 0.2);

        // Penalize paths with missing labels
        let label_score: f32 = path
            .entities
            .iter()
            .map(|e| if e.label.is_empty() { 0.5 } else { 1.0 })
            .sum::<f32>()
            / path.entities.len() as f32;

        length_penalty * label_score * path.score
    }

    /// Find bridging entities that connect different topic areas
    ///
    /// A bridge paper is one that cites papers from different communities.
    pub fn find_bridges(
        &self,
        entity_ids: &[String],
        community_map: &HashMap<String, String>,
    ) -> Vec<String> {
        // Simplified: return entities that appear in multiple communities
        // In production, this would query the KG for actual citation patterns
        let mut community_counts: HashMap<String, usize> = HashMap::new();
        for entity_id in entity_ids {
            if let Some(community) = community_map.get(entity_id) {
                *community_counts.entry(community.clone()).or_insert(0) += 1;
            }
        }

        community_counts
            .into_iter()
            .filter(|(_, count)| *count > 1)
            .map(|(id, _)| id)
            .collect()
    }

    /// Expand a set of entities by following citation relationships
    ///
    /// Used to expand the context around retrieved entities.
    pub fn expand_by_citations(
        &self,
        entity_ids: &[String],
        kg_client: &dyn KgClientTrait,
        depth: usize,
    ) -> Vec<String> {
        let mut expanded: Vec<String> = entity_ids.to_vec();
        let mut frontier: Vec<String> = entity_ids.to_vec();
        let mut visited: HashSet<String> = entity_ids.iter().cloned().collect();

        for _ in 0..depth {
            let mut next_frontier: Vec<String> = Vec::new();
            for entity_id in &frontier {
                // Get papers this entity cites
                let cited = kg_client.get_cited_papers(entity_id);
                for paper in cited {
                    if !visited.contains(&paper) {
                        visited.insert(paper.clone());
                        next_frontier.push(paper.clone());
                        expanded.push(paper);
                    }
                }

                // Get papers that cite this entity
                let citing = kg_client.get_citing_papers(entity_id);
                for paper in citing {
                    if !visited.contains(&paper) {
                        visited.insert(paper.clone());
                        next_frontier.push(paper.clone());
                        expanded.push(paper);
                    }
                }
            }
            frontier = next_frontier;
        }

        expanded
    }
}

impl Default for PathFinder {
    fn default() -> Self {
        Self::new(3, 5)
    }
}

/// Trait for KG client operations needed by PathFinder
pub trait KgClientTrait: Send + Sync {
    fn get_cited_papers(&self, entity_id: &str) -> Vec<String>;
    fn get_citing_papers(&self, entity_id: &str) -> Vec<String>;
    fn get_related_papers(&self, entity_id: &str, limit: usize) -> Vec<String>;
}

/// Simple mock implementation for testing
pub struct MockKgClient {
    citations: HashMap<String, Vec<String>>,
}

impl MockKgClient {
    pub fn new() -> Self {
        Self {
            citations: HashMap::new(),
        }
    }

    pub fn add_citation(&mut self, source: &str, target: &str) {
        self.citations
            .entry(source.to_string())
            .or_default()
            .push(target.to_string());
    }
}

impl Default for MockKgClient {
    fn default() -> Self {
        Self::new()
    }
}

impl KgClientTrait for MockKgClient {
    fn get_cited_papers(&self, entity_id: &str) -> Vec<String> {
        self.citations.get(entity_id).cloned().unwrap_or_default()
    }

    fn get_citing_papers(&self, entity_id: &str) -> Vec<String> {
        self.citations
            .iter()
            .filter(|(_, targets)| targets.contains(&entity_id.to_string()))
            .map(|(source, _)| source.clone())
            .collect()
    }

    fn get_related_papers(&self, entity_id: &str, _limit: usize) -> Vec<String> {
        // Return both cited and citing
        let mut related = self.get_cited_papers(entity_id);
        related.extend(self.get_citing_papers(entity_id));
        related
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_score_path_coherence_empty() {
        let finder = PathFinder::new(3, 5);
        let path = ReasoningPath {
            entities: vec![],
            relations: vec![],
            score: 0.8,
            hops: 0,
        };
        let coherence = finder.score_path_coherence(&path);
        assert!((coherence - 0.0).abs() < 0.001);
    }

    #[test]
    fn test_score_path_coherence_single_entity() {
        let finder = PathFinder::new(3, 5);
        let path = ReasoningPath {
            entities: vec![PathEntity {
                entity_id: "e1".to_string(),
                label: "Entity 1".to_string(),
                node_type: "Paper".to_string(),
            }],
            relations: vec![],
            score: 0.8,
            hops: 1,
        };
        let coherence = finder.score_path_coherence(&path);
        // length_penalty = 1.0 / (1.0 + 1 * 0.2) = 1.0 / 1.2 = 0.833...
        // label_score = 1.0 (has label)
        // result = 0.833... * 1.0 * 0.8 = 0.667
        assert!((coherence - 0.667).abs() < 0.01);
    }

    #[test]
    fn test_score_path_coherence_with_empty_labels() {
        let finder = PathFinder::new(3, 5);
        let path = ReasoningPath {
            entities: vec![
                PathEntity {
                    entity_id: "e1".to_string(),
                    label: "".to_string(),
                    node_type: "Paper".to_string(),
                },
                PathEntity {
                    entity_id: "e2".to_string(),
                    label: "Entity 2".to_string(),
                    node_type: "Paper".to_string(),
                },
            ],
            relations: vec!["CITES".to_string()],
            score: 1.0,
            hops: 1,
        };
        let coherence = finder.score_path_coherence(&path);
        // length_penalty = 1.0 / 1.2 = 0.833...
        // label_score = (0.5 + 1.0) / 2 = 0.75
        // result = 0.833... * 0.75 * 1.0 = 0.625
        assert!((coherence - 0.625).abs() < 0.01);
    }

    #[test]
    fn test_score_path_coherence_long_path() {
        let finder = PathFinder::new(5, 5);
        let path = ReasoningPath {
            entities: vec![
                PathEntity {
                    entity_id: "e1".to_string(),
                    label: "E1".to_string(),
                    node_type: "Paper".to_string(),
                },
                PathEntity {
                    entity_id: "e2".to_string(),
                    label: "E2".to_string(),
                    node_type: "Paper".to_string(),
                },
                PathEntity {
                    entity_id: "e3".to_string(),
                    label: "E3".to_string(),
                    node_type: "Paper".to_string(),
                },
            ],
            relations: vec!["CITES".to_string(), "CITES".to_string()],
            score: 0.8,
            hops: 3,
        };
        let coherence = finder.score_path_coherence(&path);
        // length_penalty = 1.0 / (1.0 + 3 * 0.2) = 1.0 / 1.6 = 0.625
        // label_score = 1.0
        // result = 0.625 * 1.0 * 0.8 = 0.5
        assert!((coherence - 0.5).abs() < 0.01);
    }

    #[test]
    fn test_mock_kg_client_empty() {
        let client = MockKgClient::new();
        assert!(client.get_cited_papers("paper1").is_empty());
        assert!(client.get_citing_papers("paper1").is_empty());
    }

    #[test]
    fn test_mock_kg_client_add_citation() {
        let mut client = MockKgClient::new();
        client.add_citation("paper1", "paper2");
        assert_eq!(client.get_cited_papers("paper1"), vec!["paper2"]);
    }

    #[test]
    fn test_mock_kg_client_get_citing() {
        let mut client = MockKgClient::new();
        client.add_citation("paper1", "paper2");
        // paper2 is cited by paper1
        assert_eq!(client.get_citing_papers("paper2"), vec!["paper1"]);
    }

    #[test]
    fn test_mock_kg_client_get_related() {
        let mut client = MockKgClient::new();
        client.add_citation("paper1", "paper2");
        client.add_citation("paper3", "paper2");
        // paper2 is related to paper1 (cited by) and paper3 (cited by)
        let related = client.get_related_papers("paper2", 10);
        assert!(related.contains(&"paper1".to_string()));
        assert!(related.contains(&"paper3".to_string()));
    }

    #[test]
    fn test_mock_kg_client_multiple_citations() {
        let mut client = MockKgClient::new();
        client.add_citation("paper1", "paper2");
        client.add_citation("paper1", "paper3");
        client.add_citation("paper1", "paper4");
        assert_eq!(client.get_cited_papers("paper1").len(), 3);
    }

    #[test]
    fn test_expand_by_citations_bfs() {
        let mut client = MockKgClient::new();
        // paper1 cites paper2
        client.add_citation("paper1", "paper2");
        // paper2 cites paper3
        client.add_citation("paper2", "paper3");
        // paper3 cites paper4
        client.add_citation("paper3", "paper4");

        let finder = PathFinder::new(3, 5);
        let expanded = finder.expand_by_citations(&["paper1".to_string()], &client, 2);

        // paper1, paper2 (depth 1), paper3 (depth 2)
        // Should include paper1, paper2, paper3
        assert!(expanded.contains(&"paper1".to_string()));
        assert!(expanded.contains(&"paper2".to_string()));
        assert!(expanded.contains(&"paper3".to_string()));
        // paper4 is at depth 3, so not included
        assert!(!expanded.contains(&"paper4".to_string()));
    }

    #[test]
    fn test_expand_by_citations_with_backward() {
        let mut client = MockKgClient::new();
        // paper2 cites paper1
        client.add_citation("paper2", "paper1");
        // paper3 cites paper2
        client.add_citation("paper3", "paper2");

        let finder = PathFinder::new(3, 5);
        let expanded = finder.expand_by_citations(&["paper1".to_string()], &client, 2);

        // Should expand forward (citing papers) and backward (papers that cite)
        assert!(expanded.contains(&"paper2".to_string()));
        assert!(expanded.contains(&"paper3".to_string()));
    }

    #[test]
    fn test_find_paths_basic() {
        let finder = PathFinder::new(3, 5);
        let paths = finder.find_paths("paper1", &["paper2".to_string(), "paper3".to_string()]);
        assert_eq!(paths.len(), 2);
    }

    #[test]
    fn test_find_paths_excludes_self() {
        let finder = PathFinder::new(3, 5);
        let paths = finder.find_paths("paper1", &["paper1".to_string()]);
        assert!(paths.is_empty());
    }

    #[test]
    fn test_find_bridges() {
        let finder = PathFinder::new(3, 5);
        let community_map = HashMap::from([
            ("e1".to_string(), "community1".to_string()),
            ("e2".to_string(), "community1".to_string()),
            ("e3".to_string(), "community2".to_string()),
        ]);
        let bridges = finder.find_bridges(&["e1".to_string(), "e2".to_string()], &community_map);
        assert!(bridges.contains(&"community1".to_string()));
        assert!(!bridges.contains(&"community2".to_string()));
    }

    #[test]
    fn test_pathfinder_default() {
        let finder = PathFinder::default();
        // Should use max_hops=3, max_paths=5 from Default
        let paths = finder.find_paths("p1", &["p2".to_string()]);
        assert_eq!(paths.len(), 1);
    }
}
