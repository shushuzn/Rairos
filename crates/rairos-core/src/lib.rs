//! Rairos Core — 核心数据结构和数据库操作
//!
//! Paper dataclass, SQLite database, rate limiter.
//! 完全用 Rust 重写，替代 Python 版本。

use chrono::{DateTime, Utc};
use parking_lot::Mutex;
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use thiserror::Error;
use uuid::Uuid;

// ============================================================================
// Error Types
// ============================================================================

#[derive(Error, Debug)]
pub enum CoreError {
    #[error("Database error: {0}")]
    Database(#[from] rusqlite::Error),
    #[error("Paper not found: {0}")]
    NotFound(String),
    #[error("Rate limit exceeded for endpoint: {0}")]
    RateLimited(String),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
}

pub type Result<T> = std::result::Result<T, CoreError>;

// ============================================================================
// Paper
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Paper {
    pub id: String,
    pub arxiv_id: Option<String>,
    pub title: String,
    pub authors: Vec<String>,
    pub published: DateTime<Utc>,
    pub abstract_text: String,
    pub categories: Vec<String>,
    pub parse_status: ParseStatus,
    pub metadata: PaperMetadata,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaperMetadata {
    pub cited_by: usize,
    pub references: usize,
    pub doi: Option<String>,
    pub pdf_url: Option<String>,
}

impl Default for PaperMetadata {
    fn default() -> Self {
        Self {
            cited_by: 0,
            references: 0,
            doi: None,
            pdf_url: None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ParseStatus {
    Pending,
    Parsing,
    Done,
    Failed,
}

impl std::fmt::Display for ParseStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ParseStatus::Pending => write!(f, "pending"),
            ParseStatus::Parsing => write!(f, "parsing"),
            ParseStatus::Done => write!(f, "done"),
            ParseStatus::Failed => write!(f, "failed"),
        }
    }
}

impl Paper {
    pub fn new(arxiv_id: Option<String>, title: String, abstract_text: String) -> Self {
        Self::with_metadata(arxiv_id, title, abstract_text, Vec::new(), Vec::new(), PaperMetadata::default())
    }

    pub fn with_metadata(
        arxiv_id: Option<String>,
        title: String,
        abstract_text: String,
        authors: Vec<String>,
        categories: Vec<String>,
        metadata: PaperMetadata,
    ) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            arxiv_id,
            title,
            authors,
            published: Utc::now(),
            abstract_text,
            categories,
            parse_status: ParseStatus::Pending,
            metadata,
        }
    }
}

// ============================================================================
// Research Gap
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResearchGap {
    pub id: String,
    pub category: String,
    pub description: String,
    pub severity: String,
    pub paper_ids: Vec<String>,
}

impl ResearchGap {
    pub fn new(category: &str, description: &str, severity: &str) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            category: category.to_string(),
            description: description.to_string(),
            severity: severity.to_string(),
            paper_ids: Vec::new(),
        }
    }
}

// ============================================================================
// Subscription
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Subscription {
    pub id: String,
    pub query: String,
    pub name: Option<String>,
    pub categories: Vec<String>,
    pub max_results: i32,
    pub check_interval_minutes: u64,
    pub last_check: Option<String>,
    pub last_results: Option<String>,
    pub enabled: bool,
}

impl Subscription {
    pub fn new(query: &str) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            query: query.to_string(),
            name: None,
            categories: Vec::new(),
            max_results: 10,
            check_interval_minutes: 60,
            last_check: None,
            last_results: None,
            enabled: true,
        }
    }
}

// ============================================================================
// Tag
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tag {
    pub id: String,
    pub name: String,
    pub color: Option<String>,
}

impl Tag {
    pub fn new(name: &str) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            name: name.to_string(),
            color: None,
        }
    }
}

// ============================================================================
// Citations
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Citations {
    pub citing: Vec<String>,
    pub references: Vec<String>,
}

// ============================================================================
// Database
// ============================================================================

#[derive(Clone)]
pub struct Database {
    conn: Arc<Mutex<Connection>>,
}

