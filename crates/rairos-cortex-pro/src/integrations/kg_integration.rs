//! Knowledge Graph integration for rairos-cortex-pro.
//!
//! Provides functionality to build and query knowledge graphs
//! of materials science research using Neo4j.

use serde::{Deserialize, Serialize};

/// A node in the materials knowledge graph.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KgNode {
    /// Node ID
    pub id: String,
    /// Node type
    pub node_type: KgNodeType,
    /// Node label/name
    pub label: String,
    /// Properties
    pub properties: serde_json::Value,
}

/// Type of knowledge graph node.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum KgNodeType {
    /// Material (e.g., Bi2Te3)
    Material,
    /// Property (e.g., thermoelectric figure of merit)
    Property,
    /// Method (e.g., CGCNN, DFT)
    Method,
    /// Paper
    Paper,
    /// Author
    Author,
    /// Application
    Application,
}

impl Default for KgNodeType {
    fn default() -> Self {
        KgNodeType::Material
    }
}

/// An edge in the knowledge graph.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KgEdge {
    /// Source node ID
    pub from: String,
    /// Target node ID
    pub to: String,
    /// Edge relationship type
    pub relationship: KgRelationship,
}

/// Relationship types in the knowledge graph.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum KgRelationship {
    /// Material has property
    HasProperty,
    /// Material uses method
    UsesMethod,
    /// Material enables application
    EnablesApplication,
    /// Paper reports material
    ReportsMaterial,
    /// Paper cites paper
    Cites,
    /// Author writes paper
    WritesPaper,
}

impl Default for KgRelationship {
    fn default() -> Self {
        KgRelationship::HasProperty
    }
}

/// Knowledge graph service for materials research.
#[derive(Debug, Clone)]
pub struct KnowledgeGraphService {
    enabled: bool,
}

impl KnowledgeGraphService {
    /// Create a new knowledge graph service.
    pub fn new() -> Self {
        Self { enabled: true }
    }

    /// Create a disabled service.
    pub fn disabled() -> Self {
        Self { enabled: false }
    }

    /// Check if this service is enabled.
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// Add a node to the knowledge graph.
    pub async fn add_node(&self, _node: &KgNode) -> Result<(), KgServiceError> {
        if !self.enabled {
            return Err(KgServiceError::Disabled);
        }
        Ok(())
    }

    /// Add an edge to the knowledge graph.
    pub async fn add_edge(&self, _edge: &KgEdge) -> Result<(), KgServiceError> {
        if !self.enabled {
            return Err(KgServiceError::Disabled);
        }
        Ok(())
    }

    /// Query materials by property.
    pub async fn find_materials_by_property(
        &self,
        _property: &str,
    ) -> Result<Vec<KgNode>, KgServiceError> {
        if !self.enabled {
            return Err(KgServiceError::Disabled);
        }
        Ok(vec![])
    }

    /// Find paths between two nodes.
    pub async fn find_paths(
        &self,
        _from: &str,
        _to: &str,
        _max_hops: usize,
    ) -> Result<Vec<Vec<KgNode>>, KgServiceError> {
        if !self.enabled {
            return Err(KgServiceError::Disabled);
        }
        Ok(vec![])
    }
}

impl Default for KnowledgeGraphService {
    fn default() -> Self {
        Self::new()
    }
}

/// Error type for knowledge graph operations.
#[derive(Debug, thiserror::Error)]
pub enum KgServiceError {
    #[error("Knowledge graph service is disabled")]
    Disabled,
    #[error("Graph error: {0}")]
    GraphError(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_kg_node_creation() {
        let node = KgNode {
            id: "mat-1".to_string(),
            node_type: KgNodeType::Material,
            label: "Bi2Te3".to_string(),
            properties: serde_json::json!({"formula": "Bi2Te3"}),
        };
        assert_eq!(node.label, "Bi2Te3");
    }

    #[test]
    fn test_kg_service_disabled() {
        let service = KnowledgeGraphService::disabled();
        assert!(!service.is_enabled());
    }
}
