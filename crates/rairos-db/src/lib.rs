//! rairos-db — SQLite database layer for Rairos
//!
//! Schema: papers, parse_history, tags, paper_tags, job_queue, settings,
//! paper_cache, dedup_log, citations, experiment_tables, paper_code_trace,
//! gap_history, arxiv_search_cache.
//!
//! Thread-local connection management (one connection per thread).

use chrono::Utc;
use regex::Regex;
use rusqlite::{Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use thiserror::Error;

// ============================================================================
// Error Types
// ============================================================================

#[derive(Error, Debug)]
pub enum DbError {
    #[error("Database error: {0}")]
    Database(#[from] rusqlite::Error),
    #[error("Paper not found: {0}")]
    NotFound(String),
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

pub type Result<T> = std::result::Result<T, DbError>;

// ============================================================================
// Paper
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Paper {
    // Primary key
    pub id: String,
    // Core fields
    pub source: String,
    pub title: String,
    pub authors: Vec<String>,
    pub abstract_text: String,
    pub published: String,
    pub updated: String,
    pub abs_url: String,
    pub pdf_url: String,
    pub primary_category: String,
    pub journal: String,
    pub volume: String,
    pub issue: String,
    pub page: String,
    pub doi: String,
    pub categories: String,
    pub reference_count: i64,
    // Timestamps
    pub added_at: String,
    pub updated_at: String,
    // PDF info
    pub pdf_path: String,
    pub pdf_hash: String,
    // Parse status
    pub parse_status: String,
    pub parse_error: String,
    pub parse_version: i64,
    // Parsed content
    pub plain_text: String,
    pub latex_blocks: Vec<serde_json::Value>,
    pub table_count: i64,
    pub figure_count: i64,
    pub word_count: i64,
    pub page_count: i64,
    // Note paths
    pub pnote_path: String,
    pub cnote_path: String,
    pub mnote_path: String,
    // Embedding
    #[serde(skip_serializing_if = "Option::is_none")]
    pub embed_vector: Option<Vec<f32>>,
}

impl Default for Paper {
    fn default() -> Self {
        Self {
            id: String::new(),
            source: String::new(),
            title: String::new(),
            authors: Vec::new(),
            abstract_text: String::new(),
            published: String::new(),
            updated: String::new(),
            abs_url: String::new(),
            pdf_url: String::new(),
            primary_category: String::new(),
            journal: String::new(),
            volume: String::new(),
            issue: String::new(),
            page: String::new(),
            doi: String::new(),
            categories: String::new(),
            reference_count: 0,
            added_at: String::new(),
            updated_at: String::new(),
            pdf_path: String::new(),
            pdf_hash: String::new(),
            parse_status: "pending".to_string(),
            parse_error: String::new(),
            parse_version: 0,
            plain_text: String::new(),
            latex_blocks: Vec::new(),
            table_count: 0,
            figure_count: 0,
            word_count: 0,
            page_count: 0,
            pnote_path: String::new(),
            cnote_path: String::new(),
            mnote_path: String::new(),
            embed_vector: None,
        }
    }
}

impl Paper {
    pub fn from_row(row: &rusqlite::Row) -> rusqlite::Result<Self> {
        let authors_raw: String = row.get("authors")?;
        let latex_raw: String = row.get("latex_blocks")?;
        let embed_vec: Option<Vec<u8>> = row.get("embed_vector")?;

        let authors: Vec<String> = serde_json::from_str(&authors_raw).unwrap_or_default();
        let latex_blocks: Vec<serde_json::Value> =
            serde_json::from_str(&latex_raw).unwrap_or_default();
        let embed_vector = embed_vec.map(|bytes| {
            bytes
                .chunks_exact(4)
                .map(|chunk| f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]))
                .collect()
        });

        Ok(Self {
            id: row.get("id")?,
            source: row.get("source")?,
            title: row.get("title")?,
            authors,
            abstract_text: row.get("abstract")?,
            published: row.get("published")?,
            updated: row.get("updated")?,
            abs_url: row.get("abs_url")?,
            pdf_url: row.get("pdf_url")?,
            primary_category: row.get("primary_category")?,
            journal: row.get("journal")?,
            volume: row.get("volume")?,
            issue: row.get("issue")?,
            page: row.get("page")?,
            doi: row.get("doi")?,
            categories: row.get("categories")?,
            reference_count: row.get("reference_count")?,
            added_at: row.get("added_at")?,
            updated_at: row.get("updated_at")?,
            pdf_path: row.get("pdf_path")?,
            pdf_hash: row.get("pdf_hash")?,
            parse_status: row.get("parse_status")?,
            parse_error: row.get("parse_error")?,
            parse_version: row.get("parse_version")?,
            plain_text: row.get("plain_text")?,
            latex_blocks,
            table_count: row.get("table_count")?,
            figure_count: row.get("figure_count")?,
            word_count: row.get("word_count")?,
            page_count: row.get("page_count")?,
            pnote_path: row.get("pnote_path")?,
            cnote_path: row.get("cnote_path")?,
            mnote_path: row.get("mnote_path")?,
            embed_vector,
        })
    }
}

// ============================================================================
// Search Result
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub paper_id: String,
    pub title: String,
    pub authors: Vec<String>,
    pub published: String,
    pub primary_category: String,
    pub score: f64,
    pub snippet: String,
    pub parse_status: String,
    pub source: String,
    pub abs_url: String,
    pub pdf_url: String,
}

// ============================================================================
// Database Stats
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DbStats {
    pub total_papers: i64,
    pub by_source: HashMap<String, i64>,
    pub by_status: HashMap<String, i64>,
    pub queue_queued: i64,
    pub queue_running: i64,
    pub cache_entries: i64,
    pub dedup_records: i64,
}

// ============================================================================
// Database
// ============================================================================

pub struct Database {
    db_path: Arc<std::path::PathBuf>,
}

impl Database {
    /// Open (or create) a database at the given path.
    pub fn open<P: AsRef<Path>>(db_path: P) -> Result<Self> {
        let db_path = Arc::new(db_path.as_ref().to_path_buf());
        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        Ok(Self { db_path })
    }

/// Open database from RAIROS_DB env var, falling back to ~/.rairos/rairos.db
    pub fn open_default() -> Result<Self> {
        let db_path = std::env::var("RAIROS_DB")
            .or_else(|_| std::env::var("AIROS_DB"))
            .map(std::path::PathBuf::from)
            .unwrap_or_else(|_| {
                let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
                std::path::PathBuf::from(home)
                    .join(".rairos")
                    .join("rairos.db")
            });
        Self::open(db_path)
    }

    /// Open an in-memory database (useful for testing).
    pub fn open_in_memory() -> Result<Self> {
        Self::open(":memory:")
    }

