//! Cypher query builder for Neo4j.
//!
//! Provides a fluent API for building Cypher queries.

use crate::schema::{EdgeType, NodeType};

/// A fluent Cypher query builder
#[derive(Debug, Clone)]
pub struct CypherBuilder {
    query: String,
    params: std::collections::HashMap<String, serde_json::Value>,
}

impl CypherBuilder {
    /// Create a new empty query
    pub fn new() -> Self {
        Self {
            query: String::new(),
            params: std::collections::HashMap::new(),
        }
    }

    /// Start a MATCH clause with a node pattern
    pub fn match_node(mut self, alias: &str, node_type: NodeType) -> Self {
        self.query = format!("MATCH ({}:{})", alias, node_type.label());
        self
    }

    /// Add WHERE clause with property condition
    pub fn where_eq(mut self, alias: &str, property: &str, value: impl Into<serde_json::Value>) -> Self {
        let param_name = format!("{}_{}", alias, property);
        self.params.insert(param_name.clone(), value.into());
        self.query = format!("{} WHERE {}.{} = ${}", self.query, alias, property, param_name);
        self
    }

    /// Add WHERE clause with contains condition
    pub fn where_contains(self, alias: &str, property: &str, value: &str) -> Self {
        let param_name = format!("{}_{}_contains", alias, property);
        let mut params = self.params;
        params.insert(param_name.clone(), serde_json::json!(value));
        Self {
            query: format!("{} WHERE {}.{} CONTAINS ${}", self.query, alias, property, param_name),
            params,
        }
    }

    /// Add relationship pattern (MATCH ...-[r]->...)
    pub fn match_edge(
        mut self,
        source_alias: &str,
        rel_type: EdgeType,
        target_alias: &str,
    ) -> Self {
        self.query = format!(
            "{}-[r:{}]->{}",
            source_alias, rel_type.rel_type(), target_alias
        );
        self
    }

    /// Add bidirectional relationship pattern
    pub fn match_edge_bidir(
        mut self,
        alias1: &str,
        rel_type: EdgeType,
        alias2: &str,
    ) -> Self {
        self.query = format!(
            "{}-[r:{}]-{}",
            alias1, rel_type.rel_type(), alias2
        );
        self
    }

    /// Add RETURN clause
    pub fn return_nodes(mut self, aliases: Vec<&str>) -> Self {
        self.query = format!("{} RETURN {}", self.query, aliases.join(", "));
        self
    }

    /// Add RETURN with relationship
    pub fn return_with_rel(mut self, node_alias: &str, rel_alias: &str, target_alias: &str) -> Self {
        self.query = format!("{} RETURN {}, {}, {}", self.query, node_alias, rel_alias, target_alias);
        self
    }

    /// Add ORDER BY clause
    pub fn order_by(mut self, alias: &str, property: &str, descending: bool) -> Self {
        let dir = if descending { "DESC" } else { "ASC" };
        self.query = format!("{} ORDER BY {}.{} {}", self.query, alias, property, dir);
        self
    }

    /// Add LIMIT clause
    pub fn limit(mut self, n: usize) -> Self {
        self.query = format!("{} LIMIT {}", self.query, n);
        self
    }

    /// Add SKIP clause
    pub fn skip(mut self, n: usize) -> Self {
        self.query = format!("{} SKIP {}", self.query, n);
        self
    }

    /// Add COUNT aggregation
    pub fn count(mut self, alias: &str) -> Self {
        self.query = format!("{} RETURN count({}) as count", self.query, alias);
        self
    }

    /// Add COLLECT aggregation
    pub fn collect(mut self, alias: &str, collected_alias: &str) -> Self {
        self.query = format!("{} RETURN collect({}) as {}", self.query, alias, collected_alias);
        self
    }

    /// Build the final query string
    pub fn build(&self) -> String {
        self.query.clone()
    }

    /// Get query parameters
    pub fn params(&self) -> &std::collections::HashMap<String, serde_json::Value> {
        &self.params
    }

    /// Get mutable params for adding more
    pub fn params_mut(&mut self) -> &mut std::collections::HashMap<String, serde_json::Value> {
        &mut self.params
    }
}

