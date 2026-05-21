//! Knowledge graph data structures and operations

use sqlx::sqlite::{SqlitePool, SqlitePoolOptions, SqliteRow};
use sqlx::Row;
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
    pool: Mutex<SqlitePool>,
}

impl CodeGraph {
    /// Open or create a codegraph database
    pub async fn open(path: &Path) -> Result<Self, sqlx::Error> {
        let db_url = format!("sqlite:{}?mode=rwc", path.display());
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect(&db_url)
            .await?;

        let graph = Self {
            pool: Mutex::new(pool),
        };
        graph.init_schema().await?;
        Ok(graph)
    }

    /// Initialize the database schema
    async fn init_schema(&self) -> Result<(), sqlx::Error> {
        let pool = self.pool.lock();
        let mut conn = pool.acquire().await?;

        sqlx::query(
            r#"
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
            )
            "#,
        )
        .execute(&mut *conn)
        .await?;

        sqlx::query("CREATE INDEX IF NOT EXISTS idx_nodes_name ON nodes(name)")
            .execute(&mut *conn)
            .await?;
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_nodes_kind ON nodes(kind)")
            .execute(&mut *conn)
            .await?;
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_nodes_file ON nodes(file)")
            .execute(&mut *conn)
            .await?;

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS edges (
                id INTEGER PRIMARY KEY,
                from_node INTEGER NOT NULL,
                to_node INTEGER NOT NULL,
                edge_type TEXT NOT NULL,
                FOREIGN KEY (from_node) REFERENCES nodes(id),
                FOREIGN KEY (to_node) REFERENCES nodes(id)
            )
            "#,
        )
        .execute(&mut *conn)
        .await?;

        sqlx::query("CREATE INDEX IF NOT EXISTS idx_edges_from ON edges(from_node)")
            .execute(&mut *conn)
            .await?;
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_edges_to ON edges(to_node)")
            .execute(&mut *conn)
            .await?;

        sqlx::query(
            "CREATE VIRTUAL TABLE IF NOT EXISTS nodes_fts USING fts5(
                name, docstring, content='nodes', content_rowid='id'
            )",
        )
        .execute(&mut *conn)
        .await?;

        Ok(())
    }

    /// Clear all data
    pub async fn clear(&self) -> Result<(), sqlx::Error> {
        let pool = self.pool.lock();
        let mut conn = pool.acquire().await?;

        sqlx::query("DELETE FROM edges").execute(&mut *conn).await?;
        sqlx::query("DELETE FROM nodes").execute(&mut *conn).await?;
        sqlx::query("INSERT INTO nodes_fts(nodes_fts) VALUES('rebuild')")
            .execute(&mut *conn)
            .await?;
        Ok(())
    }

    /// Add a node
    pub async fn add_node(&self, node: &Node) -> Result<i64, sqlx::Error> {
        let pool = self.pool.lock();
        let mut conn = pool.acquire().await?;

        let _result = sqlx::query(
            "INSERT INTO nodes (name, kind, file, line, col, end_line, end_col, docstring)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        )
        .bind(&node.name)
        .bind(&node.kind)
        .bind(&node.file)
        .bind(node.line as i64)
        .bind(node.col as i64)
        .bind(node.end_line as i64)
        .bind(node.end_col as i64)
        .bind(&node.docstring)
        .execute(&mut *conn)
        .await?;

        // For SQLite, we need to get last_insert_rowid separately
        let id: (i64,) = sqlx::query_as("SELECT last_insert_rowid()")
            .fetch_one(&mut *conn)
            .await?;

        Ok(id.0)
    }

    /// Add an edge
    pub async fn add_edge(
        &self,
        from_node: i64,
        to_node: i64,
        edge_type: &str,
    ) -> Result<i64, sqlx::Error> {
        let pool = self.pool.lock();
        let mut conn = pool.acquire().await?;

        let _result = sqlx::query(
            "INSERT INTO edges (from_node, to_node, edge_type) VALUES (?1, ?2, ?3)",
        )
        .bind(from_node)
        .bind(to_node)
        .bind(edge_type)
        .execute(&mut *conn)
        .await?;

        let id: (i64,) = sqlx::query_as("SELECT last_insert_rowid()")
            .fetch_one(&mut *conn)
            .await?;

        Ok(id.0)
    }

    /// Search nodes by name
    pub async fn search(&self, query: &str, limit: usize) -> Result<Vec<SearchResult>, sqlx::Error> {
        let pool = self.pool.lock();
        let mut conn = pool.acquire().await?;

        let rows: Vec<SqliteRow> = sqlx::query(
            "SELECT n.id, n.name, n.kind, n.file, n.line, n.col, n.end_line, n.end_col, n.docstring,
                    snippet(nodes_fts, 0, '<mark>', '</mark>', '...', 32) as snippet
             FROM nodes n
             JOIN nodes_fts ON n.id = nodes_fts.rowid
             WHERE nodes_fts MATCH ?1
             ORDER BY rank
             LIMIT ?2",
        )
        .bind(query)
        .bind(limit as i64)
        .fetch_all(&mut *conn)
        .await?;

        let results = rows
            .iter()
            .map(|row| {
                Ok(SearchResult {
                    node: Node {
                        id: row.get(0),
                        name: row.get(1),
                        kind: row.get(2),
                        file: row.get(3),
                        line: row.try_get::<i64, _>(4)? as u32,
                        col: row.try_get::<i64, _>(5)? as u32,
                        end_line: row.try_get::<i64, _>(6)? as u32,
                        end_col: row.try_get::<i64, _>(7)? as u32,
                        docstring: row.get::<Option<String>, _>(8),
                    },
                    snippet: row.get::<String, _>(9),
                })
            })
            .collect::<Result<Vec<_>, sqlx::Error>>()?;

        Ok(results)
    }

    /// Get callers of a node
    pub async fn get_callers(&self, node_id: i64, depth: usize) -> Result<Vec<CallResult>, sqlx::Error> {
        let pool = self.pool.lock();
        let mut conn = pool.acquire().await?;

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

                let rows: Vec<SqliteRow> = sqlx::query(
                    "SELECT n.id, n.name, n.kind, n.file, n.line, n.col, n.end_line, n.end_col, n.docstring
                     FROM nodes n
                     JOIN edges e ON n.id = e.from_node
                     WHERE e.to_node = ?1 AND e.edge_type = 'calls'",
                )
                .bind(id)
                .fetch_all(&mut *conn)
                .await?;

                for row in rows {
                    let node = Node {
                        id: row.get(0),
                        name: row.get(1),
                        kind: row.get(2),
                        file: row.get(3),
                        line: row.try_get::<i64, _>(4)? as u32,
                        col: row.try_get::<i64, _>(5)? as u32,
                        end_line: row.try_get::<i64, _>(6)? as u32,
                        end_col: row.try_get::<i64, _>(7)? as u32,
                        docstring: row.get::<Option<String>, _>(8),
                    };
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
    pub async fn get_callees(&self, node_id: i64, depth: usize) -> Result<Vec<CallResult>, sqlx::Error> {
        let pool = self.pool.lock();
        let mut conn = pool.acquire().await?;

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

                let rows: Vec<SqliteRow> = sqlx::query(
                    "SELECT n.id, n.name, n.kind, n.file, n.line, n.col, n.end_line, n.end_col, n.docstring
                     FROM nodes n
                     JOIN edges e ON n.id = e.to_node
                     WHERE e.from_node = ?1 AND e.edge_type = 'calls'",
                )
                .bind(id)
                .fetch_all(&mut *conn)
                .await?;

                for row in rows {
                    let node = Node {
                        id: row.get(0),
                        name: row.get(1),
                        kind: row.get(2),
                        file: row.get(3),
                        line: row.try_get::<i64, _>(4)? as u32,
                        col: row.try_get::<i64, _>(5)? as u32,
                        end_line: row.try_get::<i64, _>(6)? as u32,
                        end_col: row.try_get::<i64, _>(7)? as u32,
                        docstring: row.get::<Option<String>, _>(8),
                    };
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
    pub async fn get_node(&self, node_id: i64) -> Result<Option<Node>, sqlx::Error> {
        let pool = self.pool.lock();
        let mut conn = pool.acquire().await?;

        let row = sqlx::query(
            "SELECT id, name, kind, file, line, col, end_line, end_col, docstring
             FROM nodes WHERE id = ?1",
        )
        .bind(node_id)
        .fetch_optional(&mut *conn)
        .await?;

        Ok(row.map(|row| Node {
            id: row.get(0),
            name: row.get(1),
            kind: row.get(2),
            file: row.get(3),
            line: row.get::<i64, _>(4) as u32,
            col: row.get::<i64, _>(5) as u32,
            end_line: row.get::<i64, _>(6) as u32,
            end_col: row.get::<i64, _>(7) as u32,
            docstring: row.get::<Option<String>, _>(8),
        }))
    }

    /// Get statistics
    pub async fn stats(&self) -> Result<GraphStats, sqlx::Error> {
        let pool = self.pool.lock();
        let mut conn = pool.acquire().await?;

        let node_count: i64 = sqlx::query("SELECT COUNT(*) FROM nodes")
            .fetch_one(&mut *conn)
            .await?
            .get(0);

        let edge_count: i64 = sqlx::query("SELECT COUNT(*) FROM edges")
            .fetch_one(&mut *conn)
            .await?
            .get(0);

        let file_count: i64 = sqlx::query("SELECT COUNT(DISTINCT file) FROM nodes")
            .fetch_one(&mut *conn)
            .await?
            .get(0);

        Ok(GraphStats {
            nodes: node_count as usize,
            edges: edge_count as usize,
            files: file_count as usize,
        })
    }

    /// List all indexed files
    pub async fn files(&self) -> Result<Vec<String>, sqlx::Error> {
        let pool = self.pool.lock();
        let mut conn = pool.acquire().await?;

        let rows: Vec<SqliteRow> = sqlx::query("SELECT DISTINCT file FROM nodes ORDER BY file")
            .fetch_all(&mut *conn)
            .await?;

        let files = rows.iter().map(|row| row.get(0)).collect();

        Ok(files)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphStats {
    pub nodes: usize,
    pub edges: usize,
    pub files: usize,
}
