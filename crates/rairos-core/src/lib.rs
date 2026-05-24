//! Rairos Core — 核心数据结构和数据库操作
//!
//! Paper dataclass, SQLite database, rate limiter.
//! 完全用 Rust 重写，替代 Python 版本。

use chrono::{DateTime, Utc};
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use sqlx::{Row, SqlitePool};
use sqlx::sqlite::SqliteRow;
use sqlx::sqlite::{SqlitePoolOptions, SqliteConnectOptions};
use std::collections::HashMap;
use rustc_hash::FxHashSet;
use std::path::Path;
use std::sync::{Arc, LazyLock};
use thiserror::Error;
use tokio::runtime::Runtime;
use uuid::Uuid;

pub mod constants;
pub mod progress_tracker;
pub mod identifiers;
pub mod logging_utils;
pub mod i18n;
pub mod basics;
pub use basics::safe_uid;

// Re-exported orphan utility crates
pub mod retry;
pub mod cache;
pub mod core_utils;
pub mod observability;
pub mod auth;
pub mod db_migrate;
pub mod db_optimize;
pub mod crossover;
pub mod prelude;

// Re-export key types from merged utility modules
pub use retry::{RetryConfig, retry_with_backoff, CircuitOpen};
pub use cache::{CacheConfig, get_cached, set_cached};
pub use auth::{AuthError, User, is_auth_enabled, verify_login};
pub use crossover::{CodeCapsuleGene, CapsuleGene};

// ============================================================================
// Error Types
// ============================================================================

#[derive(Error, Debug)]
pub enum CoreError {
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),
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
// DbValue - sqlx equivalent of rusqlite::types::Value for query_raw
// ============================================================================

#[derive(Debug, Clone)]
pub enum DbValue {
    Null,
    Integer(i64),
    Real(f64),
    Text(String),
    Blob(Vec<u8>),
}

impl DbValue {
    pub fn from_sqlx(row: &SqliteRow, index: usize) -> Option<Self> {
        // Try each type - sqlx Row doesn't have a unified get method like rusqlite
        if let Ok(v) = row.try_get::<i64, _>(index) {
            return Some(DbValue::Integer(v));
        }
        if let Ok(v) = row.try_get::<f64, _>(index) {
            return Some(DbValue::Real(v));
        }
        if let Ok(v) = row.try_get::<String, _>(index) {
            return Some(DbValue::Text(v));
        }
        if let Ok(v) = row.try_get::<Vec<u8>, _>(index) {
            return Some(DbValue::Blob(v));
        }
        if let Ok(v) = row.try_get::<Option<String>, _>(index) {
            return match v {
                Some(s) => Some(DbValue::Text(s)),
                None => Some(DbValue::Null),
            };
        }
        None
    }
}

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

