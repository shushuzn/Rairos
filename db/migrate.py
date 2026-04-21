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
CURRENT_VERSION: Final[int] = 1

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


# Registry — key = version this migration brings you TO
_MIGRATIONS: dict[int, Migration] = {
    1: _m1_add_citations_and_tables,
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