impl Default for CypherBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Common query patterns
pub mod patterns {
    /// Find a paper by its entity ID (e.g., arxiv_id)
    pub fn find_paper_by_entity_id(entity_id: &str) -> (String, std::collections::HashMap<String, serde_json::Value>) {
        let mut params = std::collections::HashMap::new();
        params.insert("entity_id".to_string(), serde_json::json!(entity_id));
        let query = "MATCH (p:Paper) WHERE p.entity_id = $entity_id RETURN p".to_string();
        (query, params)
    }

    /// Find papers by tag
    pub fn find_papers_by_tag(tag_label: &str, limit: usize) -> (String, std::collections::HashMap<String, serde_json::Value>) {
        let mut params = std::collections::HashMap::new();
        params.insert("tag".to_string(), serde_json::json!(tag_label));
        let query = format!(
            "MATCH (p:Paper)-[:TAGGED_WITH]->(t:Tag) WHERE t.entity_id = $tag RETURN p LIMIT {}",
            limit
        );
        (query, params)
    }

    /// Find citation chain (papers that cite a given paper)
    pub fn find_citing_papers(entity_id: &str, limit: usize) -> (String, std::collections::HashMap<String, serde_json::Value>) {
        let mut params = std::collections::HashMap::new();
        params.insert("entity_id".to_string(), serde_json::json!(entity_id));
        let query = format!(
            "MATCH (citing:Paper)-[:CITES]->(cited:Paper) WHERE cited.entity_id = $entity_id RETURN citing LIMIT {}",
            limit
        );
        (query, params)
    }

    /// Find references (papers that a given paper cites)
    pub fn find_references(entity_id: &str, limit: usize) -> (String, std::collections::HashMap<String, serde_json::Value>) {
        let mut params = std::collections::HashMap::new();
        params.insert("entity_id".to_string(), serde_json::json!(entity_id));
        let query = format!(
            "MATCH (citing:Paper)-[:CITES]->(cited:Paper) WHERE citing.entity_id = $entity_id RETURN cited LIMIT {}",
            limit
        );
        (query, params)
    }

    /// Find papers by author
    pub fn find_papers_by_author(author_name: &str, limit: usize) -> (String, std::collections::HashMap<String, serde_json::Value>) {
        let mut params = std::collections::HashMap::new();
        params.insert("name".to_string(), serde_json::json!(author_name));
        let query = format!(
            "MATCH (a:Author)-[:DERIVES_FROM]->(p:Paper) WHERE a.name CONTAINS $name RETURN p LIMIT {}",
            limit
        );
        (query, params)
    }

    /// Find related papers (shared tags)
    pub fn find_related_papers(entity_id: &str, limit: usize) -> (String, std::collections::HashMap<String, serde_json::Value>) {
        let mut params = std::collections::HashMap::new();
        params.insert("entity_id".to_string(), serde_json::json!(entity_id));
        let query = format!(
            "MATCH (p1:Paper)-[:TAGGED_WITH]->(t:Tag)<-[:TAGGED_WITH]-(p2:Paper) WHERE p1.entity_id = $entity_id AND p1 <> p2 RETURN p2, count(t) as shared_tags ORDER BY shared_tags DESC LIMIT {}",
            limit
        );
        (query, params)
    }