impl Default for Paper {
    fn default() -> Self {
        Self {
            id: String::new(),
            arxiv_id: None,
            title: String::new(),
            authors: Vec::new(),
            published: DateTime::default(),
            abstract_text: String::new(),
            categories: Vec::new(),
            parse_status: ParseStatus::Pending,
            metadata: PaperMetadata::default(),
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PaperMetadata {
    pub cited_by: usize,
    pub references: usize,
    pub doi: Option<String>,
    pub pdf_url: Option<String>,
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
        Self::with_metadata(
            arxiv_id,
            title,
            abstract_text,
            Vec::new(),
            Vec::new(),
            PaperMetadata::default(),
        )
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
    pub topic: String,
    pub session_id: Option<String>,
    pub gap_type: String,
    pub gap_title: String,
    pub gap_title_hash: Option<String>,
    pub category: String,
    pub description: String,
    pub severity: String,
    pub novelty_score: f64,
    pub priority: String,
    pub paper_ids: Vec<String>,
    pub created_at: String,
}

impl ResearchGap {
    pub fn new(
        topic: &str,
        gap_type: &str,
        gap_title: &str,
        category: &str,
        description: &str,
        severity: &str,
        priority: &str,
    ) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            topic: topic.to_string(),
            session_id: None,
            gap_type: gap_type.to_string(),
            gap_title: gap_title.to_string(),
            gap_title_hash: None,
            category: category.to_string(),
            description: description.to_string(),
            severity: severity.to_string(),
            novelty_score: 0.5,
            priority: priority.to_string(),
            paper_ids: Vec::new(),
            created_at: chrono::Utc::now().to_rfc3339(),
        }
    }

    pub fn new_simple(category: &str, description: &str, severity: &str) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            topic: String::new(),
            session_id: None,
            gap_type: category.to_string(),
            gap_title: String::new(),
            gap_title_hash: None,
            category: String::new(),
            description: description.to_string(),
            severity: severity.to_string(),
            novelty_score: 0.5,
            priority: "medium".to_string(),
            paper_ids: Vec::new(),
            created_at: chrono::Utc::now().to_rfc3339(),
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobQueueEntry {
    pub id: i64,
    pub paper_id: String,
    pub job_type: String,
    pub status: String,
    pub priority: i64,
    pub created_at: String,
    pub started_at: Option<String>,
    pub completed_at: Option<String>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DedupLogEntry {
    pub id: String,
    pub target_id: String,
    pub duplicate_id: String,
    pub keep_policy: String,
    pub created_at: String,
}

// ============================================================================
// PaperCodeTrace
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaperCodeTrace {
    pub id: i64,
    pub paper_id: String,
    pub code_path: String,
    pub module_name: String,
    pub framework: String,
    pub total_code_lines: i64,
    pub tagged_lines: i64,
    pub untagged_ranges: Vec<serde_json::Value>,
    pub unreferenced_sources: Vec<serde_json::Value>,
    pub paper_section_refs: Vec<serde_json::Value>,
    pub gap_ids: Vec<serde_json::Value>,
    pub benchmark_pass_rate: Option<f64>,
    pub created_at: String,
}

impl PaperCodeTrace {
    // Use rusqlite::Row accessor (not available as trait method on Row in this crate version)
}

// ============================================================================
// Search Result (from rairos-db, used by rairos-db-py bindings)
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
// Database
// ============================================================================

#[derive(Clone)]
pub struct Database {
    pool: Arc<Mutex<SqlitePool>>,
    rt: Arc<Runtime>,
}

impl Database {
    /// Helper async function to create the pool
    async fn create_pool_async(options: SqliteConnectOptions) -> std::result::Result<SqlitePool, sqlx::Error> {
        SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(options)
            .await
    }
    
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let rt = Runtime::new().map_err(CoreError::Io)?;
        let path_str = path.as_ref().to_string_lossy();
        // Use create_if_missing to create the database file if it doesn't exist
        let options = SqliteConnectOptions::new()
            .filename(&*path_str)
            .create_if_missing(true)
            .pragma("journal_mode", "WAL")
            .pragma("foreign_keys", "ON");
        
        // Create the pool synchronously using the runtime
        let pool = rt.block_on(Self::create_pool_async(options)).map_err(CoreError::Database)?;
        
        let db = Self {
            pool: Arc::new(Mutex::new(pool)),
            rt: Arc::new(rt),
        };
        db.init_schema()?;
        Ok(db)
    }

    fn init_schema(&self) -> Result<()> {
        let rt = self.rt.clone();
        let pool = self.pool.lock().clone();
        rt.block_on(async move {
            // Create tables
            sqlx::query(
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
                )
                "#,
            )
            .execute(&pool).await?;

            sqlx::query(
                r#"
                CREATE VIRTUAL TABLE IF NOT EXISTS papers_fts USING fts5(
                    title, abstract_text, authors, categories,
                    content='papers',
                    content_rowid='rowid'
                )
                "#,
            )
            .execute(&pool).await?;

            sqlx::query(
                r#"
                CREATE TABLE IF NOT EXISTS research_gaps (
                    id TEXT PRIMARY KEY,
                    topic TEXT NOT NULL DEFAULT '',
                    session_id TEXT,
                    gap_type TEXT NOT NULL DEFAULT '',
                    gap_title TEXT NOT NULL DEFAULT '',
                    gap_title_hash TEXT,
                    category TEXT DEFAULT '',
                    description TEXT DEFAULT '',
                    severity TEXT DEFAULT 'medium',
                    novelty_score REAL DEFAULT 0.5,
                    priority TEXT DEFAULT 'medium',
                    paper_ids TEXT NOT NULL,
                    created_at TEXT NOT NULL DEFAULT (datetime('now'))
                )
                "#,
            )
            .execute(&pool).await?;

            sqlx::query(
                r#"
                CREATE TABLE IF NOT EXISTS evo_suggestions (
                    id TEXT PRIMARY KEY,
                    gap_id TEXT NOT NULL,
                    direction TEXT NOT NULL,
                    confidence REAL NOT NULL,
                    paper_ids TEXT NOT NULL,
                    status TEXT NOT NULL DEFAULT 'pending',
                    created_at TEXT NOT NULL DEFAULT (datetime('now')),
                    FOREIGN KEY (gap_id) REFERENCES research_gaps(id)
                )
                "#,
            )
            .execute(&pool).await?;

            sqlx::query(
                r#"
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
                )
                "#,
            )
            .execute(&pool).await?;

            sqlx::query(
                r#"
                CREATE TABLE IF NOT EXISTS tags (
                    id TEXT PRIMARY KEY,
                    name TEXT NOT NULL UNIQUE,
                    color TEXT,
                    created_at TEXT NOT NULL DEFAULT (datetime('now'))
                )
                "#,
            )
            .execute(&pool).await?;

            sqlx::query(
                r#"
                CREATE TABLE IF NOT EXISTS paper_tags (
                    paper_id TEXT NOT NULL,
                    tag_id TEXT NOT NULL,
                    created_at TEXT NOT NULL DEFAULT (datetime('now')),
                    PRIMARY KEY (paper_id, tag_id),
                    FOREIGN KEY (paper_id) REFERENCES papers(id) ON DELETE CASCADE,
                    FOREIGN KEY (tag_id) REFERENCES tags(id) ON DELETE CASCADE
                )
                "#,
            )
            .execute(&pool).await?;

            sqlx::query(
                r#"
                CREATE TABLE IF NOT EXISTS citations (
                    id TEXT PRIMARY KEY,
                    source_id TEXT NOT NULL,
                    target_id TEXT NOT NULL,
                    created_at TEXT NOT NULL DEFAULT (datetime('now')),
                    UNIQUE(source_id, target_id),
                    FOREIGN KEY (source_id) REFERENCES papers(id) ON DELETE CASCADE,
                    FOREIGN KEY (target_id) REFERENCES papers(id) ON DELETE CASCADE
                )
                "#,
            )
            .execute(&pool).await?;

            sqlx::query(
                r#"
                CREATE TABLE IF NOT EXISTS paper_cache (
                    uid TEXT PRIMARY KEY,
                    data TEXT NOT NULL,
                    cached_at TEXT NOT NULL DEFAULT (datetime('now'))
                )
                "#,
            )
            .execute(&pool).await?;

            sqlx::query(
                r#"
                CREATE TABLE IF NOT EXISTS job_queue (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    paper_id TEXT NOT NULL,
                    job_type TEXT NOT NULL DEFAULT 'parse',
                    status TEXT NOT NULL DEFAULT 'queued',
                    priority INTEGER NOT NULL DEFAULT 5,
                    created_at TEXT NOT NULL DEFAULT (datetime('now')),
                    started_at TEXT,
                    completed_at TEXT,
                    error TEXT
                )
                "#,
            )
            .execute(&pool).await?;

            sqlx::query(
                r#"
                CREATE TABLE IF NOT EXISTS dedup_log (
                    id TEXT PRIMARY KEY,
                    target_id TEXT NOT NULL,
                    duplicate_id TEXT NOT NULL,
                    keep_policy TEXT NOT NULL,
                    created_at TEXT NOT NULL DEFAULT (datetime('now'))
                )
                "#,
            )
            .execute(&pool).await?;

            sqlx::query(
                r#"
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
                )
                "#,
            )
            .execute(&pool).await?;

            // Create indexes
            sqlx::query("CREATE INDEX IF NOT EXISTS idx_papers_arxiv ON papers(arxiv_id)")
                .execute(&pool).await?;
            sqlx::query("CREATE INDEX IF NOT EXISTS idx_papers_status ON papers(parse_status)")
                .execute(&pool).await?;
            sqlx::query("CREATE INDEX IF NOT EXISTS idx_papers_published ON papers(published)")
                .execute(&pool).await?;
            sqlx::query("CREATE INDEX IF NOT EXISTS idx_papers_reading ON papers(reading_status)")
                .execute(&pool).await?;
            sqlx::query("CREATE INDEX IF NOT EXISTS idx_gaps_topic ON research_gaps(topic)")
                .execute(&pool).await?;
            sqlx::query("CREATE INDEX IF NOT EXISTS idx_subscriptions_enabled ON subscriptions(enabled)")
                .execute(&pool).await?;

            sqlx::query(
                r#"
                CREATE TABLE IF NOT EXISTS parse_history (
                    id             INTEGER PRIMARY KEY AUTOINCREMENT,
                    paper_id       TEXT NOT NULL,
                    attempted_at   TEXT NOT NULL DEFAULT (datetime('now')),
                    duration_sec   REAL,
                    status         TEXT NOT NULL,
                    error          TEXT DEFAULT '',
                    parse_version  INTEGER,
                    pdf_hash       TEXT,
                    file_size      INTEGER,
                    FOREIGN KEY (paper_id) REFERENCES papers(id) ON DELETE CASCADE
                )
                "#,
            )
            .execute(&pool).await?;

            sqlx::query(
                r#"
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
                    created_at    TEXT NOT NULL DEFAULT (datetime('now')),
                    FOREIGN KEY (paper_id) REFERENCES papers(id) ON DELETE CASCADE
                )
                "#,
            )
            .execute(&pool).await?;

            sqlx::query(
                r#"
                CREATE TABLE IF NOT EXISTS arxiv_search_cache (
                    query_hash   TEXT PRIMARY KEY,
                    query        TEXT NOT NULL,
                    results_json TEXT NOT NULL,
                    created_at   TEXT NOT NULL DEFAULT (datetime('now')),
                    hit_count    INTEGER DEFAULT 1
                )
                "#,
            )
            .execute(&pool).await?;

            sqlx::query("CREATE INDEX IF NOT EXISTS idx_parse_history_paper ON parse_history(paper_id)")
                .execute(&pool).await?;
            sqlx::query("CREATE INDEX IF NOT EXISTS idx_experiment_tables_paper ON experiment_tables(paper_id)")
                .execute(&pool).await?;

            // Create triggers
            sqlx::query(
                r#"
                CREATE TRIGGER IF NOT EXISTS papers_ai AFTER INSERT ON papers BEGIN
                    INSERT INTO papers_fts(rowid, title, abstract_text, authors, categories)
                    VALUES (NEW.rowid, NEW.title, NEW.abstract_text, NEW.authors, NEW.categories);
                END
                "#,
            )
            .execute(&pool).await?;

            sqlx::query(
                r#"
                CREATE TRIGGER IF NOT EXISTS papers_ad AFTER DELETE ON papers BEGIN
                    INSERT INTO papers_fts(papers_fts, rowid, title, abstract_text, authors, categories)
                    VALUES ('delete', OLD.rowid, OLD.title, OLD.abstract_text, OLD.authors, OLD.categories);
                END
                "#,
            )
            .execute(&pool).await?;

            sqlx::query(
                r#"
                CREATE TRIGGER IF NOT EXISTS papers_au AFTER UPDATE ON papers BEGIN
                    INSERT INTO papers_fts(papers_fts, rowid, title, abstract_text, authors, categories)
                    VALUES ('delete', OLD.rowid, OLD.title, OLD.abstract_text, OLD.authors, OLD.categories);
                    INSERT INTO papers_fts(rowid, title, abstract_text, authors, categories)
                    VALUES (NEW.rowid, NEW.title, NEW.abstract_text, NEW.authors, NEW.categories);
                END
                "#,
            )
            .execute(&pool).await?;

            Ok(())
        })
    }

    pub fn insert_paper(&self, paper: &Paper) -> Result<()> {
        let rt = self.rt.clone();
        let pool = self.pool.lock().clone();
        let authors_json = serde_json::to_string(&paper.authors)?;
        let categories_json = serde_json::to_string(&paper.categories)?;
        let published = paper.published.to_rfc3339();
        let status = paper.parse_status.to_string();
        rt.block_on(async move {
            sqlx::query(
                r#"INSERT INTO papers
                   (id, arxiv_id, title, authors, published, abstract_text, categories,
                    parse_status, cited_by, references_cnt, doi, pdf_url)
                   VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)"#,
            )
            .bind(&paper.id)
            .bind(&paper.arxiv_id)
            .bind(&paper.title)
            .bind(&authors_json)
            .bind(&published)
            .bind(&paper.abstract_text)
            .bind(&categories_json)
            .bind(&status)
            .bind(paper.metadata.cited_by as i64)
            .bind(paper.metadata.references as i64)
            .bind(&paper.metadata.doi)
            .bind(&paper.metadata.pdf_url)
            .execute(&pool)
            .await?;
            Ok(())
        })
    }

    pub fn get_paper(&self, id: &str) -> Result<Paper> {
        let rt = self.rt.clone();
        let pool = self.pool.lock().clone();
        let id = id.to_string();
        rt.block_on(async move {
            let row = sqlx::query(
                "SELECT id, arxiv_id, title, authors, published, abstract_text, categories,
                        parse_status, cited_by, references_cnt, doi, pdf_url
                 FROM papers WHERE id = ?1",
            )
            .bind(&id)
            .fetch_one(&pool)
            .await?;
            Self::row_to_paper(&row)
        })
    }

    pub fn get_paper_by_arxiv(&self, arxiv_id: &str) -> Result<Option<Paper>> {
        let rt = self.rt.clone();
        let pool = self.pool.lock().clone();
        let arxiv_id = arxiv_id.to_string();
        rt.block_on(async move {
            let result = sqlx::query(
                "SELECT id, arxiv_id, title, authors, published, abstract_text, categories,
                        parse_status, cited_by, references_cnt, doi, pdf_url
                 FROM papers WHERE arxiv_id = ?1",
            )
            .bind(&arxiv_id)
            .fetch_optional(&pool)
            .await?;
            match result {
                Some(row) => Ok(Some(Self::row_to_paper(&row)?)),
                None => Ok(None),
            }
        })
    }

    pub fn list_papers(
        &self,
        status: Option<ParseStatus>,
        limit: usize,
        offset: usize,
    ) -> Result<Vec<Paper>> {
        let rt = self.rt.clone();
        let pool = self.pool.lock().clone();
        let status_str = status.map(|s| s.to_string());

        rt.block_on(async move {
            let sql = "SELECT id, arxiv_id, title, authors, published, abstract_text,
                       categories, parse_status, cited_by, references_cnt, doi, pdf_url
                       FROM papers";
            let sql_with_status = format!(
                "{} WHERE parse_status = ?1 ORDER BY published DESC LIMIT ?2 OFFSET ?3",
                sql
            );
            let sql_no_status = format!("{} ORDER BY published DESC LIMIT ?1 OFFSET ?2", sql);

            let mut papers_vec: Vec<Paper> = Vec::with_capacity(limit);

            match &status_str {
                Some(s) => {
                    let rows = sqlx::query(&sql_with_status)
                        .bind(s)
                        .bind(limit as i64)
                        .bind(offset as i64)
                        .fetch_all(&pool)
                        .await?;
                    for row in rows {
                        papers_vec.push(Self::row_to_paper(&row)?);
                    }
                }
                None => {
                    let rows = sqlx::query(&sql_no_status)
                        .bind(limit as i64)
                        .bind(offset as i64)
                        .fetch_all(&pool)
                        .await?;
                    for row in rows {
                        papers_vec.push(Self::row_to_paper(&row)?);
                    }
                }
            }
            Ok(papers_vec)
        })
    }

    pub fn update_paper_status(&self, id: &str, status: ParseStatus) -> Result<()> {
        let rt = self.rt.clone();
        let pool = self.pool.lock().clone();
        let id = id.to_string();
        let status_str = status.to_string();
        rt.block_on(async move {
            sqlx::query("UPDATE papers SET parse_status = ?1, updated_at = datetime('now') WHERE id = ?2")
                .bind(&status_str)
                .bind(&id)
                .execute(&pool)
                .await?;
            Ok(())
        })
    }

    pub fn delete_paper(&self, id: &str) -> Result<()> {
        let rt = self.rt.clone();
        let pool = self.pool.lock().clone();
        let id = id.to_string();
        rt.block_on(async move {
            sqlx::query("DELETE FROM papers WHERE id = ?1")
                .bind(&id)
                .execute(&pool)
                .await?;
            Ok(())
        })
    }

    /// Check if a paper exists by ID.
    pub fn paper_exists(&self, id: &str) -> bool {
        let rt = self.rt.clone();
        let pool = self.pool.lock().clone();
        let id = id.to_string();
        rt.block_on(async move {
            sqlx::query("SELECT 1 FROM papers WHERE id = ?1")
                .bind(&id)
                .fetch_optional(&pool)
                .await
                .map(|opt| opt.is_some())
                .unwrap_or(false)
        })
    }

    /// Get plain_text for a paper (needed for citation extraction, etc.).
    pub fn get_paper_plain_text(&self, id: &str) -> Result<Option<String>> {
        let rt = self.rt.clone();
        let pool = self.pool.lock().clone();
        let id = id.to_string();
        rt.block_on(async move {
            let result = sqlx::query("SELECT plain_text FROM papers WHERE id = ?1")
                .bind(&id)
                .fetch_optional(&pool)
                .await?;
            match result {
                Some(row) => Ok(Some(row.try_get(0)?)),
                None => Ok(None),
            }
        })
    }

    /// Merge duplicate papers into the primary. Copies non-empty fields from
    /// duplicates where the primary has empty/null values. Transfers tags,
    /// citations, queued jobs, then deletes the duplicates.
    /// Returns true if any duplicates were merged.
    pub fn merge_papers(&self, primary_id: &str, duplicate_ids: &[&str]) -> Result<bool> {
        let rt = self.rt.clone();
        let pool = self.pool.lock().clone();
        let primary_id = primary_id.to_string();
        let duplicate_ids: Vec<String> = duplicate_ids.iter().map(|&s| s.to_string()).collect();

        rt.block_on(async move {
            // Check primary exists
            let primary_exists = sqlx::query("SELECT 1 FROM papers WHERE id = ?1")
                .bind(&primary_id)
                .fetch_optional(&pool)
                .await?
                .is_some();
            if !primary_exists {
                return Ok(false);
            }

            // Batch existence check: single query for all duplicates
            let mut existing = std::collections::HashSet::new();
            for chunk in duplicate_ids.chunks(999) {
                let placeholders: Vec<String> = chunk.iter().enumerate()
                    .map(|(i, _)| format!("?{}", i + 1)).collect();
                let sql = format!("SELECT id FROM papers WHERE id IN ({})", placeholders.join(","));
                let mut q = sqlx::query(&sql);
                for id in chunk {
                    q = q.bind(id);
                }
                let rows = q.fetch_all(&pool).await?;
                for row in rows {
                    let id: String = row.try_get(0)?;
                    existing.insert(id);
                }
            }

            let valid_dup_ids: Vec<String> = duplicate_ids.into_iter()
                .filter(|id| id != &primary_id && existing.contains(id))
                .collect();

            if valid_dup_ids.is_empty() {
                return Ok(false);
            }

            // Execute all operations under a single transaction
            let mut tx = pool.begin().await?;

            for dup_id in &valid_dup_ids {
                // ── Single combined UPDATE for all text fields ──
                sqlx::query(
                    "UPDATE papers SET \
                     title = COALESCE(NULLIF(title, ''), (SELECT title FROM papers WHERE id = ?1)), \
                     authors = COALESCE(NULLIF(authors, ''), (SELECT authors FROM papers WHERE id = ?1)), \
                     abstract_text = COALESCE(NULLIF(abstract_text, ''), (SELECT abstract_text FROM papers WHERE id = ?1)), \
                     doi = COALESCE(NULLIF(doi, ''), (SELECT doi FROM papers WHERE id = ?1)), \
                     pdf_url = COALESCE(NULLIF(pdf_url, ''), (SELECT pdf_url FROM papers WHERE id = ?1)), \
                     pdf_path = COALESCE(NULLIF(pdf_path, ''), (SELECT pdf_path FROM papers WHERE id = ?1)), \
                     pdf_hash = COALESCE(NULLIF(pdf_hash, ''), (SELECT pdf_hash FROM papers WHERE id = ?1)), \
                     categories = COALESCE(NULLIF(categories, ''), (SELECT categories FROM papers WHERE id = ?1)), \
                     plain_text = COALESCE(NULLIF(plain_text, ''), (SELECT plain_text FROM papers WHERE id = ?1)) \
                     WHERE id = ?2",
                )
                .bind(dup_id)
                .bind(&primary_id)
                .execute(&mut *tx)
                .await?;

                // ── Single combined UPDATE for all integer fields ──
                sqlx::query(
                    "UPDATE papers SET \
                     cited_by = CASE WHEN cited_by = 0 THEN (SELECT cited_by FROM papers WHERE id = ?1) ELSE cited_by END, \
                     references_cnt = CASE WHEN references_cnt = 0 THEN (SELECT references_cnt FROM papers WHERE id = ?1) ELSE references_cnt END, \
                     table_count = CASE WHEN table_count IS NULL OR table_count = 0 THEN (SELECT table_count FROM papers WHERE id = ?1) ELSE table_count END, \
                     figure_count = CASE WHEN figure_count IS NULL OR figure_count = 0 THEN (SELECT figure_count FROM papers WHERE id = ?1) ELSE figure_count END, \
                     word_count = CASE WHEN word_count IS NULL OR word_count = 0 THEN (SELECT word_count FROM papers WHERE id = ?1) ELSE word_count END, \
                     page_count = CASE WHEN page_count IS NULL OR page_count = 0 THEN (SELECT page_count FROM papers WHERE id = ?1) ELSE page_count END \
                     WHERE id = ?2",
                )
                .bind(dup_id)
                .bind(&primary_id)
                .execute(&mut *tx)
                .await?;

                // ── Parse status: copy if primary is 'pending' and dup is not ──
                sqlx::query(
                    "UPDATE papers SET parse_status = (SELECT parse_status FROM papers WHERE id = ?1) \
                     WHERE id = ?2 AND parse_status = 'pending' \
                     AND (SELECT parse_status FROM papers WHERE id = ?1) != 'pending'",
                )
                .bind(dup_id)
                .bind(&primary_id)
                .execute(&mut *tx)
                .await?;

                // ── Transfer tags from duplicate to primary ──
                sqlx::query(
                    "INSERT OR IGNORE INTO paper_tags (paper_id, tag_id) \
                     SELECT ?1, tag_id FROM paper_tags WHERE paper_id = ?2",
                )
                .bind(&primary_id)
                .bind(dup_id)
                .execute(&mut *tx)
                .await?;

                // ── Transfer queued jobs from duplicate to primary ──
                sqlx::query(
                    "UPDATE job_queue SET paper_id = ?1 WHERE paper_id = ?2 AND status = 'queued'",
                )
                .bind(&primary_id)
                .bind(dup_id)
                .execute(&mut *tx)
                .await?;

                // ── Transfer citations (both incoming and outgoing) ──
                sqlx::query("UPDATE OR IGNORE citations SET target_id = ?1 WHERE target_id = ?2")
                    .bind(&primary_id)
                    .bind(dup_id)
                    .execute(&mut *tx)
                    .await?;

                sqlx::query("UPDATE OR IGNORE citations SET source_id = ?1 WHERE source_id = ?2")
                    .bind(&primary_id)
                    .bind(dup_id)
                    .execute(&mut *tx)
                    .await?;

                // ── Delete duplicate (cascades to paper_tags, citations) ──
                sqlx::query("DELETE FROM papers WHERE id = ?1")
                    .bind(dup_id)
                    .execute(&mut *tx)
                    .await?;
            }

            tx.commit().await?;
            Ok(!valid_dup_ids.is_empty())
        })
    }

    /// Log a deduplication event.
    pub fn log_dedup(&self, target_id: &str, duplicate_id: &str, keep_policy: &str) -> Result<()> {
        let rt = self.rt.clone();
        let pool = self.pool.lock().clone();
        let id = Uuid::new_v4().to_string();
        rt.block_on(async move {
            sqlx::query(
                "INSERT INTO dedup_log (id, target_id, duplicate_id, keep_policy) VALUES (?1, ?2, ?3, ?4)",
            )
            .bind(&id)
            .bind(target_id)
            .bind(duplicate_id)
            .bind(keep_policy)
            .execute(&pool)
            .await?;
            Ok(())
        })
    }

    /// Get deduplication log entries.
    pub fn get_dedup_log(&self, limit: usize) -> Result<Vec<DedupLogEntry>> {
        let rt = self.rt.clone();
        let pool = self.pool.lock().clone();
        rt.block_on(async move {
            let rows = sqlx::query(
                "SELECT id, target_id, duplicate_id, keep_policy, created_at \
                 FROM dedup_log ORDER BY created_at DESC LIMIT ?1",
            )
            .bind(limit as i64)
            .fetch_all(&pool)
            .await?;
            let mut entries = Vec::with_capacity(limit);
            for row in rows {
                entries.push(DedupLogEntry {
                    id: row.try_get(0)?,
                    target_id: row.try_get(1)?,
                    duplicate_id: row.try_get(2)?,
                    keep_policy: row.try_get(3)?,
                    created_at: row.try_get(4)?,
                });
            }
            Ok(entries)
        })
    }

    /// Get embedding vector for a paper (bytes as f32 array).
    pub fn get_embedding(&self, paper_id: &str) -> Result<Option<Vec<f32>>> {
        let rt = self.rt.clone();
        let pool = self.pool.lock().clone();
        let paper_id = paper_id.to_string();
        rt.block_on(async move {
            let result = sqlx::query(
                "SELECT embed_vector FROM papers WHERE id = ?1 AND embed_vector IS NOT NULL",
            )
            .bind(&paper_id)
            .fetch_optional(&pool)
            .await;
            match result {
                Ok(Some(row)) => {
                    let blob: Vec<u8> = row.try_get(0)?;
                    let vec: Vec<f32> = blob
                        .chunks_exact(4)
                        .map(|chunk| f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]))
                        .collect();
                    Ok(Some(vec))
                }
                Ok(None) => Ok(None),
                Err(sqlx::Error::RowNotFound) => Ok(None),
                Err(e) => Err(CoreError::Database(e)),
            }
        })
    }

    /// List all paper IDs that have embeddings.
    pub fn list_papers_with_embeddings(&self) -> Result<Vec<String>> {
        let rt = self.rt.clone();
        let pool = self.pool.lock().clone();
        rt.block_on(async move {
            let rows = sqlx::query("SELECT id FROM papers WHERE embed_vector IS NOT NULL")
                .fetch_all(&pool)
                .await?;
            let mut ids = Vec::new();
            for row in rows {
                ids.push(row.try_get(0)?);
            }
            Ok(ids)
        })
    }

    fn row_to_paper(row: &SqliteRow) -> Result<Paper> {
        let authors_str: String = row.try_get(3)?;
        let categories_str: String = row.try_get(6)?;
        let status_str: String = row.try_get(7)?;
        let published_str: String = row.try_get(4)?;
        Ok(Paper {
            id: row.try_get(0)?,
            arxiv_id: row.try_get(1)?,
            title: row.try_get(2)?,
            authors: serde_json::from_str(&authors_str).unwrap_or_default(),
            published: DateTime::parse_from_rfc3339(&published_str)
                .map(|dt| dt.with_timezone(&Utc))
                .unwrap_or_else(|_| Utc::now()),
            abstract_text: row.try_get(5)?,
            categories: serde_json::from_str(&categories_str).unwrap_or_default(),
            parse_status: match status_str.as_str() {
                "pending" => ParseStatus::Pending,
                "parsing" => ParseStatus::Parsing,
                "done" => ParseStatus::Done,
                "failed" => ParseStatus::Failed,
                _ => ParseStatus::Pending,
            },
            metadata: PaperMetadata {
                cited_by: row.try_get::<i64, _>(8)? as usize,
                references: row.try_get::<i64, _>(9)? as usize,
                doi: row.try_get(10)?,
                pdf_url: row.try_get(11)?,
            },
        })
    }

    pub fn insert_gap(&self, gap: &ResearchGap) -> Result<()> {
        let rt = self.rt.clone();
        let pool = self.pool.lock().clone();
        let paper_ids_json = serde_json::to_string(&gap.paper_ids)?;
        rt.block_on(async move {
            sqlx::query(
                "INSERT INTO research_gaps (id, topic, session_id, gap_type, gap_title, gap_title_hash, category, description, severity, novelty_score, priority, paper_ids, created_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
            )
            .bind(&gap.id)
            .bind(&gap.topic)
            .bind(&gap.session_id)
            .bind(&gap.gap_type)
            .bind(&gap.gap_title)
            .bind(&gap.gap_title_hash)
            .bind(&gap.category)
            .bind(&gap.description)
            .bind(&gap.severity)
            .bind(gap.novelty_score)
            .bind(&gap.priority)
            .bind(&paper_ids_json)
            .bind(&gap.created_at)
            .execute(&pool)
            .await?;
            Ok(())
        })
    }

    pub fn list_gaps(&self, limit: usize, offset: usize) -> Result<Vec<ResearchGap>> {
        let rt = self.rt.clone();
        let pool = self.pool.lock().clone();
        rt.block_on(async move {
            let rows = sqlx::query(
                "SELECT id, topic, session_id, gap_type, gap_title, gap_title_hash, category, description, severity, novelty_score, priority, paper_ids, created_at FROM research_gaps ORDER BY created_at DESC LIMIT ?1 OFFSET ?2",
            )
            .bind(limit as i64)
            .bind(offset as i64)
            .fetch_all(&pool)
            .await?;
            let mut gaps = Vec::with_capacity(limit);
            for row in rows {
                gaps.push(ResearchGap {
                    id: row.try_get(0)?,
                    topic: row.try_get(1)?,
                    session_id: row.try_get(2)?,
                    gap_type: row.try_get(3)?,
                    gap_title: row.try_get(4)?,
                    gap_title_hash: row.try_get(5)?,
                    category: row.try_get(6)?,
                    description: row.try_get(7)?,
                    severity: row.try_get(8)?,
                    novelty_score: row.try_get(9)?,
                    priority: row.try_get(10)?,
                    paper_ids: serde_json::from_str(&row.try_get::<String, _>(11)?).unwrap_or_default(),
                    created_at: row.try_get(12)?,
                });
            }
            Ok(gaps)
        })
    }

    pub fn get_gap(&self, id: &str) -> Result<Option<ResearchGap>> {
        let rt = self.rt.clone();
        let pool = self.pool.lock().clone();
        let id = id.to_string();
        rt.block_on(async move {
            let result = sqlx::query(
                "SELECT id, topic, session_id, gap_type, gap_title, gap_title_hash, category, description, severity, novelty_score, priority, paper_ids, created_at FROM research_gaps WHERE id = ?1",
            )
            .bind(&id)
            .fetch_optional(&pool)
            .await;
            match result {
                Ok(Some(row)) => Ok(Some(ResearchGap {
                    id: row.try_get(0)?,
                    topic: row.try_get(1)?,
                    session_id: row.try_get(2)?,
                    gap_type: row.try_get(3)?,
                    gap_title: row.try_get(4)?,
                    gap_title_hash: row.try_get(5)?,
                    category: row.try_get(6)?,
                    description: row.try_get(7)?,
                    severity: row.try_get(8)?,
                    novelty_score: row.try_get(9)?,
                    priority: row.try_get(10)?,
                    paper_ids: serde_json::from_str(&row.try_get::<String, _>(11)?).unwrap_or_default(),
                    created_at: row.try_get(12)?,
                })),
                Ok(None) => Ok(None),
                Err(sqlx::Error::RowNotFound) => Ok(None),
                Err(e) => Err(CoreError::Database(e)),
            }
        })
    }

    pub fn delete_gap(&self, id: &str) -> Result<()> {
        let rt = self.rt.clone();
        let pool = self.pool.lock().clone();
        let id = id.to_string();
        rt.block_on(async move {
            sqlx::query("DELETE FROM research_gaps WHERE id = ?1")
                .bind(&id)
                .execute(&pool)
                .await?;
            Ok(())
        })
    }

    pub fn search_papers(&self, query: &str, limit: usize) -> Result<Vec<Paper>> {
        let rt = self.rt.clone();
        let pool = self.pool.lock().clone();
        let pattern = format!("%{}%", query);
        rt.block_on(async move {
            let rows = sqlx::query(
                "SELECT id, arxiv_id, title, authors, published, abstract_text, categories,
                        parse_status, cited_by, references_cnt, doi, pdf_url
                 FROM papers
                 WHERE title LIKE ?1 OR abstract_text LIKE ?1
                 ORDER BY published DESC LIMIT ?2",
            )
            .bind(&pattern)
            .bind(limit as i64)
            .fetch_all(&pool)
            .await?;
            let mut papers: Vec<Paper> = Vec::with_capacity(limit);
            for row in rows {
                papers.push(Self::row_to_paper(&row)?);
            }
            Ok(papers)
        })
    }

    pub fn count_papers(&self) -> Result<i64> {
        let rt = self.rt.clone();
        let pool = self.pool.lock().clone();
        rt.block_on(async move {
            let row = sqlx::query("SELECT COUNT(*) FROM papers")
                .fetch_one(&pool)
                .await?;
            let count: i64 = row.try_get(0)?;
            Ok(count)
        })
    }

    pub fn stats(&self) -> Result<DbStats> {
        let rt = self.rt.clone();
        let pool = self.pool.lock().clone();
        rt.block_on(async move {
            let row = sqlx::query(
                "SELECT
                    (SELECT COUNT(*) FROM papers) as total,
                    (SELECT COUNT(*) FROM papers WHERE parse_status = 'pending') as pending,
                    (SELECT COUNT(*) FROM papers WHERE parse_status = 'done') as done,
                    (SELECT COUNT(*) FROM research_gaps) as gaps",
            )
            .fetch_one(&pool)
            .await?;
            Ok(DbStats {
                total: row.try_get(0)?,
                pending: row.try_get(1)?,
                done: row.try_get(2)?,
                gaps: row.try_get(3)?,
            })
        })
    }

    pub fn search_papers_fts(&self, query: &str, limit: usize) -> Result<Vec<Paper>> {
        let rt = self.rt.clone();
        let pool = self.pool.lock().clone();
        let query = query.to_string();
        rt.block_on(async move {
            let rows = sqlx::query(
                r#"SELECT p.id, p.arxiv_id, p.title, p.authors, p.published, p.abstract_text,
                          p.categories, p.parse_status, p.cited_by, p.references_cnt, p.doi, p.pdf_url
                   FROM papers p
                   INNER JOIN papers_fts fts ON p.rowid = fts.rowid
                   WHERE papers_fts MATCH ?1
                   ORDER BY rank
                   LIMIT ?2"#,
            )
            .bind(&query)
            .bind(limit as i64)
            .fetch_all(&pool)
            .await?;
            let mut papers: Vec<Paper> = Vec::with_capacity(limit);
            for row in rows {
                papers.push(Self::row_to_paper(&row)?);
            }
            Ok(papers)
        })
    }

    pub fn search_papers_smart(&self, query: &str, limit: usize) -> Result<Vec<Paper>> {
        let query = query.trim();
        if query.is_empty() {
            return Ok(Vec::new());
        }

        let word_count = query.split_whitespace().count();
        if word_count >= 2 {
            if let Ok(papers) = self.search_papers_fts(query, limit) {
                if !papers.is_empty() {
                    return Ok(papers);
                }
            }
        }

        let rt = self.rt.clone();
        let pool = self.pool.lock().clone();
        let owned_query = query.to_string();
        rt.block_on(async move {
            let pattern = format!("%{}%", owned_query);
            let rows = sqlx::query(
                "SELECT id, arxiv_id, title, authors, published, abstract_text, categories,
                        parse_status, cited_by, references_cnt, doi, pdf_url
                 FROM papers
                 WHERE title LIKE ?1 OR abstract_text LIKE ?1
                 ORDER BY published DESC LIMIT ?2",
            )
            .bind(&pattern)
            .bind(limit as i64)
            .fetch_all(&pool)
            .await?;
            let mut papers: Vec<Paper> = Vec::with_capacity(limit);
            for row in rows {
                papers.push(Self::row_to_paper(&row)?);
            }
            Ok(papers)
        })
    }

    pub fn update_paper_full_text(
        &self,
        id: &str,
        plain_text: &str,
        table_count: i32,
        figure_count: i32,
        word_count: i32,
        page_count: i32,
    ) -> Result<()> {
        let rt = self.rt.clone();
        let pool = self.pool.lock().clone();
        let id = id.to_string();
        let plain_text = plain_text.to_string();
        rt.block_on(async move {
            sqlx::query(
                "UPDATE papers SET plain_text = ?1, table_count = ?2, figure_count = ?3, word_count = ?4, page_count = ?5, updated_at = datetime('now') WHERE id = ?6",
            )
            .bind(&plain_text)
            .bind(table_count)
            .bind(figure_count)
            .bind(word_count)
            .bind(page_count)
            .bind(&id)
            .execute(&pool)
            .await?;
            Ok(())
        })
    }

    pub fn insert_subscription(&self, sub: &Subscription) -> Result<()> {
        let rt = self.rt.clone();
        let pool = self.pool.lock().clone();
        let sub = sub.clone();
        rt.block_on(async move {
            sqlx::query(
                r#"INSERT INTO subscriptions (id, query, name, categories, max_results, check_interval_minutes, last_check, last_results, enabled)
                   VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)"#,
            )
            .bind(&sub.id)
            .bind(&sub.query)
            .bind(&sub.name)
            .bind(serde_json::to_string(&sub.categories)?)
            .bind(sub.max_results)
            .bind(sub.check_interval_minutes as i32)
            .bind(&sub.last_check)
            .bind(&sub.last_results)
            .bind(sub.enabled as i32)
            .execute(&pool)
            .await?;
            Ok(())
        })
    }

    pub fn list_subscriptions(&self, enabled_only: bool) -> Result<Vec<Subscription>> {
        let rt = self.rt.clone();
        let pool = self.pool.lock().clone();
        rt.block_on(async move {
            let sql = if enabled_only {
                "SELECT id, query, name, categories, max_results, check_interval_minutes, last_check, last_results, enabled FROM subscriptions WHERE enabled = 1"
            } else {
                "SELECT id, query, name, categories, max_results, check_interval_minutes, last_check, last_results, enabled FROM subscriptions"
            };
            let rows = sqlx::query(sql).fetch_all(&pool).await?;
            let mut subs = Vec::new();
            for row in rows {
                let categories_str: Option<String> = row.try_get(3)?;
                let last_check: Option<String> = row.try_get(6)?;
                let last_results: Option<String> = row.try_get(7)?;
                subs.push(Subscription {
                    id: row.try_get(0)?,
                    query: row.try_get(1)?,
                    name: row.try_get(2)?,
                    categories: categories_str
                        .map(|s| serde_json::from_str(&s).unwrap_or_default())
                        .unwrap_or_default(),
                    max_results: row.try_get(4)?,
                    check_interval_minutes: row.try_get::<i32, _>(5)? as u64,
                    last_check,
                    last_results,
                    enabled: row.try_get::<i32, _>(8)? != 0,
                });
            }
            Ok(subs)
        })
    }

    pub fn delete_subscription(&self, id: &str) -> Result<()> {
        let rt = self.rt.clone();
        let pool = self.pool.lock().clone();
        let id = id.to_string();
        rt.block_on(async move {
            sqlx::query("DELETE FROM subscriptions WHERE id = ?1")
                .bind(&id)
                .execute(&pool)
                .await?;
            Ok(())
        })
    }

    pub fn update_subscription_last_check(
        &self,
        id: &str,
        last_check: &str,
        last_results: &str,
    ) -> Result<()> {
        let rt = self.rt.clone();
        let pool = self.pool.lock().clone();
        let id = id.to_string();
        let last_check = last_check.to_string();
        let last_results = last_results.to_string();
        rt.block_on(async move {
            sqlx::query("UPDATE subscriptions SET last_check = ?1, last_results = ?2 WHERE id = ?3")
                .bind(&last_check)
                .bind(&last_results)
                .bind(&id)
                .execute(&pool)
                .await?;
            Ok(())
        })
    }

    pub fn insert_tag(&self, tag: &Tag) -> Result<()> {
        let rt = self.rt.clone();
        let pool = self.pool.lock().clone();
        let tag = tag.clone();
        rt.block_on(async move {
            sqlx::query("INSERT INTO tags (id, name, color) VALUES (?1, ?2, ?3)")
                .bind(&tag.id)
                .bind(&tag.name)
                .bind(&tag.color)
                .execute(&pool)
                .await?;
            Ok(())
        })
    }

    pub fn list_tags(&self) -> Result<Vec<Tag>> {
        let rt = self.rt.clone();
        let pool = self.pool.lock().clone();
        rt.block_on(async move {
            let rows = sqlx::query("SELECT id, name, color FROM tags ORDER BY name")
                .fetch_all(&pool)
                .await?;
            let mut tags = Vec::new();
            for row in rows {
                tags.push(Tag {
                    id: row.try_get(0)?,
                    name: row.try_get(1)?,
                    color: row.try_get(2)?,
                });
            }
            Ok(tags)
        })
    }

    pub fn delete_tag(&self, id: &str) -> Result<()> {
        let rt = self.rt.clone();
        let pool = self.pool.lock().clone();
        let id = id.to_string();
        rt.block_on(async move {
            sqlx::query("DELETE FROM tags WHERE id = ?1")
                .bind(&id)
                .execute(&pool)
                .await?;
            Ok(())
        })
    }

    pub fn add_paper_tag(&self, paper_id: &str, tag_id: &str) -> Result<()> {
        let rt = self.rt.clone();
        let pool = self.pool.lock().clone();
        let paper_id = paper_id.to_string();
        let tag_id = tag_id.to_string();
        rt.block_on(async move {
            sqlx::query("INSERT OR IGNORE INTO paper_tags (paper_id, tag_id) VALUES (?1, ?2)")
                .bind(&paper_id)
                .bind(&tag_id)
                .execute(&pool)
                .await?;
            Ok(())
        })
    }

    pub fn remove_paper_tag(&self, paper_id: &str, tag_id: &str) -> Result<()> {
        let rt = self.rt.clone();
        let pool = self.pool.lock().clone();
        let paper_id = paper_id.to_string();
        let tag_id = tag_id.to_string();
        rt.block_on(async move {
            sqlx::query("DELETE FROM paper_tags WHERE paper_id = ?1 AND tag_id = ?2")
                .bind(&paper_id)
                .bind(&tag_id)
                .execute(&pool)
                .await?;
            Ok(())
        })
    }

    pub fn get_paper_tags(&self, paper_id: &str) -> Result<Vec<Tag>> {
        let rt = self.rt.clone();
        let pool = self.pool.lock().clone();
        let paper_id = paper_id.to_string();
        rt.block_on(async move {
            let rows = sqlx::query(
                "SELECT t.id, t.name, t.color FROM tags t INNER JOIN paper_tags pt ON t.id = pt.tag_id WHERE pt.paper_id = ?1",
            )
            .bind(&paper_id)
            .fetch_all(&pool)
            .await?;
            let mut tags = Vec::new();
            for row in rows {
                tags.push(Tag {
                    id: row.try_get(0)?,
                    name: row.try_get(1)?,
                    color: row.try_get(2)?,
                });
            }
            Ok(tags)
        })
    }

    pub fn insert_citation(&self, source_id: &str, target_id: &str) -> Result<()> {
        let rt = self.rt.clone();
        let pool = self.pool.lock().clone();
        let id = Uuid::new_v4().to_string();
        let source_id = source_id.to_string();
        let target_id = target_id.to_string();
        rt.block_on(async move {
            sqlx::query("INSERT OR IGNORE INTO citations (id, source_id, target_id) VALUES (?1, ?2, ?3)")
                .bind(&id)
                .bind(&source_id)
                .bind(&target_id)
                .execute(&pool)
                .await?;
            Ok(())
        })
    }

    pub fn list_all_citations(&self) -> Result<Vec<(String, String)>> {
        let rt = self.rt.clone();
        let pool = self.pool.lock().clone();
        rt.block_on(async move {
            let rows = sqlx::query("SELECT source_id, target_id FROM citations")
                .fetch_all(&pool)
                .await?;
            let mut result = Vec::new();
            for row in rows {
                result.push((row.try_get(0)?, row.try_get(1)?));
            }
            Ok(result)
        })
    }

    pub fn get_citations(&self, paper_id: &str) -> Result<Citations> {
        let rt = self.rt.clone();
        let pool = self.pool.lock().clone();
        let paper_id = paper_id.to_string();
        rt.block_on(async move {
            let citing_rows = sqlx::query("SELECT source_id FROM citations WHERE target_id = ?1")
                .bind(&paper_id)
                .fetch_all(&pool)
                .await?;
            let citing: Vec<String> = citing_rows
                .iter()
                .filter_map(|row| row.try_get(0).ok())
                .collect();

            let refs_rows = sqlx::query("SELECT target_id FROM citations WHERE source_id = ?1")
                .bind(&paper_id)
                .fetch_all(&pool)
                .await?;
            let references: Vec<String> = refs_rows
                .iter()
                .filter_map(|row| row.try_get(0).ok())
                .collect();

            Ok(Citations { citing, references })
        })
    }

    pub fn find_similar_by_vector(
        &self,
        embedding: &[f32],
        limit: usize,
    ) -> Result<Vec<(String, f32)>> {
        let rt = self.rt.clone();
        let pool = self.pool.lock().clone();
        let embedding_vec: Vec<f32> = embedding.to_vec();
        rt.block_on(async move {
            let rows = sqlx::query("SELECT id, embed_vector FROM papers WHERE embed_vector IS NOT NULL")
                .fetch_all(&pool)
                .await?;

            let mut results: Vec<(String, f32)> = Vec::with_capacity(limit);
            for row in rows {
                let id: String = row.try_get(0)?;
                let blob: Vec<u8> = row.try_get(1)?;
                if blob.len() != embedding_vec.len() * 4 {
                    continue;
                }
                // SAFETY: blob.len() == embedding_vec.len() * 4 ensures we have exactly
                // the right number of f32 bytes. SQLite stores these bytes directly,
                // and the pointer alignment is valid for f32 read on our target platforms.
                // This pattern (byte vec to typed slice) is a common database interop practice.
                let stored: &[f32] = unsafe {
                    std::slice::from_raw_parts(
                        blob.as_ptr() as *const f32,
                        embedding_vec.len()
                    )
                };
                let similarity = cosine_similarity(&embedding_vec, stored);
                results.push((id, similarity));
            }

            results.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
            results.truncate(limit);
            Ok(results)
        })
    }

    pub fn set_paper_embedding(&self, paper_id: &str, embedding: &[f32]) -> Result<()> {
        let rt = self.rt.clone();
        let pool = self.pool.lock().clone();
        let paper_id = paper_id.to_string();
        let blob: Vec<u8> = embedding.iter().flat_map(|f| f.to_le_bytes()).collect();
        rt.block_on(async move {
            sqlx::query("UPDATE papers SET embed_vector = ?1 WHERE id = ?2")
                .bind(&blob)
                .bind(&paper_id)
                .execute(&pool)
                .await?;
            Ok(())
        })
    }

    /// Get embedding coverage stats: (total_with_text, with_embedding).
    pub fn get_embedding_stats(&self) -> Result<(i64, i64)> {
        let rt = self.rt.clone();
        let pool = self.pool.lock().clone();
        rt.block_on(async move {
            let total_row = sqlx::query(
                "SELECT COUNT(*) FROM papers WHERE title != '' OR abstract_text != ''",
            )
            .fetch_one(&pool)
            .await?;
            let total: i64 = total_row.try_get(0)?;

            let emb_row = sqlx::query(
                "SELECT COUNT(*) FROM papers WHERE embed_vector IS NOT NULL",
            )
            .fetch_one(&pool)
            .await?;
            let with_emb: i64 = emb_row.try_get(0)?;

            Ok((total, with_emb))
        })
    }

    /// Get papers that don't have embeddings yet.
    pub fn get_papers_without_embeddings(&self, limit: i64) -> Result<Vec<Paper>> {
        let rt = self.rt.clone();
        let pool = self.pool.lock().clone();
        rt.block_on(async move {
            let rows = sqlx::query(
                "SELECT id, arxiv_id, title, authors, published, abstract_text,
                        categories, parse_status, cited_by, references_cnt, doi, pdf_url
                 FROM papers WHERE embed_vector IS NULL AND (title != '' OR abstract_text != '')
                 LIMIT ?1",
            )
            .bind(limit)
            .fetch_all(&pool)
            .await?;
            let mut papers = Vec::with_capacity(limit as usize);
            for row in rows {
                papers.push(Self::row_to_paper(&row)?);
            }
            Ok(papers)
        })
    }

    /// Find papers similar to the given paper by embedding cosine similarity.
    pub fn find_similar(&self, paper_id: &str, top_k: usize, threshold: f32) -> Result<Vec<(String, f32)>> {
        let embedding = self.get_embedding(paper_id)?;
        let target = match embedding {
            Some(v) => v,
            None => return Ok(vec![]),
        };

        let mut all = self.find_similar_by_vector(&target, top_k)?;
        all.retain(|(_, score)| *score >= threshold);
        Ok(all)
    }

    /// Compute cosine similarity between two papers' embeddings.
    pub fn get_similarity(&self, paper_id1: &str, paper_id2: &str) -> Result<Option<f32>> {
        let e1 = self.get_embedding(paper_id1)?;
        let e2 = self.get_embedding(paper_id2)?;
        match (e1, e2) {
            (Some(v1), Some(v2)) => Ok(Some(cosine_similarity(&v1, &v2))),
            _ => Ok(None),
        }
    }

    // ─── Queue Operations ─────────────────────────────────────────────────

    pub fn enqueue_job(&self, paper_id: &str, job_type: &str, priority: i64) -> Result<i64> {
        let rt = self.rt.clone();
        let pool = self.pool.lock().clone();
        let paper_id = paper_id.to_string();
        let job_type = job_type.to_string();
        let now = chrono::Utc::now().format("%Y-%m-%dT%H:%M:%S").to_string();
        rt.block_on(async move {
            let result = sqlx::query(
                "INSERT INTO job_queue (paper_id, job_type, status, priority, created_at) VALUES (?1, ?2, 'queued', ?3, ?4)",
            )
            .bind(&paper_id)
            .bind(&job_type)
            .bind(priority)
            .bind(&now)
            .execute(&pool)
            .await?;
            Ok(result.last_insert_rowid())
        })
    }

    pub fn dequeue_job(&self) -> Result<Option<JobQueueEntry>> {
        let rt = self.rt.clone();
        let pool = self.pool.lock().clone();
        let now = chrono::Utc::now().format("%Y-%m-%dT%H:%M:%S").to_string();
        rt.block_on(async move {
            let row = sqlx::query(
                "SELECT id, paper_id, job_type, status, priority, created_at, started_at, completed_at, error
                 FROM job_queue WHERE status = 'queued' ORDER BY priority DESC, created_at ASC LIMIT 1",
            )
            .fetch_optional(&pool)
            .await?;

            match row {
                Some(row) => {
                    let entry = JobQueueEntry {
                        id: row.try_get(0)?,
                        paper_id: row.try_get(1)?,
                        job_type: row.try_get(2)?,
                        status: "running".to_string(),
                        priority: row.try_get(4)?,
                        created_at: row.try_get(5)?,
                        started_at: Some(now.clone()),
                        completed_at: row.try_get(7)?,
                        error: row.try_get(8)?,
                    };
                    let entry_id = entry.id;
                    sqlx::query("UPDATE job_queue SET status = 'running', started_at = ?1 WHERE id = ?2")
                        .bind(&now)
                        .bind(entry_id)
                        .execute(&pool)
                        .await?;
                    Ok(Some(entry))
                }
                None => Ok(None),
            }
        })
    }

    pub fn get_queue_jobs(&self, status: Option<&str>, limit: i64) -> Result<Vec<JobQueueEntry>> {
        let rt = self.rt.clone();
        let pool = self.pool.lock().clone();
        rt.block_on(async move {
            let sql = if status.is_some() {
                "SELECT id, paper_id, job_type, status, priority, created_at, started_at, completed_at, error
                     FROM job_queue WHERE status = ?1 ORDER BY priority DESC, created_at ASC LIMIT ?2"
            } else {
                "SELECT id, paper_id, job_type, status, priority, created_at, started_at, completed_at, error
                     FROM job_queue ORDER BY priority DESC, created_at ASC LIMIT ?1"
            };

            let rows = if let Some(s) = status {
                sqlx::query(sql)
                    .bind(s)
                    .bind(limit)
                    .fetch_all(&pool)
                    .await?
            } else {
                sqlx::query(sql)
                    .bind(limit)
                    .fetch_all(&pool)
                    .await?
            };

            let mut results = Vec::with_capacity(limit as usize);
            for row in rows {
                results.push(map_job_row(&row)?);
            }
            Ok(results)
        })
    }

    pub fn cancel_job(&self, job_id: i64) -> Result<bool> {
        let rt = self.rt.clone();
        let pool = self.pool.lock().clone();
        rt.block_on(async move {
            let result = sqlx::query(
                "UPDATE job_queue SET status = 'cancelled' WHERE id = ?1 AND status = 'queued'",
            )
            .bind(job_id)
            .execute(&pool)
            .await?;
            Ok(result.rows_affected() > 0)
        })
    }

    pub fn clear_pending_papers(&self) -> Result<i64> {
        let rt = self.rt.clone();
        let pool = self.pool.lock().clone();
        rt.block_on(async move {
            let count_row = sqlx::query("SELECT COUNT(*) FROM papers WHERE parse_status = 'pending'")
                .fetch_one(&pool)
                .await?;
            let count: i64 = count_row.try_get(0)?;

            sqlx::query("DELETE FROM papers WHERE parse_status = 'pending'")
                .execute(&pool)
                .await?;
            Ok(count)
        })
    }

    /// List recent paper-code traces across all papers.
    pub fn list_paper_code_traces(&self, limit: i64) -> Result<Vec<PaperCodeTrace>> {
        let rt = self.rt.clone();
        let pool = self.pool.lock().clone();
        rt.block_on(async move {
            let rows = sqlx::query(
                "SELECT * FROM paper_code_trace ORDER BY created_at DESC LIMIT ?1",
            )
            .bind(limit)
            .fetch_all(&pool)
            .await?;
            let mut traces = Vec::with_capacity(limit as usize);
            for row in rows {
                traces.push(map_paper_code_trace_row(&row)?);
            }
            Ok(traces)
        })
    }

    /// Get paper-code traces for a specific paper.
    pub fn get_paper_code_trace(&self, paper_id: &str) -> Result<Vec<PaperCodeTrace>> {
        let rt = self.rt.clone();
        let pool = self.pool.lock().clone();
        let paper_id = paper_id.to_string();
        rt.block_on(async move {
            let rows = sqlx::query(
                "SELECT * FROM paper_code_trace WHERE paper_id = ? ORDER BY created_at DESC",
            )
            .bind(&paper_id)
            .fetch_all(&pool)
            .await?;
            let mut traces = Vec::new();
            for row in rows {
                traces.push(map_paper_code_trace_row(&row)?);
            }
            Ok(traces)
        })
    }

    // ============================================================================
    // Methods from rairos-db — adapted to rairos-core's connection pattern
    // ============================================================================

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
        params: Vec<DbValue>,
    ) -> Result<Vec<HashMap<String, DbValue>>> {
        let rt = self.rt.clone();
        let pool = self.pool.lock().clone();
        rt.block_on(async move {
            let mut query = sqlx::query(sql);
            for param in &params {
                query = match param {
                    DbValue::Null => query.bind(Option::<String>::None),
                    DbValue::Integer(i) => query.bind(i),
                    DbValue::Real(r) => query.bind(r),
                    DbValue::Text(s) => query.bind(s),
                    DbValue::Blob(b) => query.bind(b),
                };
            }
            let rows = query.fetch_all(&pool).await?;

            let mut results = Vec::new();
            for row in rows {
                let mut map = HashMap::new();
                // Use column index to iterate instead of unbounded loop
                let columns = row.columns();
                let num_cols = columns.len();
                for i in 0..num_cols {
                    // Use try_get with column name by accessing via &str index
                    if let Ok(name) = row.try_get::<&str, _>(i) {
                        if let Some(value) = DbValue::from_sqlx(&row, i) {
                            map.insert(name.to_string(), value);
                        }
                    }
                }
                results.push(map);
            }
            Ok(results)
        })
    }

    /// Execute a transaction with explicit BEGIN/COMMIT/ROLLBACK semantics.
    pub fn transaction<T, F>(&self, mut f: F) -> Result<T>
    where
        F: FnMut(&mut sqlx::Transaction<'_, sqlx::Sqlite>) -> Result<T>,
    {
        let rt = self.rt.clone();
        let pool = self.pool.lock().clone();
        rt.block_on(async move {
            let mut tx = pool.begin().await?;
            let result = f(&mut tx);
            match result {
                Ok(v) => {
                    tx.commit().await?;
                    Ok(v)
                }
                Err(e) => {
                    tx.rollback().await?;
                    Err(e)
                }
            }
        })
    }

    /// Delete all data from all tables (for test isolation).
    /// Unlike rairos-db, we do NOT delete the physical file since rairos-core
    /// uses a persistent connection.
    pub fn clear_all(&self) -> Result<()> {
        let rt = self.rt.clone();
        let pool = self.pool.lock().clone();
        rt.block_on(async move {
            sqlx::query("DELETE FROM paper_tags").execute(&pool).await?;
            sqlx::query("DELETE FROM citations").execute(&pool).await?;
            sqlx::query("DELETE FROM job_queue").execute(&pool).await?;
            sqlx::query("DELETE FROM dedup_log").execute(&pool).await?;
            sqlx::query("DELETE FROM paper_code_trace").execute(&pool).await?;
            sqlx::query("DELETE FROM evo_suggestions").execute(&pool).await?;
            sqlx::query("DELETE FROM research_gaps").execute(&pool).await?;
            sqlx::query("DELETE FROM subscriptions").execute(&pool).await?;
            sqlx::query("DELETE FROM tags").execute(&pool).await?;
            sqlx::query("DELETE FROM paper_cache").execute(&pool).await?;
            sqlx::query("DELETE FROM papers_fts").execute(&pool).await?;
            sqlx::query("DELETE FROM papers").execute(&pool).await?;
            Ok(())
        })
    }

    /// Insert or update a paper. Returns true if the paper was newly inserted.
    pub fn upsert_paper(&self, paper: &Paper) -> Result<bool> {
        let rt = self.rt.clone();
        let pool = self.pool.lock().clone();
        let paper = paper.clone();
        rt.block_on(async move {
            // Check if exists
            let exists = sqlx::query("SELECT 1 FROM papers WHERE id = ?1")
                .bind(&paper.id)
                .fetch_optional(&pool)
                .await?
                .is_some();

            if exists {
                sqlx::query(
                    "UPDATE papers SET
                     arxiv_id=?1, title=?2, authors=?3, published=?4,
                     abstract_text=?5, categories=?6, parse_status=?7,
                     cited_by=?8, references_cnt=?9, doi=?10, pdf_url=?11,
                     updated_at=datetime('now')
                     WHERE id=?12",
                )
                .bind(&paper.arxiv_id)
                .bind(&paper.title)
                .bind(serde_json::to_string(&paper.authors)?)
                .bind(paper.published.to_rfc3339())
                .bind(&paper.abstract_text)
                .bind(serde_json::to_string(&paper.categories)?)
                .bind(paper.parse_status.to_string())
                .bind(paper.metadata.cited_by as i64)
                .bind(paper.metadata.references as i64)
                .bind(&paper.metadata.doi)
                .bind(&paper.metadata.pdf_url)
                .bind(&paper.id)
                .execute(&pool)
                .await?;
                Ok(false)
            } else {
                sqlx::query(
                    r#"INSERT INTO papers
                       (id, arxiv_id, title, authors, published, abstract_text, categories,
                        parse_status, cited_by, references_cnt, doi, pdf_url)
                       VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)"#,
                )
                .bind(&paper.id)
                .bind(&paper.arxiv_id)
                .bind(&paper.title)
                .bind(serde_json::to_string(&paper.authors)?)
                .bind(paper.published.to_rfc3339())
                .bind(&paper.abstract_text)
                .bind(serde_json::to_string(&paper.categories)?)
                .bind(paper.parse_status.to_string())
                .bind(paper.metadata.cited_by as i64)
                .bind(paper.metadata.references as i64)
                .bind(&paper.metadata.doi)
                .bind(&paper.metadata.pdf_url)
                .execute(&pool)
                .await?;
                Ok(true)
            }
        })
    }

    /// Bulk upsert papers. Returns (inserted, updated) counts.
    pub fn upsert_papers_bulk(&self, papers: &[Paper]) -> Result<(i64, i64)> {
        if papers.is_empty() {
            return Ok((0, 0));
        }
        let rt = self.rt.clone();
        let pool = self.pool.lock().clone();
        let paper_ids: Vec<String> = papers.iter().map(|p| p.id.clone()).collect();
        let placeholders: Vec<String> = paper_ids.iter().map(|_| "?".to_string()).collect();
        let sql = format!(
            "SELECT id FROM papers WHERE id IN ({})",
            placeholders.join(",")
        );

        rt.block_on(async move {
            // Build query with dynamic placeholders
            let mut query = sqlx::query(&sql);
            for id in &paper_ids {
                query = query.bind(id);
            }
            let rows = query.fetch_all(&pool).await?;
            let existing: FxHashSet<String> = rows
                .iter()
                .filter_map(|row| row.try_get(0).ok())
                .collect();

            let mut inserted: i64 = 0;
            let mut updated: i64 = 0;

            for paper in papers {
                if existing.contains(&paper.id) {
                    sqlx::query(
                        "UPDATE papers SET
                         arxiv_id=?1, title=?2, authors=?3, published=?4,
                         abstract_text=?5, categories=?6, parse_status=?7,
                         cited_by=?8, references_cnt=?9, doi=?10, pdf_url=?11,
                         updated_at=datetime('now')
                         WHERE id=?12",
                    )
                    .bind(&paper.arxiv_id)
                    .bind(&paper.title)
                    .bind(serde_json::to_string(&paper.authors)?)
                    .bind(paper.published.to_rfc3339())
                    .bind(&paper.abstract_text)
                    .bind(serde_json::to_string(&paper.categories)?)
                    .bind(paper.parse_status.to_string())
                    .bind(paper.metadata.cited_by as i64)
                    .bind(paper.metadata.references as i64)
                    .bind(&paper.metadata.doi)
                    .bind(&paper.metadata.pdf_url)
                    .bind(&paper.id)
                    .execute(&pool)
                    .await?;
                    updated += 1;
                } else {
                    sqlx::query(
                        r#"INSERT INTO papers
                           (id, arxiv_id, title, authors, published, abstract_text, categories,
                            parse_status, cited_by, references_cnt, doi, pdf_url)
                           VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)"#,
                    )
                    .bind(&paper.id)
                    .bind(&paper.arxiv_id)
                    .bind(&paper.title)
                    .bind(serde_json::to_string(&paper.authors)?)
                    .bind(paper.published.to_rfc3339())
                    .bind(&paper.abstract_text)
                    .bind(serde_json::to_string(&paper.categories)?)
                    .bind(paper.parse_status.to_string())
                    .bind(paper.metadata.cited_by as i64)
                    .bind(paper.metadata.references as i64)
                    .bind(&paper.metadata.doi)
                    .bind(&paper.metadata.pdf_url)
                    .execute(&pool)
                    .await?;
                    inserted += 1;
                }
            }
            Ok((inserted, updated))
        })
    }

    /// Export papers as a portable format: Vec<HashMap<field_name, json_value>>.
    pub fn export_papers(
        &self,
        limit: usize,
        extra_fields: bool,
    ) -> Result<Vec<HashMap<String, serde_json::Value>>> {
        let rt = self.rt.clone();
        let pool = self.pool.lock().clone();
        rt.block_on(async move {
            let fields: &[&str] = if extra_fields {
                &[
                    "id", "arxiv_id", "title", "authors", "published", "abstract_text",
                    "categories", "parse_status", "doi", "pdf_url",
                ]
            } else {
                &["id", "title", "authors", "published", "abstract_text", "parse_status"]
            };

            let sql = if limit > 0 {
                format!(
                    "SELECT {} FROM papers ORDER BY published DESC LIMIT ?1",
                    fields.join(",")
                )
            } else {
                format!("SELECT {} FROM papers ORDER BY published DESC", fields.join(","))
            };

            let rows = if limit > 0 {
                sqlx::query(&sql)
                    .bind(limit as i64)
                    .fetch_all(&pool)
                    .await?
            } else {
                sqlx::query(&sql)
                    .fetch_all(&pool)
                    .await?
            };

            let mut result = Vec::with_capacity(limit);
            for row in rows {
                let mut m = HashMap::new();
                for (i, f) in fields.iter().enumerate() {
                    if let Some(val) = DbValue::from_sqlx(&row, i) {
                        let json_val = match val {
                            DbValue::Null => serde_json::Value::Null,
                            DbValue::Integer(i) => serde_json::json!(i),
                            DbValue::Real(r) => serde_json::json!(r),
                            DbValue::Text(s) => serde_json::json!(s),
                            DbValue::Blob(b) => serde_json::json!(base64_encode(&b)),
                        };
                        m.insert(f.to_string(), json_val);
                    }
                }
                result.push(m);
            }
            Ok(result)
        })
    }

    /// Rebuild the FTS5 index from all papers. Returns count of indexed rows.
    pub fn rebuild_fts_index(&self) -> Result<i64> {
        let rt = self.rt.clone();
        let pool = self.pool.lock().clone();
        rt.block_on(async move {
            sqlx::query("DELETE FROM papers_fts").execute(&pool).await?;
            let result = sqlx::query(
                "INSERT INTO papers_fts(rowid, title, abstract_text, authors, categories)
                 SELECT rowid, title, abstract_text, authors, categories FROM papers",
            )
            .execute(&pool)
            .await?;
            Ok(result.rows_affected() as i64)
        })
    }

    /// Search papers with limit/offset/category/parse_status filters via LIKE.
    /// Returns (results, total_count).
    pub fn search_papers_ext(
        &self,
        query: &str,
        limit: i64,
        offset: i64,
        category: Option<&str>,
        parse_status: Option<&str>,
    ) -> Result<(Vec<SearchResult>, i64)> {
        let rt = self.rt.clone();
        let pool = self.pool.lock().clone();
        let pattern = format!("%{}%", query);

        rt.block_on(async move {
            let mut where_clauses: Vec<String> = Vec::new();
            let mut params_vec: Vec<DbValue> = Vec::new();

            // FTS match via LIKE (since FTS5 requires specific query syntax, we use LIKE for compatibility)
            where_clauses.push("(p.title LIKE ? OR p.abstract_text LIKE ?)".to_string());
            params_vec.push(DbValue::Text(pattern.clone()));
            params_vec.push(DbValue::Text(pattern));

            if let Some(cat) = category {
                where_clauses.push("p.categories LIKE ?".to_string());
                params_vec.push(DbValue::Text(format!("%{}%", cat)));
            }
            if let Some(ps) = parse_status {
                where_clauses.push("p.parse_status = ?".to_string());
                params_vec.push(DbValue::Text(ps.to_string()));
            }

            let where_clause = where_clauses.join(" AND ");

            // Count query
            let count_sql = format!("SELECT COUNT(*) FROM papers p WHERE {}", where_clause);
            let mut count_query = sqlx::query(&count_sql);
            for param in &params_vec {
                match param {
                    DbValue::Null => { count_query = count_query.bind(Option::<String>::None); }
                    DbValue::Integer(i) => { count_query = count_query.bind(i); }
                    DbValue::Real(r) => { count_query = count_query.bind(r); }
                    DbValue::Text(s) => { count_query = count_query.bind(s); }
                    DbValue::Blob(b) => { count_query = count_query.bind(b); }
                }
            }
            let total: i64 = count_query
                .fetch_one(&pool)
                .await
                .map(|row| row.try_get(0).unwrap_or(0))
                .unwrap_or(0);

            // Search query
            let search_sql = format!(
                "SELECT p.id, p.title, p.authors, p.published, p.abstract_text,
                        p.categories, p.parse_status, p.doi, p.pdf_url
                 FROM papers p
                 WHERE {}
                 ORDER BY p.published DESC
                 LIMIT ? OFFSET ?",
                where_clause
            );
            let mut search_query = sqlx::query(&search_sql);
            for param in &params_vec {
                match param {
                    DbValue::Null => { search_query = search_query.bind(Option::<String>::None); }
                    DbValue::Integer(i) => { search_query = search_query.bind(i); }
                    DbValue::Real(r) => { search_query = search_query.bind(r); }
                    DbValue::Text(s) => { search_query = search_query.bind(s); }
                    DbValue::Blob(b) => { search_query = search_query.bind(b); }
                }
            }
            search_query = search_query.bind(limit).bind(offset);

            let rows = search_query.fetch_all(&pool).await?;
            let mut results = Vec::with_capacity(limit as usize);
            for row in rows {
                let authors_str: String = row.try_get(2)?;
                let authors: Vec<String> = serde_json::from_str(&authors_str).unwrap_or_default();
                let categories_str: String = row.try_get(5)?;
                let primary_cat = serde_json::from_str::<Vec<String>>(&categories_str)
                    .ok()
                    .and_then(|c| c.into_iter().next())
                    .unwrap_or_default();
                results.push(SearchResult {
                    paper_id: row.try_get(0)?,
                    title: row.try_get(1)?,
                    authors,
                    published: row.try_get::<String, _>(3)?,
                    primary_category: primary_cat,
                    score: 0.0,
                    snippet: String::new(),
                    parse_status: row.try_get(6)?,
                    source: String::new(),
                    abs_url: String::new(),
                    pdf_url: row.try_get::<Option<String>, _>(8)?.unwrap_or_default(),
                });
            }
            Ok((results, total))
        })
    }

    /// Count papers filtered by parse status.
    pub fn count_papers_with_status(&self, status: ParseStatus) -> Result<i64> {
        let rt = self.rt.clone();
        let pool = self.pool.lock().clone();
        let status_str = status.to_string();
        rt.block_on(async move {
            let row = sqlx::query("SELECT COUNT(*) FROM papers WHERE parse_status = ?1")
                .bind(&status_str)
                .fetch_one(&pool)
                .await?;
            let count: i64 = row.try_get(0)?;
            Ok(count)
        })
    }

    /// Get detailed database statistics matching rairos-db's format.
    pub fn get_detailed_stats(&self) -> Result<DetailedStats> {
        let rt = self.rt.clone();
        let pool = self.pool.lock().clone();
        rt.block_on(async move {
            let total_papers_row = sqlx::query("SELECT COUNT(*) FROM papers")
                .fetch_one(&pool)
                .await?;
            let total_papers: i64 = total_papers_row.try_get(0)?;

            let status_rows = sqlx::query("SELECT parse_status, COUNT(*) FROM papers GROUP BY parse_status")
                .fetch_all(&pool)
                .await?;
            let mut by_status = HashMap::new();
            for row in status_rows {
                let st: String = row.try_get(0)?;
                let cnt: i64 = row.try_get(1)?;
                by_status.insert(st, cnt);
            }

            let queue_queued = match sqlx::query("SELECT COUNT(*) FROM job_queue WHERE status = 'queued'")
                .fetch_one(&pool)
                .await
            {
                Ok(row) => row.try_get::<i64, _>(0).unwrap_or(0),
                Err(_) => 0,
            };

            let queue_running = match sqlx::query("SELECT COUNT(*) FROM job_queue WHERE status = 'running'")
                .fetch_one(&pool)
                .await
            {
                Ok(row) => row.try_get::<i64, _>(0).unwrap_or(0),
                Err(_) => 0,
            };

            let cache_entries = match sqlx::query("SELECT COUNT(*) FROM paper_cache")
                .fetch_one(&pool)
                .await
            {
                Ok(row) => row.try_get::<i64, _>(0).unwrap_or(0),
                Err(_) => 0,
            };

            let dedup_records = match sqlx::query("SELECT COUNT(*) FROM dedup_log")
                .fetch_one(&pool)
                .await
            {
                Ok(row) => row.try_get::<i64, _>(0).unwrap_or(0),
                Err(_) => 0,
            };

            Ok(DetailedStats {
                total_papers,
                by_status,
                queue_queued,
                queue_running,
                cache_entries,
                dedup_records,
            })
        })
    }
}

