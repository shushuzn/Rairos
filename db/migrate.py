"""Schema migration system for AI Research OS database.

Tracks schema version in the `settings` table and applies incremental migrations
forward only. Each migration is a callable that takes a connection and raises
no error if it is a no-op (idempotent).
"""
from __future__ import annotations

import logging
import sqlite3
from typing import Callable, Final

logger = logging.getLogger(__name__)

# Current schema version — bump whenever you add a new migration.
CURRENT_VERSION: Final[int] = 5

# Type alias for migration functions.
Migration = Callable[[sqlite3.Connection], None]


# ─── Migrations ────────────────────────────────────────────────────────────────


def _m1_add_citations_and_tables(conn: sqlite3.Connection) -> None:
    """Migration 1: Add citations and experiment_tables tables (baseline for v1+).

    These tables were part of the original _SCHEMA but had no version tracking.
    This migration is a no-op for any database that already has them.
    """
    conn.executescript("""
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
    """)


def _m2_add_reading_status(conn: sqlite3.Connection) -> None:
    """Migration 2: Add reading status tracking columns.

    Tracks user reading progress: unread -> reading -> completed
    """
    try:
        conn.execute("ALTER TABLE papers ADD COLUMN reading_status TEXT DEFAULT 'unread'")
    except sqlite3.OperationalError:
        pass  # Column already exists
    # Backfill existing rows — ALTER DEFAULT does not affect existing rows.
    conn.execute("UPDATE papers SET reading_status = 'unread' WHERE reading_status IS NULL")
    try:
        conn.execute("ALTER TABLE papers ADD COLUMN reading_started_at TEXT")
    except sqlite3.OperationalError:
        pass
    try:
        conn.execute("ALTER TABLE papers ADD COLUMN reading_completed_at TEXT")
    except sqlite3.OperationalError:
        pass
    # Create index for faster reading status queries
    try:
        conn.execute("CREATE INDEX IF NOT EXISTS idx_papers_reading_status ON papers(reading_status)")
    except sqlite3.OperationalError:
        pass


# Registry — key = version this migration brings you TO
def _m5_add_literature_reviews(conn: sqlite3.Connection) -> None:
    """Migration 5: Add literature_reviews table for incremental review tracking."""
    conn.executescript("""
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
    """)


def _m4_add_arxiv_subscriptions(conn: sqlite3.Connection) -> None:
    """Migration 4: Add arXiv subscription tables for smart paper discovery."""
    conn.executescript("""
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
    """)


def _m3_add_chat_sessions(conn: sqlite3.Connection) -> None:
    """Migration 3: Add chat sessions for persistent conversation history."""
    conn.executescript("""
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
    """)


_MIGRATIONS: dict[int, Migration] = {
    1: _m1_add_citations_and_tables,
    2: _m2_add_reading_status,
    3: _m3_add_chat_sessions,
    4: _m4_add_arxiv_subscriptions,
    5: _m5_add_literature_reviews,
}


def get_schema_version(conn: sqlite3.Connection) -> int:
    """Return the stored schema version, or 0 if not yet recorded."""
    try:
        cur = conn.execute(
            "SELECT value FROM settings WHERE key = 'schema_version'"
        )
        row = cur.fetchone()
        return int(row[0]) if row else 0
    except sqlite3.OperationalError:
        return 0


def set_schema_version(conn: sqlite3.Connection, version: int) -> None:
    """Record the schema version in the settings table."""
    conn.execute(
        "INSERT OR REPLACE INTO settings (key, value) VALUES ('schema_version', ?)",
        (str(version),),
    )
    conn.commit()


def run_migrations(conn: sqlite3.Connection) -> int:
    """
    Apply all pending migrations in order.

    Returns the number of migrations applied.
    """
    current = get_schema_version(conn)
    if current >= CURRENT_VERSION:
        logger.debug("Schema already at version %d", current)
        return 0

    applied = 0
    for version in range(current + 1, CURRENT_VERSION + 1):
        migration = _MIGRATIONS.get(version)
        if migration is None:
            logger.warning("No migration found for version %d — skipping", version)
            continue
        logger.info("Applying schema migration %d → %d", version - 1, version)
        migration(conn)
        set_schema_version(conn, version)
        applied += 1
        logger.info("Schema migration %d applied successfully", version)

    return applied
