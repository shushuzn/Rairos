//! Community detection and summarization for knowledge graph entities.

use std::collections::HashMap;

/// A detected community of related entities
#[derive(Debug, Clone)]
pub struct Community {
    /// Community identifier
    pub id: String,
    /// Entities in this community
    pub entities: Vec<CommunityEntity>,
    /// Community size
    pub size: usize,
    /// Representative tag/topics
    pub topics: Vec<String>,
}

/// An entity within a community
#[derive(Debug, Clone)]
pub struct CommunityEntity {
    pub entity_id: String,
    pub label: String,
    pub node_type: String,
    pub importance: f32,
}

/// Community summarization result
#[derive(Debug, Clone)]
pub struct CommunitySummary {
    pub community_id: String,
    /// Auto-generated summary of this community
    pub summary: String,
    /// Key topics/keywords
    pub keywords: Vec<String>,
    /// Representative entity IDs
    pub representatives: Vec<String>,
    /// Coverage score (how well this community covers the query)
    pub coverage_score: f32,
}

/// Community summarizer that generates summaries for detected communities
pub struct CommunitySummarizer {
    /// Maximum summary length in tokens
    #[allow(dead_code)]
    max_summary_len: usize,
}

impl CommunitySummarizer {
    pub fn new() -> Self {
        Self {
            max_summary_len: 512,
        }
    }

    /// Generate a summary for a community based on its entities
    ///
    /// In a full implementation, this would use an LLM to generate
    /// a coherent summary. Here we provide a template-based approach.
    pub fn summarize(&self, community: &Community) -> CommunitySummary {
        let keywords = self.extract_keywords(community);
        let representatives = self.find_representatives(community);

        // Generate a template summary
        let summary = self.generate_template_summary(community, &keywords);

        CommunitySummary {
            community_id: community.id.clone(),
            summary,
            keywords,
            representatives,
            coverage_score: 0.5, // Default coverage
        }
    }

    /// Extract keywords from community entities
    fn extract_keywords(&self, community: &Community) -> Vec<String> {
        let mut topic_counts: HashMap<&str, usize> = HashMap::new();

        for topic in &community.topics {
            *topic_counts.entry(topic.as_str()).or_insert(0) += 1;
        }

        // Return top 5 topics by frequency
        let mut topics: Vec<_> = topic_counts.into_iter().collect();
        topics.sort_by(|a, b| b.1.cmp(&a.1));
        topics.into_iter().take(5).map(|(t, _)| t.to_string()).collect()
    }

    /// Find representative entities (most important in community)
    fn find_representatives(&self, community: &Community) -> Vec<String> {
        let mut entities = community.entities.clone();
        entities.sort_by(|a, b| b.importance.partial_cmp(&a.importance).unwrap_or(std::cmp::Ordering::Equal));

        entities
            .into_iter()
            .take(3)
            .map(|e| e.entity_id)
            .collect()
    }

    /// Generate a template-based summary
    fn generate_template_summary(&self, community: &Community, keywords: &[String]) -> String {
        let topic_str = if keywords.is_empty() {
            "various topics".to_string()
        } else {
            keywords.join(", ")
        };

        format!(
            "This community contains {} papers covering {}. The research focuses on {} with {} key contributions.",
            community.size,
            topic_str,
            topic_str,
            community.size
        )
    }

    /// Score how well a community covers a query
    pub fn score_coverage(&self, community: &Community, query_keywords: &[String]) -> f32 {
        if query_keywords.is_empty() {
            return 0.5;
        }

        let community_keywords: std::collections::HashSet<_> =
            community.topics.iter().map(|s| s.to_lowercase()).collect();

        let query_keywords_lower: std::collections::HashSet<_> =
            query_keywords.iter().map(|s| s.to_lowercase()).collect();

        let intersection = community_keywords.intersection(&query_keywords_lower).count();
        let union = community_keywords.union(&query_keywords_lower).count();

        if union == 0 {
            0.5
        } else {
            intersection as f32 / union as f32
        }
    }
}

impl Default for CommunitySummarizer {
    fn default() -> Self {
        Self::new()
    }
}

