//! rairos-db-migrate — Schema migration system for AI Research OS database.
//!
//! Ported from `db/migrate.py`.
//!
//! Tracks schema version in the `settings` table and applies incremental migrations
//! forward only. Each migration is a callable that takes a connection and raises
//! no error if it is a no-op (idempotent).

use rusqlite::{Connection, Result as SqliteResult};
use std::collections::HashMap;

/// Current schema version — bump whenever you add a new migration.
pub const CURRENT_VERSION: i32 = 6;

/// Migration function type.
pub type Migration = fn(&Connection) -> SqliteResult<()>;

/// Apply migration 1: Add citations and experiment_tables tables.
fn m1_add_citations_and_tables(conn: &Connection) -> SqliteResult<()> {
    conn.execute_batch(
        r#"
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
            bbox_y0       REAL  DEFAULT 0,
            bbox_x1       REAL DEFAULT 0,
            bbox_y1       REAL DEFAULT 0,
            created_at    TEXT NOT NULL,
            FOREIGN KEY (paper_id) REFERENCES papers(id) ON DELETE CASCADE
        );

        CREATE INDEX IF NOT EXISTS idx_experiment_tables_paper_id ON experiment_tables(paper_id);
        "#,
    )?;
    Ok(())
}

/// Apply migration 2: Add reading status tracking columns.
fn m2_add_reading_status(conn: &Connection) -> SqliteResult<()> {
    let _ = conn.execute(
        "ALTER TABLE papers ADD COLUMN reading_status TEXT DEFAULT 'unread'",
        [],
    );
    let _ = conn.execute(
        "UPDATE papers SET reading_status = 'unread' WHERE reading_status IS NULL",
        [],
    );
    let _ = conn.execute(
        "ALTER TABLE papers ADD COLUMN reading_started_at TEXT",
        [],
    );
    let _ = conn.execute(
        "ALTER TABLE papers ADD COLUMN reading_completed_at TEXT",
        [],
    );
    let _ = conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_papers_reading_status ON papers(reading_status)",
        [],
    );
    Ok(())
}

/// Apply migration 3: Add chat sessions for persistent conversation history.
fn m3_add_chat_sessions(conn: &Connection) -> SqliteResult<()> {
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS chat_sessions (
            id            TEXT PRIMARY KEY,
            title         TEXT DEFAULT '',
            created_at    TEXT NOT NULL,
            updated_at    TEXT NOT NULL
        );

        CREATE INDEX IF NOT EXISTS idx_chat_sessions_updated ON chat_sessions(updated_at);

        CREATE TABLE IF NOT EXISTS chat_messages (
            id            INTEGER PRIMARY KEY AUTOINCREMENT,
            session_id    TEXT NOT NULL,
            role          TEXT NOT NULL,
            content       TEXT NOT NULL,
            citations     TEXT DEFAULT '[]',
            created_at    TEXT NOT NULL,
            FOREIGN KEY (session_id) REFERENCES chat_sessions(id) ON DELETE CASCADE
        );

        CREATE INDEX IF NOT EXISTS idx_chat_messages_session ON chat_messages(session_id);
        "#,
    )?;
    Ok(())
}

/// Apply migration 4: Add arXiv subscription tables for smart paper discovery.
fn m4_add_arxiv_subscriptions(conn: &Connection) -> SqliteResult<()> {
    conn.execute_batch(
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
        );

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
        );

        CREATE INDEX IF NOT EXISTS idx_arxiv_subscriptions_topic ON arxiv_subscriptions(topic);
        CREATE INDEX IF NOT EXISTS idx_arxiv_subscription_papers_sub ON arxiv_subscription_papers(subscription_id);
        "#,
    )?;
    Ok(())
}

/// Apply migration 5: Add literature_reviews table for incremental review tracking.
fn m5_add_literature_reviews(conn: &Connection) -> SqliteResult<()> {
    conn.execute_batch(
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
        );

        CREATE INDEX IF NOT EXISTS idx_literature_reviews_topic ON literature_reviews(topic);
        CREATE INDEX IF NOT EXISTS idx_literature_reviews_subscription ON literature_reviews(subscription_id);
        "#,
    )?;
    Ok(())
}

/// Apply migration 6: Add embeddings table for vector similarity search.
fn m6_add_embeddings_table(conn: &Connection) -> SqliteResult<()> {
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS embeddings (
            paper_id    TEXT PRIMARY KEY,
            vector      BLOB NOT NULL,
            updated_at  TEXT NOT NULL,
            model       TEXT DEFAULT 'nomic-embed-text'
        );

        CREATE INDEX IF NOT EXISTS idx_embeddings_updated ON embeddings(updated_at);
        "#,
    )?;
    Ok(())
}

/// Registry of migrations — key = version this migration brings you TO
fn get_migrations() -> HashMap<i32, fn(&Connection) -> SqliteResult<()>> {
    let mut m: HashMap<i32, fn(&Connection) -> SqliteResult<()>> = HashMap::new();
    m.insert(1, m1_add_citations_and_tables);
    m.insert(2, m2_add_reading_status);
    m.insert(3, m3_add_chat_sessions);
    m.insert(4, m4_add_arxiv_subscriptions);
    m.insert(5, m5_add_literature_reviews);
    m.insert(6, m6_add_embeddings_table);
    m
}

/// Get the current schema version from the settings table.
pub fn get_schema_version(conn: &Connection) -> SqliteResult<i32> {
    match conn.query_row(
        "SELECT value FROM settings WHERE key = 'schema_version'",
        [],
        |row| row.get::<_, String>(0),
    ) {
        Ok(v) => Ok(v.parse().unwrap_or(0)),
        Err(_) => Ok(0),
    }
}

/// Set the schema version in the settings table.
pub fn set_schema_version(conn: &Connection, version: i32) -> SqliteResult<()> {
    conn.execute(
        "INSERT OR REPLACE INTO settings (key, value) VALUES ('schema_version', ?1)",
        [&version.to_string()],
    )?;
    Ok(())
}

/// Apply all pending migrations in order.
///
/// Returns the number of migrations applied.
pub fn run_migrations(conn: &Connection) -> SqliteResult<i32> {
    let current = get_schema_version(conn)?;
    if current >= CURRENT_VERSION {
        log::debug!("Schema already at version {}", current);
        return Ok(0);
    }

    let migrations = get_migrations();
    let mut applied = 0;

    for version in (current + 1)..=CURRENT_VERSION {
        if let Some(migration) = migrations.get(&version) {
            log::info!("Applying schema migration {} → {}", version - 1, version);
            migration(conn)?;
            set_schema_version(conn, version)?;
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

    #[test]
    fn test_get_schema_version_nonexistent() {
        let conn = Connection::open_in_memory().unwrap();
        // Create settings table
        conn.execute(
            "CREATE TABLE IF NOT EXISTS settings (key TEXT PRIMARY KEY, value TEXT)",
            [],
        )
        .unwrap();

        let version = get_schema_version(&conn).unwrap();
        assert_eq!(version, 0);
    }

    #[test]
    fn test_set_and_get_schema_version() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute(
            "CREATE TABLE IF NOT EXISTS settings (key TEXT PRIMARY KEY, value TEXT)",
            [],
        )
        .unwrap();

        set_schema_version(&conn, 3).unwrap();
        let version = get_schema_version(&conn).unwrap();
        assert_eq!(version, 3);
    }
}