impl Database {
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let conn = Connection::open(path)?;
        conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON;")?;
        let db = Self {
            conn: Arc::new(Mutex::new(conn)),
        };
        db.init_schema()?;
        Ok(db)
    }

    fn init_schema(&self) -> Result<()> {
        let conn = self.conn.lock();
        conn.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS papers (
                id TEXT PRIMARY KEY,
                arxiv_id TEXT UNIQUE,
                title TEXT NOT NULL,
                authors TEXT NOT NULL,
                published TEXT NOT NULL,
                abstract_text TEXT NOT NULL,
                categories TEXT NOT NULL,
                parse_status TEXT NOT NULL DEFAULT 'pending',
                cited_by INTEGER NOT NULL DEFAULT 0,
                references_cnt INTEGER NOT NULL DEFAULT 0,
                doi TEXT,
                pdf_url TEXT,
                pdf_path TEXT,
                pdf_hash TEXT,
                plain_text TEXT,
                table_count INTEGER DEFAULT 0,
                figure_count INTEGER DEFAULT 0,
                word_count INTEGER DEFAULT 0,
                page_count INTEGER DEFAULT 0,
                reading_status TEXT DEFAULT 'unread',
                reading_started_at TEXT,
                reading_completed_at TEXT,
                embed_vector BLOB,
                created_at TEXT NOT NULL DEFAULT (datetime('now')),
                updated_at TEXT NOT NULL DEFAULT (datetime('now'))
            );

            CREATE VIRTUAL TABLE IF NOT EXISTS papers_fts USING fts5(
                title, abstract_text, authors, categories,
                content='papers',
                content_rowid='rowid'
            );

            CREATE TABLE IF NOT EXISTS research_gaps (
                id TEXT PRIMARY KEY,
                topic TEXT NOT NULL,
                session_id TEXT,
                gap_type TEXT NOT NULL,
                gap_title TEXT NOT NULL,
                gap_title_hash TEXT,
                novelty_score REAL DEFAULT 0.5,
                priority TEXT DEFAULT 'medium',
                paper_ids TEXT NOT NULL,
                created_at TEXT NOT NULL DEFAULT (datetime('now'))
            );

            CREATE TABLE IF NOT EXISTS evo_suggestions (
                id TEXT PRIMARY KEY,
                gap_id TEXT NOT NULL,
                direction TEXT NOT NULL,
                confidence REAL NOT NULL,
                paper_ids TEXT NOT NULL,
                status TEXT NOT NULL DEFAULT 'pending',
                created_at TEXT NOT NULL DEFAULT (datetime('now')),
                FOREIGN KEY (gap_id) REFERENCES research_gaps(id)
            );

            CREATE TABLE IF NOT EXISTS subscriptions (
                id TEXT PRIMARY KEY,
                query TEXT NOT NULL,
                name TEXT,
                categories TEXT,
                max_results INTEGER DEFAULT 10,
                check_interval_minutes INTEGER DEFAULT 60,
                last_check TEXT,
                last_results TEXT,
                enabled INTEGER DEFAULT 1,
                created_at TEXT NOT NULL DEFAULT (datetime('now'))
            );

            CREATE TABLE IF NOT EXISTS tags (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL UNIQUE,
                color TEXT,
                created_at TEXT NOT NULL DEFAULT (datetime('now'))
            );

            CREATE TABLE IF NOT EXISTS paper_tags (
                paper_id TEXT NOT NULL,
                tag_id TEXT NOT NULL,
                created_at TEXT NOT NULL DEFAULT (datetime('now')),
                PRIMARY KEY (paper_id, tag_id),
                FOREIGN KEY (paper_id) REFERENCES papers(id) ON DELETE CASCADE,
                FOREIGN KEY (tag_id) REFERENCES tags(id) ON DELETE CASCADE
            );

            CREATE TABLE IF NOT EXISTS citations (
                id TEXT PRIMARY KEY,
                source_id TEXT NOT NULL,
                target_id TEXT NOT NULL,
                created_at TEXT NOT NULL DEFAULT (datetime('now')),
                UNIQUE(source_id, target_id),
                FOREIGN KEY (source_id) REFERENCES papers(id) ON DELETE CASCADE,
                FOREIGN KEY (target_id) REFERENCES papers(id) ON DELETE CASCADE
            );

            CREATE TABLE IF NOT EXISTS paper_cache (
                uid TEXT PRIMARY KEY,
                data TEXT NOT NULL,
                cached_at TEXT NOT NULL DEFAULT (datetime('now'))
            );

            CREATE INDEX IF NOT EXISTS idx_papers_arxiv ON papers(arxiv_id);
            CREATE INDEX IF NOT EXISTS idx_papers_status ON papers(parse_status);
            CREATE INDEX IF NOT EXISTS idx_papers_published ON papers(published);
            CREATE INDEX IF NOT EXISTS idx_papers_reading ON papers(reading_status);
            CREATE INDEX IF NOT EXISTS idx_gaps_topic ON research_gaps(topic);
            CREATE INDEX IF NOT EXISTS idx_subscriptions_enabled ON subscriptions(enabled);
            "#,
        )?;

        conn.execute_batch(
            r#"
            CREATE TRIGGER IF NOT EXISTS papers_ai AFTER INSERT ON papers BEGIN
                INSERT INTO papers_fts(rowid, title, abstract_text, authors, categories)
                VALUES (NEW.rowid, NEW.title, NEW.abstract_text, NEW.authors, NEW.categories);
            END;

            CREATE TRIGGER IF NOT EXISTS papers_ad AFTER DELETE ON papers BEGIN
                INSERT INTO papers_fts(papers_fts, rowid, title, abstract_text, authors, categories)
                VALUES ('delete', OLD.rowid, OLD.title, OLD.abstract_text, OLD.authors, OLD.categories);
            END;

            CREATE TRIGGER IF NOT EXISTS papers_au AFTER UPDATE ON papers BEGIN
                INSERT INTO papers_fts(papers_fts, rowid, title, abstract_text, authors, categories)
                VALUES ('delete', OLD.rowid, OLD.title, OLD.abstract_text, OLD.authors, OLD.categories);
                INSERT INTO papers_fts(rowid, title, abstract_text, authors, categories)
                VALUES (NEW.rowid, NEW.title, NEW.abstract_text, NEW.authors, NEW.categories);
            END;
            "#,
        )?;

        Ok(())
    }

    pub fn insert_paper(&self, paper: &Paper) -> Result<()> {
        let conn = self.conn.lock();
        conn.execute(
            r#"INSERT INTO papers
               (id, arxiv_id, title, authors, published, abstract_text, categories,
                parse_status, cited_by, references_cnt, doi, pdf_url)
               VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)"#,
            params![
                paper.id,
                paper.arxiv_id,
                &paper.title,
                serde_json::to_string(&paper.authors)?,
                paper.published.to_rfc3339(),
                &paper.abstract_text,
                serde_json::to_string(&paper.categories)?,
                paper.parse_status.to_string(),
                paper.metadata.cited_by,
                paper.metadata.references,
                paper.metadata.doi,
                paper.metadata.pdf_url,
            ],
        )?;
        Ok(())
    }

    pub fn get_paper(&self, id: &str) -> Result<Paper> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare(
            "SELECT id, arxiv_id, title, authors, published, abstract_text, categories,
                    parse_status, cited_by, references_cnt, doi, pdf_url
             FROM papers WHERE id = ?1",
        )?;
        let paper = stmt.query_row([id], |row| Self::row_to_paper(row))?;
        Ok(paper)
    }

    pub fn get_paper_by_arxiv(&self, arxiv_id: &str) -> Result<Option<Paper>> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare(
            "SELECT id, arxiv_id, title, authors, published, abstract_text, categories,
                    parse_status, cited_by, references_cnt, doi, pdf_url
             FROM papers WHERE arxiv_id = ?1",
        )?;
        let result = stmt.query_row([arxiv_id], |row| {
            Ok(Some(Self::row_to_paper(row)?))
        });
        match result {
            Ok(p) => Ok(p),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    pub fn list_papers(
        &self,
        status: Option<ParseStatus>,
        limit: usize,
        offset: usize,
    ) -> Result<Vec<Paper>> {
        let conn = self.conn.lock();
        let sql = "SELECT id, arxiv_id, title, authors, published, abstract_text,
                   categories, parse_status, cited_by, references_cnt, doi, pdf_url
                   FROM papers";
        let sql_with_status = format!("{} WHERE parse_status = ?1 ORDER BY published DESC LIMIT ?2 OFFSET ?3", sql);
        let sql_no_status = format!("{} ORDER BY published DESC LIMIT ?1 OFFSET ?2", sql);

        let mut papers_vec: Vec<Paper> = Vec::new();

        match status {
            Some(s) => {
                let mut stmt = conn.prepare(&sql_with_status)?;
                let rows = stmt.query_map(
                    params![s.to_string(), limit as i64, offset as i64],
                    |row| Ok(Self::row_to_paper(row)),
                )?;
                for paper in rows {
                    papers_vec.push(paper??);
                }
            }
            None => {
                let mut stmt = conn.prepare(&sql_no_status)?;
                let rows = stmt.query_map(
                    params![limit as i64, offset as i64],
                    |row| Ok(Self::row_to_paper(row)),
                )?;
                for paper in rows {
                    papers_vec.push(paper??);
                }
            }
        }
        Ok(papers_vec)
    }

    pub fn update_paper_status(&self, id: &str, status: ParseStatus) -> Result<()> {
        let conn = self.conn.lock();
        conn.execute(
            "UPDATE papers SET parse_status = ?1, updated_at = datetime('now') WHERE id = ?2",
            params![status.to_string(), id],
        )?;
        Ok(())
    }

    pub fn delete_paper(&self, id: &str) -> Result<()> {
        let conn = self.conn.lock();
        conn.execute("DELETE FROM papers WHERE id = ?1", [id])?;
        Ok(())
    }

    /// Get embedding vector for a paper (bytes as f32 array).
    pub fn get_embedding(&self, paper_id: &str) -> Result<Option<Vec<f32>>> {
        let conn = self.conn.lock();
        let mut stmt = conn
            .prepare("SELECT embed_vector FROM papers WHERE id = ?1 AND embed_vector IS NOT NULL")?;
        let result = stmt.query_row([paper_id], |row| {
            let blob: Vec<u8> = row.get(0)?;
            Ok(blob)
        });
        match result {
            Ok(blob) => {
                let vec: Vec<f32> = blob
                    .chunks_exact(4)
                    .map(|chunk| f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]))
                    .collect();
                Ok(Some(vec))
            }
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(CoreError::Database(e)),
        }
    }

    /// List all paper IDs that have embeddings.
    pub fn list_papers_with_embeddings(&self) -> Result<Vec<String>> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare("SELECT id FROM papers WHERE embed_vector IS NOT NULL")?;
        let rows = stmt.query_map([], |row| row.get(0))?;
        let mut ids = Vec::new();
        for row in rows {
            ids.push(row?);
        }
        Ok(ids)
    }

    fn row_to_paper(row: &rusqlite::Row<'_>) -> rusqlite::Result<Paper> {
        let authors_str: String = row.get(3)?;
        let categories_str: String = row.get(6)?;
        let status_str: String = row.get(7)?;
        Ok(Paper {
            id: row.get(0)?,
            arxiv_id: row.get(1)?,
            title: row.get(2)?,
            authors: serde_json::from_str(&authors_str).unwrap_or_default(),
            published: DateTime::parse_from_rfc3339(&row.get::<_, String>(4)?)
                .map(|dt| dt.with_timezone(&Utc))
                .unwrap_or_else(|_| Utc::now()),
            abstract_text: row.get(5)?,
            categories: serde_json::from_str(&categories_str).unwrap_or_default(),
            parse_status: match status_str.as_str() {
                "pending" => ParseStatus::Pending,
                "parsing" => ParseStatus::Parsing,
                "done" => ParseStatus::Done,
                "failed" => ParseStatus::Failed,
                _ => ParseStatus::Pending,
            },
            metadata: PaperMetadata {
                cited_by: row.get(8)?,
                references: row.get(9)?,
                doi: row.get(10)?,
                pdf_url: row.get(11)?,
            },
        })
    }

    pub fn insert_gap(&self, gap: &ResearchGap) -> Result<()> {
        let conn = self.conn.lock();
        conn.execute(
            "INSERT INTO research_gaps (id, category, description, severity, paper_ids) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                gap.id,
                &gap.category,
                &gap.description,
                &gap.severity,
                serde_json::to_string(&gap.paper_ids)?,
            ],
        )?;
        Ok(())
    }

    pub fn list_gaps(&self, limit: usize, offset: usize) -> Result<Vec<ResearchGap>> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare(
            "SELECT id, category, description, severity, paper_ids FROM research_gaps ORDER BY created_at DESC LIMIT ?1 OFFSET ?2",
        )?;
        let rows = stmt.query_map(params![limit as i64, offset as i64], |row| {
            Ok(ResearchGap {
                id: row.get(0)?,
                category: row.get(1)?,
                description: row.get(2)?,
                severity: row.get(3)?,
                paper_ids: serde_json::from_str(&row.get::<_, String>(4)?).unwrap_or_default(),
            })
        })?;
        let mut gaps = Vec::new();
        for gap in rows {
            gaps.push(gap?);
        }
        Ok(gaps)
    }

    pub fn get_gap(&self, id: &str) -> Result<Option<ResearchGap>> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare(
            "SELECT id, category, description, severity, paper_ids FROM research_gaps WHERE id = ?1",
        )?;
        let result = stmt.query_row([id], |row| {
            Ok(Some(ResearchGap {
                id: row.get(0)?,
                category: row.get(1)?,
                description: row.get(2)?,
                severity: row.get(3)?,
                paper_ids: serde_json::from_str(&row.get::<_, String>(4)?).unwrap_or_default(),
            }))
        });
        match result {
            Ok(g) => Ok(g),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    pub fn delete_gap(&self, id: &str) -> Result<()> {
        let conn = self.conn.lock();
        conn.execute("DELETE FROM research_gaps WHERE id = ?1", [id])?;
        Ok(())
    }

    pub fn search_papers(&self, query: &str, limit: usize) -> Result<Vec<Paper>> {
        let conn = self.conn.lock();
        let pattern = format!("%{}%", query);
        let mut stmt = conn.prepare(
            "SELECT id, arxiv_id, title, authors, published, abstract_text, categories,
                    parse_status, cited_by, references_cnt, doi, pdf_url
             FROM papers
             WHERE title LIKE ?1 OR abstract_text LIKE ?1
             ORDER BY published DESC LIMIT ?2",
        )?;
        let rows = stmt.query_map(params![pattern, limit as i64], |row| {
            Ok(Self::row_to_paper(row))
        })?;
        let mut papers: Vec<Paper> = Vec::new();
        for paper in rows {
            papers.push(paper??);
        }
        Ok(papers)
    }

    pub fn count_papers(&self) -> Result<i64> {
        let conn = self.conn.lock();
        let count: i64 = conn.query_row("SELECT COUNT(*) FROM papers", [], |r| r.get(0))?;
        Ok(count)
    }

    pub fn stats(&self) -> Result<DbStats> {
        let conn = self.conn.lock();
        let total: i64 =
            conn.query_row("SELECT COUNT(*) FROM papers", [], |r| r.get(0))?;
        let pending: i64 = conn.query_row(
            "SELECT COUNT(*) FROM papers WHERE parse_status = 'pending'",
            [],
            |r| r.get(0),
        )?;
        let done: i64 = conn.query_row(
            "SELECT COUNT(*) FROM papers WHERE parse_status = 'done'",
            [],
            |r| r.get(0),
        )?;
        let gaps: i64 =
            conn.query_row("SELECT COUNT(*) FROM research_gaps", [], |r| r.get(0))?;
        Ok(DbStats {
            total,
            pending,
            done,
            gaps,
        })
    }

    pub fn search_papers_fts(&self, query: &str, limit: usize) -> Result<Vec<Paper>> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare(
            r#"SELECT p.id, p.arxiv_id, p.title, p.authors, p.published, p.abstract_text,
                      p.categories, p.parse_status, p.cited_by, p.references_cnt, p.doi, p.pdf_url
               FROM papers p
               INNER JOIN papers_fts fts ON p.rowid = fts.rowid
               WHERE papers_fts MATCH ?1
               ORDER BY rank
               LIMIT ?2"#
        )?;
        let rows = stmt.query_map(params![query, limit as i64], |row| {
            Self::row_to_paper(row)
        })?;
        let mut papers: Vec<Paper> = Vec::new();
        for paper in rows {
            papers.push(paper?);
        }
        Ok(papers)
    }

    pub fn update_paper_full_text(&self, id: &str, plain_text: &str, table_count: i32, figure_count: i32, word_count: i32, page_count: i32) -> Result<()> {
        let conn = self.conn.lock();
        conn.execute(
            "UPDATE papers SET plain_text = ?1, table_count = ?2, figure_count = ?3, word_count = ?4, page_count = ?5, updated_at = datetime('now') WHERE id = ?6",
            params![plain_text, table_count, figure_count, word_count, page_count, id],
        )?;
        Ok(())
    }

    pub fn insert_subscription(&self, sub: &Subscription) -> Result<()> {
        let conn = self.conn.lock();
        conn.execute(
            r#"INSERT INTO subscriptions (id, query, name, categories, max_results, check_interval_minutes, last_check, last_results, enabled)
               VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)"#,
            params![
                sub.id,
                &sub.query,
                &sub.name,
                serde_json::to_string(&sub.categories)?,
                sub.max_results,
                sub.check_interval_minutes as i32,
                sub.last_check.as_ref().map(|s| s.as_str()),
                sub.last_results.as_ref().map(|s| s.as_str()),
                sub.enabled as i32,
            ],
        )?;
        Ok(())
    }

    pub fn list_subscriptions(&self, enabled_only: bool) -> Result<Vec<Subscription>> {
        let conn = self.conn.lock();
        let sql = if enabled_only {
            "SELECT id, query, name, categories, max_results, check_interval_minutes, last_check, last_results, enabled FROM subscriptions WHERE enabled = 1"
        } else {
            "SELECT id, query, name, categories, max_results, check_interval_minutes, last_check, last_results, enabled FROM subscriptions"
        };
        let mut stmt = conn.prepare(sql)?;
        let rows = stmt.query_map([], |row| {
            let categories_str: Option<String> = row.get(3)?;
            let last_check: Option<String> = row.get(6)?;
            let last_results: Option<String> = row.get(7)?;
            Ok(Subscription {
                id: row.get(0)?,
                query: row.get(1)?,
                name: row.get(2)?,
                categories: categories_str.map(|s| serde_json::from_str(&s).unwrap_or_default()).unwrap_or_default(),
                max_results: row.get(4)?,
                check_interval_minutes: row.get::<_, i32>(5)? as u64,
                last_check,
                last_results,
                enabled: row.get::<_, i32>(8)? != 0,
            })
        })?;
        let mut subs = Vec::new();
        for sub in rows {
            subs.push(sub?);
        }
        Ok(subs)
    }

    pub fn delete_subscription(&self, id: &str) -> Result<()> {
        let conn = self.conn.lock();
        conn.execute("DELETE FROM subscriptions WHERE id = ?1", [id])?;
        Ok(())
    }

    pub fn update_subscription_last_check(&self, id: &str, last_check: &str, last_results: &str) -> Result<()> {
        let conn = self.conn.lock();
        conn.execute(
            "UPDATE subscriptions SET last_check = ?1, last_results = ?2 WHERE id = ?3",
            params![last_check, last_results, id],
        )?;
        Ok(())
    }

    pub fn insert_tag(&self, tag: &Tag) -> Result<()> {
        let conn = self.conn.lock();
        conn.execute(
            "INSERT INTO tags (id, name, color) VALUES (?1, ?2, ?3)",
            params![&tag.id, &tag.name, &tag.color],
        )?;
        Ok(())
    }

    pub fn list_tags(&self) -> Result<Vec<Tag>> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare("SELECT id, name, color FROM tags ORDER BY name")?;
        let rows = stmt.query_map([], |row| {
            Ok(Tag {
                id: row.get(0)?,
                name: row.get(1)?,
                color: row.get(2)?,
            })
        })?;
        let mut tags = Vec::new();
        for tag in rows {
            tags.push(tag?);
        }
        Ok(tags)
    }

    pub fn delete_tag(&self, id: &str) -> Result<()> {
        let conn = self.conn.lock();
        conn.execute("DELETE FROM tags WHERE id = ?1", [id])?;
        Ok(())
    }

    pub fn add_paper_tag(&self, paper_id: &str, tag_id: &str) -> Result<()> {
        let conn = self.conn.lock();
        conn.execute(
            "INSERT OR IGNORE INTO paper_tags (paper_id, tag_id) VALUES (?1, ?2)",
            params![paper_id, tag_id],
        )?;
        Ok(())
    }

    pub fn remove_paper_tag(&self, paper_id: &str, tag_id: &str) -> Result<()> {
        let conn = self.conn.lock();
        conn.execute(
            "DELETE FROM paper_tags WHERE paper_id = ?1 AND tag_id = ?2",
            params![paper_id, tag_id],
        )?;
        Ok(())
    }

    pub fn get_paper_tags(&self, paper_id: &str) -> Result<Vec<Tag>> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare(
            "SELECT t.id, t.name, t.color FROM tags t INNER JOIN paper_tags pt ON t.id = pt.tag_id WHERE pt.paper_id = ?1"
        )?;
        let rows = stmt.query_map([paper_id], |row| {
            Ok(Tag {
                id: row.get(0)?,
                name: row.get(1)?,
                color: row.get(2)?,
            })
        })?;
        let mut tags = Vec::new();
        for tag in rows {
            tags.push(tag?);
        }
        Ok(tags)
    }

    pub fn insert_citation(&self, source_id: &str, target_id: &str) -> Result<()> {
        let conn = self.conn.lock();
        let id = Uuid::new_v4().to_string();
        conn.execute(
            "INSERT OR IGNORE INTO citations (id, source_id, target_id) VALUES (?1, ?2, ?3)",
            params![&id, source_id, target_id],
        )?;
        Ok(())
    }

    pub fn get_citations(&self, paper_id: &str) -> Result<Citations> {
        let conn = self.conn.lock();
        let mut citing_stmt = conn.prepare("SELECT source_id FROM citations WHERE target_id = ?1")?;
        let citing: Vec<String> = citing_stmt.query_map([paper_id], |row| row.get(0))?
            .filter_map(|r| r.ok())
            .collect();

        let mut refs_stmt = conn.prepare("SELECT target_id FROM citations WHERE source_id = ?1")?;
        let references: Vec<String> = refs_stmt.query_map([paper_id], |row| row.get(0))?
            .filter_map(|r| r.ok())
            .collect();

        Ok(Citations { citing, references })
    }

    pub fn find_similar_by_vector(&self, embedding: &[f32], limit: usize) -> Result<Vec<(String, f32)>> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare(
            "SELECT id, embed_vector FROM papers WHERE embed_vector IS NOT NULL"
        )?;
        let rows = stmt.query_map([], |row| {
            let id: String = row.get(0)?;
            let blob: Vec<u8> = row.get(1)?;
            Ok((id, blob))
        })?;

        let mut results: Vec<(String, f32)> = Vec::new();
        for row in rows {
            let (id, blob) = row?;
            if blob.len() != embedding.len() * 4 {
                continue;
            }
            let stored: &[f32] = unsafe {
                std::slice::from_raw_parts(blob.as_ptr() as *const f32, embedding.len())
            };
            let similarity = cosine_similarity(embedding, stored);
            results.push((id, similarity));
        }

        results.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        results.truncate(limit);
        Ok(results)
    }

    pub fn set_paper_embedding(&self, paper_id: &str, embedding: &[f32]) -> Result<()> {
        let conn = self.conn.lock();
        let blob: Vec<u8> = embedding.iter()
            .flat_map(|f| f.to_le_bytes())
            .collect();
        conn.execute(
            "UPDATE papers SET embed_vector = ?1 WHERE id = ?2",
            params![blob, paper_id],
        )?;
        Ok(())
    }
}

fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    let dot: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm_a == 0.0 || norm_b == 0.0 {
        0.0
    } else {
        dot / (norm_a * norm_b)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DbStats {
    pub total: i64,
    pub pending: i64,
    pub done: i64,
    pub gaps: i64,
}

// ============================================================================
// Rate Limiter
// ============================================================================

#[derive(Clone)]
pub struct RateLimiter {
    state: Arc<Mutex<RateLimiterState>>,
    default_config: RateLimitConfig,
}

#[derive(Clone, Copy, Debug)]
pub struct RateLimitConfig {
    pub max_per_second: f64,
    pub max_per_minute: f64,
    pub max_per_hour: f64,
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            max_per_second: 10.0,
            max_per_minute: 500.0,
            max_per_hour: 5000.0,
        }
    }
}

struct RateLimiterState {
    limiters: HashMap<String, Limiter>,
}

struct Limiter {
    second_start: f64,
    second_count: usize,
    minute_start: f64,
    minute_count: usize,
    hour_start: f64,
    hour_count: usize,
    config: RateLimitConfig,
}

impl RateLimiter {
    pub fn new() -> Self {
        Self {
            state: Arc::new(Mutex::new(RateLimiterState {
                limiters: HashMap::new(),
            })),
            default_config: RateLimitConfig::default(),
        }
    }

