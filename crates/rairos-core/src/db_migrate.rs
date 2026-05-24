//! rairos-db-migrate — Schema migration system for AI Research OS database.
//!
//! Ported from `db/migrate.py`.
//!
//! Tracks schema version in the `settings` table and applies incremental migrations
//! forward only. Each migration is a callable that takes a connection and raises
//! no error if it is a no-op (idempotent).

use sqlx::sqlite::SqliteConnection;
use sqlx::Row;
use std::collections::HashMap;

/// Current schema version — bump whenever you add a new migration.
pub const CURRENT_VERSION: i32 = 6;

/// Migration function type - async version.
pub type Migration =
    for<'a> fn(conn: &'a mut SqliteConnection) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<(), sqlx::Error>> + 'a>>;

/// Apply migration 1: Add citations and experiment_tables tables.
async fn m1_add_citations_and_tables(conn: &mut SqliteConnection) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS citations (
            id          INTEGER PRIMARY KEY AUTOINCREMENT,
            source_id   TEXT NOT NULL,
            target_id   TEXT NOT NULL,
            created_at  TEXT NOT NULL,
            FOREIGN KEY (source_id)  REFERENCES papers(id)  ON DELETE CASCADE,
            FOREIGN KEY (target_id)  REFERENCES papers(id)  ON DELETE CASCADE,
            UNIQUE(source_id, target_id)
        )
        "#,
    )
    .execute(&mut *conn)
    .await?;

    sqlx::query("CREATE INDEX IF NOT EXISTS idx_citations_source ON citations(source_id)")
        .execute(&mut *conn)
        .await?;
    sqlx::query("CREATE INDEX IF NOT EXISTS idx_citations_target ON citations(target_id)")
        .execute(&mut *conn)
        .await?;

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
            bbox_y0       REAL  DEFAULT 0,
            bbox_x1       REAL DEFAULT 0,
            bbox_y1       REAL DEFAULT 0,
            created_at    TEXT NOT NULL,
            FOREIGN KEY (paper_id) REFERENCES papers(id) ON DELETE CASCADE
        )
        "#,
    )
    .execute(&mut *conn)
    .await?;

    sqlx::query(
        "CREATE INDEX IF NOT EXISTS idx_experiment_tables_paper_id ON experiment_tables(paper_id)",
    )
    .execute(&mut *conn)
    .await?;

    Ok(())
}

/// Apply migration 2: Add reading status tracking columns.
async fn m2_add_reading_status(conn: &mut SqliteConnection) -> Result<(), sqlx::Error> {
    let _ = sqlx::query("ALTER TABLE papers ADD COLUMN reading_status TEXT DEFAULT 'unread'")
        .execute(&mut *conn)
        .await;
    let _ = sqlx::query("UPDATE papers SET reading_status = 'unread' WHERE reading_status IS NULL")
        .execute(&mut *conn)
        .await;
    let _ = sqlx::query("ALTER TABLE papers ADD COLUMN reading_started_at TEXT")
        .execute(&mut *conn)
        .await;
    let _ = sqlx::query("ALTER TABLE papers ADD COLUMN reading_completed_at TEXT")
        .execute(&mut *conn)
        .await;
    let _ = sqlx::query(
        "CREATE INDEX IF NOT EXISTS idx_papers_reading_status ON papers(reading_status)",
    )
    .execute(&mut *conn)
    .await;
    Ok(())
}

/// Apply migration 3: Add chat sessions for persistent conversation history.
async fn m3_add_chat_sessions(conn: &mut SqliteConnection) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS chat_sessions (
            id            TEXT PRIMARY KEY,
            title         TEXT DEFAULT '',
            created_at    TEXT NOT NULL,
            updated_at    TEXT NOT NULL
        )
        "#,
    )
    .execute(&mut *conn)
    .await?;

    sqlx::query("CREATE INDEX IF NOT EXISTS idx_chat_sessions_updated ON chat_sessions(updated_at)")
        .execute(&mut *conn)
        .await?;

    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS chat_messages (
            id            INTEGER PRIMARY KEY AUTOINCREMENT,
            session_id    TEXT NOT NULL,
            role          TEXT NOT NULL,
            content       TEXT NOT NULL,
            citations     TEXT DEFAULT '[]',
            created_at    TEXT NOT NULL,
            FOREIGN KEY (session_id) REFERENCES chat_sessions(id) ON DELETE CASCADE
        )
        "#,
    )
    .execute(&mut *conn)
    .await?;

    sqlx::query("CREATE INDEX IF NOT EXISTS idx_chat_messages_session ON chat_messages(session_id)")
        .execute(&mut *conn)
        .await?;

    Ok(())
}

/// Apply migration 4: Add arXiv subscription tables for smart paper discovery.
async fn m4_add_arxiv_subscriptions(conn: &mut SqliteConnection) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS arxiv_subscriptions (
            id              TEXT PRIMARY KEY,
            topic           TEXT NOT NULL,
            keywords        TEXT DEFAULT '[]',
            max_results     INTEGER DEFAULT 10,
            min_score       REAL DEFAULT 0.5,
            last_checked   TEXT,
            last_check_id   TEXT DEFAULT '',
            enabled         INTEGER DEFAULT 1,
            created_at      TEXT NOT NULL
        )
        "#,
    )
    .execute(&mut *conn)
    .await?;

    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS arxiv_subscription_papers (
            id              INTEGER PRIMARY KEY AUTOINCREMENT,
            subscription_id TEXT NOT NULL,
            arxiv_id        TEXT NOT NULL,
            title           TEXT,
            score           REAL,
            gap_coverage    REAL,
            semantic_sim    REAL,
            published       TEXT,
            notified_at     TEXT,
            created_at      TEXT NOT NULL,
            FOREIGN KEY (subscription_id) REFERENCES arxiv_subscriptions(id) ON DELETE CASCADE,
            UNIQUE(subscription_id, arxiv_id)
        )
        "#,
    )
    .execute(&mut *conn)
    .await?;

    sqlx::query(
        "CREATE INDEX IF NOT EXISTS idx_arxiv_subscriptions_topic ON arxiv_subscriptions(topic)",
    )
    .execute(&mut *conn)
    .await?;
    sqlx::query(
        "CREATE INDEX IF NOT EXISTS idx_arxiv_subscription_papers_sub ON arxiv_subscription_papers(subscription_id)",
    )
    .execute(&mut *conn)
    .await?;

    Ok(())
}