fn map_paper_code_trace_row(row: &SqliteRow) -> Result<PaperCodeTrace> {
    let untagged: String = row.try_get("untagged_ranges")?;
    let unreferenced: String = row.try_get("unreferenced_sources")?;
    let refs: String = row.try_get("paper_section_refs")?;
    let gap_ids_str: String = row.try_get("gap_ids")?;

    Ok(PaperCodeTrace {
        id: row.try_get("id")?,
        paper_id: row.try_get("paper_id")?,
        code_path: row.try_get("code_path")?,
        module_name: row.try_get("module_name")?,
        framework: row.try_get("framework")?,
        total_code_lines: row.try_get("total_code_lines")?,
        tagged_lines: row.try_get("tagged_lines")?,
        untagged_ranges: serde_json::from_str(&untagged).unwrap_or_default(),
        unreferenced_sources: serde_json::from_str(&unreferenced).unwrap_or_default(),
        paper_section_refs: serde_json::from_str(&refs).unwrap_or_default(),
        gap_ids: serde_json::from_str(&gap_ids_str).unwrap_or_default(),
        benchmark_pass_rate: row.try_get("benchmark_pass_rate")?,
        created_at: row.try_get("created_at")?,
    })
}

fn map_job_row(row: &SqliteRow) -> Result<JobQueueEntry> {
    Ok(JobQueueEntry {
        id: row.try_get(0)?,
        paper_id: row.try_get(1)?,
        job_type: row.try_get(2)?,
        status: row.try_get(3)?,
        priority: row.try_get(4)?,
        created_at: row.try_get(5)?,
        started_at: row.try_get(6)?,
        completed_at: row.try_get(7)?,
        error: row.try_get(8)?,
    })
}

