//! Neo4j client for knowledge graph operations.
//!
//! Uses the Neo4j HTTP API for database operations.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::error::KgError;
use crate::schema::{EdgeType, KgEdge, KgNode, NodeType};
use crate::cypher::patterns;

/// Neo4j database configuration
#[derive(Debug, Clone)]
pub struct Neo4jConfig {
    pub uri: String,
    pub database: String,
    pub username: String,
    pub password: String,
}

impl Default for Neo4jConfig {
    fn default() -> Self {
        Self {
            uri: "http://localhost:7474".to_string(),
            database: "neo4j".to_string(),
            username: "neo4j".to_string(),
            password: "password".to_string(),
        }
    }
}

impl Neo4jConfig {
    pub fn new(uri: &str, database: &str, username: &str, password: &str) -> Self {
        Self {
            uri: uri.to_string(),
            database: database.to_string(),
            username: username.to_string(),
            password: password.to_string(),
        }
    }
}

/// Neo4j HTTP API client
#[derive(Clone)]
pub struct Neo4jKgClient {
    config: Neo4jConfig,
    http_client: reqwest::Client,
}

impl Neo4jKgClient {
    /// Create a new Neo4j client
    pub fn new(config: Neo4jConfig) -> Self {
        Self {
            config,
            http_client: reqwest::Client::new(),
        }
    }

    /// Execute a Cypher query and return results
    pub async fn execute_cypher(
        &self,
        query: &str,
        params: HashMap<String, serde_json::Value>,
    ) -> Result<Vec<serde_json::Value>, KgError> {
        let url = format!("{}/db/{}/tx/commit", self.config.uri, self.config.database);

        #[derive(Serialize)]
        struct TxRequest {
            statements: Vec<Statement>,
        }

        #[derive(Serialize)]
        struct Statement {
            statement: String,
            parameters: HashMap<String, serde_json::Value>,
        }

        #[derive(Deserialize)]
        struct TxResponse {
            results: Vec<QueryResult>,
            errors: Vec<Neo4jError>,
        }

        #[derive(Deserialize)]
        struct QueryResult {
            columns: Vec<String>,
            data: Vec<RowData>,
        }

        #[derive(Deserialize)]
        struct RowData {
            row: Vec<serde_json::Value>,
        }

        #[derive(Deserialize)]
        struct Neo4jError {
            code: String,
            message: String,
        }

        let request = TxRequest {
            statements: vec![Statement {
                statement: query.to_string(),
                parameters: params,
            }],
        };

        let response = self
            .http_client
            .post(&url)
            .header("Content-Type", "application/json")
            .basic_auth(&self.config.username, Some(&self.config.password))
            .json(&request)
            .send()
            .await
            .map_err(|e| KgError::ConnectionError(e.to_string()))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(KgError::QueryError(format!(
                "HTTP {}: {}",
                status, body
            )));
        }

        let tx_resp: TxResponse = response
            .json()
            .await
            .map_err(|e| KgError::QueryError(e.to_string()))?;

        if !tx_resp.errors.is_empty() {
            let err = &tx_resp.errors[0];
            return Err(KgError::QueryError(format!(
                "{}: {}",
                err.code, err.message
            )));
        }

        if tx_resp.results.is_empty() {
            return Ok(vec![]);
        }

        let result = &tx_resp.results[0];
        let columns = &result.columns;

        let rows: Vec<serde_json::Value> = result
            .data
            .iter()
            .map(|row| {
                let obj: serde_json::Map<String, serde_json::Value> = columns
                    .iter()
                    .zip(row.row.iter())
                    .map(|(col, val)| (col.clone(), val.clone()))
                    .collect();
                serde_json::Value::Object(obj)
            })
            .collect();

