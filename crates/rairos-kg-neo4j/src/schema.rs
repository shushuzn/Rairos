//! Schema definitions for knowledge graph nodes and edges.
//!
//! These types mirror the schema from `rairos-kg` (SQLite) but are adapted
//! for Neo4j's graph model.

use serde::{Deserialize, Serialize};

/// Node types in the knowledge graph
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NodeType {
    Paper,
    Tag,
    Author,
    PNote,
    CNote,
    MNote,
    Figure,
    Table,
}

impl NodeType {
    /// Returns the Neo4j label for this node type
    pub fn label(&self) -> &'static str {
        match self {
            NodeType::Paper => "Paper",
            NodeType::Tag => "Tag",
            NodeType::Author => "Author",
            NodeType::PNote => "PNote",
            NodeType::CNote => "CNote",
            NodeType::MNote => "MNote",
            NodeType::Figure => "Figure",
            NodeType::Table => "Table",
        }
    }

    /// Parse from string (case-insensitive)
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "paper" => Some(NodeType::Paper),
            "tag" => Some(NodeType::Tag),
            "author" => Some(NodeType::Author),
            "p_note" | "pnote" => Some(NodeType::PNote),
            "c_note" | "cnote" => Some(NodeType::CNote),
            "m_note" | "mnote" => Some(NodeType::MNote),
            "figure" => Some(NodeType::Figure),
            "table" => Some(NodeType::Table),
            _ => None,
        }
    }
}

impl std::fmt::Display for NodeType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.label())
    }
}

/// Edge types (relationship types) in the knowledge graph
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EdgeType {
    Cite,
    Derive,
    SameTag,
    InComparison,
    HasNote,
    AboutTag,
    HasFigure,
    HasTable,
}

impl EdgeType {
    /// Returns the Neo4j relationship type for this edge type
    pub fn rel_type(&self) -> &'static str {
        match self {
            EdgeType::Cite => "CITES",
            EdgeType::Derive => "DERIVES_FROM",
            EdgeType::SameTag => "TAGGED_WITH",
            EdgeType::InComparison => "IN_COMPARISON_WITH",
            EdgeType::HasNote => "HAS_NOTE",
            EdgeType::AboutTag => "ABOUT_TAG",
            EdgeType::HasFigure => "HAS_FIGURE",
            EdgeType::HasTable => "HAS_TABLE",
        }
    }

    /// Parse from string (case-insensitive)
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "cite" | "cites" => Some(EdgeType::Cite),
            "derive" | "derives" | "derives_from" => Some(EdgeType::Derive),
            "same_tag" | "tagged_with" | "sametag" => Some(EdgeType::SameTag),
            "in_comparison" | "incomparison" => Some(EdgeType::InComparison),
            "has_note" | "hasnote" => Some(EdgeType::HasNote),
            "about_tag" | "abouttag" => Some(EdgeType::AboutTag),
            "has_figure" | "hasfigure" => Some(EdgeType::HasFigure),
            "has_table" | "hastable" => Some(EdgeType::HasTable),
            _ => None,
        }
    }
}

impl std::fmt::Display for EdgeType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.rel_type())
    }
}

/// A node in the knowledge graph
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KgNode {
    /// Internal Neo4j node ID
    pub id: String,
    /// Business identifier (e.g., arxiv_id for papers)
    pub entity_id: String,
    /// Display label
    pub label: String,
    /// Node type
    #[serde(rename = "type")]
    pub node_type: NodeType,
    /// Additional properties as JSON
    pub properties: serde_json::Value,
}

impl KgNode {
    /// Create a Paper node
    pub fn paper(entity_id: &str, title: &str, properties: serde_json::Value) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            entity_id: entity_id.to_string(),
            label: title.to_string(),
            node_type: NodeType::Paper,
            properties,
        }
    }

    /// Create a Tag node
    pub fn tag(entity_id: &str, label: &str) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            entity_id: entity_id.to_string(),
            label: label.to_string(),
            node_type: NodeType::Tag,
            properties: serde_json::Value::Object(serde_json::Map::new()),
        }
    }

    /// Create an Author node
    pub fn author(entity_id: &str, name: &str) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            entity_id: entity_id.to_string(),
            label: name.to_string(),
            node_type: NodeType::Author,
            properties: serde_json::json!({ "name": name }),
        }
    }

    /// Create a Note node (P/C/M Note)
    pub fn note(entity_id: &str, label: &str, note_type: NodeType) -> Self {
        debug_assert!(
            matches!(note_type, NodeType::PNote | NodeType::CNote | NodeType::MNote),
            "note_type must be PNote, CNote, or MNote"
        );
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            entity_id: entity_id.to_string(),
            label: label.to_string(),
            node_type: note_type,
            properties: serde_json::Value::Object(serde_json::Map::new()),
        }
    }
}

