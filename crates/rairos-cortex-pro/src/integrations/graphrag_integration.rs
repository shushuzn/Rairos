//! Enhanced GraphRAG integration with LightRAG-style dual-level retrieval.
//!
//! Based on research from:
//! - LightRAG (arXiv:2410.05779) - dual-level retrieval (local + global)
//! - Agentic GraphRAG (arXiv:2605.18770) - Neo4j KG construction
//! - XGRAG (arXiv:2604.20859) - causal graph perturbations
//!
//! ## Architecture
//!
//! ```text
//! Query Input
//!      │
//!      ▼
//! ┌─────────────────┐
//! │  Query Analysis  │ ← Parse query intent
//! └────────┬────────┘
//!          │
//!     ┌────┴────┐
//!     ▼         ▼
//! ┌───────┐ ┌────────┐
//! │Local  │ │Global  │ ← Dual-level retrieval
//! │Search │ │Search  │
//! └───┬───┘ └───┬────┘
//!     │         │
//!     └────┬────┘
//!          ▼
//! ┌─────────────────┐
//! │Result Fusion   │ ← Merge local + global
//! └────────┬───────┘
//!          │
//!          ▼
//!     Answer Gen
//! ```

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use chrono::{DateTime, Utc};

/// Query type for RAG system
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum QueryType {
    /// Entity-specific query (local retrieval)
    Entity,
    /// Community-level query (global retrieval)
    Community,
    /// Relationship query between entities
    Relationship,
    /// Comparison query
    Comparison,
    /// General query (hybrid)
    General,
}

/// A question about materials science.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RagQuestion {
    /// The question text
    pub question: String,
    /// Optional context or constraints
    pub context: Option<String>,
    /// Maximum number of sources to retrieve
    pub max_sources: usize,
    /// Identified query type
    pub query_type: Option<QueryType>,
    /// Key entities identified
    pub entities: Vec<String>,
}

impl Default for RagQuestion {
    fn default() -> Self {
        Self {
            question: String::new(),
            context: None,
            max_sources: 5,
            query_type: None,
            entities: Vec::new(),
        }
    }
}

impl RagQuestion {
    /// Analyze query to determine type and entities
    pub fn analyze(&mut self) {
        let q = self.question.to_lowercase();

        // Detect query type
        self.query_type = Some(
            if q.contains("compare") || q.contains("vs") || q.contains("versus") {
                QueryType::Comparison
            } else if q.contains("relationship") || q.contains("how does") || q.contains("interact") {
                QueryType::Relationship
            } else if q.contains("what is") || q.contains("who is") || q.contains("describe") {
                // Check if it's about a specific entity or general
                if self.entities.len() <= 2 {
                    QueryType::Entity
                } else {
                    QueryType::General
                }
            } else if q.contains("all") || q.contains("list") || q.contains("summarize") {
                QueryType::Community
            } else {
                QueryType::General
            }
        );
    }

    /// Add an entity to the query
    pub fn add_entity(&mut self, entity: &str) {
        if !self.entities.contains(&entity.to_string()) {
            self.entities.push(entity.to_string());
        }
    }
}

/// An answer from the RAG system.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RagAnswer {
    /// The generated answer
    pub answer: String,
    /// Source documents used
    pub sources: Vec<RagSource>,
    /// Confidence score
    pub confidence: f32,
    /// Retrieved entities
    pub entities: Vec<String>,
    /// Community context (for global queries)
    pub community_context: Option<String>,
    /// Whether hybrid retrieval was used
    pub used_hybrid: bool,
}

/// A source document used in RAG.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RagSource {
    /// Document ID
    pub doc_id: String,
    /// Document title or description
    pub title: String,
    /// Relevant excerpt
    pub excerpt: String,
    /// Relevance score
    pub score: f32,
    /// Source type
    pub source_type: SourceType,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SourceType {
    /// Vector similarity search result
    Vector,
    /// Knowledge graph result
    KnowledgeGraph,
    /// Hybrid (both)
    Hybrid,
}

/// Knowledge graph entity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KgEntity {
    /// Entity ID
    pub id: String,
    /// Entity name
    pub name: String,
    /// Entity type
    pub entity_type: String,
    /// Properties
    pub properties: HashMap<String, serde_json::Value>,
    /// Importance score
    pub importance: f32,
}