    /// Execute a raw SQL query with values and return results as maps.
    pub fn query_raw(
        &self,
        sql: &str,
        params: Vec<rusqlite::types::Value>,
    ) -> Result<Vec<HashMap<String, rusqlite::types::Value>>> {
        self.with_conn(|conn| {
            let mut stmt = conn.prepare(sql)?;
            let column_count = stmt.column_count();
            let column_names: Vec<String> = (0..column_count)
                .map(|i| stmt.column_name(i).unwrap_or("?").to_string())
                .collect();

            let mut results = Vec::new();
            let mut rows = stmt.query(rusqlite::params_from_iter(&params))?;
            while let Some(row) = rows.next()? {
                let mut map = HashMap::new();
                for (i, name) in column_names.iter().enumerate() {
                    let value: rusqlite::types::Value = row.get(i)?;
                    map.insert(name.clone(), value);
                }
                results.push(map);
            }
            Ok(results)
        })
    }

    /// Initialize the database schema and FTS5 virtual table.
    pub fn init(&self) -> Result<()> {
        self.with_conn(|conn| {
            conn.execute_batch(SCHEMA)?;
            conn.execute_batch(FTS_SCHEMA)?;
            Ok(())
        })?;
        Ok(())
    }

    /// Returns a connection for the database.
    /// Always opens a fresh connection (no caching) to ensure test isolation.
    /// Creates the DB file if it doesn't exist (e.g. after clear_all deleted it).
    fn with_conn<F, T>(&self, f: F) -> Result<T>
    where
        F: FnOnce(&Connection) -> Result<T>,
    {
        let path = self.db_path.as_path();
        // If DB file doesn't exist (e.g. clear_all deleted it), recreate it
        if !path.exists() {
            if let Some(parent) = path.parent() {
                let _ = std::fs::create_dir_all(parent);
            }
            let _ = std::fs::File::create(path);
            // Also clean up any stale WAL/SHM
            let _ = std::fs::remove_file(path.with_extension("db-wal"));
            let _ = std::fs::remove_file(path.with_extension("db-shm"));
        }
        let conn = Connection::open(path)?;
        conn.execute_batch(
            "PRAGMA journal_mode=WAL;
             PRAGMA synchronous=NORMAL;
             PRAGMA foreign_keys=OFF;
             PRAGMA busy_timeout=30000;
             PRAGMA cache_size=-102400;",
        )?;
        f(&conn)
    }

    /// Delete all papers and embeddings (for test isolation).
    /// Physically removes the DB file so the next connection gets a fresh DB,
    /// then reinitializes the schema so the DB is immediately usable.
    pub fn clear_all(&self) -> Result<()> {
        let path = self.db_path.as_path();
        drop(std::fs::File::open(path).ok());
        let _ = std::fs::remove_file(path);
        let _ = std::fs::remove_file(path.with_extension("db-wal"));
        let _ = std::fs::remove_file(path.with_extension("db-shm"));
        // Reinitialize schema so the DB is immediately usable after clear
        self.init()
    }

    /// Execute a transaction.
    #[allow(dead_code)]
    pub fn transaction<T, F>(&self, f: F) -> Result<T>
    where
        F: FnOnce(&Connection) -> Result<T>,
    {
        self.with_conn(|conn| {
            let tx = conn.unchecked_transaction()?;
            let result = f(conn);
            if result.is_ok() {
                tx.commit()?;
            } else {
                tx.rollback()?;
            }
            Ok(result?)
        })
    }

    // -------------------------------------------------------------------------
    // Paper methods
    // -------------------------------------------------------------------------

    /// Insert or replace a paper. Returns the paper.
    #[allow(clippy::too_many_arguments)]
    pub fn upsert_paper(
        &self,
        paper_id: &str,
        source: &str,
        title: &str,
        authors: &[String],
        abstract_text: &str,
        published: &str,
        updated: &str,
        abs_url: &str,
        pdf_url: &str,
        primary_category: &str,
        journal: &str,
        volume: &str,
        issue: &str,
        page: &str,
        doi: &str,
        categories: &str,
        reference_count: i64,
        pdf_path: &str,
        pdf_hash: &str,
    ) -> Result<Paper> {
        let now = utcnow();
        let authors_json = serde_json::to_string(authors)?;

        self.with_conn(|conn| {
            let exists: bool = conn
                .query_row("SELECT 1 FROM papers WHERE id = ?", [paper_id], |_| {
                    Ok(true)
                })
                .optional()?
                .is_some();

            if exists {
                conn.execute(
                    "UPDATE papers SET
                     title=:t, authors=:a, abstract=:ab, published=:p, updated=:u,
                     abs_url=:au, pdf_url=:pu, primary_category=:pc, journal=:j,
                     volume=:v, issue=:i, page=:pg, doi=:d, categories=:c,
                     reference_count=:rc, updated_at=:ua,
                     pdf_path=COALESCE(NULLIF(:pp,''), pdf_path),
                     pdf_hash=COALESCE(NULLIF(:ph,''), pdf_hash)
                     WHERE id=:id",
                    rusqlite::named_params! {
                        ":id": paper_id,
                        ":t": title,
                        ":a": authors_json,
                        ":ab": abstract_text,
                        ":p": published,
                        ":u": updated,
                        ":au": abs_url,
                        ":pu": pdf_url,
                        ":pc": primary_category,
                        ":j": journal,
                        ":v": volume,
                        ":i": issue,
                        ":pg": page,
                        ":d": doi,
                        ":c": categories,
                        ":rc": reference_count,
                        ":ua": now,
                        ":pp": pdf_path,
                        ":ph": pdf_hash,
                    },
                )?;
            } else {
                conn.execute(
                    "INSERT INTO papers (
                     id, source, title, authors, abstract, published, updated,
                     abs_url, pdf_url, primary_category, journal, volume, issue,
                     page, doi, categories, reference_count, added_at, updated_at,
                     pdf_path, pdf_hash
                    ) VALUES (
                     :id, :source, :title, :authors, :abstract, :published, :updated,
                     :abs_url, :pdf_url, :primary_category, :journal, :volume, :issue,
                     :page, :doi, :categories, :reference_count, :added_at, :updated_at,
                     :pdf_path, :pdf_hash
                    )",
                    rusqlite::named_params! {
                        ":id": paper_id,
                        ":source": source,
                        ":title": title,
                        ":authors": authors_json,
                        ":abstract": abstract_text,
                        ":published": published,
                        ":updated": updated,
                        ":abs_url": abs_url,
                        ":pdf_url": pdf_url,
                        ":primary_category": primary_category,
                        ":journal": journal,
                        ":volume": volume,
                        ":issue": issue,
                        ":page": page,
                        ":doi": doi,
                        ":categories": categories,
                        ":reference_count": reference_count,
                        ":added_at": now,
                        ":updated_at": now,
                        ":pdf_path": pdf_path,
                        ":pdf_hash": pdf_hash,
                    },
                )?;
            }

            // Sync FTS
            let _ = self._sync_fts_internal(conn, paper_id, title, abstract_text);

            Ok(())
        })?;

