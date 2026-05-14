//! Rairos KG — Knowledge Graph Manager
//!
//! Manages the paper knowledge graph: nodes, edges, and queries.
//! Replaces: kg/manager.py, kg/queries.py

use rairos_core::Paper;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use thiserror::Error;

// ============================================================================
// Error Types
// ============================================================================

#[derive(Error, Debug)]
pub enum KgError {
    #[error("Node not found: {0}")]
    NodeNotFound(String),
    #[error("Edge not found")]
    EdgeNotFound,
    #[error("Invalid operation: {0}")]
    InvalidOperation(String),
    #[error("Database error: {0}")]
    Database(String),
}

impl From<rusqlite::Error> for KgError {
    fn from(e: rusqlite::Error) -> Self {
        KgError::Database(e.to_string())
    }
}

// ============================================================================
// Node Types
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KgNode {
    pub id: String,
    pub paper_id: String,
    pub label: String,
    pub node_type: String,
    pub properties: HashMap<String, String>,
}

impl KgNode {
    pub fn from_paper(paper: &Paper) -> Self {
        let mut props = HashMap::new();
        props.insert("title".to_string(), paper.title.clone());
        props.insert("arxiv_id".to_string(), paper.arxiv_id.clone().unwrap_or_default());
        props.insert("cited_by".to_string(), paper.metadata.cited_by.to_string());
        Self {
            id: paper.id.clone(),
            paper_id: paper.id.clone(),
            label: paper.title.clone(),
            node_type: "paper".to_string(),
            properties: props,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KgEdge {
    pub id: String,
    pub source: String,
    pub target: String,
    pub relation: String,
    pub weight: f32,
    pub properties: HashMap<String, String>,
}

impl KgEdge {
    pub fn cites(source: &str, target: &str) -> Self {
        Self {
            id: format!("{}->{}", source, target),
            source: source.to_string(),
            target: target.to_string(),
            relation: "cites".to_string(),
            weight: 1.0,
            properties: HashMap::new(),
        }
    }

    pub fn related_to(source: &str, target: &str, weight: f32) -> Self {
        Self {
            id: format!("{}~{}", source, target),
            source: source.to_string(),
            target: target.to_string(),
            relation: "related_to".to_string(),
            weight,
            properties: HashMap::new(),
        }
    }
}

// ============================================================================
// SQLite Database
// ============================================================================

#[derive(Debug)]
pub struct KgDatabase {
    conn: rusqlite::Connection,
    path: std::path::PathBuf,
}

impl KgDatabase {
    /// Open or create a KG database at the given path
    pub fn new(path: std::path::PathBuf) -> Result<Self, KgError> {
        if let Some(p) = path.parent() {
            std::fs::create_dir_all(p).map_err(|e| KgError::Database(e.to_string()))?;
        }
        let conn = rusqlite::Connection::open(&path)?;
        let db = KgDatabase { conn, path };
        db.init_tables()?;
        Ok(db)
    }

    /// Create tables and indexes
    fn init_tables(&self) -> Result<(), KgError> {
        self.conn.execute_batch("
            CREATE TABLE IF NOT EXISTS kg_nodes (
                id TEXT PRIMARY KEY,
                type TEXT NOT NULL,
                entity_id TEXT NOT NULL DEFAULT '',
                label TEXT NOT NULL DEFAULT '',
                properties_json TEXT NOT NULL DEFAULT '{}',
                created_at TEXT NOT NULL DEFAULT (datetime('now')),
                UNIQUE(type, entity_id)
            );
            CREATE TABLE IF NOT EXISTS kg_edges (
                id TEXT PRIMARY KEY,
                source_id TEXT NOT NULL REFERENCES kg_nodes(id),
                target_id TEXT NOT NULL REFERENCES kg_nodes(id),
                relation_type TEXT NOT NULL,
                weight REAL NOT NULL DEFAULT 1.0,
                properties_json TEXT NOT NULL DEFAULT '{}',
                created_at TEXT NOT NULL DEFAULT (datetime('now'))
            );
            CREATE INDEX IF NOT EXISTS idx_nodes_type ON kg_nodes(type);
            CREATE INDEX IF NOT EXISTS idx_nodes_entity ON kg_nodes(type, entity_id);
            CREATE INDEX IF NOT EXISTS idx_edges_source ON kg_edges(source_id);
            CREATE INDEX IF NOT EXISTS idx_edges_target ON kg_edges(target_id);
            CREATE INDEX IF NOT EXISTS idx_edges_rel ON kg_edges(relation_type);
        ")?;
        Ok(())
    }

    /// Generate a UUID v4
    fn gen_id() -> String {
        uuid::Uuid::new_v4().to_string()
    }

    fn now() -> String {
        chrono::Utc::now().format("%Y-%m-%dT%H:%M:%S").to_string()
    }

    fn props_to_json(props: &HashMap<String, String>) -> String {
        serde_json::to_string(props).unwrap_or_else(|_| "{}".to_string())
    }

    fn json_to_props(json: &str) -> HashMap<String, String> {
        serde_json::from_str(json).unwrap_or_default()
    }

    // ── Node CRUD ──────────────────────────────────────────────────────────

    pub fn add_node(&self, node_type: &str, entity_id: &str, label: &str, properties: HashMap<String, String>) -> Result<String, KgError> {
        let id = Self::gen_id();
        let props_json = Self::props_to_json(&properties);
        let now = Self::now();
        self.conn.execute(
            "INSERT OR IGNORE INTO kg_nodes (id, type, entity_id, label, properties_json, created_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            rusqlite::params![id, node_type, entity_id, label, props_json, now],
        )?;
        let existing: Option<String> = self.conn.query_row(
            "SELECT id FROM kg_nodes WHERE type = ?1 AND entity_id = ?2",
            rusqlite::params![node_type, entity_id],
            |row| row.get(0),
        ).ok();
        Ok(existing.unwrap_or(id))
    }

    /// Add a node with an explicit ID. Used by KnowledgeGraph to sync IDs.
    fn add_node_with_id(&self, node_id: &str, node_type: &str, entity_id: &str, label: &str, properties: HashMap<String, String>) -> Result<(), KgError> {
        let props_json = Self::props_to_json(&properties);
        let now = Self::now();
        self.conn.execute(
            "INSERT OR IGNORE INTO kg_nodes (id, type, entity_id, label, properties_json, created_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            rusqlite::params![node_id, node_type, entity_id, label, props_json, now],
        )?;
        Ok(())
    }

    pub fn upsert_node(&self, node_type: &str, entity_id: &str, label: &str, properties: HashMap<String, String>) -> Result<String, KgError> {
        let id = Self::gen_id();
        let props_json = Self::props_to_json(&properties);
        let now = Self::now();
        self.conn.execute(
            "INSERT INTO kg_nodes (id, type, entity_id, label, properties_json, created_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6)
             ON CONFLICT(type, entity_id) DO UPDATE SET label = ?4, properties_json = ?5, created_at = ?6",
            rusqlite::params![id, node_type, entity_id, label, props_json, now],
        )?;
        let existing: Option<String> = self.conn.query_row(
            "SELECT id FROM kg_nodes WHERE type = ?1 AND entity_id = ?2",
            rusqlite::params![node_type, entity_id],
            |row| row.get(0),
        ).ok();
        Ok(existing.unwrap_or(id))
    }

    pub fn get_node(&self, node_id: &str) -> Result<Option<KgNode>, KgError> {
        let mut stmt = self.conn.prepare("SELECT id, type, entity_id, label, properties_json FROM kg_nodes WHERE id = ?1")?;
        let mut rows = stmt.query(rusqlite::params![node_id])?;
        match rows.next()? {
            Some(row) => Ok(Some(KgNode {
                id: row.get(0)?,
                paper_id: row.get::<_, String>(2)?, // entity_id as paper_id
                label: row.get(3)?,
                node_type: row.get(1)?,
                properties: Self::json_to_props(&row.get::<_, String>(4)?),
            })),
            None => Ok(None),
        }
    }

    pub fn get_node_by_entity(&self, node_type: &str, entity_id: &str) -> Result<Option<KgNode>, KgError> {
        let mut stmt = self.conn.prepare("SELECT id, type, entity_id, label, properties_json FROM kg_nodes WHERE type = ?1 AND entity_id = ?2")?;
        let mut rows = stmt.query(rusqlite::params![node_type, entity_id])?;
        match rows.next()? {
            Some(row) => Ok(Some(KgNode {
                id: row.get(0)?,
                paper_id: row.get::<_, String>(2)?,
                label: row.get(3)?,
                node_type: row.get(1)?,
                properties: Self::json_to_props(&row.get::<_, String>(4)?),
            })),
            None => Ok(None),
        }
    }

    pub fn get_all_nodes(&self, node_type: Option<&str>) -> Result<Vec<KgNode>, KgError> {
        let sql = match node_type {
            Some(_) => "SELECT id, type, entity_id, label, properties_json FROM kg_nodes WHERE type = ?1 ORDER BY created_at DESC",
            None => "SELECT id, type, entity_id, label, properties_json FROM kg_nodes ORDER BY created_at DESC",
        };
        let mut stmt = self.conn.prepare(sql)?;

        // Use the same closure type by collecting into Vec directly
        let map_fn = |row: &rusqlite::Row| -> rusqlite::Result<KgNode> {
            Ok(KgNode {
                id: row.get(0)?,
                paper_id: row.get::<_, String>(2)?,
                label: row.get(3)?,
                node_type: row.get(1)?,
                properties: KgDatabase::json_to_props(&row.get::<_, String>(4)?),
            })
        };

        let rows: Vec<KgNode> = match node_type {
            Some(nt) => stmt.query_map(rusqlite::params![nt], map_fn)?.collect::<Result<Vec<_>, _>>()?,
            None => stmt.query_map([], map_fn)?.collect::<Result<Vec<_>, _>>()?,
        };
        Ok(rows)
    }

    // ── Edge CRUD ──────────────────────────────────────────────────────────

    pub fn add_edge(&self, source_id: &str, target_id: &str, relation_type: &str, weight: f64, properties: HashMap<String, String>) -> Result<String, KgError> {
        let id = format!("{}-{}-{}", source_id, target_id, Self::gen_id().chars().take(8).collect::<String>());
        let props_json = Self::props_to_json(&properties);
        self.conn.execute(
            "INSERT OR IGNORE INTO kg_edges (id, source_id, target_id, relation_type, weight, properties_json, created_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            rusqlite::params![id, source_id, target_id, relation_type, weight, props_json, Self::now()],
        )?;
        Ok(id)
    }

    pub fn get_edges_by_node(&self, node_id: &str, direction: &str, rel_type: Option<&str>) -> Result<Vec<KgEdge>, KgError> {
        let sql = match (direction, rel_type) {
            ("out", Some(_)) => "SELECT id, source_id, target_id, relation_type, weight, properties_json FROM kg_edges WHERE source_id = ?1 AND relation_type = ?2",
            ("in", Some(_)) => "SELECT id, source_id, target_id, relation_type, weight, properties_json FROM kg_edges WHERE target_id = ?1 AND relation_type = ?2",
            (_, Some(_)) => "SELECT id, source_id, target_id, relation_type, weight, properties_json FROM kg_edges WHERE (source_id = ?1 OR target_id = ?1) AND relation_type = ?2",
            ("out", None) => "SELECT id, source_id, target_id, relation_type, weight, properties_json FROM kg_edges WHERE source_id = ?1",
            ("in", None) => "SELECT id, source_id, target_id, relation_type, weight, properties_json FROM kg_edges WHERE target_id = ?1",
            (_, None) => "SELECT id, source_id, target_id, relation_type, weight, properties_json FROM kg_edges WHERE source_id = ?1 OR target_id = ?1",
        };
        let mut stmt = self.conn.prepare(sql)?;
        let map_fn = |row: &rusqlite::Row| {
            Ok(KgEdge {
                id: row.get(0)?,
                source: row.get(1)?,
                target: row.get(2)?,
                relation: row.get(3)?,
                weight: row.get::<_, f64>(4)? as f32,
                properties: KgDatabase::json_to_props(&row.get::<_, String>(5)?),
            })
        };
        let rows: Vec<KgEdge> = match rel_type {
            Some(rt) => stmt.query_map(rusqlite::params![node_id, rt], map_fn)?.collect::<Result<Vec<_>, _>>()?,
            None => stmt.query_map(rusqlite::params![node_id], map_fn)?.collect::<Result<Vec<_>, _>>()?,
        };
        Ok(rows)
    }

    // ── BFS Neighbor Traversal ─────────────────────────────────────────────

    pub fn get_neighbors(&self, node_id: &str, depth: u32, relation_type: Option<&str>) -> Result<Vec<(KgNode, KgEdge, u32)>, KgError> {
        let mut visited = HashSet::new();
        visited.insert(node_id.to_string());
        let mut results = Vec::new();
        let mut current_level = vec![node_id.to_string()];

        for d in 1..=depth {
            let mut next_level = Vec::new();
            for nid in &current_level {
                let edges = self.get_edges_by_node(nid, "both", relation_type)?;
                for edge in &edges {
                    let neighbor_id = if edge.source == *nid { &edge.target } else { &edge.source };
                    if visited.insert(neighbor_id.clone()) {
                        if let Some(neighbor) = self.get_node(neighbor_id)? {
                            results.push((neighbor, edge.clone(), d));
                            next_level.push(neighbor_id.clone());
                        }
                    }
                }
            }
            current_level = next_level;
        }

        Ok(results)
    }

    // ── Stats ──────────────────────────────────────────────────────────────

    pub fn stats(&self) -> Result<KgStats, KgError> {
        let total_nodes: i64 = self.conn.query_row("SELECT COUNT(*) FROM kg_nodes", [], |r| r.get(0))?;
        let total_edges: i64 = self.conn.query_row("SELECT COUNT(*) FROM kg_edges", [], |r| r.get(0))?;
        let paper_nodes: i64 = self.conn.query_row("SELECT COUNT(*) FROM kg_nodes WHERE type = 'paper'", [], |r| r.get(0))?;
        let concept_nodes: i64 = self.conn.query_row("SELECT COUNT(*) FROM kg_nodes WHERE type = 'concept'", [], |r| r.get(0))?;
        let avg_degree = if total_nodes > 0 { total_edges as f32 / total_nodes as f32 } else { 0.0 };
        Ok(KgStats { total_nodes: total_nodes as usize, total_edges: total_edges as usize, avg_degree, paper_nodes: paper_nodes as usize, concept_nodes: concept_nodes as usize })
    }

    // ── Full graph export ──────────────────────────────────────────────────

    pub fn export_json(&self) -> Result<serde_json::Value, KgError> {
        let nodes = self.get_all_nodes(None)?;
        let mut edges = Vec::new();
        // Get ALL edges
        let mut stmt = self.conn.prepare("SELECT id, source_id, target_id, relation_type, weight, properties_json FROM kg_edges")?;
        let rows = stmt.query_map([], |row| {
            Ok(KgEdge {
                id: row.get(0)?,
                source: row.get(1)?,
                target: row.get(2)?,
                relation: row.get(3)?,
                weight: row.get::<_, f64>(4)? as f32,
                properties: KgDatabase::json_to_props(&row.get::<_, String>(5)?),
            })
        })?;
        for row in rows {
            edges.push(row?);
        }
        Ok(serde_json::json!({ "nodes": nodes, "edges": edges }))
    }

    /// Get database path
    pub fn path(&self) -> &std::path::Path {
        &self.path
    }
}

// ============================================================================
// Knowledge Graph (in-memory + optional SQLite persistence)
// ============================================================================

#[derive(Debug, Default)]
pub struct KnowledgeGraph {
    pub nodes: HashMap<String, KgNode>,
    pub edges: Vec<KgEdge>,
    pub outgoing: HashMap<String, Vec<String>>,
    pub incoming: HashMap<String, Vec<String>>,
    db: Option<KgDatabase>,
}

impl KnowledgeGraph {
    pub fn new() -> Self {
        Self::default()
    }

    /// Create with optional SQLite database connection
    pub fn with_db(db_path: std::path::PathBuf) -> Result<Self, KgError> {
        let db = KgDatabase::new(db_path)?;
        let mut graph = Self::default();
        graph.db = Some(db);
        Ok(graph)
    }

    pub fn set_db(&mut self, db: KgDatabase) {
        // Load all nodes from DB into memory
        if let Ok(nodes) = db.get_all_nodes(None) {
            for n in nodes {
                self.nodes.insert(n.id.clone(), n);
            }
        }
        self.db = Some(db);
    }

    pub fn add_paper(&mut self, paper: &Paper) {
        let node = KgNode::from_paper(paper);
        self.add_node(node);
    }

    pub fn add_node(&mut self, node: KgNode) {
        if let Some(ref db) = self.db {
            let _ = db.add_node_with_id(&node.id, &node.node_type, &node.paper_id, &node.label, node.properties.clone());
        }
        self.nodes.insert(node.id.clone(), node);
    }

    pub fn add_edge(&mut self, edge: KgEdge) {
        if !self.nodes.contains_key(&edge.source) || !self.nodes.contains_key(&edge.target) {
            tracing::warn!("Edge references unknown node: {} -> {}", edge.source, edge.target);
            return;
        }
        if let Some(ref db) = self.db {
            let _ = db.add_edge(&edge.source, &edge.target, &edge.relation, edge.weight as f64, edge.properties.clone());
        }
        self.edges.push(edge.clone());
        self.outgoing.entry(edge.source.clone()).or_default().push(edge.target.clone());
        self.incoming.entry(edge.target.clone()).or_default().push(edge.source.clone());
    }

    pub fn add_citation(&mut self, source_id: &str, target_id: &str) {
        self.add_edge(KgEdge::cites(source_id, target_id));
    }

    pub fn get_node(&self, id: &str) -> Option<&KgNode> {
        self.nodes.get(id)
    }

    pub fn nodes(&self) -> &HashMap<String, KgNode> {
        &self.nodes
    }

    pub fn edges(&self) -> &[KgEdge] {
        &self.edges
    }

    pub fn get_citing(&self, paper_id: &str) -> Vec<&KgNode> {
        self.incoming.get(paper_id)
            .map(|ids| ids.iter().filter_map(|id| self.nodes.get(id)).collect())
            .unwrap_or_default()
    }

    pub fn get_references(&self, paper_id: &str) -> Vec<&KgNode> {
        self.outgoing.get(paper_id)
            .map(|ids| ids.iter().filter_map(|id| self.nodes.get(id)).collect())
            .unwrap_or_default()
    }

    pub fn get_related(&self, paper_id: &str) -> Vec<&KgNode> {
        self.edges.iter()
            .filter(|e| e.source == paper_id && e.relation == "related_to")
            .filter_map(|e| self.nodes.get(&e.target))
            .collect()
    }

    pub fn find_path(&self, start: &str, end: &str) -> Option<Vec<String>> {
        if !self.nodes.contains_key(start) || !self.nodes.contains_key(end) {
            return None;
        }
        let mut visited = HashSet::new();
        let mut queue = vec![vec![start.to_string()]];
        while let Some(path) = queue.pop() {
            let current = path.last().unwrap();
            if current == end { return Some(path); }
            if visited.contains(current) { continue; }
            visited.insert(current.clone());
            if let Some(neighbors) = self.outgoing.get(current) {
                for neighbor in neighbors {
                    if !visited.contains(neighbor) {
                        let mut new_path = path.clone();
                        new_path.push(neighbor.clone());
                        queue.push(new_path);
                    }
                }
            }
        }
        None
    }

    pub fn stats(&self) -> KgStats {
        let node_count = self.nodes.len();
        let edge_count = self.edges.len();
        let avg_degree = if node_count > 0 { self.edges.len() as f32 / node_count as f32 } else { 0.0 };
        let paper_nodes = self.nodes.values().filter(|n| n.node_type == "paper").count();
        let concept_nodes = self.nodes.values().filter(|n| n.node_type == "concept").count();
        KgStats { total_nodes: node_count, total_edges: edge_count, avg_degree, paper_nodes, concept_nodes }
    }

    pub fn export_json(&self) -> serde_json::Value {
        serde_json::json!({
            "nodes": self.nodes.values().collect::<Vec<_>>(),
            "edges": self.edges,
        })
    }

    /// Access the kg database if connected
    pub fn database(&self) -> Option<&KgDatabase> {
        self.db.as_ref()
    }

    pub fn default_path() -> std::path::PathBuf {
        dirs::home_dir()
            .unwrap_or_else(|| std::path::PathBuf::from("."))
            .join(".ai_research_os").join("kg")
    }

    pub fn graph_path() -> std::path::PathBuf {
        Self::default_path().join("graph.json")
    }

    pub fn db_path() -> std::path::PathBuf {
        Self::default_path().join("kg.db")
    }

    pub fn load() -> std::io::Result<Self> {
        // Try SQLite first, fall back to JSON
        let db_path = Self::db_path();
        if db_path.exists() {
            if let Ok(db) = KgDatabase::new(db_path) {
                let mut graph = KnowledgeGraph::new();
                graph.set_db(db);
                return Ok(graph);
            }
        }
        let path = Self::graph_path();
        if !path.exists() { return Ok(Self::new()); }
        let text = std::fs::read_to_string(&path)?;
        let data: serde_json::Value = serde_json::from_str(&text)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        let nodes: Vec<KgNode> = serde_json::from_value(data.get("nodes").cloned().unwrap_or_default()).unwrap_or_default();
        let edges: Vec<KgEdge> = serde_json::from_value(data.get("edges").cloned().unwrap_or_default()).unwrap_or_default();
        let mut graph = Self::new();
        for node in nodes { graph.add_node(node); }
        for edge in edges { graph.add_edge(edge); }
        Ok(graph)
    }

    pub fn save(&self) -> std::io::Result<()> {
        // Always save JSON (backward compat)
        let path = Self::graph_path();
        if let Some(parent) = path.parent() { std::fs::create_dir_all(parent)?; }
        let json = serde_json::json!({ "nodes": self.nodes.values().collect::<Vec<_>>(), "edges": self.edges });
        std::fs::write(&path, serde_json::to_string_pretty(&json).unwrap_or_default())?;
        Ok(())
    }
}

// ============================================================================
// Statistics
// ============================================================================

#[derive(Debug, Serialize)]
pub struct KgStats {
    pub total_nodes: usize,
    pub total_edges: usize,
    pub avg_degree: f32,
    pub paper_nodes: usize,
    pub concept_nodes: usize,
}

// ============================================================================
// Graph Algorithms (unchanged)
// ============================================================================

pub struct GraphAlgorithms;

impl GraphAlgorithms {
    pub fn rank_papers(graph: &KnowledgeGraph) -> HashMap<String, f32> {
        let mut scores: HashMap<String, f32> = graph.nodes.keys().map(|id| (id.clone(), 1.0)).collect();
        let damping = 0.85;
        for _ in 0..20 {
            let mut new_scores: HashMap<String, f32> = HashMap::new();
            for node_id in scores.keys() {
                let incoming = graph.incoming.get(node_id);
                let mut contribution = 0.0;
                if let Some(incoming_ids) = incoming {
                    for inc_id in incoming_ids {
                        let out_degree = graph.outgoing.get(inc_id).map(|v| v.len()).unwrap_or(1);
                        if out_degree > 0 {
                            contribution += scores.get(inc_id).unwrap_or(&0.0) / out_degree as f32;
                        }
                    }
                }
                new_scores.insert(node_id.clone(), (1.0 - damping) + damping * contribution);
            }
            scores = new_scores;
        }
        scores
    }

    pub fn most_central(graph: &KnowledgeGraph) -> Option<(String, f32)> {
        Self::rank_papers(graph).into_iter().max_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal))
    }

    pub fn detect_communities(graph: &KnowledgeGraph) -> HashMap<String, usize> {
        let mut communities: HashMap<String, usize> = graph.nodes.keys().enumerate().map(|(i, id)| (id.clone(), i)).collect();
        let mut changed = true;
        let mut iterations = 0;
        while changed && iterations < 10 {
            changed = false;
            iterations += 1;
            for node_id in graph.nodes.keys() {
                let neighbors = graph.outgoing.get(node_id).map(|v| v.as_slice()).unwrap_or(&[]);
                if neighbors.is_empty() { continue; }
                let mut label_counts: HashMap<usize, usize> = HashMap::new();
                for neighbor_id in neighbors {
                    if let Some(&label) = communities.get(neighbor_id) {
                        *label_counts.entry(label).or_insert(0) += 1;
                    }
                }
                if let Some(&current_label) = communities.get(node_id) {
                    if let Some((new_label, count)) = label_counts.into_iter().max_by_key(|(_, c)| *c) {
                        if count > 1 && new_label != current_label {
                            communities.insert(node_id.clone(), new_label);
                            changed = true;
                        }
                    }
                }
            }
        }
        communities
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn test_db() -> (KgDatabase, std::path::PathBuf) {
        use std::sync::atomic::{AtomicU64, Ordering};
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        let unique = COUNTER.fetch_add(1, Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!("rairos_kg_{}_{}", std::process::id(), unique));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("test.db");
        let db = KgDatabase::new(path.clone()).unwrap();
        (db, dir)
    }

    // ── Existing KnowledgeGraph tests (unchanged) ──────────────────────────

    #[test]
    fn test_add_paper() {
        let paper = Paper::new(Some("2301.00001".into()), "Test Paper".into(), "Abstract".into());
        let mut graph = KnowledgeGraph::new();
        graph.add_paper(&paper);
        assert_eq!(graph.nodes.len(), 1);
        assert_eq!(graph.get_node(&paper.id).unwrap().label, "Test Paper");
    }

    #[test]
    fn test_citation_chain() {
        let p1 = Paper::new(Some("1".into()), "Paper 1".into(), "A".into());
        let p2 = Paper::new(Some("2".into()), "Paper 2".into(), "B".into());
        let mut graph = KnowledgeGraph::new();
        graph.add_paper(&p1);
        graph.add_paper(&p2);
        graph.add_citation(&p2.id, &p1.id);
        assert_eq!(graph.get_citing(&p1.id).len(), 1);
        assert_eq!(graph.get_references(&p2.id).len(), 1);
    }

    #[test]
    fn test_find_path() {
        let p1 = Paper::new(Some("1".into()), "Paper 1".into(), "A".into());
        let p2 = Paper::new(Some("2".into()), "Paper 2".into(), "B".into());
        let p3 = Paper::new(Some("3".into()), "Paper 3".into(), "C".into());
        let mut graph = KnowledgeGraph::new();
        graph.add_paper(&p1); graph.add_paper(&p2); graph.add_paper(&p3);
        graph.add_citation(&p2.id, &p1.id);
        graph.add_citation(&p3.id, &p2.id);
        let path = graph.find_path(&p3.id, &p1.id);
        assert!(path.is_some());
        assert_eq!(path.unwrap().len(), 3);
    }

    #[test]
    fn test_pagerank() {
        let p1 = Paper::new(Some("1".into()), "Paper 1".into(), "A".into());
        let p2 = Paper::new(Some("2".into()), "Paper 2".into(), "B".into());
        let mut graph = KnowledgeGraph::new();
        graph.add_paper(&p1); graph.add_paper(&p2);
        graph.add_citation(&p2.id, &p1.id);
        let ranks = GraphAlgorithms::rank_papers(&graph);
        assert_eq!(ranks.len(), 2);
    }

    // ── New SQLite tests ───────────────────────────────────────────────────

    #[test]
    fn test_db_add_get_node() {
        let (db, dir) = test_db();
        let id = db.add_node("paper", "2401.00001", "Test Paper", HashMap::new()).unwrap();
        let node = db.get_node(&id).unwrap().unwrap();
        assert_eq!(node.label, "Test Paper");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_db_get_node_by_entity() {
        let (db, dir) = test_db();
        db.add_node("paper", "2401.00001", "Test Paper", HashMap::new()).unwrap();
        let node = db.get_node_by_entity("paper", "2401.00001").unwrap().unwrap();
        assert_eq!(node.label, "Test Paper");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_db_add_edge() {
        let (db, dir) = test_db();
        let n1 = db.add_node("paper", "2401.00001", "Paper A", HashMap::new()).unwrap();
        let n2 = db.add_node("paper", "2401.00002", "Paper B", HashMap::new()).unwrap();
        db.add_edge(&n1, &n2, "cites", 1.0, HashMap::new()).unwrap();
        let edges = db.get_edges_by_node(&n1, "out", None).unwrap();
        assert_eq!(edges.len(), 1);
        assert_eq!(edges[0].relation, "cites");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_db_upsert_node() {
        let (db, dir) = test_db();
        let mut props = HashMap::new();
        props.insert("key".into(), "value".into());
        let id1 = db.upsert_node("paper", "2401.00001", "Original", props.clone()).unwrap();
        // Upsert same entity — should update, not create new
        let id2 = db.upsert_node("paper", "2401.00001", "Updated", props).unwrap();
        assert_eq!(id1, id2, "upsert should return same ID");
        let node = db.get_node(&id1).unwrap().unwrap();
        assert_eq!(node.label, "Updated", "label should be updated");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_db_get_neighbors() {
        let (db, dir) = test_db();
        let n1 = db.add_node("paper", "2401.00001", "Paper A", HashMap::new()).unwrap();
        let n2 = db.add_node("paper", "2401.00002", "Paper B", HashMap::new()).unwrap();
        let n3 = db.add_node("paper", "2401.00003", "Paper C", HashMap::new()).unwrap();
        db.add_edge(&n1, &n2, "cites", 1.0, HashMap::new()).unwrap();
        db.add_edge(&n2, &n3, "cites", 1.0, HashMap::new()).unwrap();

        // Depth 1: only n2
        let neighbors = db.get_neighbors(&n1, 1, None).unwrap();
        assert_eq!(neighbors.len(), 1);
        assert_eq!(neighbors[0].0.label, "Paper B");

        // Depth 2: n2 and n3
        let neighbors = db.get_neighbors(&n1, 2, None).unwrap();
        assert_eq!(neighbors.len(), 2);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_db_stats() {
        let (db, dir) = test_db();
        db.add_node("paper", "1", "Paper 1", HashMap::new()).unwrap();
        db.add_node("paper", "2", "Paper 2", HashMap::new()).unwrap();
        db.add_node("concept", "ml", "Machine Learning", HashMap::new()).unwrap();
        let stats = db.stats().unwrap();
        assert_eq!(stats.total_nodes, 3);
        assert_eq!(stats.paper_nodes, 2);
        assert_eq!(stats.concept_nodes, 1);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_db_export_json() {
        let (db, dir) = test_db();
        db.add_node("paper", "1", "Paper", HashMap::new()).unwrap();
        let json = db.export_json().unwrap();
        assert!(json["nodes"].as_array().unwrap().len() >= 1);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_knowledge_graph_with_db() {
        let dir = std::env::temp_dir().join(format!("rairos_kg_integration_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let db_path = dir.join("test_kg.db");

        let mut graph = KnowledgeGraph::with_db(db_path.clone()).unwrap();
        let p1 = Paper::new(Some("1".into()), "Paper 1".into(), "A".into());
        let p2 = Paper::new(Some("2".into()), "Paper 2".into(), "B".into());
        graph.add_paper(&p1);
        graph.add_paper(&p2);
        graph.add_citation(&p2.id, &p1.id);

        assert_eq!(graph.nodes.len(), 2);
        assert_eq!(graph.get_citing(&p1.id).len(), 1);

        // Verify data persisted in SQLite
        let db = KgDatabase::new(db_path).unwrap();
        let nodes = db.get_all_nodes(None).unwrap();
        assert_eq!(nodes.len(), 2, "should have 2 nodes in SQLite");
        let edges = db.get_edges_by_node(nodes[0].id.as_str(), "both", None).unwrap();
        assert!(!edges.is_empty(), "should have edges in SQLite");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
