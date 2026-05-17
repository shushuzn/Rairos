//! Knowledge graph data structures and operations

use rusqlite::{params, Connection, Result as SqlResult};
use serde::{Deserialize, Serialize};
use std::path::Path;
use parking_lot::Mutex;

/// A code symbol (function, struct, enum, etc.)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Node {
    pub id: i64,
    pub name: String,
    pub kind: String,
    pub file: String,
    pub line: u32,
    pub col: u32,
    pub end_line: u32,
    pub end_col: u32,
    #[serde(default)]
    pub docstring: Option<String>,
}

/// An edge between symbols (calls, imports, extends, etc.)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Edge {
    pub id: i64,
    pub from_node: i64,
    pub to_node: i64,
    pub edge_type: String,
}

/// Full text search result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub node: Node,
    pub snippet: String,
}

/// Call graph result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CallResult {
    pub node: Node,
    pub depth: usize,
}

/// The knowledge graph
pub struct CodeGraph {
    db: Mutex<Connection>,
}

impl CodeGraph {
    /// Open or create a codegraph database
    pub fn open(path: &Path) -> SqlResult<Self> {
        let conn = Connection::open(path)?;
        let graph = Self {
            db: Mutex::new(conn),
        };
        graph.init_schema()?;
        Ok(graph)
    }

    /// Initialize the database schema
    fn init_schema(&self) -> SqlResult<()> {
        let conn = self.db.lock();
        conn.execute_batch(r#"
            CREATE TABLE IF NOT EXISTS nodes (
                id INTEGER PRIMARY KEY,
                name TEXT NOT NULL,
                kind TEXT NOT NULL,
                file TEXT NOT NULL,
                line INTEGER NOT NULL,
                col INTEGER NOT NULL,
                end_line INTEGER NOT NULL,
                end_col INTEGER NOT NULL,
                docstring TEXT
            );
            CREATE INDEX IF NOT EXISTS idx_nodes_name ON nodes(name);
            CREATE INDEX IF NOT EXISTS idx_nodes_kind ON nodes(kind);
            CREATE INDEX IF NOT EXISTS idx_nodes_file ON nodes(file);
            
            CREATE TABLE IF NOT EXISTS edges (
                id INTEGER PRIMARY KEY,
                from_node INTEGER NOT NULL,
                to_node INTEGER NOT NULL,
                edge_type TEXT NOT NULL,
                FOREIGN KEY (from_node) REFERENCES nodes(id),
                FOREIGN KEY (to_node) REFERENCES nodes(id)
            );
            CREATE INDEX IF NOT EXISTS idx_edges_from ON edges(from_node);
            CREATE INDEX IF NOT EXISTS idx_edges_to ON edges(to_node);
            
            CREATE VIRTUAL TABLE IF NOT EXISTS nodes_fts USING fts5(
                name, docstring, content='nodes', content_rowid='id'
            );
        "#)?;
        Ok(())
    }

    /// Clear all data
    pub fn clear(&self) -> SqlResult<()> {
        let conn = self.db.lock();
        conn.execute("DELETE FROM edges", [])?;
        conn.execute("DELETE FROM nodes", [])?;
        conn.execute("INSERT INTO nodes_fts(nodes_fts) VALUES('rebuild')", [])?;
        Ok(())
    }

    /// Add a node
    pub fn add_node(&self, node: &Node) -> SqlResult<i64> {
        let conn = self.db.lock();
        conn.execute(
            "INSERT INTO nodes (name, kind, file, line, col, end_line, end_col, docstring) 
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                node.name,
                node.kind,
                node.file,
                node.line as i64,
                node.col as i64,
                node.end_line as i64,
                node.end_col as i64,
                node.docstring
            ],
        )?;
        Ok(conn.last_insert_rowid())
    }

    /// Add an edge
    pub fn add_edge(&self, from_node: i64, to_node: i64, edge_type: &str) -> SqlResult<i64> {
        let conn = self.db.lock();
        conn.execute(
            "INSERT INTO edges (from_node, to_node, edge_type) VALUES (?1, ?2, ?3)",
            params![from_node, to_node, edge_type],
        )?;
        Ok(conn.last_insert_rowid())
    }

    /// Search nodes by name
    pub fn search(&self, query: &str, limit: usize) -> SqlResult<Vec<SearchResult>> {
        let conn = self.db.lock();
        let mut stmt = conn.prepare(
            "SELECT n.id, n.name, n.kind, n.file, n.line, n.col, n.end_line, n.end_col, n.docstring,
                    snippet(nodes_fts, 0, '<mark>', '</mark>', '...', 32) as snippet
             FROM nodes n
             JOIN nodes_fts ON n.id = nodes_fts.rowid
             WHERE nodes_fts MATCH ?1
             ORDER BY rank
             LIMIT ?2"
        )?;
        
        let rows = stmt.query_map(params![query, limit as i64], |row| {
            Ok(SearchResult {
                node: Node {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    kind: row.get(2)?,
                    file: row.get(3)?,
                    line: row.get::<_, i64>(4)? as u32,
                    col: row.get::<_, i64>(5)? as u32,
                    end_line: row.get::<_, i64>(6)? as u32,
                    end_col: row.get::<_, i64>(7)? as u32,
                    docstring: row.get(8)?,
                },
                snippet: row.get(9)?,
            })
        })?;
        
        rows.collect()
    }