        self.get_paper(paper_id)?
            .ok_or_else(|| DbError::NotFound(paper_id.to_string()))
    }

    /// Get a single paper by ID.
    pub fn get_paper(&self, paper_id: &str) -> Result<Option<Paper>> {
        self.with_conn(|conn| {
            let result = conn.query_row(
                "SELECT * FROM papers WHERE id = ?",
                [paper_id],
                Paper::from_row,
            );
            match result {
                Ok(paper) => Ok(Some(paper)),
                Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
                Err(e) => Err(DbError::Database(e)),
            }
        })
    }

    /// Get multiple papers by IDs. Returns a map of paper_id -> Paper.
    pub fn get_papers_bulk(&self, paper_ids: &[String]) -> Result<HashMap<String, Paper>> {
        if paper_ids.is_empty() {
            return Ok(HashMap::new());
        }
        self.with_conn(|conn| {
            let placeholders: Vec<String> = paper_ids.iter().map(|_| "?".to_string()).collect();
            let sql = format!(
                "SELECT * FROM papers WHERE id IN ({})",
                placeholders.join(",")
            );
            let mut stmt = conn.prepare(&sql)?;
            let rows = stmt.query_map(rusqlite::params_from_iter(paper_ids), Paper::from_row)?;
            let mut map = HashMap::new();
            for row in rows {
                let paper = row?;
                map.insert(paper.id.clone(), paper);
            }
            Ok(map)
        })
    }

    /// Bulk upsert papers. Returns (inserted, updated) counts.
    pub fn upsert_papers_bulk(&self, papers: &[PaperInput], source: &str) -> Result<(i64, i64)> {
        if papers.is_empty() {
            return Ok((0, 0));
        }
        let now = utcnow();

        self.with_conn(|conn| {
            // Pre-fetch existing IDs
            let paper_ids: Vec<&str> = papers.iter().map(|p| p.paper_id.as_str()).collect();
            let placeholders: Vec<String> = paper_ids.iter().map(|_| "?".to_string()).collect();
            let sql = format!(
                "SELECT id FROM papers WHERE id IN ({})",
                placeholders.join(",")
            );
            let mut stmt = conn.prepare(&sql)?;
            let existing: std::collections::HashSet<String> = stmt
                .query_map(rusqlite::params_from_iter(paper_ids), |row| row.get(0))?
                .filter_map(|r| r.ok())
                .collect();

            let mut inserted: i64 = 0;
            let mut updated: i64 = 0;

            for p in papers {
                let authors_json = serde_json::to_string(&p.authors)?;
                if existing.contains(&p.paper_id) {
                    conn.execute(
                        "UPDATE papers SET
                         title=:t, authors=:a, abstract=:ab, published=:pu, updated=:u,
                         abs_url=:au, pdf_url=:pu2, primary_category=:pc, journal=:j,
                         volume=:v, issue=:i, page=:pg, doi=:d, categories=:c,
                         reference_count=:rc, updated_at=:ua,
                         pdf_path=:pp, pdf_hash=:ph
                         WHERE id=:id",
                        rusqlite::named_params! {
                            ":id": p.paper_id,
                            ":t": p.title,
                            ":a": authors_json,
                            ":ab": p.abstract_text,
                            ":pu": p.published,
                            ":u": p.updated,
                            ":au": p.abs_url,
                            ":pu2": p.pdf_url,
                            ":pc": p.primary_category,
                            ":j": p.journal,
                            ":v": p.volume,
                            ":i": p.issue,
                            ":pg": p.page,
                            ":d": p.doi,
                            ":c": p.categories,
                            ":rc": p.reference_count,
                            ":ua": now,
                            ":pp": p.pdf_path,
                            ":ph": p.pdf_hash,
                        },
                    )?;
                    updated += 1;
                } else {
                    conn.execute(
                        "INSERT INTO papers (
                         id, source, title, authors, abstract, published, updated,
                         abs_url, pdf_url, primary_category, journal, volume, issue,
                         page, doi, categories, reference_count, added_at, updated_at,
                         pdf_path, pdf_hash
                        ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
                        rusqlite::params![
                            p.paper_id,
                            source,
                            p.title,
                            authors_json,
                            p.abstract_text,
                            p.published,
                            p.updated,
                            p.abs_url,
                            p.pdf_url,
                            p.primary_category,
                            p.journal,
                            p.volume,
                            p.issue,
                            p.page,
                            p.doi,
                            p.categories,
                            p.reference_count,
                            now,
                            now,
                            p.pdf_path,
                            p.pdf_hash,
                        ],
                    )?;
                    inserted += 1;
                }
            }
            Ok((inserted, updated))
        })
    }

    /// Update the parse status and parsed content fields.
    #[allow(clippy::too_many_arguments)]
    pub fn update_parse_status(
        &self,
        paper_id: &str,
        status: &str,
        error: &str,
        plain_text: &str,
        latex_blocks: &[serde_json::Value],
        table_count: i64,
        figure_count: i64,
        word_count: i64,
        page_count: i64,
    ) -> Result<()> {
        let now = utcnow();
        let latex_json = serde_json::to_string(latex_blocks)?;

        self.with_conn(|conn| {
            // Get current parse_version, title, abstract
            let (version, title, abstract_text): (i64, String, String) = conn
                .query_row(
                    "SELECT parse_version, title, abstract FROM papers WHERE id = ?",
                    [paper_id],
                    |row| {
                        Ok((
                            row.get::<_, i64>(0)?,
                            row.get::<_, String>(1)?,
                            row.get::<_, String>(2)?,
                        ))
                    },
                )
                .optional()?
                .unwrap_or((0, String::new(), String::new()));

            let new_version = version + 1;

            conn.execute(
                "UPDATE papers SET
                 parse_status=:status, parse_error=:error,
                 plain_text=:plain_text, latex_blocks=:latex_blocks,
                 table_count=:tc, figure_count=:fc, word_count=:wc, page_count=:pc,
                 parse_version=:pv, updated_at=:ua
                 WHERE id=:id",
                rusqlite::named_params! {
                    ":status": status,
                    ":error": error,
                    ":plain_text": plain_text,
                    ":latex_blocks": latex_json,
                    ":tc": table_count,
                    ":fc": figure_count,
                    ":wc": word_count,
                    ":pc": page_count,
                    ":pv": new_version,
                    ":ua": now,
                    ":id": paper_id,
                },
            )?;

            // Sync FTS
            let _ = self._sync_fts_internal(conn, paper_id, &title, &abstract_text);

            Ok(())
        })
    }

    /// Count papers, optionally filtered by parse_status.
    pub fn paper_count(&self, status: Option<&str>) -> Result<i64> {
        self.with_conn(|conn| {
            let count = match status {
                Some(s) => conn.query_row(
                    "SELECT COUNT(*) FROM papers WHERE parse_status = ?",
                    [s],
                    |row| row.get(0),
                ),
                None => conn.query_row("SELECT COUNT(*) FROM papers", [], |row| row.get(0)),
            }?;
            Ok(count)
        })
    }

    /// Check whether a paper exists.
    pub fn paper_exists(&self, paper_id: &str) -> Result<bool> {
        self.with_conn(|conn| {
            let exists: bool = conn
                .query_row("SELECT 1 FROM papers WHERE id = ?", [paper_id], |_| {
                    Ok(true)
                })
                .optional()?
                .is_some();
            Ok(exists)
        })
    }

    /// Get all papers (newest first), with optional limit/offset.
    pub fn get_papers(&self, limit: i64, offset: i64) -> Result<Vec<Paper>> {
        self.with_conn(|conn| {
            let mut stmt =
                conn.prepare("SELECT * FROM papers ORDER BY added_at DESC LIMIT ? OFFSET ?")?;
            let rows = stmt.query_map([limit, offset], Paper::from_row)?;
            let mut papers = Vec::new();
            for row in rows {
                papers.push(row?);
            }
            Ok(papers)
        })
    }

    /// List papers with filters and sort.
    #[allow(clippy::too_many_arguments)]
    pub fn list_papers(
        &self,
        limit: i64,
        offset: i64,
        source: Option<&str>,
        category: Option<&str>,
        date_from: Option<&str>,
        date_to: Option<&str>,
        parse_status: Option<&str>,
        sort_by: &str,
        sort_order: &str,
    ) -> Result<(Vec<Paper>, i64)> {
        self.with_conn(|conn| {
            let mut where_parts: Vec<&str> = Vec::new();
            let mut params_vec: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();

            if let Some(s) = source {
                where_parts.push("source = ?");
                params_vec.push(Box::new(s.to_string()));
            }
            if let Some(c) = category {
                where_parts.push("primary_category = ?");
                params_vec.push(Box::new(c.to_string()));
            }
            if let Some(df) = date_from {
                where_parts.push("published >= ?");
                params_vec.push(Box::new(df.to_string()));
            }
            if let Some(dt) = date_to {
                where_parts.push("published <= ?");
                params_vec.push(Box::new(dt.to_string()));
            }
            if let Some(ps) = parse_status {
                where_parts.push("parse_status = ?");
                params_vec.push(Box::new(ps.to_string()));
            }

            let where_clause = if where_parts.is_empty() {
                String::new()
            } else {
                format!("WHERE {}", where_parts.join(" AND "))
            };

            let allowed_sort = ["added_at", "published", "title"];
            let sort_col = if allowed_sort.contains(&sort_by) {
                sort_by
            } else {
                "added_at"
            };
            let order = if sort_order.eq_ignore_ascii_case("desc") {
                "DESC"
            } else {
                "ASC"
            };

            // Count
            let count_sql = format!("SELECT COUNT(*) FROM papers {}", where_clause);
            let params_ref: Vec<&dyn rusqlite::ToSql> =
                params_vec.iter().map(|p| p.as_ref()).collect();
            let total: i64 = conn.query_row(&count_sql, params_ref.as_slice(), |row| row.get(0))?;

            // List
            let sql = format!(
                "SELECT * FROM papers {} ORDER BY {} {} LIMIT ? OFFSET ?",
                where_clause, sort_col, order
            );
            let mut all_params = params_vec;
            all_params.push(Box::new(limit));
            all_params.push(Box::new(offset));
            let params_ref: Vec<&dyn rusqlite::ToSql> =
                all_params.iter().map(|p| p.as_ref()).collect();

            let mut stmt = conn.prepare(&sql)?;
            let rows = stmt.query_map(params_ref.as_slice(), Paper::from_row)?;
            let mut papers = Vec::new();
            for row in rows {
                papers.push(row?);
            }

            Ok((papers, total))
        })
    }

    /// Delete a paper and its FTS entry. Returns true if a row was deleted.
    pub fn delete_paper(&self, paper_id: &str) -> Result<bool> {
        self.with_conn(|conn| {
            conn.execute("DELETE FROM papers_fts WHERE paper_id = ?", [paper_id])?;
            let n = conn.execute("DELETE FROM papers WHERE id = ?", [paper_id])?;
            Ok(n > 0)
        })
    }

    /// Full-text search with BM25 ranking.
    #[allow(clippy::too_many_arguments)]
    pub fn search_papers(
        &self,
        query: &str,
        limit: i64,
        offset: i64,
        source: Option<&str>,
        category: Option<&str>,
        date_from: Option<&str>,
        date_to: Option<&str>,
        parse_status: Option<&str>,
    ) -> Result<(Vec<SearchResult>, i64)> {
        let result = self.with_conn(|conn| {
            let fts_query = build_fts_query(query);

            let mut where_parts: Vec<String> = Vec::new();
            let mut params_vec: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();

            if let Some(s) = source {
                where_parts.push("p.source = ?".to_string());
                params_vec.push(Box::new(s.to_string()));
            }
            if let Some(c) = category {
                where_parts.push("p.primary_category = ?".to_string());
                params_vec.push(Box::new(c.to_string()));
            }
            if let Some(df) = date_from {
                where_parts.push("p.published >= ?".to_string());
                params_vec.push(Box::new(df.to_string()));
            }
            if let Some(dt) = date_to {
                where_parts.push("p.published <= ?".to_string());
                params_vec.push(Box::new(dt.to_string()));
            }
            if let Some(ps) = parse_status {
                where_parts.push("p.parse_status = ?".to_string());
                params_vec.push(Box::new(ps.to_string()));
            }

            let extra_where = if where_parts.is_empty() {
                String::new()
            } else {
                format!(" AND {}", where_parts.join(" AND "))
            };

            // Count
            let count_sql = format!(
                "SELECT COUNT(*) FROM papers_fts fts
                 JOIN papers p ON p.id = fts.paper_id
                 WHERE papers_fts MATCH ?{}",
                extra_where
            );
            let all_count_params: Vec<&dyn rusqlite::ToSql> =
                std::iter::once(&fts_query as &dyn rusqlite::ToSql)
                    .chain(params_vec.iter().map(|p| p.as_ref()))
                    .collect();
            let total: i64 = conn
                .query_row(&count_sql, all_count_params.as_slice(), |row| row.get(0))
                .unwrap_or(0);

            // Search
            let search_sql = format!(
                "SELECT
                    papers_fts.paper_id,
                    papers_fts.title,
                    papers_fts.abstract,
                    bm25(papers_fts) AS score,
                    snippet(papers_fts, 0, '**', '**', '...', 30) AS snippet
                 FROM papers_fts
                 JOIN papers p ON p.id = papers_fts.paper_id
                 WHERE papers_fts MATCH ?{}
                 ORDER BY score
                 LIMIT ? OFFSET ?",
                extra_where
            );
            let mut all_params: Vec<Box<dyn rusqlite::ToSql>> =
                std::iter::once(Box::new(fts_query.to_string()) as Box<dyn rusqlite::ToSql>)
                    .chain(params_vec)
                    .collect();
            all_params.push(Box::new(limit));
            all_params.push(Box::new(offset));
            let all_params_ref: Vec<&dyn rusqlite::ToSql> =
                all_params.iter().map(|p| p.as_ref()).collect();

            let mut stmt = conn.prepare(&search_sql)?;
            let fts_rows = stmt.query_map(all_params_ref.as_slice(), |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, f64>(3)?,
                    row.get::<_, String>(4)?,
                ))
            })?;

            let mut paper_ids = Vec::new();
            let mut fts_data = Vec::new();
            for r in fts_rows {
                let (pid, title, abstract_text, score, snippet) = r?;
                paper_ids.push(pid.clone());
                fts_data.push((pid, title, abstract_text, score, snippet));
            }

            if paper_ids.is_empty() {
                return Ok((Vec::new(), total));
            }

            // Look up full paper data
            let placeholders: Vec<String> = paper_ids.iter().map(|_| "?".to_string()).collect();
            let lookup_sql = format!(
                "SELECT * FROM papers WHERE id IN ({})",
                placeholders.join(",")
            );
            let mut lookup_stmt = conn.prepare(&lookup_sql)?;
            let paper_map: HashMap<String, Paper> = lookup_stmt
                .query_map(rusqlite::params_from_iter(paper_ids), Paper::from_row)?
                .filter_map(|r| r.ok())
                .map(|p| (p.id.clone(), p))
                .collect();

            let results: Vec<SearchResult> = fts_data
                .into_iter()
                .filter_map(|(pid, _, _, score, snippet)| {
                    let paper = paper_map.get(&pid)?;
                    Some(SearchResult {
                        paper_id: pid,
                        title: paper.title.clone(),
                        authors: paper.authors.clone(),
                        published: paper.published.clone(),
                        primary_category: paper.primary_category.clone(),
                        score,
                        snippet,
                        parse_status: paper.parse_status.clone(),
                        source: paper.source.clone(),
                        abs_url: paper.abs_url.clone(),
                        pdf_url: paper.pdf_url.clone(),
                    })
                })
                .collect();

            Ok((results, total))
        });

        match result {
            Ok(v) => Ok(v),
            Err(_) => {
                // Fallback to LIKE search on FTS error
                self._search_like(
                    query,
                    limit,
                    offset,
                    source,
                    category,
                    date_from,
                    date_to,
                    parse_status,
                )
            }
        }
    }

    /// Internal FTS sync — must be called within a with_conn closure.
    fn _sync_fts_internal(
        &self,
        conn: &Connection,
        paper_id: &str,
        title: &str,
        abstract_text: &str,
    ) -> Result<()> {
        conn.execute("DELETE FROM papers_fts WHERE paper_id = ?", [paper_id])?;
        conn.execute(
            "INSERT INTO papers_fts(paper_id, title, abstract, plain_text) VALUES (?, ?, ?, '')",
            rusqlite::params![paper_id, title, abstract_text],
        )?;
        Ok(())
    }

    /// Rebuild the entire FTS index from all papers. Returns count of indexed papers.
    pub fn rebuild_fts_index(&self) -> Result<i64> {
        self.with_conn(|conn| {
            conn.execute("DELETE FROM papers_fts", [])?;
            let count = conn.execute(
                "INSERT INTO papers_fts(paper_id, title, abstract, plain_text)
                 SELECT id, title, abstract, COALESCE(plain_text, '') FROM papers",
                [],
            )?;
            Ok(count as i64)
        })
    }

    /// LIKE-based fallback search.
    #[allow(clippy::too_many_arguments)]
    fn _search_like(
        &self,
        query: &str,
        limit: i64,
        offset: i64,
        source: Option<&str>,
        category: Option<&str>,
        date_from: Option<&str>,
        date_to: Option<&str>,
        parse_status: Option<&str>,
    ) -> Result<(Vec<SearchResult>, i64)> {
        self.with_conn(|conn| {
            let q = format!("%{}%", query);
            let mut where_parts = vec!["(title LIKE ? OR abstract LIKE ? OR plain_text LIKE ?)"];
            let mut params_vec: Vec<Box<dyn rusqlite::ToSql>> =
                vec![Box::new(q.clone()), Box::new(q.clone()), Box::new(q)];

            if let Some(s) = source {
                where_parts.push("source = ?");
                params_vec.push(Box::new(s.to_string()));
            }
            if let Some(c) = category {
                where_parts.push("primary_category = ?");
                params_vec.push(Box::new(c.to_string()));
            }
            if let Some(df) = date_from {
                where_parts.push("published >= ?");
                params_vec.push(Box::new(df.to_string()));
            }
            if let Some(dt) = date_to {
                where_parts.push("published <= ?");
                params_vec.push(Box::new(dt.to_string()));
            }
            if let Some(ps) = parse_status {
                where_parts.push("parse_status = ?");
                params_vec.push(Box::new(ps.to_string()));
            }

            let where_clause = format!("WHERE {}", where_parts.join(" AND "));
            let params_ref: Vec<&dyn rusqlite::ToSql> =
                params_vec.iter().map(|p| p.as_ref()).collect();

            let count_sql = format!("SELECT COUNT(*) FROM papers {}", where_clause);
            let total: i64 = conn.query_row(&count_sql, params_ref.as_slice(), |row| row.get(0))?;

            let sql = format!(
                "SELECT id, title, authors, published, primary_category,
                        source, parse_status, abs_url, pdf_url
                 FROM papers {} ORDER BY added_at DESC LIMIT ? OFFSET ?",
                where_clause
            );
            let mut all_params = params_vec;
            all_params.push(Box::new(limit));
            all_params.push(Box::new(offset));
            let all_params_ref: Vec<&dyn rusqlite::ToSql> =
                all_params.iter().map(|p| p.as_ref()).collect();

            let mut stmt = conn.prepare(&sql)?;
            let rows = stmt.query_map(all_params_ref.as_slice(), |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, String>(4)?,
                    row.get::<_, String>(5)?,
                    row.get::<_, String>(6)?,
                    row.get::<_, String>(7)?,
                    row.get::<_, String>(8)?,
                ))
            })?;

            let results: Vec<SearchResult> = rows
                .filter_map(|r| r.ok())
                .map(
                    |(
                        id,
                        title,
                        authors_raw,
                        published,
                        primary_category,
                        source,
                        parse_status,
                        abs_url,
                        pdf_url,
                    )| {
                        let authors: Vec<String> =
                            serde_json::from_str(&authors_raw).unwrap_or_default();
                        SearchResult {
                            paper_id: id,
                            title,
                            authors,
                            published,
                            primary_category,
                            score: 0.0,
                            snippet: format!("...{}...", query),
                            parse_status,
                            source,
                            abs_url,
                            pdf_url,
                        }
                    },
                )
                .collect();

            Ok((results, total))
        })
    }

    /// Get database statistics.
    pub fn get_stats(&self) -> Result<DbStats> {
        self.with_conn(|conn| {
            let total_papers: i64 =
                conn.query_row("SELECT COUNT(*) FROM papers", [], |row| row.get(0))?;

            let mut by_source = HashMap::new();
            let mut stmt = conn.prepare("SELECT source, COUNT(*) FROM papers GROUP BY source")?;
            let rows = stmt.query_map([], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
            })?;
            for r in rows {
                let (src, cnt) = r?;
                by_source.insert(src, cnt);
            }

            let mut by_status = HashMap::new();
            let mut stmt =
                conn.prepare("SELECT parse_status, COUNT(*) FROM papers GROUP BY parse_status")?;
            let rows = stmt.query_map([], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
            })?;
            for r in rows {
                let (st, cnt) = r?;
                by_status.insert(st, cnt);
            }

            let queue_queued: i64 = conn
                .query_row(
                    "SELECT COUNT(*) FROM job_queue WHERE status = 'queued'",
                    [],
                    |row| row.get(0),
                )
                .unwrap_or(0);
            let queue_running: i64 = conn
                .query_row(
                    "SELECT COUNT(*) FROM job_queue WHERE status = 'running'",
                    [],
                    |row| row.get(0),
                )
                .unwrap_or(0);
            let cache_entries: i64 = conn
                .query_row("SELECT COUNT(*) FROM paper_cache", [], |row| row.get(0))
                .unwrap_or(0);
            let dedup_records: i64 = conn
                .query_row("SELECT COUNT(*) FROM dedup_log", [], |row| row.get(0))
                .unwrap_or(0);

            Ok(DbStats {
                total_papers,
                by_source,
                by_status,
                queue_queued,
                queue_running,
                cache_entries,
                dedup_records,
            })
        })
    }

    /// Export papers as (header_fields, rows).
    #[allow(clippy::type_complexity)]
    pub fn export_papers(
        &self,
        _format: &str,
        limit: i64,
    ) -> Result<(Vec<String>, Vec<HashMap<String, serde_json::Value>>)> {
        self.with_conn(|conn| {
            let fields = [
                "id",
                "source",
                "title",
                "authors",
                "abstract",
                "published",
                "doi",
                "primary_category",
                "parse_status",
                "added_at",
            ];

            let sql = if limit > 0 {
                format!(
                    "SELECT {} FROM papers ORDER BY added_at DESC LIMIT {}",
                    fields.join(","),
                    limit
                )
            } else {
                format!(
                    "SELECT {} FROM papers ORDER BY added_at DESC",
                    fields.join(",")
                )
            };

            let mut stmt = conn.prepare(&sql)?;
            let rows_iter = stmt.query_map([], |row| {
                let mut m = HashMap::new();
                for (i, f) in fields.iter().enumerate() {
                    let val: rusqlite::types::Value = row.get(i)?;
                    let json_val = match val {
                        rusqlite::types::Value::Null => serde_json::Value::Null,
                        rusqlite::types::Value::Integer(i) => serde_json::json!(i),
                        rusqlite::types::Value::Real(r) => serde_json::json!(r),
                        rusqlite::types::Value::Text(s) => serde_json::json!(s),
                        rusqlite::types::Value::Blob(b) => serde_json::json!(base64_encode(&b)),
                    };
                    m.insert(f.to_string(), json_val);
                }
                Ok(m)
            })?;

            let mut rows = Vec::new();
            for r in rows_iter {
                rows.push(r?);
            }
            Ok((fields.iter().map(|s| s.to_string()).collect(), rows))
        })
    }
}