#[inline(always)]
pub fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    let dot: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm_a == 0.0 || norm_b == 0.0 {
        0.0
    } else {
        dot / (norm_a * norm_b)
    }
}

/// Simple base64 encoding (for export_papers blob conversion).
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DbStats {
    pub total: i64,
    pub pending: i64,
    pub done: i64,
    pub gaps: i64,
}

/// Detailed database statistics matching rairos-db's get_stats format.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DetailedStats {
    pub total_papers: i64,
    pub by_status: HashMap<String, i64>,
    pub queue_queued: i64,
    pub queue_running: i64,
    pub cache_entries: i64,
    pub dedup_records: i64,
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

    #[test]
    fn test_slugify() {
        assert_eq!(super::slugify("Hello World!", 80), "Hello-World");
        assert_eq!(super::slugify("  Test   Title  ", 80), "Test-Title");
        assert_eq!(super::slugify("Paper: A Study", 80), "Paper-A-Study");
        assert_eq!(super::slugify("", 80), "Paper");
        assert_eq!(super::slugify("Hello", 3), "Hel");
        assert_eq!(super::slugify("Hello-World_v1.0", 80), "Hello-World_v10");
    }

    #[test]
    fn test_safe_uid() {
        assert_eq!(basics::safe_uid("hello world"), "hello_world");
        assert_eq!(basics::safe_uid("test@123"), "test_123");
        assert_eq!(basics::safe_uid(""), "");
    }

    #[test]
    fn test_achievement_system() {
        let mut system = super::AchievementSystem::new();
        assert_eq!(system.total_points(), 0);

        // First import achievement
        let unlocked = system.update_stats(None, None, None, None, Some(1));
        assert_eq!(unlocked.len(), 1);
        assert_eq!(unlocked[0].id, "first_import");
        assert_eq!(system.total_points(), 10);

        // Already unlocked, should not unlock again
        let unlocked = system.update_stats(None, None, None, None, Some(2));
        assert_eq!(unlocked.len(), 0);
        assert_eq!(system.total_points(), 10);

        // Paper collector at 10 papers
        let unlocked = system.update_stats(Some(10), None, None, None, None);
        assert_eq!(unlocked.len(), 1);
        assert_eq!(unlocked[0].id, "paper_collector");
        assert_eq!(system.total_points(), 60); // 10 + 50

        let pending = system.get_pending_achievements();
        assert!(pending.len() >= 6); // Most should still be pending
    }
}