/// Knowledge graph relationship
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KgRelation {
    /// Relation ID
    pub id: String,
    /// Source entity
    pub source: String,
    /// Target entity
    pub target: String,
    /// Relation type
    pub relation_type: String,
    /// Properties
    pub properties: HashMap<String, serde_json::Value>,
    /// Weight/confidence
    pub weight: f32,
}

/// A knowledge graph for RAG
#[derive(Debug, Clone, Default)]
pub struct KnowledgeGraph {
    /// Entities by ID
    pub entities: HashMap<String, KgEntity>,
    /// Relations by ID
    pub relations: HashMap<String, KgRelation>,
    /// Entity name index
    pub name_index: HashMap<String, String>,
}

impl KnowledgeGraph {
    /// Create a new empty knowledge graph
    pub fn new() -> Self {
        Self::default()
    }

    /// Add an entity
    pub fn add_entity(&mut self, entity: KgEntity) {
        self.name_index.insert(entity.name.clone(), entity.id.clone());
        self.entities.insert(entity.id.clone(), entity);
    }

    /// Add a relation
    pub fn add_relation(&mut self, relation: KgRelation) {
        self.relations.insert(relation.id.clone(), relation);
    }

    /// Find entity by name
    pub fn find_entity(&self, name: &str) -> Option<&KgEntity> {
        self.name_index.get(name).and_then(|id| self.entities.get(id))
    }

    /// Get entities by type
    pub fn get_entities_by_type(&self, entity_type: &str) -> Vec<&KgEntity> {
        self.entities
            .values()
            .filter(|e| e.entity_type == entity_type)
            .collect()
    }

    /// Get relations for an entity
    pub fn get_relations_for(&self, entity_id: &str) -> Vec<&KgRelation> {
        self.relations
            .values()
            .filter(|r| r.source == entity_id || r.target == entity_id)
            .collect()
    }

    /// Get community (connected entities)
    pub fn get_community(&self, entity_id: &str, depth: usize) -> HashSet<String> {
        let mut community = HashSet::new();
        let mut to_visit = vec![entity_id];
        let mut current_depth = 0;

        while current_depth < depth && !to_visit.is_empty() {
            let next_to_visit = to_visit.clone();
            to_visit.clear();

            for eid in next_to_visit {
                if community.contains(eid) {
                    continue;
                }
                community.insert(eid.to_string());

                // Find connected entities
                for rel in self.get_relations_for(eid) {
                    if !community.contains(&rel.source) {
                        to_visit.push(&rel.source);
                    }
                    if !community.contains(&rel.target) {
                        to_visit.push(&rel.target);
                    }
                }
            }
            current_depth += 1;
        }

        community
    }
}

/// RAG service with LightRAG-style dual-level retrieval
#[derive(Debug, Clone)]
pub struct EnhancedRagService {
    /// Whether service is enabled
    enabled: bool,
    /// Knowledge graph
    kg: Option<KnowledgeGraph>,
    /// Embedding model name
    embedding_model: String,
    /// Vector dimension
    vector_dim: usize,
}

impl EnhancedRagService {
    /// Create a new enhanced RAG service
    pub fn new() -> Self {
        Self {
            enabled: true,
            kg: None,
            embedding_model: "sentence-transformers/all-MiniLM-L6-v2".to_string(),
            vector_dim: 384,
        }
    }

    /// Create with a knowledge graph
    pub fn with_knowledge_graph(mut self, kg: KnowledgeGraph) -> Self {
        self.kg = Some(kg);
        self
    }

    /// Check if this service is enabled
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// Analyze query and determine retrieval strategy
    fn analyze_query(&self, question: &RagQuestion) -> QueryType {
        let mut q = question.clone();
        q.analyze();
        q.query_type.unwrap_or(QueryType::General)
    }

    /// Local retrieval - find specific entity information
    async fn local_retrieval(&self, entity: &str) -> Vec<RagSource> {
        // In full implementation, would query vector store
        // and knowledge graph for specific entity
        vec![
            RagSource {
                doc_id: format!("local-{}", entity),
                title: format!("Information about {}", entity),
                excerpt: format!("Detailed information about {} from knowledge graph...", entity),
                score: 0.95,
                source_type: SourceType::Hybrid,
            },
        ]
    }