// ============================================================================
// Helpers
// ============================================================================

fn utcnow() -> String {
    Utc::now().format("%Y-%m-%dT%H:%M:%S").to_string()
}

fn base64_encode(data: &[u8]) -> String {
    const ALPHABET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut result = String::new();
    for chunk in data.chunks(3) {
        let b = match chunk.len() {
            1 => [chunk[0], 0, 0],
            2 => [chunk[0], chunk[1], 0],
            _ => [chunk[0], chunk[1], chunk[2]],
        };
        result.push(ALPHABET[(b[0] >> 2) as usize] as char);
        result.push(ALPHABET[((b[0] & 0x03) << 4 | b[1] >> 4) as usize] as char);
        if chunk.len() > 1 {
            result.push(ALPHABET[((b[1] & 0x0f) << 2 | b[2] >> 6) as usize] as char);
        } else {
            result.push('=');
        }
        if chunk.len() > 2 {
            result.push(ALPHABET[(b[2] & 0x3f) as usize] as char);
        } else {
            result.push('=');
        }
    }
    result
}

fn build_fts_query(query: &str) -> String {
    let re = Regex::new(r"[A-Za-z0-9_]+").unwrap();
    let tokens: Vec<String> = re
        .find_iter(query)
        .map(|m| m.as_str().replace('-', "_"))
        .collect();

    if tokens.is_empty() {
        format!("\"{}\"", query)
    } else if tokens.len() == 1 {
        tokens[0].clone()
    } else {
        tokens.join(" OR ")
    }
}