/// An edge (relationship) in the knowledge graph
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KgEdge {
    /// Internal Neo4j relationship ID
    pub id: String,
    /// Source node ID
    pub source: String,
    /// Target node ID
    pub target: String,
    /// Edge type
    #[serde(rename = "type")]
    pub edge_type: EdgeType,
    /// Relationship weight (for algorithms)
    pub weight: f32,
    /// Additional properties as JSON
    pub properties: serde_json::Value,
}

impl KgEdge {
    /// Create a citation edge
    pub fn cites(source: &str, target: &str) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            source: source.to_string(),
            target: target.to_string(),
            edge_type: EdgeType::Cite,
            weight: 1.0,
            properties: serde_json::Value::Object(serde_json::Map::new()),
        }
    }

    /// Create a derive edge (Author → Paper)
    pub fn derive(source: &str, target: &str, weight: f32) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            source: source.to_string(),
            target: target.to_string(),
            edge_type: EdgeType::Derive,
            weight,
            properties: serde_json::Value::Object(serde_json::Map::new()),
        }
    }

    /// Create a same-tag edge (Paper → Tag)
    pub fn same_tag(source: &str, target: &str) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            source: source.to_string(),
            target: target.to_string(),
            edge_type: EdgeType::SameTag,
            weight: 1.0,
            properties: serde_json::Value::Object(serde_json::Map::new()),
        }
    }
}