// ============================================================================
// String Utilities
// ============================================================================

static RE_SPACES: LazyLock<regex::Regex> = LazyLock::new(|| regex::Regex::new(r" {2,}").expect("valid regex"));
static RE_NONWORD: LazyLock<regex::Regex> = LazyLock::new(|| regex::Regex::new(r"[^\w\s\-]").expect("valid regex"));
static RE_DASHES: LazyLock<regex::Regex> = LazyLock::new(|| regex::Regex::new(r"-{2,}").expect("valid regex"));

/// Create a slug from a title. Use slugify_default for the standard 80-char limit.
pub fn slugify(title: &str, max_len: usize) -> String {
    if title.is_empty() {
        return "Paper".to_string();
    }
    let t = title.trim();
    let t = RE_SPACES.replace_all(t, " ");
    let t = RE_NONWORD.replace_all(&t, "");
    let t = t.replace(' ', "-");
    let t = RE_DASHES.replace_all(&t, "-").trim_matches('-').to_string();
    if t.len() > max_len {
        t[..max_len].trim_end_matches('-').to_string()
    } else {
        t
    }
}

/// Create a slug with default max_len of 80.
pub fn slugify_default(title: &str) -> String {
    slugify(title, 80)
}

// Default research directory names (canonical order).
// ============================================================================
// API Endpoints
// Note: ARXIV_API, SEMANTIC_API, CROSSREF_WORKS, DOI_RESOLVER are now in constants.rs