/// Group entities into communities based on shared attributes
pub fn group_by_topics(entities: &[CommunityEntity]) -> Vec<Community> {
    let mut topic_groups: HashMap<String, Vec<CommunityEntity>> = HashMap::new();

    for entity in entities {
        // Group by node_type as a simple heuristic
        let key = entity.node_type.clone();
        topic_groups.entry(key).or_default().push(entity.clone());
    }

    topic_groups
        .into_iter()
        .enumerate()
        .map(|(i, (topic, entities))| {
            let id = format!("community_{}", i);
            let size = entities.len();
            let importance_sum: f32 = entities.iter().map(|e| e.importance).sum();
            let entities: Vec<CommunityEntity> = if importance_sum > 0.0 {
                entities
                    .into_iter()
                    .map(|mut e| {
                        e.importance /= importance_sum;
                        e
                    })
                    .collect()
            } else {
                entities
            };
            Community {
                id,
                entities,
                size,
                topics: vec![topic],
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_score_coverage_empty_keywords() {
        let summarizer = CommunitySummarizer::new();
        let community = Community {
            id: "c1".to_string(),
            entities: vec![],
            size: 0,
            topics: vec!["machine learning".to_string()],
        };
        let score = summarizer.score_coverage(&community, &[]);
        assert!((score - 0.5).abs() < 0.001);
    }

    #[test]
    fn test_score_coverage_full_match() {
        let summarizer = CommunitySummarizer::new();
        let community = Community {
            id: "c1".to_string(),
            entities: vec![],
            size: 5,
            topics: vec!["machine learning".to_string(), "deep learning".to_string()],
        };
        // Query matches exactly
        let score = summarizer.score_coverage(&community, &["machine learning".to_string(), "deep learning".to_string()]);
        assert!((score - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_score_coverage_partial_match() {
        let summarizer = CommunitySummarizer::new();
        let community = Community {
            id: "c1".to_string(),
            entities: vec![],
            size: 3,
            topics: vec!["machine learning".to_string(), "deep learning".to_string()],
        };
        // Query has 1 out of 2 matching
        let score = summarizer.score_coverage(&community, &["machine learning".to_string(), "physics".to_string()]);
        // intersection = 1, union = 3
        assert!((score - 1.0 / 3.0).abs() < 0.001);
    }

    #[test]
    fn test_score_coverage_no_match() {
        let summarizer = CommunitySummarizer::new();
        let community = Community {
            id: "c1".to_string(),
            entities: vec![],
            size: 2,
            topics: vec!["machine learning".to_string()],
        };
        let score = summarizer.score_coverage(&community, &["physics".to_string(), "chemistry".to_string()]);
        // intersection = 0, union = 3
        assert!((score - 0.0).abs() < 0.001);
    }

    #[test]
    fn test_score_coverage_case_insensitive() {
        let summarizer = CommunitySummarizer::new();
        let community = Community {
            id: "c1".to_string(),
            entities: vec![],
            size: 1,
            topics: vec!["Machine Learning".to_string()],
        };
        let score = summarizer.score_coverage(&community, &["machine learning".to_string()]);
        // Should match case-insensitively
        assert!((score - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_group_by_topics_empty() {
        let groups = group_by_topics(&[]);
        assert!(groups.is_empty());
    }

    #[test]
    fn test_group_by_topics_single_type() {
        let entities = vec![
            CommunityEntity {
                entity_id: "e1".to_string(),
                label: "Paper 1".to_string(),
                node_type: "Paper".to_string(),
                importance: 0.5,
            },
            CommunityEntity {
                entity_id: "e2".to_string(),
                label: "Paper 2".to_string(),
                node_type: "Paper".to_string(),
                importance: 0.5,
            },
        ];
        let groups = group_by_topics(&entities);
        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].size, 2);
        assert_eq!(groups[0].topics, vec!["Paper"]);
    }

    #[test]
    fn test_group_by_topics_multiple_types() {
        let entities = vec![
            CommunityEntity {
                entity_id: "e1".to_string(),
                label: "Paper 1".to_string(),
                node_type: "Paper".to_string(),
                importance: 0.4,
            },
            CommunityEntity {
                entity_id: "e2".to_string(),
                label: "Author 1".to_string(),
                node_type: "Author".to_string(),
                importance: 0.6,
            },
        ];
        let groups = group_by_topics(&entities);
        assert_eq!(groups.len(), 2);
    }

    #[test]
    fn test_group_by_topics_importance_normalized() {
        let entities = vec![
            CommunityEntity {
                entity_id: "e1".to_string(),
                label: "Paper 1".to_string(),
                node_type: "Paper".to_string(),
                importance: 0.2,
            },
            CommunityEntity {
                entity_id: "e2".to_string(),
                label: "Paper 2".to_string(),
                node_type: "Paper".to_string(),
                importance: 0.3,
            },
        ];
        let groups = group_by_topics(&entities);
        assert_eq!(groups.len(), 1);
        // Importance should be normalized (sum to 1.0)
        let total_importance: f32 = groups[0].entities.iter().map(|e| e.importance).sum();
        assert!((total_importance - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_community_summarizer_default() {
        let summarizer = CommunitySummarizer::default();
        // Default is created via ::new()
        let community = Community {
            id: "c1".to_string(),
            entities: vec![
                CommunityEntity {
                    entity_id: "e1".to_string(),
                    label: "Test".to_string(),
                    node_type: "Paper".to_string(),
                    importance: 1.0,
                },
            ],
            size: 1,
            topics: vec!["testing".to_string()],
        };
        let summary = summarizer.summarize(&community);
        assert_eq!(summary.community_id, "c1");
        assert!(!summary.summary.is_empty());
    }
}