/// A generic graph record returned from queries
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphRecord {
    pub node: KgNode,
    pub relationships: Vec<KgEdge>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_node_creation() {
        let paper = KgNode::paper("2101.12345", "Attention Is All You Need", serde_json::json!({}));
        assert_eq!(paper.entity_id, "2101.12345");
        assert_eq!(paper.node_type, NodeType::Paper);
    }

    #[test]
    fn test_node_type_from_str() {
        // Test exact matches
        assert_eq!(NodeType::from_str("paper"), Some(NodeType::Paper));
        assert_eq!(NodeType::from_str("tag"), Some(NodeType::Tag));
        assert_eq!(NodeType::from_str("author"), Some(NodeType::Author));

        // Test case insensitivity
        assert_eq!(NodeType::from_str("PAPER"), Some(NodeType::Paper));
        assert_eq!(NodeType::from_str("Paper"), Some(NodeType::Paper));
        assert_eq!(NodeType::from_str("AuThOr"), Some(NodeType::Author));

        // Test snake_case variants
        assert_eq!(NodeType::from_str("p_note"), Some(NodeType::PNote));
        assert_eq!(NodeType::from_str("pnote"), Some(NodeType::PNote));
        assert_eq!(NodeType::from_str("c_note"), Some(NodeType::CNote));
        assert_eq!(NodeType::from_str("cnote"), Some(NodeType::CNote));
        assert_eq!(NodeType::from_str("m_note"), Some(NodeType::MNote));
        assert_eq!(NodeType::from_str("mnote"), Some(NodeType::MNote));

        // Test figure and table
        assert_eq!(NodeType::from_str("figure"), Some(NodeType::Figure));
        assert_eq!(NodeType::from_str("table"), Some(NodeType::Table));

        // Test invalid
        assert_eq!(NodeType::from_str("invalid"), None);
        assert_eq!(NodeType::from_str(""), None);
    }

    #[test]
    fn test_edge_type_from_str() {
        // Test exact matches
        assert_eq!(EdgeType::from_str("cite"), Some(EdgeType::Cite));
        assert_eq!(EdgeType::from_str("derive"), Some(EdgeType::Derive));
        assert_eq!(EdgeType::from_str("same_tag"), Some(EdgeType::SameTag));

        // Test case insensitivity
        assert_eq!(EdgeType::from_str("CITE"), Some(EdgeType::Cite));
        assert_eq!(EdgeType::from_str("Cite"), Some(EdgeType::Cite));
        assert_eq!(EdgeType::from_str("DERIVE"), Some(EdgeType::Derive));

        // Test alternative names
        assert_eq!(EdgeType::from_str("cites"), Some(EdgeType::Cite));
        assert_eq!(EdgeType::from_str("derives"), Some(EdgeType::Derive));
        assert_eq!(EdgeType::from_str("derives_from"), Some(EdgeType::Derive));
        assert_eq!(EdgeType::from_str("tagged_with"), Some(EdgeType::SameTag));
        assert_eq!(EdgeType::from_str("sametag"), Some(EdgeType::SameTag));

        // Test other edge types
        assert_eq!(EdgeType::from_str("in_comparison"), Some(EdgeType::InComparison));
        assert_eq!(EdgeType::from_str("incomparison"), Some(EdgeType::InComparison));
        assert_eq!(EdgeType::from_str("has_note"), Some(EdgeType::HasNote));
        assert_eq!(EdgeType::from_str("hasnote"), Some(EdgeType::HasNote));
        assert_eq!(EdgeType::from_str("about_tag"), Some(EdgeType::AboutTag));
        assert_eq!(EdgeType::from_str("has_figure"), Some(EdgeType::HasFigure));
        assert_eq!(EdgeType::from_str("has_table"), Some(EdgeType::HasTable));

        // Test invalid
        assert_eq!(EdgeType::from_str("invalid"), None);
        assert_eq!(EdgeType::from_str(""), None);
    }

    #[test]
    fn test_kg_edge_cites() {
        let edge = KgEdge::cites("source_id", "target_id");
        assert_eq!(edge.source, "source_id");
        assert_eq!(edge.target, "target_id");
        assert_eq!(edge.edge_type, EdgeType::Cite);
        assert_eq!(edge.weight, 1.0);
        assert!(edge.properties.is_object());
    }

    #[test]
    fn test_node_type_label() {
        assert_eq!(NodeType::Paper.label(), "Paper");
        assert_eq!(NodeType::Tag.label(), "Tag");
        assert_eq!(NodeType::Author.label(), "Author");
        assert_eq!(NodeType::PNote.label(), "PNote");
        assert_eq!(NodeType::CNote.label(), "CNote");
        assert_eq!(NodeType::MNote.label(), "MNote");
        assert_eq!(NodeType::Figure.label(), "Figure");
        assert_eq!(NodeType::Table.label(), "Table");
    }

    #[test]
    fn test_edge_type_rel_type() {
        assert_eq!(EdgeType::Cite.rel_type(), "CITES");
        assert_eq!(EdgeType::Derive.rel_type(), "DERIVES_FROM");
        assert_eq!(EdgeType::SameTag.rel_type(), "TAGGED_WITH");
        assert_eq!(EdgeType::InComparison.rel_type(), "IN_COMPARISON_WITH");
        assert_eq!(EdgeType::HasNote.rel_type(), "HAS_NOTE");
        assert_eq!(EdgeType::AboutTag.rel_type(), "ABOUT_TAG");
        assert_eq!(EdgeType::HasFigure.rel_type(), "HAS_FIGURE");
        assert_eq!(EdgeType::HasTable.rel_type(), "HAS_TABLE");
    }

    #[test]
    fn test_kg_node_paper() {
        let props = serde_json::json!({"year": 2017, "venue": "NeurIPS"});
        let paper = KgNode::paper("1706.03762", "Attention Is All You Need", props.clone());
        assert_eq!(paper.entity_id, "1706.03762");
        assert_eq!(paper.label, "Attention Is All You Need");
        assert_eq!(paper.node_type, NodeType::Paper);
        assert_eq!(paper.properties, props);
        // ID should be a valid UUID string
        assert!(!paper.id.is_empty());
    }

    #[test]
    fn test_kg_node_display() {
        assert_eq!(format!("{}", NodeType::Paper), "Paper");
        assert_eq!(format!("{}", NodeType::Author), "Author");
        assert_eq!(format!("{}", EdgeType::Cite), "CITES");
        assert_eq!(format!("{}", EdgeType::Derive), "DERIVES_FROM");
    }
}