// ============================================================================
// Output Files
// Note: RADAR_FILE, TIMELINE_FILE are now in constants.rs

// ============================================================================
// Research Tree
// ============================================================================

pub const DEFAULT_RESEARCH_DIRS: &[&str] = &[
    "00-Radar",
    "01-Foundations",
    "02-Models",
    "03-Training",
    "04-Scaling",
    "05-Alignment",
    "06-Agents",
    "07-Infrastructure",
    "08-Optimization",
    "09-Evaluation",
    "10-Applications",
    "11-Future-Directions",
];

// ============================================================================
// Achievement System
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Achievement {
    pub id: String,
    pub name: String,
    pub description: String,
    pub icon: String,
    pub points: u32,
    pub unlocked_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserStats {
    pub papers_processed: u32,
    pub api_calls_saved: u32,
    pub hours_saved: f64,
    pub searches_performed: u32,
    pub imports_performed: u32,
}

impl Default for UserStats {
    fn default() -> Self {
        Self {
            papers_processed: 0,
            api_calls_saved: 0,
            hours_saved: 0.0,
            searches_performed: 0,
            imports_performed: 0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AchievementSystem {
    achievements: HashMap<String, Achievement>,
    total_points: u32,
    pub user_stats: UserStats,
}

impl Default for AchievementSystem {
    fn default() -> Self {
        Self::new()
    }
}

impl AchievementSystem {
    pub fn new() -> Self {
        let mut achievements = HashMap::new();
        achievements.insert(
            "first_import".to_string(),
            Achievement {
                id: "first_import".to_string(),
                name: "🚀 首次导入".to_string(),
                description: "成功导入第一篇论文".to_string(),
                icon: "📥".to_string(),
                points: 10,
                unlocked_at: None,
            },
        );
        achievements.insert(
            "paper_collector".to_string(),
            Achievement {
                id: "paper_collector".to_string(),
                name: "📚 论文收集者".to_string(),
                description: "导入10篇论文".to_string(),
                icon: "📚".to_string(),
                points: 50,
                unlocked_at: None,
            },
        );
        achievements.insert(
            "researcher_100".to_string(),
            Achievement {
                id: "researcher_100".to_string(),
                name: "🎓 研究达人".to_string(),
                description: "导入100篇论文".to_string(),
                icon: "🎓".to_string(),
                points: 200,
                unlocked_at: None,
            },
        );
        achievements.insert(
            "api_saver".to_string(),
            Achievement {
                id: "api_saver".to_string(),
                name: "💰 API节流侠".to_string(),
                description: "通过缓存节省100次API调用".to_string(),
                icon: "💰".to_string(),
                points: 100,
                unlocked_at: None,
            },
        );
        achievements.insert(
            "time_saver".to_string(),
            Achievement {
                id: "time_saver".to_string(),
                name: "⏰ 时间管理大师".to_string(),
                description: "节省10小时研究时间".to_string(),
                icon: "⏰".to_string(),
                points: 150,
                unlocked_at: None,
            },
        );
        achievements.insert(
            "speed_demon".to_string(),
            Achievement {
                id: "speed_demon".to_string(),
                name: "⚡ 速度达人".to_string(),
                description: "批量导入50篇论文".to_string(),
                icon: "⚡".to_string(),
                points: 100,
                unlocked_at: None,
            },
        );
        achievements.insert(
            "cache_master".to_string(),
            Achievement {
                id: "cache_master".to_string(),
                name: "🗄️ 缓存大师".to_string(),
                description: "缓存命中率超过80%".to_string(),
                icon: "🗄️".to_string(),
                points: 75,
                unlocked_at: None,
            },
        );
        achievements.insert(
            "search_expert".to_string(),
            Achievement {
                id: "search_expert".to_string(),
                name: "🔍 搜索专家".to_string(),
                description: "执行100次搜索".to_string(),
                icon: "🔍".to_string(),
                points: 50,
                unlocked_at: None,
            },
        );

        Self {
            achievements,
            total_points: 0,
            user_stats: UserStats::default(),
        }
    }

    pub fn unlock_achievement(&mut self, achievement_id: &str) -> Option<&Achievement> {
        let achievement = self.achievements.get_mut(achievement_id)?;
        if achievement.unlocked_at.is_none() {
            achievement.unlocked_at = Some(chrono::Utc::now().to_rfc3339());
            self.total_points += achievement.points;
            Some(achievement)
        } else {
            None
        }
    }

    #[allow(clippy::collapsible_if)]
    pub fn check_and_unlock(&mut self) -> Vec<Achievement> {
        let mut unlocked_ids: Vec<String> = Vec::new();

        if self.user_stats.imports_performed >= 1 {
            if self.unlock_achievement("first_import").is_some() {
                unlocked_ids.push("first_import".to_string());
            }
        }
        if self.user_stats.papers_processed >= 10 {
            if self.unlock_achievement("paper_collector").is_some() {
                unlocked_ids.push("paper_collector".to_string());
            }
        }
        if self.user_stats.papers_processed >= 100 {
            if self.unlock_achievement("researcher_100").is_some() {
                unlocked_ids.push("researcher_100".to_string());
            }
        }
        if self.user_stats.api_calls_saved >= 100 {
            if self.unlock_achievement("api_saver").is_some() {
                unlocked_ids.push("api_saver".to_string());
            }
        }
        if self.user_stats.hours_saved >= 10.0 {
            if self.unlock_achievement("time_saver").is_some() {
                unlocked_ids.push("time_saver".to_string());
            }
        }
        if self.user_stats.papers_processed >= 50 {
            if self.unlock_achievement("speed_demon").is_some() {
                unlocked_ids.push("speed_demon".to_string());
            }
        }

        unlocked_ids
            .into_iter()
            .filter_map(|id| self.achievements.get(&id).cloned())
            .collect()
    }

    pub fn update_stats(
        &mut self,
        papers_processed: Option<u32>,
        api_calls_saved: Option<u32>,
        hours_saved: Option<f64>,
        searches_performed: Option<u32>,
        imports_performed: Option<u32>,
    ) -> Vec<Achievement> {
        if let Some(v) = papers_processed {
            self.user_stats.papers_processed = v;
        }
        if let Some(v) = api_calls_saved {
            self.user_stats.api_calls_saved = v;
        }
        if let Some(v) = hours_saved {
            self.user_stats.hours_saved = v;
        }
        if let Some(v) = searches_performed {
            self.user_stats.searches_performed = v;
        }
        if let Some(v) = imports_performed {
            self.user_stats.imports_performed = v;
        }
        self.check_and_unlock()
    }

    pub fn get_unlocked_achievements(&self) -> Vec<Achievement> {
        self.achievements
            .values()
            .filter(|a| a.unlocked_at.is_some())
            .cloned()
            .collect()
    }

    pub fn get_pending_achievements(&self) -> Vec<Achievement> {
        self.achievements
            .values()
            .filter(|a| a.unlocked_at.is_none())
            .cloned()
            .collect()
    }

    pub fn total_points(&self) -> u32 {
        self.total_points
    }
}

/// Compute Jaccard similarity between two string collections.
pub fn jaccard_similarity(a: &[String], b: &[String]) -> f64 {
    if a.is_empty() || b.is_empty() {
        return 0.0;
    }
    let set_a: FxHashSet<&str> = a.iter().map(|s| s.as_str()).collect();
    let set_b: FxHashSet<&str> = b.iter().map(|s| s.as_str()).collect();
    let intersection = set_a.intersection(&set_b).count();
    let union = set_a.union(&set_b).count();
    intersection as f64 / union as f64
}

// ========== Code Gene: 18b0f121 ==========
// add HashMap cache for hot path optimization
pub fn cached_hot_path(cache: &mut std::collections::HashMap<String, f32>, key: String, compute: impl FnOnce() -> f32) -> f32 {
    *cache.entry(key).or_insert_with(compute)
}