/// Apply migration 5: Add literature_reviews table for incremental review tracking.
async fn m5_add_literature_reviews(conn: &mut SqliteConnection) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS literature_reviews (
            id              TEXT PRIMARY KEY,
            topic           TEXT NOT NULL,
            subscription_id TEXT,
            file_path       TEXT,
            paper_count     INTEGER DEFAULT 0,
            last_updated    TEXT,
            created_at      TEXT NOT NULL,
            FOREIGN KEY (subscription_id) REFERENCES arxiv_subscriptions(id) ON DELETE SET NULL
        )
        "#,
    )
    .execute(&mut *conn)
    .await?;

    sqlx::query("CREATE INDEX IF NOT EXISTS idx_literature_reviews_topic ON literature_reviews(topic)")
        .execute(&mut *conn)
        .await?;
    sqlx::query(
        "CREATE INDEX IF NOT EXISTS idx_literature_reviews_subscription ON literature_reviews(subscription_id)",
    )
    .execute(&mut *conn)
    .await?;

    Ok(())
}

/// Apply migration 6: Add embeddings table for vector similarity search.
async fn m6_add_embeddings_table(conn: &mut SqliteConnection) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS embeddings (
            paper_id    TEXT PRIMARY KEY,
            vector      BLOB NOT NULL,
            updated_at  TEXT NOT NULL,
            model       TEXT DEFAULT 'nomic-embed-text'
        )
        "#,
    )
    .execute(&mut *conn)
    .await?;

    sqlx::query("CREATE INDEX IF NOT EXISTS idx_embeddings_updated ON embeddings(updated_at)")
        .execute(&mut *conn)
        .await?;

    Ok(())
}

/// Registry of migrations — key = version this migration brings you TO
fn get_migrations() -> HashMap<i32, Migration> {
    let mut m: HashMap<i32, Migration> = HashMap::new();
    m.insert(1, |conn| Box::pin(m1_add_citations_and_tables(conn)));
    m.insert(2, |conn| Box::pin(m2_add_reading_status(conn)));
    m.insert(3, |conn| Box::pin(m3_add_chat_sessions(conn)));
    m.insert(4, |conn| Box::pin(m4_add_arxiv_subscriptions(conn)));
    m.insert(5, |conn| Box::pin(m5_add_literature_reviews(conn)));
    m.insert(6, |conn| Box::pin(m6_add_embeddings_table(conn)));
    m
}

/// Get the current schema version from the settings table.
pub async fn get_schema_version(conn: &mut SqliteConnection) -> Result<i32, sqlx::Error> {
    match sqlx::query("SELECT value FROM settings WHERE key = 'schema_version'")
        .fetch_one(conn)
        .await
    {
        Ok(row) => {
            let v: String = row.get(0);
            Ok(v.parse().unwrap_or(0))
        }
        Err(_) => Ok(0),
    }
}

/// Set the schema version in the settings table.
pub async fn set_schema_version(conn: &mut SqliteConnection, version: i32) -> Result<(), sqlx::Error> {
    sqlx::query("INSERT OR REPLACE INTO settings (key, value) VALUES ('schema_version', ?1)")
        .bind(version.to_string())
        .execute(&mut *conn)
        .await?;
    Ok(())
}

/// Apply all pending migrations in order.
///
/// Returns the number of migrations applied.
pub async fn run_migrations(conn: &mut SqliteConnection) -> Result<i32, sqlx::Error> {
    let current = get_schema_version(conn).await?;
    if current >= CURRENT_VERSION {
        log::debug!("Schema already at version {}", current);
        return Ok(0);
    }

    let migrations = get_migrations();
    let mut applied = 0;

    for version in (current + 1)..=CURRENT_VERSION {
        if let Some(migration) = migrations.get(&version) {
            log::info!("Applying schema migration {} → {}", version - 1, version);
            migration(conn).await?;
            set_schema_version(conn, version).await?;
            applied += 1;
            log::info!("Schema migration {} applied successfully", version);
        } else {
            log::warn!("No migration found for version {} — skipping", version);
        }
    }

    Ok(applied)
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::Connection;

    #[tokio::test]
    async fn test_get_schema_version_nonexistent() {
        let mut conn = SqliteConnection::connect("sqlite::memory:").await.unwrap();
        // Create settings table
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS settings (key TEXT PRIMARY KEY, value TEXT)",
        )
        .execute(&mut conn)
        .await
        .unwrap();

        let version = get_schema_version(&mut conn).await.unwrap();
        assert_eq!(version, 0);
    }

    #[tokio::test]
    async fn test_set_and_get_schema_version() {
        let mut conn = SqliteConnection::connect("sqlite::memory:").await.unwrap();
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS settings (key TEXT PRIMARY KEY, value TEXT)",
        )
        .execute(&mut conn)
        .await
        .unwrap();

        set_schema_version(&mut conn, 3).await.unwrap();
        let version = get_schema_version(&mut conn).await.unwrap();
        assert_eq!(version, 3);
    }
}