// ============================================================================
// Input types
// ============================================================================

/// Input for bulk upsert.
#[derive(Default)]
pub struct PaperInput {
    pub paper_id: String,
    pub title: String,
    pub authors: Vec<String>,
    pub abstract_text: String,
    pub published: String,
    pub updated: String,
    pub abs_url: String,
    pub pdf_url: String,
    pub primary_category: String,
    pub journal: String,
    pub volume: String,
    pub issue: String,
    pub page: String,
    pub doi: String,
    pub categories: String,
    pub reference_count: i64,
    pub pdf_path: String,
    pub pdf_hash: String,
}

// ============================================================================
// Schema
// ============================================================================

const SCHEMA: &str = r#"
CREATE TABLE IF NOT EXISTS papers (
    id               TEXT PRIMARY KEY,
    source           TEXT NOT NULL,
    title            TEXT DEFAULT '',
    authors          TEXT DEFAULT '[]',
    abstract         TEXT DEFAULT '',
    published        TEXT DEFAULT '',
    updated          TEXT DEFAULT '',
    abs_url          TEXT DEFAULT '',
    pdf_url          TEXT DEFAULT '',
    primary_category TEXT DEFAULT '',
    journal          TEXT DEFAULT '',
    volume           TEXT DEFAULT '',
    issue            TEXT DEFAULT '',
    page             TEXT DEFAULT '',
    doi              TEXT DEFAULT '',
    categories       TEXT DEFAULT '',
    reference_count  INTEGER DEFAULT 0,
    added_at         TEXT NOT NULL,
    updated_at       TEXT NOT NULL,
    pdf_path         TEXT DEFAULT '',
    pdf_hash         TEXT DEFAULT '',
    parse_status     TEXT DEFAULT 'pending',
    parse_error      TEXT DEFAULT '',
    parse_version    INTEGER DEFAULT 0,
    plain_text       TEXT DEFAULT '',
    latex_blocks     TEXT DEFAULT '[]',
    table_count      INTEGER DEFAULT 0,
    figure_count     INTEGER DEFAULT 0,
    word_count       INTEGER DEFAULT 0,
    page_count       INTEGER DEFAULT 0,
    pnote_path       TEXT DEFAULT '',
    cnote_path       TEXT DEFAULT '',
    mnote_path       TEXT DEFAULT '',
    embed_vector     BLOB DEFAULT NULL
);