    pub fn get_or_create(&self, endpoint: &str) -> RateLimiterHandle {
        let mut state = self.state.lock();
        if !state.limiters.contains_key(endpoint) {
            state.limiters.insert(
                endpoint.to_string(),
                Limiter {
                    second_start: 0.0,
                    second_count: 0,
                    minute_start: 0.0,
                    minute_count: 0,
                    hour_start: 0.0,
                    hour_count: 0,
                    config: self.default_config,
                },
            );
        }
        RateLimiterHandle {
            endpoint: endpoint.to_string(),
            state: self.state.clone(),
        }
    }
}

pub struct RateLimiterHandle {
    endpoint: String,
    state: Arc<Mutex<RateLimiterState>>,
}

impl RateLimiterHandle {
    /// Check if request is allowed without blocking
    pub fn can(&self) -> bool {
        let now = Self::now();
        let mut state = self.state.lock();
        let limiter = state.limiters.get_mut(&self.endpoint).unwrap();
        Self::sliding_window_check(limiter, now) == 0.0
    }

    /// Wait until a slot is available, returns approximate wait time
    pub fn wait_for_slot(&self) -> f64 {
        let now = Self::now();
        let mut state = self.state.lock();
        let limiter = state.limiters.get_mut(&self.endpoint).unwrap();
        let delay = Self::sliding_window_check(limiter, now);
        if delay > 0.0 {
            std::thread::sleep(std::time::Duration::from_secs_f64(delay));
        }
        limiter.second_count += 1;
        limiter.minute_count += 1;
        limiter.hour_count += 1;
        delay
    }