        Ok(rows)
    }

    /// Execute a write query (CREATE, MERGE, etc.) and return stats
    pub async fn execute_write(
        &self,
        query: &str,
        params: HashMap<String, serde_json::Value>,
    ) -> Result<WriteStats, KgError> {
        #[derive(Deserialize)]
        struct TxResponse {
            results: Vec<QueryResult>,
            errors: Vec<Neo4jError>,
        }

        #[derive(Deserialize)]
        struct QueryResult {
            stats: Option<UpdateStats>,
        }

        #[derive(Deserialize)]
        struct UpdateStats {
            #[serde(rename = "nodesCreated")]
            nodes_created: Option<i32>,
            #[serde(rename = "nodesDeleted")]
            nodes_deleted: Option<i32>,
            #[serde(rename = "relationshipsCreated")]
            relationships_created: Option<i32>,
            #[serde(rename = "relationshipsDeleted")]
            relationships_deleted: Option<i32>,
        }

        #[derive(Deserialize)]
        struct Neo4jError {
            code: String,
            message: String,
        }

        let url = format!("{}/db/{}/tx/commit", self.config.uri, self.config.database);

        #[derive(Serialize)]
        struct TxRequest {
            statements: Vec<Statement>,
        }

        #[derive(Serialize)]
        struct Statement {
            statement: String,
            parameters: HashMap<String, serde_json::Value>,
        }

        let request = TxRequest {
            statements: vec![Statement {
                statement: query.to_string(),
                parameters: params,
            }],
        };

        let response = self
            .http_client
            .post(&url)
            .header("Content-Type", "application/json")
            .basic_auth(&self.config.username, Some(&self.config.password))
            .json(&request)
            .send()
            .await
            .map_err(|e| KgError::ConnectionError(e.to_string()))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(KgError::QueryError(format!("HTTP {}: {}", status, body)));
        }

        let tx_resp: TxResponse = response
            .json()
            .await
            .map_err(|e| KgError::QueryError(e.to_string()))?;

        if !tx_resp.errors.is_empty() {
            let err = &tx_resp.errors[0];
            return Err(KgError::QueryError(format!("{}: {}", err.code, err.message)));
        }

        let stats = tx_resp.results[0]
            .stats
            .as_ref()
            .map(|s| WriteStats {
                nodes_created: s.nodes_created.unwrap_or(0),
                nodes_deleted: s.nodes_deleted.unwrap_or(0),
                relationships_created: s.relationships_created.unwrap_or(0),
                relationships_deleted: s.relationships_deleted.unwrap_or(0),
            })
            .unwrap_or_default();

        Ok(stats)
    }

    // =========================================================================
    // Node Operations
    // =========================================================================

    /// Create or update a Paper node
    pub async fn upsert_paper(
        &self,
        entity_id: &str,
        title: &str,
        properties: serde_json::Value,
    ) -> Result<KgNode, KgError> {
        let mut params = HashMap::new();
        params.insert("entity_id".to_string(), serde_json::json!(entity_id));
        params.insert("title".to_string(), serde_json::json!(title));
        params.insert("properties".to_string(), properties);

        let query = r#"
            MERGE (p:Paper {entity_id: $entity_id})
            SET p.title = $title, p += $properties
            RETURN p
        "#;

        let results = self.execute_cypher(query, params).await?;

        if let Some(row) = results.first() {
            let p = &row["p"];
            Ok(self.node_from_neo4j(p, NodeType::Paper)?)
        } else {
            Err(KgError::NodeNotFound(entity_id.to_string()))
        }
    }

    /// Create or update a Tag node
    pub async fn upsert_tag(&self, entity_id: &str, label: &str) -> Result<KgNode, KgError> {
        let mut params = HashMap::new();
        params.insert("entity_id".to_string(), serde_json::json!(entity_id));
        params.insert("label".to_string(), serde_json::json!(label));

        let query = r#"
            MERGE (t:Tag {entity_id: $entity_id})
            SET t.label = $label
            RETURN t
        "#;

        let results = self.execute_cypher(query, params).await?;

        if let Some(row) = results.first() {
            let t = &row["t"];
            Ok(self.node_from_neo4j(t, NodeType::Tag)?)
        } else {
            Err(KgError::NodeNotFound(entity_id.to_string()))
        }
    }

    /// Create or update an Author node
    pub async fn upsert_author(&self, entity_id: &str, name: &str) -> Result<KgNode, KgError> {
        let mut params = HashMap::new();
        params.insert("entity_id".to_string(), serde_json::json!(entity_id));
        params.insert("name".to_string(), serde_json::json!(name));

        let query = r#"
            MERGE (a:Author {entity_id: $entity_id})
            SET a.name = $name
            RETURN a
        "#;

        let results = self.execute_cypher(query, params).await?;

        if let Some(row) = results.first() {
            let a = &row["a"];
            Ok(self.node_from_neo4j(a, NodeType::Author)?)
        } else {
            Err(KgError::NodeNotFound(entity_id.to_string()))
        }
    }

    /// Get a paper by entity_id
    pub async fn get_paper(&self, entity_id: &str) -> Result<Option<KgNode>, KgError> {
        let (query, params) = patterns::find_paper_by_entity_id(entity_id);
        let results = self.execute_cypher(&query, params).await?;

        if let Some(row) = results.first() {
            let p = &row["p"];
            Ok(Some(self.node_from_neo4j(p, NodeType::Paper)?))
        } else {
            Ok(None)
        }
    }

    /// Get papers by tag
    pub async fn get_papers_by_tag(
        &self,
        tag: &str,
        limit: usize,
    ) -> Result<Vec<KgNode>, KgError> {
        let (query, params) = patterns::find_papers_by_tag(tag, limit);
        let results = self.execute_cypher(&query, params).await?;

        results
            .iter()
            .map(|row| {
                let p = &row["p"];
                self.node_from_neo4j(p, NodeType::Paper)
            })
            .collect()
    }

    /// Get papers by author name
    pub async fn get_papers_by_author(
        &self,
        author_name: &str,
        limit: usize,
    ) -> Result<Vec<KgNode>, KgError> {
        let (query, params) = patterns::find_papers_by_author(author_name, limit);
        let results = self.execute_cypher(&query, params).await?;

        results
            .iter()
            .map(|row| {
                let p = &row["p"];
                self.node_from_neo4j(p, NodeType::Paper)
            })
            .collect()
    }

    /// Get related papers (sharing tags)
    pub async fn get_related_papers(
        &self,
        entity_id: &str,
        limit: usize,
    ) -> Result<Vec<KgNode>, KgError> {
        let (query, params) = patterns::find_related_papers(entity_id, limit);
        let results = self.execute_cypher(&query, params).await?;

        results
            .iter()
            .map(|row| {
                let p = &row["p2"];
                self.node_from_neo4j(p, NodeType::Paper)
            })
            .collect()
    }

    /// Delete a node by entity_id
    pub async fn delete_node(&self, node_type: NodeType, entity_id: &str) -> Result<(), KgError> {
        let mut params = HashMap::new();
        params.insert("entity_id".to_string(), serde_json::json!(entity_id));

        let query = format!(
            "MATCH (n:{}) WHERE n.entity_id = $entity_id DETACH DELETE n",
            node_type.label()
        );

        self.execute_write(&query, params).await?;
        Ok(())
    }

    // =========================================================================
    // Edge Operations
    // =========================================================================

    /// Create a citation relationship
    pub async fn add_citation(
        &self,
        citing_entity_id: &str,
        cited_entity_id: &str,
    ) -> Result<KgEdge, KgError> {
        let mut params = HashMap::new();
        params.insert("citing".to_string(), serde_json::json!(citing_entity_id));
        params.insert("cited".to_string(), serde_json::json!(cited_entity_id));

        let query = r#"
            MATCH (citing:Paper {entity_id: $citing})
            MATCH (cited:Paper {entity_id: $cited})
            MERGE (citing)-[r:CITES]->(cited)
            RETURN r
        "#;

        let results = self.execute_cypher(query, params).await?;

        if let Some(row) = results.first() {
            let r = &row["r"];
            self.edge_from_neo4j(r, citing_entity_id, cited_entity_id)
        } else {
            Err(KgError::EdgeNotFound(format!(
                "{} -> {}",
                citing_entity_id, cited_entity_id
            )))
        }
    }

    /// Create a "derives from" relationship (Author → Paper)
    pub async fn add_author_paper(
        &self,
        author_entity_id: &str,
        paper_entity_id: &str,
    ) -> Result<KgEdge, KgError> {
        let mut params = HashMap::new();
        params.insert("author".to_string(), serde_json::json!(author_entity_id));
        params.insert("paper".to_string(), serde_json::json!(paper_entity_id));

        let query = r#"
            MATCH (a:Author {entity_id: $author})
            MATCH (p:Paper {entity_id: $paper})
            MERGE (a)-[r:DERIVES_FROM]->(p)
            RETURN r
        "#;

        let results = self.execute_cypher(query, params).await?;

        if let Some(row) = results.first() {
            let r = &row["r"];
            self.edge_from_neo4j(r, author_entity_id, paper_entity_id)
        } else {
            Err(KgError::EdgeNotFound(format!(
                "{} -> {}",
                author_entity_id, paper_entity_id
            )))
        }
    }

    /// Create a "tagged with" relationship (Paper → Tag)
    pub async fn add_paper_tag(
        &self,
        paper_entity_id: &str,
        tag_entity_id: &str,
    ) -> Result<KgEdge, KgError> {
        let mut params = HashMap::new();
        params.insert("paper".to_string(), serde_json::json!(paper_entity_id));
        params.insert("tag".to_string(), serde_json::json!(tag_entity_id));

        let query = r#"
            MATCH (p:Paper {entity_id: $paper})
            MATCH (t:Tag {entity_id: $tag})
            MERGE (p)-[r:TAGGED_WITH]->(t)
            RETURN r
        "#;

        let results = self.execute_cypher(query, params).await?;

        if let Some(row) = results.first() {
            let r = &row["r"];
            self.edge_from_neo4j(r, paper_entity_id, tag_entity_id)
        } else {
            Err(KgError::EdgeNotFound(format!(
                "{} -> {}",
                paper_entity_id, tag_entity_id
            )))
        }
    }

    /// Delete an edge between two nodes
    pub async fn delete_edge(
        &self,
        source_entity_id: &str,
        target_entity_id: &str,
        edge_type: EdgeType,
    ) -> Result<(), KgError> {
        let mut params = HashMap::new();
        params.insert("source".to_string(), serde_json::json!(source_entity_id));
        params.insert("target".to_string(), serde_json::json!(target_entity_id));

        let query = format!(
            r#"
            MATCH (s {{entity_id: $source}})-[r:{}]->(t {{entity_id: $target}})
            DELETE r
            "#,
            edge_type.rel_type()
        );

        self.execute_write(&query, params).await?;
        Ok(())
    }

    // =========================================================================
    // Utility Methods
    // =========================================================================

    /// Convert a Neo4j node JSON to KgNode
    fn node_from_neo4j(&self, node: &serde_json::Value, node_type: NodeType) -> Result<KgNode, KgError> {
        let obj = node
            .as_object()
            .ok_or_else(|| KgError::SerializationError(
                serde_json::from_str::<serde_json::Value>("null").unwrap_err()
            ))?;

        let id = obj
            .get("elementId")
            .or_else(|| obj.get("id"))
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        let entity_id = obj
            .get("entity_id")
            .or_else(|| obj.get("properties").and_then(|p| p.get("entity_id")))
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        let label = obj
            .get("label")
            .or_else(|| {
                // For Paper, use title; for Author, use name; for Tag, use entity_id
                let props = obj.get("properties")?;
                match node_type {
                    NodeType::Paper => props.get("title"),
                    NodeType::Author => props.get("name"),
                    NodeType::Tag => props.get("label"),
                    _ => None,
                }
            })
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        let properties = obj.get("properties").cloned().unwrap_or(serde_json::json!({}));

        Ok(KgNode {
            id,
            entity_id,
            label,
            node_type,
            properties,
        })
    }

    /// Convert a Neo4j relationship JSON to KgEdge
    fn edge_from_neo4j(
        &self,
        rel: &serde_json::Value,
        source: &str,
        target: &str,
    ) -> Result<KgEdge, KgError> {
        let obj = rel
            .as_object()
            .ok_or_else(|| KgError::SerializationError(
                serde_json::from_str::<serde_json::Value>("null").unwrap_err()
            ))?;

        let id = obj
            .get("elementId")
            .or_else(|| obj.get("id"))
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        let rel_type_str = obj
            .get("type")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        let edge_type = EdgeType::from_str(rel_type_str)
            .ok_or_else(|| KgError::InvalidEdgeType(rel_type_str.to_string()))?;

        let properties = obj.get("properties").cloned().unwrap_or(serde_json::json!({}));

        Ok(KgEdge {
            id,
            source: source.to_string(),
            target: target.to_string(),
            edge_type,
            weight: 1.0,
            properties,
        })
    }

    /// Check if the database is reachable
    pub async fn health_check(&self) -> Result<bool, KgError> {
        let url = format!("{}/", self.config.uri);

        let response = self
            .http_client
            .get(&url)
            .basic_auth(&self.config.username, Some(&self.config.password))
            .send()
            .await
            .map_err(|e| KgError::ConnectionError(e.to_string()))?;

        Ok(response.status().is_success())
    }
}

/// Statistics from a write operation
#[derive(Debug, Clone, Default)]
pub struct WriteStats {
    pub nodes_created: i32,
    pub nodes_deleted: i32,
    pub relationships_created: i32,
    pub relationships_deleted: i32,
}