    /// Global retrieval - community-level information
    async fn global_retrieval(&self, query: &str) -> Vec<RagSource> {
        // In full implementation, would use community detection
        // and graph traversal for broader context
        vec![
            RagSource {
                doc_id: "global-context".to_string(),
                title: "Community context".to_string(),
                excerpt: format!("Global context for query: {}...", query),
                score: 0.85,
                source_type: SourceType::KnowledgeGraph,
            },
        ]
    }

    /// Hybrid retrieval combining local and global
    pub async fn query(&self, question: &RagQuestion) -> Result<RagAnswer, RagServiceError> {
        if !self.enabled {
            return Err(RagServiceError::Disabled);
        }

        // Analyze query type
        let query_type = self.analyze_query(question);

        // Perform dual-level retrieval based on query type
        let (sources, community_context, used_hybrid) = match query_type {
            QueryType::Entity => {
                // Entity-specific query - primarily local
                let mut sources = Vec::new();
                for entity in &question.entities {
                    sources.extend(self.local_retrieval(entity).await);
                }
                (sources, None, false)
            }
            QueryType::Community => {
                // Community query - primarily global
                let sources = self.global_retrieval(&question.question).await;
                (sources, Some("Community context retrieved".to_string()), false)
            }
            QueryType::Relationship => {
                // Relationship query - both local and global
                let mut sources = Vec::new();
                for entity in &question.entities {
                    sources.extend(self.local_retrieval(entity).await);
                }
                sources.extend(self.global_retrieval(&question.question).await);
                (sources, Some("Relationship context from knowledge graph".to_string()), true)
            }
            QueryType::Comparison | QueryType::General => {
                // General query - use hybrid
                let mut sources = Vec::new();
                for entity in &question.entities {
                    sources.extend(self.local_retrieval(entity).await);
                }
                sources.extend(self.global_retrieval(&question.question).await);
                (sources, Some("Combined context for comparison".to_string()), true)
            }
        };

        // Sort by score
        let mut sources = sources;
        sources.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap());
        let sources: Vec<_> = sources.into_iter().take(question.max_sources).collect();

        // Extract entities
        let entities: Vec<_> = sources
            .iter()
            .filter(|s| s.source_type != SourceType::Vector)
            .map(|s| s.title.clone())
            .collect();

        Ok(RagAnswer {
            answer: format!("Answer based on dual-level retrieval (type: {:?})...", query_type),
            sources,
            confidence: 0.85,
            entities,
            community_context,
            used_hybrid,
        })
    }

    /// Query with hybrid retrieval (vector + knowledge graph)
    pub async fn query_hybrid(
        &self,
        question: &RagQuestion,
    ) -> Result<RagAnswer, RagServiceError> {
        self.query(question).await
    }

    /// Add entity to knowledge graph
    pub async fn add_entity(&mut self, entity: KgEntity) {
        if let Some(ref mut kg) = self.kg {
            kg.add_entity(entity);
        } else {
            let mut kg = KnowledgeGraph::new();
            kg.add_entity(entity);
            self.kg = Some(kg);
        }
    }

    /// Add relation to knowledge graph
    pub async fn add_relation(&mut self, relation: KgRelation) {
        if let Some(ref mut kg) = self.kg {
            kg.add_relation(relation);
        }
    }

    /// Index a document for RAG retrieval
    pub async fn index_document(
        &self,
        _doc_id: &str,
        _content: &str,
        _metadata: serde_json::Value,
    ) -> Result<(), RagServiceError> {
        if !self.enabled {
            return Err(RagServiceError::Disabled);
        }
        // In full implementation, would:
        // 1. Extract entities and relations from content
        // 2. Generate embeddings
        // 3. Store in vector database
        // 4. Store in knowledge graph
        Ok(())
    }

    /// Get knowledge graph statistics
    pub fn kg_stats(&self) -> Option<KgStats> {
        self.kg.as_ref().map(|kg| KgStats {
            entity_count: kg.entities.len(),
            relation_count: kg.relations.len(),
        })
    }
}

impl Default for EnhancedRagService {
    fn default() -> Self {
        Self::new()
    }
}

/// Knowledge graph statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KgStats {
    pub entity_count: usize,
    pub relation_count: usize,
}

/// Error type for RAG operations.
#[derive(Debug, thiserror::Error)]
pub enum RagServiceError {
    #[error("RAG service is disabled")]
    Disabled,
    #[error("RAG error: {0}")]
    RagError(String),
    #[error("Knowledge graph not initialized")]
    KgNotInitialized,
}