    /// Get callers of a node
    pub fn get_callers(&self, node_id: i64, depth: usize) -> SqlResult<Vec<CallResult>> {
        let conn = self.db.lock();
        let mut results = Vec::new();
        
        let mut visited = std::collections::HashSet::new();
        let mut current_ids = vec![node_id];
        
        for d in 0..depth {
            let mut next_ids = Vec::new();
            for &id in &current_ids {
                if visited.contains(&id) {
                    continue;
                }
                visited.insert(id);
                
                let mut stmt = conn.prepare(
                    "SELECT n.id, n.name, n.kind, n.file, n.line, n.col, n.end_line, n.end_col, n.docstring
                     FROM nodes n
                     JOIN edges e ON n.id = e.from_node
                     WHERE e.to_node = ?1 AND e.edge_type = 'calls'"
                )?;
                
                let rows = stmt.query_map(params![id], |row| {
                    Ok(Node {
                        id: row.get(0)?,
                        name: row.get(1)?,
                        kind: row.get(2)?,
                        file: row.get(3)?,
                        line: row.get::<_, i64>(4)? as u32,
                        col: row.get::<_, i64>(5)? as u32,
                        end_line: row.get::<_, i64>(6)? as u32,
                        end_col: row.get::<_, i64>(7)? as u32,
                        docstring: row.get(8)?,
                    })
                })?;
                
                for node in rows.flatten() {
                    let node_id = node.id;
                    results.push(CallResult { node, depth: d + 1 });
                    next_ids.push(node_id);
                }
            }
            current_ids = next_ids;
        }
        
        Ok(results)
    }

    /// Get callees of a node
    pub fn get_callees(&self, node_id: i64, depth: usize) -> SqlResult<Vec<CallResult>> {
        let conn = self.db.lock();
        let mut results = Vec::new();
        
        let mut visited = std::collections::HashSet::new();
        let mut current_ids = vec![node_id];
        
        for d in 0..depth {
            let mut next_ids = Vec::new();
            for &id in &current_ids {
                if visited.contains(&id) {
                    continue;
                }
                visited.insert(id);
                
                let mut stmt = conn.prepare(
                    "SELECT n.id, n.name, n.kind, n.file, n.line, n.col, n.end_line, n.end_col, n.docstring
                     FROM nodes n
                     JOIN edges e ON n.id = e.to_node
                     WHERE e.from_node = ?1 AND e.edge_type = 'calls'"
                )?;
                
                let rows = stmt.query_map(params![id], |row| {
                    Ok(Node {
                        id: row.get(0)?,
                        name: row.get(1)?,
                        kind: row.get(2)?,
                        file: row.get(3)?,
                        line: row.get::<_, i64>(4)? as u32,
                        col: row.get::<_, i64>(5)? as u32,
                        end_line: row.get::<_, i64>(6)? as u32,
                        end_col: row.get::<_, i64>(7)? as u32,
                        docstring: row.get(8)?,
                    })
                })?;
                
                for node in rows.flatten() {
                    let node_id = node.id;
                    results.push(CallResult { node, depth: d + 1 });
                    next_ids.push(node_id);
                }
            }
            current_ids = next_ids;
        }
        
        Ok(results)
    }

    /// Get a node by ID
    pub fn get_node(&self, node_id: i64) -> SqlResult<Option<Node>> {
        let conn = self.db.lock();
        let mut stmt = conn.prepare(
            "SELECT id, name, kind, file, line, col, end_line, end_col, docstring
             FROM nodes WHERE id = ?1"
        )?;
        
        let mut rows = stmt.query(params![node_id])?;
        
        if let Some(row) = rows.next()? {
            Ok(Some(Node {
                id: row.get(0)?,
                name: row.get(1)?,
                kind: row.get(2)?,
                file: row.get(3)?,
                line: row.get::<_, i64>(4)? as u32,
                col: row.get::<_, i64>(5)? as u32,
                end_line: row.get::<_, i64>(6)? as u32,
                end_col: row.get::<_, i64>(7)? as u32,
                docstring: row.get(8)?,
            }))
        } else {
            Ok(None)
        }
    }

    /// Get statistics
    pub fn stats(&self) -> SqlResult<GraphStats> {
        let conn = self.db.lock();
        let node_count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM nodes", [], |row| row.get(0)
        )?;
        let edge_count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM edges", [], |row| row.get(0)
        )?;
        let file_count: i64 = conn.query_row(
            "SELECT COUNT(DISTINCT file) FROM nodes", [], |row| row.get(0)
        )?;
        
        Ok(GraphStats {
            nodes: node_count as usize,
            edges: edge_count as usize,
            files: file_count as usize,
        })
    }

    /// List all indexed files
    pub fn files(&self) -> SqlResult<Vec<String>> {
        let conn = self.db.lock();
        let mut stmt = conn.prepare("SELECT DISTINCT file FROM nodes ORDER BY file")?;
        let rows = stmt.query_map([], |row| row.get(0))?;
        rows.collect()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphStats {
    pub nodes: usize,
    pub edges: usize,
    pub files: usize,
}