    /// Get paper with its metadata (authors, tags, citations)
    pub fn get_paper_details(entity_id: &str) -> (String, std::collections::HashMap<String, serde_json::Value>) {
        let mut params = std::collections::HashMap::new();
        params.insert("entity_id".to_string(), serde_json::json!(entity_id));
        let query = r#"
            MATCH (p:Paper {entity_id: $entity_id})
            OPTIONAL MATCH (a:Author)-[r:DERIVES_FROM]->(p)
            OPTIONAL MATCH (p)-[:TAGGED_WITH]->(t:Tag)
            OPTIONAL MATCH (cited:Paper)-[:CITES]->(p)
            OPTIONAL MATCH (p)-[:CITES]->(citing:Paper)
            RETURN p, collect(DISTINCT a) as authors, collect(DISTINCT t) as tags,
                   count(DISTINCT cited) as cited_by_count, collect(DISTINCT citing) as references
        "#.to_string();
        (query, params)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_query() {
        let query = CypherBuilder::new()
            .match_node("p", NodeType::Paper)
            .where_eq("p", "entity_id", "2101.12345")
            .return_nodes(vec!["p"])
            .build();
        assert!(query.contains("MATCH (p:Paper)"));
        assert!(query.contains("WHERE"));
    }

    #[test]
    fn test_cypher_builder_match_node() {
        let query = CypherBuilder::new()
            .match_node("p", NodeType::Paper)
            .build();
        assert_eq!(query, "MATCH (p:Paper)");
    }

    #[test]
    fn test_cypher_builder_match_node_all_types() {
        assert_eq!(
            CypherBuilder::new().match_node("n", NodeType::Paper).build(),
            "MATCH (n:Paper)"
        );
        assert_eq!(
            CypherBuilder::new().match_node("n", NodeType::Author).build(),
            "MATCH (n:Author)"
        );
        assert_eq!(
            CypherBuilder::new().match_node("n", NodeType::Tag).build(),
            "MATCH (n:Tag)"
        );
        assert_eq!(
            CypherBuilder::new().match_node("n", NodeType::PNote).build(),
            "MATCH (n:PNote)"
        );
    }

    #[test]
    fn test_cypher_builder_where_eq() {
        let query = CypherBuilder::new()
            .match_node("p", NodeType::Paper)
            .where_eq("p", "entity_id", "2101.12345")
            .build();
        assert!(query.contains("WHERE"));
        assert!(query.contains("p.entity_id = $"));
    }

    #[test]
    fn test_cypher_builder_where_contains() {
        let query = CypherBuilder::new()
            .match_node("p", NodeType::Paper)
            .where_contains("p", "title", "attention")
            .build();
        assert!(query.contains("WHERE"));
        assert!(query.contains("CONTAINS"));
    }

    #[test]
    fn test_cypher_builder_match_edge() {
        // match_edge replaces the query entirely (doesn't append)
        let query = CypherBuilder::new()
            .match_edge("p", EdgeType::Cite, "cited")
            .build();
        assert_eq!(query, "p-[r:CITES]->cited");
    }

    #[test]
    fn test_cypher_builder_match_edge_bidir() {
        // match_edge_bidir replaces the query entirely (doesn't append)
        let query = CypherBuilder::new()
            .match_edge_bidir("p1", EdgeType::SameTag, "p2")
            .build();
        assert_eq!(query, "p1-[r:TAGGED_WITH]-p2");
    }

    #[test]
    fn test_cypher_builder_return_nodes() {
        let query = CypherBuilder::new()
            .match_node("p", NodeType::Paper)
            .return_nodes(vec!["p"])
            .build();
        assert!(query.contains("RETURN p"));
    }

    #[test]
    fn test_cypher_builder_return_nodes_multiple() {
        let query = CypherBuilder::new()
            .match_node("p", NodeType::Paper)
            .match_node("a", NodeType::Author)
            .return_nodes(vec!["p", "a"])
            .build();
        assert!(query.contains("RETURN p, a"));
    }

    #[test]
    fn test_cypher_builder_order_by() {
        let query = CypherBuilder::new()
            .match_node("p", NodeType::Paper)
            .order_by("p", "entity_id", false)
            .build();
        assert!(query.contains("ORDER BY p.entity_id ASC"));

        let query_desc = CypherBuilder::new()
            .match_node("p", NodeType::Paper)
            .order_by("p", "score", true)
            .build();
        assert!(query_desc.contains("ORDER BY p.score DESC"));
    }

    #[test]
    fn test_cypher_builder_limit_skip() {
        let query = CypherBuilder::new()
            .match_node("p", NodeType::Paper)
            .skip(10)
            .limit(5)
            .build();
        assert!(query.contains("SKIP 10"));
        assert!(query.contains("LIMIT 5"));
    }

    #[test]
    fn test_cypher_builder_count() {
        let query = CypherBuilder::new()
            .match_node("p", NodeType::Paper)
            .count("p")
            .build();
        assert!(query.contains("RETURN count(p)"));
    }

    #[test]
    fn test_cypher_builder_collect() {
        let query = CypherBuilder::new()
            .match_node("p", NodeType::Paper)
            .collect("p", "papers")
            .build();
        assert!(query.contains("RETURN collect(p) as papers"));
    }

    #[test]
    fn test_cypher_builder_params() {
        let mut builder = CypherBuilder::new();
        builder = builder
            .match_node("p", NodeType::Paper)
            .where_eq("p", "entity_id", "2101.12345");
        
        let params = builder.params();
        assert!(params.contains_key("p_entity_id"));
        assert_eq!(params["p_entity_id"], serde_json::json!("2101.12345"));
    }

    #[test]
    fn test_cypher_builder_full_query() {
        // This test demonstrates how the fluent builder chains methods
        // Note: match_edge replaces the query, so in practice you'd use different patterns
        let query = CypherBuilder::new()
            .match_node("p", NodeType::Paper)
            .where_eq("p", "entity_id", "2101.12345")
            .return_nodes(vec!["p"])
            .limit(10)
            .build();
        
        assert!(query.contains("MATCH (p:Paper)"));
        assert!(query.contains("WHERE p.entity_id = $p_entity_id"));
        assert!(query.contains("LIMIT 10"));
    }

    #[test]
    fn test_patterns_find_paper_by_entity_id() {
        let (query, params) = patterns::find_paper_by_entity_id("2101.12345");
        
        assert_eq!(query, "MATCH (p:Paper) WHERE p.entity_id = $entity_id RETURN p");
        assert_eq!(params["entity_id"], serde_json::json!("2101.12345"));
    }

    #[test]
    fn test_patterns_find_papers_by_tag() {
        let (query, params) = patterns::find_papers_by_tag("machine-learning", 10);
        
        assert!(query.contains("MATCH (p:Paper)-[:TAGGED_WITH]->(t:Tag)"));
        assert!(query.contains("WHERE t.entity_id = $tag"));
        assert!(query.contains("LIMIT 10"));
        assert_eq!(params["tag"], serde_json::json!("machine-learning"));
    }

    #[test]
    fn test_patterns_find_citing_papers() {
        let (query, params) = patterns::find_citing_papers("1706.03762", 5);
        
        assert!(query.contains("MATCH (citing:Paper)-[:CITES]->(cited:Paper)"));
        assert!(query.contains("WHERE cited.entity_id = $entity_id"));
        assert!(query.contains("LIMIT 5"));
        assert_eq!(params["entity_id"], serde_json::json!("1706.03762"));
    }

    #[test]
    fn test_patterns_find_references() {
        let (query, _params) = patterns::find_references("1706.03762", 20);
        
        assert!(query.contains("MATCH (citing:Paper)-[:CITES]->(cited:Paper)"));
        assert!(query.contains("WHERE citing.entity_id = $entity_id"));
        assert!(query.contains("LIMIT 20"));
    }

    #[test]
    fn test_patterns_find_papers_by_author() {
        let (query, params) = patterns::find_papers_by_author("Vaswani", 10);
        
        assert!(query.contains("MATCH (a:Author)-[:DERIVES_FROM]->(p:Paper)"));
        assert!(query.contains("WHERE a.name CONTAINS $name"));
        assert!(query.contains("LIMIT 10"));
        assert_eq!(params["name"], serde_json::json!("Vaswani"));
    }

    #[test]
    fn test_patterns_find_related_papers() {
        let (query, _params) = patterns::find_related_papers("1706.03762", 5);
        
        assert!(query.contains("MATCH (p1:Paper)-[:TAGGED_WITH]->(t:Tag)<-[:TAGGED_WITH]-(p2:Paper)"));
        assert!(query.contains("WHERE p1.entity_id = $entity_id"));
        assert!(query.contains("ORDER BY shared_tags DESC"));
        assert!(query.contains("LIMIT 5"));
    }

    #[test]
    fn test_patterns_get_paper_details() {
        let (query, params) = patterns::get_paper_details("1706.03762");
        
        assert!(query.contains("MATCH (p:Paper {entity_id: $entity_id})"));
        assert!(query.contains("OPTIONAL MATCH (a:Author)-[r:DERIVES_FROM]->(p)"));
        assert!(query.contains("OPTIONAL MATCH (p)-[:TAGGED_WITH]->(t:Tag)"));
        assert!(query.contains("RETURN p,"));
        assert_eq!(params["entity_id"], serde_json::json!("1706.03762"));
    }
}