/// Legacy RAG service for backward compatibility
pub type RagService = EnhancedRagService;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_query_type_analysis() {
        let mut q = RagQuestion {
            question: "What is the ZT of Bi2Te3?".to_string(),
            context: None,
            max_sources: 5,
            query_type: None,
            entities: vec!["Bi2Te3".to_string()],
        };
        q.analyze();
        assert_eq!(q.query_type, Some(QueryType::Entity));
    }

    #[test]
    fn test_comparison_query() {
        let mut q = RagQuestion {
            question: "Compare Bi2Te3 vs PbTe thermoelectric properties".to_string(),
            context: None,
            max_sources: 5,
            query_type: None,
            entities: vec!["Bi2Te3".to_string(), "PbTe".to_string()],
        };
        q.analyze();
        assert_eq!(q.query_type, Some(QueryType::Comparison));
    }

    #[test]
    fn test_knowledge_graph() {
        let mut kg = KnowledgeGraph::new();

        kg.add_entity(KgEntity {
            id: "e1".to_string(),
            name: "Bi2Te3".to_string(),
            entity_type: "Material".to_string(),
            properties: HashMap::new(),
            importance: 0.9,
        });

        kg.add_relation(KgRelation {
            id: "r1".to_string(),
            source: "e1".to_string(),
            target: "e1".to_string(),
            relation_type: "similar_to".to_string(),
            properties: HashMap::new(),
            weight: 0.8,
        });

        assert_eq!(kg.entities.len(), 1);
        assert_eq!(kg.relations.len(), 1);

        let found = kg.find_entity("Bi2Te3");
        assert!(found.is_some());
        assert_eq!(found.unwrap().entity_type, "Material");
    }

    #[test]
    fn test_community_retrieval() {
        let mut kg = KnowledgeGraph::new();

        // Create a small graph
        kg.add_entity(KgEntity {
            id: "e1".to_string(),
            name: "Material1".to_string(),
            entity_type: "Material".to_string(),
            properties: HashMap::new(),
            importance: 0.9,
        });
        kg.add_entity(KgEntity {
            id: "e2".to_string(),
            name: "Material2".to_string(),
            entity_type: "Material".to_string(),
            properties: HashMap::new(),
            importance: 0.8,
        });

        kg.add_relation(KgRelation {
            id: "r1".to_string(),
            source: "e1".to_string(),
            target: "e2".to_string(),
            relation_type: "similar".to_string(),
            properties: HashMap::new(),
            weight: 0.7,
        });

        let community = kg.get_community("e1", 1);
        assert!(community.contains("e1"));
        assert!(community.contains("e2"));
    }

    #[tokio::test]
    async fn test_enhanced_rag_query() {
        let service = EnhancedRagService::new();

        let question = RagQuestion {
            question: "What is the thermoelectric property of Bi2Te3?".to_string(),
            context: None,
            max_sources: 3,
            query_type: None,
            entities: vec!["Bi2Te3".to_string()],
        };

        let result = service.query(&question).await;
        assert!(result.is_ok());

        let answer = result.unwrap();
        assert!(!answer.sources.is_empty());
    }

    #[tokio::test]
    async fn test_hybrid_query() {
        let mut service = EnhancedRagService::new();

        // Add a knowledge graph
        let mut kg = KnowledgeGraph::new();
        kg.add_entity(KgEntity {
            id: "mat1".to_string(),
            name: "Bi2Te3".to_string(),
            entity_type: "Thermoelectric".to_string(),
            properties: HashMap::new(),
            importance: 0.95,
        });
        service = service.with_knowledge_graph(kg);

        let question = RagQuestion {
            question: "Compare thermoelectric materials".to_string(),
            context: None,
            max_sources: 5,
            query_type: None,
            entities: vec!["Bi2Te3".to_string()],
        };

        let result = service.query(&question).await.unwrap();
        assert!(result.used_hybrid);
        assert!(result.community_context.is_some());
    }

    #[test]
    fn test_kg_stats() {
        let service = EnhancedRagService::new();
        assert!(service.kg_stats().is_none());

        let service = EnhancedRagService::new().with_knowledge_graph(KnowledgeGraph::new());
        let stats = service.kg_stats().unwrap();
        assert_eq!(stats.entity_count, 0);
    }
}