    /// Reset all counters for this endpoint
    pub fn reset(&self) {
        let mut state = self.state.lock();
        if let Some(limiter) = state.limiters.get_mut(&self.endpoint) {
            let now = Self::now();
            limiter.second_start = now;
            limiter.second_count = 0;
            limiter.minute_start = now;
            limiter.minute_count = 0;
            limiter.hour_start = now;
            limiter.hour_count = 0;
        }
    }

    fn sliding_window_check(limiter: &mut Limiter, now: f64) -> f64 {
        // Advance windows if needed
        if now - limiter.second_start >= 1.0 {
            limiter.second_start = now;
            limiter.second_count = 0;
        }
        if now - limiter.minute_start >= 60.0 {
            limiter.minute_start = now;
            limiter.minute_count = 0;
        }
        if now - limiter.hour_start >= 3600.0 {
            limiter.hour_start = now;
            limiter.hour_count = 0;
        }

        let second_delay = if (limiter.second_count as f64) >= limiter.config.max_per_second {
            1.0 - (now - limiter.second_start)
        } else {
            0.0
        };

        let minute_delay = if (limiter.minute_count as f64) >= limiter.config.max_per_minute {
            60.0 - (now - limiter.minute_start)
        } else {
            0.0
        };

        let hour_delay = if (limiter.hour_count as f64) >= limiter.config.max_per_hour {
            3600.0 - (now - limiter.hour_start)
        } else {
            0.0
        };

        second_delay.max(minute_delay).max(hour_delay)
    }

    fn now() -> f64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs_f64()
    }
}

impl Default for RateLimiter {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_paper_new() {
        let paper = Paper::new(
            Some("2301.00001".into()),
            "Test Paper".into(),
            "Abstract text".into(),
        );
        assert!(!paper.id.is_empty());
        assert_eq!(paper.parse_status, ParseStatus::Pending);
    }

    #[test]
    fn test_rate_limiter_basic() {
        let rl = RateLimiter::new();
        let handle = rl.get_or_create("test");
        assert!(handle.can());
        handle.reset();
        assert!(handle.can());
    }

    #[test]
    fn test_rate_limiter_sliding_window() {
        let rl = RateLimiter::new();
        let handle = rl.get_or_create("sliding_test");
        handle.reset();
        for _ in 0..100 {
            if !handle.can() {
                handle.wait_for_slot();
            }
        }
    }

    #[test]
    fn test_parse_status_display() {
        assert_eq!(ParseStatus::Pending.to_string(), "pending");
        assert_eq!(ParseStatus::Done.to_string(), "done");
    }
}