CREATE TABLE IF NOT EXISTS parse_history (
    id             INTEGER PRIMARY KEY AUTOINCREMENT,
    paper_id       TEXT NOT NULL,
    attempted_at   TEXT NOT NULL,
    duration_sec   REAL,
    status         TEXT NOT NULL,
    error          TEXT DEFAULT '',
    parse_version  INTEGER,
    pdf_hash       TEXT,
    file_size      INTEGER,
    FOREIGN KEY (paper_id) REFERENCES papers(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS tags (
    id   INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT UNIQUE NOT NULL
);

CREATE TABLE IF NOT EXISTS paper_tags (
    paper_id TEXT NOT NULL,
    tag_id   INTEGER NOT NULL,
    PRIMARY KEY (paper_id, tag_id),
    FOREIGN KEY (paper_id) REFERENCES papers(id) ON DELETE CASCADE,
    FOREIGN KEY (tag_id) REFERENCES tags(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS job_queue (
    id            INTEGER PRIMARY KEY AUTOINCREMENT,
    paper_id      TEXT NOT NULL,
    job_type      TEXT NOT NULL,
    priority      INTEGER DEFAULT 5,
    status        TEXT DEFAULT 'queued',
    created_at    TEXT NOT NULL,
    started_at    TEXT DEFAULT '',
    completed_at  TEXT DEFAULT '',
    error         TEXT DEFAULT '',
    FOREIGN KEY (paper_id) REFERENCES papers(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS settings (
    key   TEXT PRIMARY KEY,
    value TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS paper_cache (
    uid  TEXT PRIMARY KEY,
    data TEXT NOT NULL,
    cached_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS dedup_log (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    target_id   TEXT NOT NULL,
    duplicate_id TEXT NOT NULL,
    keep_policy TEXT NOT NULL,
    logged_at   TEXT NOT NULL,
    FOREIGN KEY (target_id)   REFERENCES papers(id) ON DELETE CASCADE,
    FOREIGN KEY (duplicate_id) REFERENCES papers(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_papers_parse_status ON papers(parse_status);
CREATE INDEX IF NOT EXISTS idx_papers_source ON papers(source);
CREATE INDEX IF NOT EXISTS idx_papers_added_at ON papers(added_at);
CREATE INDEX IF NOT EXISTS idx_papers_published ON papers(published);
CREATE INDEX IF NOT EXISTS idx_papers_primary_category ON papers(primary_category);
CREATE INDEX IF NOT EXISTS idx_papers_title ON papers(title);
CREATE INDEX IF NOT EXISTS idx_parse_history_paper_id ON parse_history(paper_id);
CREATE INDEX IF NOT EXISTS idx_job_queue_status ON job_queue(status);

CREATE TABLE IF NOT EXISTS citations (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    source_id   TEXT NOT NULL,
    target_id   TEXT NOT NULL,
    created_at  TEXT NOT NULL,
    FOREIGN KEY (source_id)  REFERENCES papers(id)  ON DELETE CASCADE,
    FOREIGN KEY (target_id)  REFERENCES papers(id)  ON DELETE CASCADE,
    UNIQUE(source_id, target_id)
);

CREATE INDEX IF NOT EXISTS idx_citations_source ON citations(source_id);
CREATE INDEX IF NOT EXISTS idx_citations_target ON citations(target_id);

CREATE TABLE IF NOT EXISTS experiment_tables (
    id            INTEGER PRIMARY KEY AUTOINCREMENT,
    paper_id      TEXT NOT NULL,
    table_caption TEXT DEFAULT '',
    page          INTEGER DEFAULT 0,
    headers       TEXT DEFAULT '[]',
    rows          TEXT DEFAULT '[]',
    bbox_x0       REAL DEFAULT 0,
    bbox_y0       REAL DEFAULT 0,
    bbox_x1       REAL DEFAULT 0,
    bbox_y1       REAL DEFAULT 0,
    created_at    TEXT NOT NULL,
    FOREIGN KEY (paper_id) REFERENCES papers(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_experiment_tables_paper_id ON experiment_tables(paper_id);

CREATE TABLE IF NOT EXISTS paper_code_trace (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    paper_id        TEXT NOT NULL,
    code_path       TEXT NOT NULL,
    module_name     TEXT NOT NULL,
    framework       TEXT NOT NULL DEFAULT 'pytorch',
    total_code_lines INTEGER NOT NULL DEFAULT 0,
    tagged_lines    INTEGER NOT NULL DEFAULT 0,
    untagged_ranges TEXT NOT NULL DEFAULT '[]',
    unreferenced_sources TEXT NOT NULL DEFAULT '[]',
    paper_section_refs TEXT NOT NULL DEFAULT '[]',
    gap_ids         TEXT NOT NULL DEFAULT '[]',
    benchmark_pass_rate REAL DEFAULT 0.0,
    created_at      TEXT NOT NULL,
    FOREIGN KEY (paper_id) REFERENCES papers(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_paper_code_trace_paper_id ON paper_code_trace(paper_id);

CREATE TABLE IF NOT EXISTS gap_history (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    topic           TEXT    NOT NULL,
    session_id      TEXT    NOT NULL,
    gap_type        TEXT    NOT NULL,
    gap_title_hash  TEXT    NOT NULL,
    gap_title       TEXT    NOT NULL,
    gap_hash        TEXT    NOT NULL,
    novelty_score   REAL    DEFAULT 0.0,
    priority        INTEGER DEFAULT 0,
    created_at      TEXT    NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_gap_history_topic  ON gap_history(topic);
CREATE INDEX IF NOT EXISTS idx_gap_history_hash   ON gap_history(gap_hash);
CREATE INDEX IF NOT EXISTS idx_gap_history_session ON gap_history(session_id);

CREATE TABLE IF NOT EXISTS arxiv_search_cache (
    query_hash   TEXT PRIMARY KEY,
    query        TEXT    NOT NULL,
    results_json TEXT    NOT NULL,
    created_at   TEXT    NOT NULL,
    hit_count    INTEGER DEFAULT 1
);
"#;

const FTS_SCHEMA: &str = r#"
CREATE VIRTUAL TABLE IF NOT EXISTS papers_fts USING fts5(
    paper_id UNINDEXED,
    title,
    abstract,
    plain_text,
    tokenize='porter unicode61'
);
"#;

// ============================================================================
// PyO3 Python bindings
// ============================================================================

// ============================================================================
// PyO3 Python bindings
// ============================================================================

#[cfg(not(windows))]
mod py_bindings {
    use pyo3::prelude::*;
    use std::collections::HashMap;

    fn db_err_to_py(err: crate::DbError) -> PyErr {
        PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(err.to_string())
    }

    #[pyclass]
    struct PyDatabase {
        inner: crate::Database,
    }

    #[pymethods]
    impl PyDatabase {
        #[new]
        #[pyo3(signature = (db_path=None))]
        fn new(db_path: Option<String>) -> PyResult<Self> {
            let inner = match db_path {
                Some(p) => crate::Database::open(p).map_err(db_err_to_py)?,
                None => crate::Database::open_default().map_err(db_err_to_py)?,
            };
            Ok(Self { inner })
        }

        fn init_(&self) -> PyResult<()> {
            self.inner.init().map_err(db_err_to_py)
        }

        #[pyo3(signature = (status=None))]
        fn paper_count(&self, status: Option<&str>) -> PyResult<i64> {
            self.inner.paper_count(status).map_err(db_err_to_py)
        }

        fn paper_exists(&self, paper_id: &str) -> PyResult<bool> {
            self.inner.paper_exists(paper_id).map_err(db_err_to_py)
        }

        fn delete_paper(&self, paper_id: &str) -> PyResult<bool> {
            self.inner.delete_paper(paper_id).map_err(db_err_to_py)
        }

        fn get_total_papers(&self) -> PyResult<i64> {
            let stats = self.inner.get_stats().map_err(db_err_to_py)?;
            Ok(stats.total_papers)
        }

        fn upsert_paper(&self, data: &Bound<'_, PyAny>) -> PyResult<Option<String>> {
            let paper_id: String = data.get_item("paper_id")?.extract()?;
            let title: String = data
                .get_item("title")
                .ok()
                .and_then(|a| a.extract().ok())
                .unwrap_or_default();
            let authors: Vec<String> = data
                .get_item("authors")
                .ok()
                .and_then(|a| a.extract().ok())
                .unwrap_or_default();
            let abstract_text: String = data
                .get_item("abstract")
                .ok()
                .and_then(|a| a.extract().ok())
                .unwrap_or_default();
            let published: String = data
                .get_item("published")
                .ok()
                .and_then(|a| a.extract().ok())
                .unwrap_or_default();
            let abs_url: String = data
                .get_item("abs_url")
                .ok()
                .and_then(|a| a.extract().ok())
                .unwrap_or_default();
            let pdf_url: String = data
                .get_item("pdf_url")
                .ok()
                .and_then(|a| a.extract().ok())
                .unwrap_or_default();
            let primary_category: String = data
                .get_item("primary_category")
                .ok()
                .and_then(|a| a.extract().ok())
                .unwrap_or_default();
            let source: String = data
                .get_item("source")
                .ok()
                .and_then(|a| a.extract().ok())
                .unwrap_or_default();

            let result = self.inner.upsert_paper(
                &paper_id,
                &source,
                &title,
                &authors,
                &abstract_text,
                &published,
                "",
                &abs_url,
                &pdf_url,
                &primary_category,
                "",
                "",
                "",
                "",
                "",
                "",
                0,
                "",
                "",
            );
            match result {
                Ok(p) => {
                    let json = serde_json::json!({
                        "id": p.id,
                        "title": p.title,
                        "authors": p.authors,
                        "abstract": p.abstract_text,
                        "published": p.published,
                        "source": p.source,
                        "parse_status": p.parse_status,
                        "primary_category": p.primary_category,
                        "added_at": p.added_at,
                    });
                    Ok(Some(serde_json::to_string(&json).unwrap_or_default()))
                }
                Err(e) => Err(db_err_to_py(e)),
            }
        }

        fn get_paper(&self, paper_id: &str) -> PyResult<Option<String>> {
            let result = self.inner.get_paper(paper_id);
            match result {
                Ok(Some(p)) => {
                    let json = serde_json::json!({
                        "id": p.id,
                        "title": p.title,
                        "authors": p.authors,
                        "abstract": p.abstract_text,
                        "published": p.published,
                        "source": p.source,
                        "parse_status": p.parse_status,
                        "primary_category": p.primary_category,
                        "added_at": p.added_at,
                    });
                    Ok(Some(serde_json::to_string(&json).unwrap_or_default()))
                }
                Ok(None) => Ok(None),
                Err(e) => Err(db_err_to_py(e)),
            }
        }

        fn search_papers(
            &self,
            query: &str,
            limit: Option<i64>,
            offset: Option<i64>,
            source: Option<&str>,
            category: Option<&str>,
            parse_status: Option<&str>,
        ) -> PyResult<String> {
            let limit = limit.unwrap_or(20);
            let offset = offset.unwrap_or(0);
            let (results, total) = self
                .inner
                .search_papers(
                    query,
                    limit,
                    offset,
                    source,
                    category,
                    None,
                    None,
                    parse_status,
                )
                .map_err(db_err_to_py)?;
            let results_json: Vec<serde_json::Value> = results
                .into_iter()
                .map(|r| {
                    serde_json::json!({
                        "paper_id": r.paper_id,
                        "title": r.title,
                        "score": r.score,
                        "snippet": r.snippet,
                    })
                })
                .collect();
            let json = serde_json::json!({ "results": results_json, "total": total });
            Ok(serde_json::to_string(&json).unwrap_or_default())
        }

        fn list_papers(
            &self,
            limit: Option<i64>,
            offset: Option<i64>,
            source: Option<&str>,
            category: Option<&str>,
            parse_status: Option<&str>,
        ) -> PyResult<String> {
            let limit = limit.unwrap_or(100);
            let offset = offset.unwrap_or(0);
            let (papers, total) = self
                .inner
                .list_papers(
                    limit,
                    offset,
                    source,
                    category,
                    None,
                    None,
                    parse_status,
                    "added_at",
                    "desc",
                )
                .map_err(db_err_to_py)?;
            let papers_json: Vec<serde_json::Value> = papers
                .into_iter()
                .map(|p| {
                    serde_json::json!({
                        "id": p.id,
                        "title": p.title,
                        "authors": p.authors,
                        "abstract": p.abstract_text,
                        "published": p.published,
                        "source": p.source,
                        "parse_status": p.parse_status,
                        "primary_category": p.primary_category,
                        "added_at": p.added_at,
                    })
                })
                .collect();
            let json = serde_json::json!({ "papers": papers_json, "total": total });
            Ok(serde_json::to_string(&json).unwrap_or_default())
        }

        fn get_stats(&self) -> PyResult<HashMap<String, i64>> {
            let stats = self.inner.get_stats().map_err(db_err_to_py)?;
            let mut map = HashMap::new();
            map.insert("total_papers".to_string(), stats.total_papers);
            map.insert("queue_queued".to_string(), stats.queue_queued);
            map.insert("queue_running".to_string(), stats.queue_running);
            Ok(map)
        }

        fn clear_all(&self) -> PyResult<()> {
            self.inner.clear_all().map_err(db_err_to_py)
        }
    }
}
