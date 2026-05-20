//! Rairos KG — Knowledge Graph Manager
//!
//! Manages the paper knowledge graph: nodes, edges, and queries.
//! Replaces: kg/manager.py, kg/queries.py

use rairos_core::constants::{AIROS_DIR_NAME, KG_DIR, KG_GRAPH_FILE};
use rairos_core::Paper;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet, VecDeque};
use parking_lot::{Mutex, MutexGuard};
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum KgNodeType {
    Paper,
    Tag,
    Author,
    PNote,
    CNote,
    MNote,
    Figure,
    Table,
}

impl KgNodeType {
    pub fn as_str(&self) -> &'static str {
        match self {
            KgNodeType::Paper => "paper",
            KgNodeType::Tag => "tag",
            KgNodeType::Author => "author",
            KgNodeType::PNote => "p_note",
            KgNodeType::CNote => "c_note",
            KgNodeType::MNote => "m_note",
            KgNodeType::Figure => "figure",
            KgNodeType::Table => "table",
        }
    }

    pub fn from_string(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "paper" => KgNodeType::Paper,
            "tag" => KgNodeType::Tag,
            "author" => KgNodeType::Author,
            "p_note" | "p-note" => KgNodeType::PNote,
            "c_note" | "c-note" => KgNodeType::CNote,
            "m_note" | "m-note" => KgNodeType::MNote,
            "figure" => KgNodeType::Figure,
            "table" => KgNodeType::Table,
            _ => KgNodeType::Paper,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum KgEdgeType {
    Cite,
    Derive,
    SameTag,
    InComparison,
    HasNote,
    AboutTag,
    HasFigure,
    HasTable,
}

impl KgEdgeType {
    pub fn as_str(&self) -> &'static str {
        match self {
            KgEdgeType::Cite => "cite",
            KgEdgeType::Derive => "derive",
            KgEdgeType::SameTag => "same_tag",
            KgEdgeType::InComparison => "in_comparison",
            KgEdgeType::HasNote => "has_note",
            KgEdgeType::AboutTag => "about_tag",
            KgEdgeType::HasFigure => "has_figure",
            KgEdgeType::HasTable => "has_table",
        }
    }

    pub fn from_string(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "cite" => KgEdgeType::Cite,
            "derive" => KgEdgeType::Derive,
            "same_tag" => KgEdgeType::SameTag,
            "in_comparison" => KgEdgeType::InComparison,
            "has_note" => KgEdgeType::HasNote,
            "about_tag" => KgEdgeType::AboutTag,
            "has_figure" => KgEdgeType::HasFigure,
            "has_table" => KgEdgeType::HasTable,
            _ => KgEdgeType::Cite,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KgNode {
    pub id: String,
    pub entity_id: String,
    pub label: String,
    pub node_type: String,
    pub properties: serde_json::Value,
}

impl KgNode {
    pub fn from_paper(paper: &Paper) -> Self {
        Self {
            id: paper.id.clone(),
            entity_id: paper.arxiv_id.clone().unwrap_or_else(|| paper.id.clone()),
            label: paper.title.clone(),
            node_type: KgNodeType::Paper.as_str().to_string(),
            properties: serde_json::json!({
                "title": paper.title,
                "arxiv_id": paper.arxiv_id,
                "cited_by": paper.metadata.cited_by,
            }),
        }
    }

    pub fn new_tag(id: &str, label: &str) -> Self {
        Self {
            id: id.to_string(),
            entity_id: id.to_string(),
            label: label.to_string(),
            node_type: KgNodeType::Tag.as_str().to_string(),
            properties: serde_json::Value::Object(serde_json::Map::new()),
        }
    }

    pub fn new_author(id: &str, name: &str) -> Self {
        Self {
            id: id.to_string(),
            entity_id: id.to_string(),
            label: name.to_string(),
            node_type: KgNodeType::Author.as_str().to_string(),
            properties: serde_json::json!({"name": name}),
        }
    }

    pub fn new_note(id: &str, note_type: KgNodeType, label: &str) -> Self {
        Self {
            id: id.to_string(),
            entity_id: id.to_string(),
            label: label.to_string(),
            node_type: note_type.as_str().to_string(),
            properties: serde_json::Value::Object(serde_json::Map::new()),
        }
    }

    pub fn node_type_enum(&self) -> KgNodeType {
        KgNodeType::from_string(&self.node_type)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KgEdge {
    pub id: String,
    pub source: String,
    pub target: String,
    pub relation: String,
    pub weight: f32,
    pub properties: serde_json::Value,
}

impl KgEdge {
    pub fn new(source: &str, target: &str, rel_type: KgEdgeType, weight: f32) -> Self {
        Self {
            id: format!("{}-{}-{:x}", source, target, rand::random::<u32>()),
            source: source.to_string(),
            target: target.to_string(),
            relation: rel_type.as_str().to_string(),
            weight,
            properties: serde_json::Value::Object(serde_json::Map::new()),
        }
    }

    pub fn cites(source: &str, target: &str) -> Self {
        Self::new(source, target, KgEdgeType::Cite, 1.0)
    }

    pub fn related_to(source: &str, target: &str, weight: f32) -> Self {
        Self::new(source, target, KgEdgeType::Derive, weight)
    }

    pub fn same_tag(source: &str, target: &str) -> Self {
        Self::new(source, target, KgEdgeType::SameTag, 1.0)
    }

    pub fn has_note(source: &str, target: &str) -> Self {
        Self::new(source, target, KgEdgeType::HasNote, 1.0)
    }

    pub fn relation_enum(&self) -> KgEdgeType {
        KgEdgeType::from_string(&self.relation)
    }
}

// ============================================================================
// SQLite Database
// ============================================================================

#[derive(Debug)]
pub struct KgDatabase {
    conn: Mutex<rusqlite::Connection>,
    path: std::path::PathBuf,
}

impl KgDatabase {
    fn lock(&self) -> MutexGuard<'_, rusqlite::Connection> {
        self.conn.lock()
    }

    /// Open or create a KG database at the given path
    pub fn new(path: std::path::PathBuf) -> Result<Self, KgError> {
        if let Some(p) = path.parent() {
            std::fs::create_dir_all(p).map_err(|e| KgError::Database(e.to_string()))?;
        }
        let conn = rusqlite::Connection::open(&path)?;
        let db = KgDatabase { conn: Mutex::new(conn), path };
        db.init_tables()?;
        Ok(db)
    }

    /// Create tables and indexes
    fn init_tables(&self) -> Result<(), KgError> {
        let guard = self.lock();
        guard.execute_batch("
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

    fn props_to_json(props: &serde_json::Value) -> String {
        serde_json::to_string(props).unwrap_or_else(|_| "{}".to_string())
    }

    fn json_to_props(json: &str) -> serde_json::Value {
        serde_json::from_str(json).unwrap_or_default()
    }

    // ── Node CRUD ──────────────────────────────────────────────────────────

    pub fn add_node(&self, node_type: &str, entity_id: &str, label: &str, properties: serde_json::Value) -> Result<String, KgError> {
        let id = Self::gen_id();
        let props_json = Self::props_to_json(&properties);
        let now = Self::now();
        self.lock().execute(
            "INSERT OR IGNORE INTO kg_nodes (id, type, entity_id, label, properties_json, created_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            rusqlite::params![id, node_type, entity_id, label, props_json, now],
        )?;
        let existing: Option<String> = self.lock().query_row(
            "SELECT id FROM kg_nodes WHERE type = ?1 AND entity_id = ?2",
            rusqlite::params![node_type, entity_id],
            |row| row.get(0),
        ).ok();
        Ok(existing.unwrap_or(id))
    }

    /// Add a node with an explicit ID. Used by KnowledgeGraph to sync IDs.
    fn add_node_with_id(&self, node_id: &str, node_type: &str, entity_id: &str, label: &str, properties: serde_json::Value) -> Result<(), KgError> {
        let props_json = Self::props_to_json(&properties);
        let now = Self::now();
        self.lock().execute(
            "INSERT OR IGNORE INTO kg_nodes (id, type, entity_id, label, properties_json, created_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            rusqlite::params![node_id, node_type, entity_id, label, props_json, now],
        )?;
        Ok(())
    }

    pub fn upsert_node(&self, node_type: &str, entity_id: &str, label: &str, properties: serde_json::Value) -> Result<String, KgError> {
        let id = Self::gen_id();
        let props_json = Self::props_to_json(&properties);
        let now = Self::now();
        self.lock().execute(
            "INSERT INTO kg_nodes (id, type, entity_id, label, properties_json, created_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6)
             ON CONFLICT(type, entity_id) DO UPDATE SET label = ?4, properties_json = ?5, created_at = ?6",
            rusqlite::params![id, node_type, entity_id, label, props_json, now],
        )?;
        let existing: Option<String> = self.lock().query_row(
            "SELECT id FROM kg_nodes WHERE type = ?1 AND entity_id = ?2",
            rusqlite::params![node_type, entity_id],
            |row| row.get(0),
        ).ok();
        Ok(existing.unwrap_or(id))
    }

    pub fn get_node(&self, node_id: &str) -> Result<Option<KgNode>, KgError> {
        let _g = self.lock();
        let mut stmt = _g.prepare("SELECT id, type, entity_id, label, properties_json FROM kg_nodes WHERE id = ?1")?;
        let mut rows = stmt.query(rusqlite::params![node_id])?;
        match rows.next()? {
            Some(row) => Ok(Some(KgNode {
                id: row.get(0)?,
                entity_id: row.get::<_, String>(2)?,
                label: row.get(3)?,
                node_type: row.get(1)?,
                properties: Self::json_to_props(&row.get::<_, String>(4)?),
            })),
            None => Ok(None),
        }
    }

    pub fn get_node_by_entity(&self, node_type: &str, entity_id: &str) -> Result<Option<KgNode>, KgError> {
        let _g = self.lock();
        let mut stmt = _g.prepare("SELECT id, type, entity_id, label, properties_json FROM kg_nodes WHERE type = ?1 AND entity_id = ?2")?;
        let mut rows = stmt.query(rusqlite::params![node_type, entity_id])?;
        match rows.next()? {
            Some(row) => Ok(Some(KgNode {
                id: row.get(0)?,
                entity_id: row.get::<_, String>(2)?,
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
        let _g = self.lock();
        let mut stmt = _g.prepare(sql)?;

        // Use the same closure type by collecting into Vec directly
        let map_fn = |row: &rusqlite::Row| -> rusqlite::Result<KgNode> {
            Ok(KgNode {
                id: row.get(0)?,
                entity_id: row.get::<_, String>(2)?,
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

    pub fn get_nodes_batch(&self, node_ids: &[String]) -> Result<HashMap<String, KgNode>, KgError> {
        if node_ids.is_empty() {
            return Ok(HashMap::new());
        }
        let placeholders: Vec<String> = node_ids.iter().map(|_| "?".to_string()).collect();
        let sql = format!(
            "SELECT id, type, entity_id, label, properties_json FROM kg_nodes WHERE id IN ({})",
            placeholders.join(",")
        );
        let _g = self.lock();
        let mut stmt = _g.prepare(&sql)?;
        let params: Vec<&dyn rusqlite::ToSql> = node_ids.iter().map(|s| s as &dyn rusqlite::ToSql).collect();
        let map_fn = |row: &rusqlite::Row| -> rusqlite::Result<KgNode> {
            Ok(KgNode {
                id: row.get(0)?,
                node_type: row.get(1)?,
                entity_id: row.get::<_, String>(2)?,
                label: row.get(3)?,
                properties: KgDatabase::json_to_props(&row.get::<_, String>(4)?),
            })
        };
        let rows: Vec<KgNode> = stmt.query_map(params.as_slice(), map_fn)?.collect::<Result<Vec<_>, _>>()?;
        Ok(rows.into_iter().map(|n| (n.id.clone(), n)).collect())
    }

    pub fn get_edges_by_nodes_batch(&self, node_ids: &[String], direction: &str, rel_type: Option<&str>) -> Result<Vec<KgEdge>, KgError> {
        if node_ids.is_empty() {
            return Ok(Vec::new());
        }
        let placeholders: Vec<String> = node_ids.iter().map(|_| "?".to_string()).collect();
        let sql = match (direction, rel_type) {
            ("out", Some(_)) =>
                format!("SELECT id, source_id, target_id, relation_type, weight, properties_json FROM kg_edges WHERE source_id IN ({}) AND relation_type = ?", placeholders.join(",")),
            ("in", Some(_)) =>
                format!("SELECT id, source_id, target_id, relation_type, weight, properties_json FROM kg_edges WHERE target_id IN ({}) AND relation_type = ?", placeholders.join(",")),
            (_, Some(_)) =>
                format!("SELECT id, source_id, target_id, relation_type, weight, properties_json FROM kg_edges WHERE (source_id IN ({}) OR target_id IN ({})) AND relation_type = ?", placeholders.join(","), placeholders.join(",")),
            ("out", None) =>
                format!("SELECT id, source_id, target_id, relation_type, weight, properties_json FROM kg_edges WHERE source_id IN ({})", placeholders.join(",")),
            ("in", None) =>
                format!("SELECT id, source_id, target_id, relation_type, weight, properties_json FROM kg_edges WHERE target_id IN ({})", placeholders.join(",")),
            (_, None) =>
                format!("SELECT id, source_id, target_id, relation_type, weight, properties_json FROM kg_edges WHERE source_id IN ({}) OR target_id IN ({})", placeholders.join(","), placeholders.join(",")),
        };
        let _g = self.lock();
        let mut stmt = _g.prepare(&sql)?;
        let map_fn = |row: &rusqlite::Row| -> rusqlite::Result<KgEdge> {
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
            Some(rt) => {
                let mut params_vec: Vec<String> = node_ids.to_vec();
                if direction != "out" && direction != "in" {
                    params_vec.push(node_ids.last().unwrap().clone());
                }
                params_vec.push(rt.to_string());
                let params: Vec<&dyn rusqlite::ToSql> = params_vec.iter().map(|s| s as &dyn rusqlite::ToSql).collect();
                stmt.query_map(params.as_slice(), map_fn)?.collect::<Result<Vec<_>, _>>()?
            }
            None => {
                let mut params_vec: Vec<String> = node_ids.to_vec();
                if direction != "out" && direction != "in" {
                    params_vec.push(node_ids.last().unwrap().clone());
                }
                let params: Vec<&dyn rusqlite::ToSql> = params_vec.iter().map(|s| s as &dyn rusqlite::ToSql).collect();
                stmt.query_map(params.as_slice(), map_fn)?.collect::<Result<Vec<_>, _>>()?
            }
        };
        Ok(rows)
    }

    // ── Edge CRUD ──────────────────────────────────────────────────────────

    pub fn add_edge(&self, source_id: &str, target_id: &str, relation_type: &str, weight: f64, properties: serde_json::Value) -> Result<String, KgError> {
        let id = format!("{}-{}-{}", source_id, target_id, Self::gen_id().chars().take(8).collect::<String>());
        let props_json = Self::props_to_json(&properties);
        self.lock().execute(
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
        let _g = self.lock();
        let mut stmt = _g.prepare(sql)?;
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
            let edges = self.get_edges_by_nodes_batch(&current_level, "both", relation_type)?;
            let mut next_level_ids = Vec::new();
            let mut discovered: HashMap<String, KgEdge> = HashMap::new();

            for edge in &edges {
                let neighbor_id = if current_level.contains(&edge.source) { &edge.target } else { &edge.source };
                if visited.insert(neighbor_id.clone()) {
                    next_level_ids.push(neighbor_id.clone());
                    discovered.insert(neighbor_id.clone(), edge.clone());
                }
            }

            if !next_level_ids.is_empty() {
                let nodes = self.get_nodes_batch(&next_level_ids)?;
                for (nid, edge) in discovered {
                    if let Some(node) = nodes.get(&nid) {
                        results.push((node.clone(), edge, d));
                    }
                }
            }
            current_level = next_level_ids;
        }

        Ok(results)
    }

    // ── Stats ──────────────────────────────────────────────────────────────

    pub fn stats(&self) -> Result<KgStats, KgError> {
        let total_nodes: i64 = self.lock().query_row("SELECT COUNT(*) FROM kg_nodes", [], |r| r.get(0))?;
        let total_edges: i64 = self.lock().query_row("SELECT COUNT(*) FROM kg_edges", [], |r| r.get(0))?;
        let paper_nodes: i64 = self.lock().query_row("SELECT COUNT(*) FROM kg_nodes WHERE type = 'paper'", [], |r| r.get(0))?;
        let concept_nodes: i64 = self.lock().query_row("SELECT COUNT(*) FROM kg_nodes WHERE type = 'concept'", [], |r| r.get(0))?;
        let avg_degree = if total_nodes > 0 { total_edges as f32 / total_nodes as f32 } else { 0.0 };
        Ok(KgStats { total_nodes: total_nodes as usize, total_edges: total_edges as usize, avg_degree, paper_nodes: paper_nodes as usize, concept_nodes: concept_nodes as usize })
    }

    // ── Full graph export ──────────────────────────────────────────────────

    pub fn export_json(&self, limit: Option<usize>) -> Result<serde_json::Value, KgError> {
        let limit = limit.unwrap_or(10000).min(100000);
        let nodes = self.get_all_nodes(None)?;
        let nodes_to_export = if nodes.len() > limit { &nodes[..limit] } else { &nodes };
        let mut edges = Vec::new();
        let _g = self.lock();
        let mut stmt = _g.prepare(
            "SELECT id, source_id, target_id, relation_type, weight, properties_json FROM kg_edges LIMIT ?1"
        )?;
        let rows = stmt.query_map([limit as i64], |row| {
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
        Ok(serde_json::json!({
            "nodes": nodes_to_export,
            "edges": edges,
            "truncated": nodes.len() > limit
        }))
    }

    /// Get database path
    pub fn path(&self) -> &std::path::Path {
        &self.path
    }

    // ── Integration Pipeline ────────────────────────────────────────────────

    /// Process a new paper: create Paper node, Tag nodes, Author nodes, and connections.
    /// Mirrors kg/integration.py::on_paper_processed()
    pub fn on_paper_processed(
        &self,
        paper_uid: &str,
        title: &str,
        authors: &[String],
        tags: &[String],
        year: &str,
    ) -> Result<String, KgError> {
        // Create Paper node
        let mut props = serde_json::json!({"title": title, "year": year});
        if let Some(obj) = props.as_object_mut() {
            obj.insert("year".into(), serde_json::json!(year));
        }
        let paper_id = self.upsert_node(KgNodeType::Paper.as_str(), paper_uid, title, props)?;

        // Create Tag nodes + same_tag edges
        for tag in tags {
            let tag_label = tag.trim();
            if tag_label.is_empty() { continue; }
            let tag_id = self.upsert_node(KgNodeType::Tag.as_str(), tag_label, tag_label, serde_json::json!({}))?;
            self.add_edge(&paper_id, &tag_id, KgEdgeType::SameTag.as_str(), 1.0, serde_json::json!({}))?;
        }

        // Create Author nodes + derive edges
        for author in authors {
            let author_id = self.add_node(KgNodeType::Author.as_str(), author, author, serde_json::json!({"name": author}))?;
            self.add_edge(&paper_id, &author_id, KgEdgeType::Derive.as_str(), 1.0, serde_json::json!({}))?;
        }

        Ok(paper_id)
    }

    /// Record citation relationships between papers.
    /// Mirrors kg/integration.py::on_citations_fetched()
    pub fn on_citations_fetched(
        &self,
        paper_uid: &str,
        cited: &[String],
        citing: &[String],
    ) -> Result<(), KgError> {
        // Get or create the paper node
        let center = match self.get_node_by_entity(KgNodeType::Paper.as_str(), paper_uid)? {
            Some(n) => n,
            None => return Err(KgError::NodeNotFound(paper_uid.to_string())),
        };

        // Add cite edges: center ← cited (center cites these)
        for cited_id in cited {
            let target = match self.get_node_by_entity(KgNodeType::Paper.as_str(), cited_id)? {
                Some(n) => n,
                None => continue,
            };
            self.add_edge(&center.id, &target.id, KgEdgeType::Cite.as_str(), 1.0, serde_json::json!({}))?;
        }

        // Add cite edges: center → citing (these cite center)
        for citing_id in citing {
            let source = match self.get_node_by_entity(KgNodeType::Paper.as_str(), citing_id)? {
                Some(n) => n,
                None => continue,
            };
            self.add_edge(&source.id, &center.id, KgEdgeType::Cite.as_str(), 1.0, serde_json::json!({}))?;
        }

        Ok(())
    }

    /// Create an M-Note node and connect papers with in_comparison edges.
    /// Mirrors kg/integration.py::on_mnote_created()
    pub fn on_mnote_created(&self, mnote_id: &str, member_paper_uids: &[String]) -> Result<String, KgError> {
        let note_id = self.add_node(KgNodeType::MNote.as_str(), mnote_id, mnote_id, serde_json::json!({"type": "m_note"}))?;
        for paper_uid in member_paper_uids {
            if let Some(paper) = self.get_node_by_entity(KgNodeType::Paper.as_str(), paper_uid)? {
                self.add_edge(&note_id, &paper.id, KgEdgeType::InComparison.as_str(), 1.0, serde_json::json!({}))?;
            }
        }
        Ok(note_id)
    }

    /// Create Figure/Table nodes and connect to paper with has_figure/has_table edges.
    /// Mirrors kg/integration.py::on_charts_indexed()
    pub fn on_charts_indexed(&self, paper_uid: &str, figure_ids: &[String], table_ids: &[String]) -> Result<(), KgError> {
        let paper = match self.get_node_by_entity(KgNodeType::Paper.as_str(), paper_uid)? {
            Some(p) => p,
            None => return Err(KgError::NodeNotFound(paper_uid.to_string())),
        };
        for fig_id in figure_ids {
            let fig_node = self.add_node(KgNodeType::Figure.as_str(), fig_id, fig_id, serde_json::json!({}))?;
            self.add_edge(&paper.id, &fig_node, KgEdgeType::HasFigure.as_str(), 1.0, serde_json::json!({}))?;
        }
        for tbl_id in table_ids {
            let tbl_node = self.add_node(KgNodeType::Table.as_str(), tbl_id, tbl_id, serde_json::json!({}))?;
            self.add_edge(&paper.id, &tbl_node, KgEdgeType::HasTable.as_str(), 1.0, serde_json::json!({}))?;
        }
        Ok(())
    }

    /// Rebuild the graph from a list of papers (upserts all + connects citations).
    pub fn rebuild_from_papers(&self, papers: &[KgNode], citations: &[(String, String)]) -> Result<KgStats, KgError> {
        for paper in papers {
            let _ = self.upsert_node(&paper.node_type, &paper.entity_id, &paper.label, paper.properties.clone());
        }
        for (source, target) in citations {
            let src = self.get_node_by_entity(KgNodeType::Paper.as_str(), source);
            let tgt = self.get_node_by_entity(KgNodeType::Paper.as_str(), target);
            if let (Ok(Some(s)), Ok(Some(t))) = (src, tgt) {
                let _ = self.add_edge(&s.id, &t.id, KgEdgeType::Cite.as_str(), 1.0, serde_json::json!({}));
            }
        }
        self.stats()
    }

    /// Query the graph by keyword against node labels.
    pub fn query_by_keyword(&self, keyword: &str, limit: usize) -> Result<Vec<KgNode>, KgError> {
        let pattern = format!("%{}%", keyword);
        let _g = self.lock();
        let mut stmt = _g.prepare(
            "SELECT id, type, entity_id, label, properties_json FROM kg_nodes WHERE label LIKE ?1 OR entity_id LIKE ?1 LIMIT ?2"
        )?;
        let rows = stmt.query_map(rusqlite::params![pattern, limit as i64], |row| {
            Ok(KgNode {
                id: row.get(0)?,
                entity_id: row.get::<_, String>(2)?,
                label: row.get(3)?,
                node_type: row.get(1)?,
                properties: KgDatabase::json_to_props(&row.get::<_, String>(4)?),
            })
        })?;
        let mut result = Vec::new();
        for row in rows {
            result.push(row?);
        }
        Ok(result)
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
        let graph = Self { db: Some(db), ..Default::default() };
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
            let _ = db.add_node_with_id(&node.id, &node.node_type, &node.entity_id, &node.label, node.properties.clone());
        }
        self.nodes.insert(node.id.clone(), node);
    }

    pub fn add_edge(&mut self, edge: KgEdge) {
        let source = edge.source.clone();
        let target = edge.target.clone();
        if !self.nodes.contains_key(&source) || !self.nodes.contains_key(&target) {
            tracing::warn!("Edge references unknown node: {} -> {}", source, target);
            return;
        }
        if let Some(ref db) = self.db {
            let _ = db.add_edge(&source, &target, &edge.relation, edge.weight as f64, edge.properties.clone());
        }
        self.edges.push(edge);
        self.outgoing.entry(source.clone()).or_default().push(target.clone());
        self.incoming.entry(target).or_default().push(source);
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
        let mut visited: HashSet<&str> = HashSet::new();
        let mut queue: VecDeque<(String, Vec<String>)> = VecDeque::new();
        queue.push_back((start.to_string(), vec![start.to_string()]));
        visited.insert(start);

        while let Some((current, path)) = queue.pop_front() {
            if current == end {
                return Some(path);
            }
            if let Some(neighbors) = self.outgoing.get(&current) {
                for neighbor in neighbors {
                    if visited.insert(neighbor.as_str()) {
                        let mut new_path = path.clone();
                        new_path.push(neighbor.clone());
                        queue.push_back((neighbor.clone(), new_path));
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

    pub fn export_json(&self, limit: Option<usize>) -> serde_json::Value {
        let limit = limit.unwrap_or(10000).min(100000);
        let nodes: Vec<_> = self.nodes.values().take(limit).collect();
        let edges: Vec<_> = self.edges.iter().take(limit).collect();
        serde_json::json!({
            "nodes": nodes,
            "edges": edges,
            "truncated": self.nodes.len() > limit || self.edges.len() > limit
        })
    }

    /// Access the kg database if connected
    pub fn database(&self) -> Option<&KgDatabase> {
        self.db.as_ref()
    }

    /// Get the ego subgraph for a paper (paper + neighbors up to depth)
    pub fn get_paper_subgraph(&self, paper_id: &str, depth: u32, include_notes: bool) -> Result<KgSubgraph, KgError> {
        let db = self.db.as_ref().ok_or_else(|| KgError::Database("No database connected".into()))?;
        let center = db.get_node_by_entity(KgNodeType::Paper.as_str(), paper_id)?
            .ok_or_else(|| KgError::NodeNotFound(paper_id.to_string()))?;
        let mut sub_nodes: HashMap<String, KgNode> = HashMap::new();
        let mut sub_edges: Vec<KgEdge> = Vec::new();
        sub_nodes.insert(center.id.clone(), center.clone());

        let neighbors = db.get_neighbors(&center.id, depth, None)?;
        for (node, edge, _) in &neighbors {
            if !include_notes {
                let nt = KgNodeType::from_string(&node.node_type);
                if matches!(nt, KgNodeType::PNote | KgNodeType::CNote | KgNodeType::MNote) {
                    continue;
                }
            }
            sub_nodes.insert(node.id.clone(), node.clone());
            sub_edges.push(edge.clone());
        }

        let mut nodes: Vec<KgNode> = sub_nodes.into_values().collect();
        nodes.sort_by(|a, b| a.id.cmp(&b.id));
        sub_edges.sort_by(|a, b| a.id.cmp(&b.id));

        Ok(KgSubgraph { nodes, edges: sub_edges, center_id: center.id })
    }

    /// Get the subgraph for a tag (all papers + notes connected by same_tag edges)
    pub fn get_tag_ecosystem(&self, tag: &str) -> Result<KgSubgraph, KgError> {
        let db = self.db.as_ref().ok_or_else(|| KgError::Database("No database connected".into()))?;
        let tag_node = db.get_node_by_entity(KgNodeType::Tag.as_str(), tag)?
            .ok_or_else(|| KgError::NodeNotFound(format!("Tag: {}", tag)))?;

        let edges = db.get_edges_by_node(&tag_node.id, "both", Some(KgEdgeType::SameTag.as_str()))?;
        let mut node_ids: HashSet<String> = HashSet::new();
        node_ids.insert(tag_node.id.clone());
        for edge in &edges {
            node_ids.insert(edge.source.clone());
            node_ids.insert(edge.target.clone());
        }

        let nodes = db.get_all_nodes(None)?;
        let sub_nodes: Vec<KgNode> = nodes.into_iter().filter(|n| node_ids.contains(&n.id)).collect();
        Ok(KgSubgraph { nodes: sub_nodes, edges, center_id: tag_node.id })
    }

    pub fn default_path() -> std::path::PathBuf {
        dirs::home_dir()
            .unwrap_or_else(|| std::path::PathBuf::from("."))
            .join(AIROS_DIR_NAME).join(KG_DIR)
    }

    pub fn graph_path() -> std::path::PathBuf {
        Self::default_path().join(KG_GRAPH_FILE)
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

/// Subgraph result (for paper subgraph / tag ecosystem queries)
#[derive(Debug, Clone, Serialize)]
pub struct KgSubgraph {
    pub nodes: Vec<KgNode>,
    pub edges: Vec<KgEdge>,
    pub center_id: String,
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
            let mut new_scores: HashMap<String, f32> = std::collections::HashMap::new();
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
                let mut label_counts: HashMap<usize, usize> = std::collections::HashMap::new();
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
        let id = db.add_node("paper", "2401.00001", "Test Paper", serde_json::json!({})).unwrap();
        let node = db.get_node(&id).unwrap().expect("node should exist after add_node");
        assert_eq!(node.label, "Test Paper");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_db_get_node_by_entity() {
        let (db, dir) = test_db();
        db.add_node("paper", "2401.00001", "Test Paper", serde_json::json!({})).unwrap();
        let node = db.get_node_by_entity("paper", "2401.00001").unwrap().expect("paper node should exist");
        assert_eq!(node.label, "Test Paper");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_db_add_edge() {
        let (db, dir) = test_db();
        let n1 = db.add_node("paper", "2401.00001", "Paper A", serde_json::json!({})).unwrap();
        let n2 = db.add_node("paper", "2401.00002", "Paper B", serde_json::json!({})).unwrap();
        db.add_edge(&n1, &n2, "cites", 1.0, serde_json::json!({})).unwrap();
        let edges = db.get_edges_by_node(&n1, "out", None).unwrap();
        assert_eq!(edges.len(), 1);
        assert_eq!(edges[0].relation, "cites");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_db_upsert_node() {
        let (db, dir) = test_db();
        let props = serde_json::json!({"key": "value"});
        let id1 = db.upsert_node("paper", "2401.00001", "Original", props.clone()).unwrap();
        // Upsert same entity — should update, not create new
        let id2 = db.upsert_node("paper", "2401.00001", "Updated", props).unwrap();
        assert_eq!(id1, id2, "upsert should return same ID");
        let node = db.get_node(&id1).unwrap().expect("node should exist after upsert");
        assert_eq!(node.label, "Updated", "label should be updated");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_db_get_neighbors() {
        let (db, dir) = test_db();
        let n1 = db.add_node("paper", "2401.00001", "Paper A", serde_json::json!({})).unwrap();
        let n2 = db.add_node("paper", "2401.00002", "Paper B", serde_json::json!({})).unwrap();
        let n3 = db.add_node("paper", "2401.00003", "Paper C", serde_json::json!({})).unwrap();
        db.add_edge(&n1, &n2, "cites", 1.0, serde_json::json!({})).unwrap();
        db.add_edge(&n2, &n3, "cites", 1.0, serde_json::json!({})).unwrap();

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
        db.add_node("paper", "1", "Paper 1", serde_json::json!({})).unwrap();
        db.add_node("paper", "2", "Paper 2", serde_json::json!({})).unwrap();
        db.add_node("concept", "ml", "Machine Learning", serde_json::json!({})).unwrap();
        let stats = db.stats().unwrap();
        assert_eq!(stats.total_nodes, 3);
        assert_eq!(stats.paper_nodes, 2);
        assert_eq!(stats.concept_nodes, 1);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_db_export_json() {
        let (db, dir) = test_db();
        db.add_node("paper", "1", "Paper", serde_json::json!({})).unwrap();
        let json = db.export_json(None).unwrap();
        assert!(!json["nodes"].as_array().unwrap().is_empty());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_knowledge_graph_with_db() {
        use std::sync::atomic::{AtomicU64, Ordering};
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        let unique = COUNTER.fetch_add(1, Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!("rairos_kg_int_{}_{}", std::process::id(), unique));
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

    #[test]
    fn test_node_type_enum() {
        assert_eq!(KgNodeType::Paper.as_str(), "paper");
        assert_eq!(KgNodeType::Tag.as_str(), "tag");
        assert_eq!(KgNodeType::Author.as_str(), "author");
        assert_eq!(KgNodeType::from_string("paper"), KgNodeType::Paper);
        assert_eq!(KgNodeType::from_string("TAG"), KgNodeType::Tag);
        assert_eq!(KgNodeType::from_string("p_note"), KgNodeType::PNote);
    }

    #[test]
    fn test_edge_type_enum() {
        assert_eq!(KgEdgeType::Cite.as_str(), "cite");
        assert_eq!(KgEdgeType::SameTag.as_str(), "same_tag");
        assert_eq!(KgEdgeType::from_string("cite"), KgEdgeType::Cite);
        assert_eq!(KgEdgeType::from_string("same_tag"), KgEdgeType::SameTag);
    }

    #[test]
    fn test_constructor_helpers() {
        let tag = KgNode::new_tag("test_tag", "Test Tag");
        assert_eq!(tag.node_type, "tag");
        assert_eq!(tag.label, "Test Tag");

        let author = KgNode::new_author("auth1", "John Doe");
        assert_eq!(author.node_type, "author");

        let note = KgNode::new_note("note1", KgNodeType::PNote, "A note");
        assert_eq!(note.node_type, "p_note");
    }

    #[test]
    fn test_edge_constructors() {
        let e = KgEdge::cites("a", "b");
        assert_eq!(e.relation, "cite");
        assert_eq!(e.source, "a");

        let e2 = KgEdge::same_tag("a", "b");
        assert_eq!(e2.relation, "same_tag");

        let e3 = KgEdge::has_note("a", "b");
        assert_eq!(e3.relation, "has_note");
    }

    #[test]
    fn test_get_paper_subgraph() {
        use std::sync::atomic::{AtomicU64, Ordering};
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        let unique = COUNTER.fetch_add(1, Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!("rairos_kg_sub_{}_{}", std::process::id(), unique));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let db_path = dir.join("subgraph_test.db");

        let mut graph = KnowledgeGraph::with_db(db_path.clone()).unwrap();
        let p1 = Paper::new(Some("sub1".into()), "Paper 1".into(), "Abstract 1".into());
        let p2 = Paper::new(Some("sub2".into()), "Paper 2".into(), "Abstract 2".into());
        graph.add_paper(&p1);
        graph.add_paper(&p2);
        graph.add_citation(&p2.id, &p1.id);

        let subgraph = graph.get_paper_subgraph("sub1", 1, false).unwrap();
        assert!(subgraph.nodes.iter().any(|n| n.label == "Paper 1"), "subgraph should contain center");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_on_paper_processed() {
        use std::sync::atomic::{AtomicU64, Ordering};
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        let unique = COUNTER.fetch_add(1, Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!("rairos_kg_int_{}_{}", std::process::id(), unique));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let db_path = dir.join("integration_test.db");
        let db = KgDatabase::new(db_path).unwrap();

        let authors = vec!["John Doe".to_string(), "Jane Smith".to_string()];
        let tags = vec!["transformer".to_string(), "attention".to_string()];
        let _paper_id = db.on_paper_processed("2401.00001", "Test Paper", &authors, &tags, "2024").unwrap();

        // Verify paper node exists
        let paper = db.get_node_by_entity("paper", "2401.00001").unwrap().expect("paper node should exist after on_paper_processed");
        assert_eq!(paper.label, "Test Paper");

        // Verify tags exist
        let tag = db.get_node_by_entity("tag", "transformer").unwrap().expect("tag node should exist");
        assert_eq!(tag.label, "transformer");

        // Verify edges exist (paper → tag via same_tag)
        let edges = db.get_edges_by_node(&paper.id, "out", Some("same_tag")).unwrap();
        assert_eq!(edges.len(), 2, "should have 2 same_tag edges (one per tag)");

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_on_citations_fetched() {
        use std::sync::atomic::{AtomicU64, Ordering};
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        let unique = COUNTER.fetch_add(1, Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!("rairos_kg_cit_{}_{}", std::process::id(), unique));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let db_path = dir.join("citation_test.db");
        let db = KgDatabase::new(db_path).unwrap();

        // Setup: create 3 paper nodes
        db.on_paper_processed("center", "Center Paper", &[], &[], "2024").unwrap();
        db.on_paper_processed("cited1", "Cited Paper 1", &[], &[], "2023").unwrap();
        db.on_paper_processed("cited2", "Cited Paper 2", &[], &[], "2023").unwrap();

        // Record citations
        let cited = vec!["cited1".to_string(), "cited2".to_string()];
        db.on_citations_fetched("center", &cited, &[]).unwrap();

        // Verify edges
        let center = db.get_node_by_entity("paper", "center").unwrap().expect("center node should exist");
        let edges = db.get_edges_by_node(&center.id, "out", Some("cite")).unwrap();
        assert_eq!(edges.len(), 2, "center should cite 2 papers");

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_query_by_keyword() {
        use std::sync::atomic::{AtomicU64, Ordering};
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        let unique = COUNTER.fetch_add(1, Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!("rairos_kg_q_{}_{}", std::process::id(), unique));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let db_path = dir.join("query_test.db");
        let db = KgDatabase::new(db_path).unwrap();

        db.on_paper_processed("2401.00001", "Transformer Attention", &[], &["transformer".into(), "attention".into()], "2024").unwrap();
        db.on_paper_processed("2401.00002", "CNN Vision", &[], &["vision".into()], "2023").unwrap();

        let results = db.query_by_keyword("transformer", 10).unwrap();
        assert!(!results.is_empty(), "should find transformer in at least one node");
        assert_eq!(results[0].entity_id, "2401.00001");

        let results = db.query_by_keyword("attention", 10).unwrap();
        assert!(!results.is_empty(), "should match attention keyword");

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_on_mnote_created() {
        use std::sync::atomic::{AtomicU64, Ordering};
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        let unique = COUNTER.fetch_add(1, Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!("rairos_kg_mn_{}_{}", std::process::id(), unique));
        let _ = std::fs::remove_dir_all(&dir); std::fs::create_dir_all(&dir).unwrap();
        let db = KgDatabase::new(dir.join("mnote_test.db")).unwrap();
        db.on_paper_processed("paper1", "Paper 1", &[], &[], "2024").unwrap();
        db.on_paper_processed("paper2", "Paper 2", &[], &[], "2024").unwrap();
        let members = vec!["paper1".to_string(), "paper2".to_string()];
        db.on_mnote_created("mnote_001", &members).unwrap();
        let mnote = db.get_node_by_entity("m_note", "mnote_001").unwrap().expect("mnote node should exist");
        assert_eq!(mnote.node_type, "m_note");
        let edges = db.get_edges_by_node(&mnote.id, "out", Some("in_comparison")).unwrap();
        assert_eq!(edges.len(), 2);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_on_charts_indexed() {
        use std::sync::atomic::{AtomicU64, Ordering};
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        let unique = COUNTER.fetch_add(1, Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!("rairos_kg_ch_{}_{}", std::process::id(), unique));
        let _ = std::fs::remove_dir_all(&dir); std::fs::create_dir_all(&dir).unwrap();
        let db = KgDatabase::new(dir.join("charts_test.db")).unwrap();
        db.on_paper_processed("demo", "Demo Paper", &[], &[], "2024").unwrap();
        let figs = vec!["fig1".to_string(), "fig2".to_string()];
        let tbls = vec!["tbl1".to_string()];
        db.on_charts_indexed("demo", &figs, &tbls).unwrap();
        let fig_node = db.get_node_by_entity("figure", "fig1").unwrap().expect("fig node should exist");
        assert_eq!(fig_node.node_type, "figure");
        let tbl_node = db.get_node_by_entity("table", "tbl1").unwrap().expect("tbl node should exist");
        assert_eq!(tbl_node.node_type, "table");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_rebuild_from_papers() {
        use std::sync::atomic::{AtomicU64, Ordering};
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        let unique = COUNTER.fetch_add(1, Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!("rairos_kg_rb_{}_{}", std::process::id(), unique));
        let _ = std::fs::remove_dir_all(&dir); std::fs::create_dir_all(&dir).unwrap();
        let db = KgDatabase::new(dir.join("rebuild_test.db")).unwrap();
        db.on_paper_processed("p1", "Paper 1", &[], &[], "2024").unwrap();
        let stats = db.stats().unwrap();
        assert!(stats.total_nodes >= 1);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
