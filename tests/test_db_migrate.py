"""Tests for db/migrate.py."""

import sys
import sqlite3
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parent.parent))
from db.migrate import CURRENT_VERSION


class TestCurrentVersion:
    def test_version_is_int(self):
        assert isinstance(CURRENT_VERSION, int)
        assert CURRENT_VERSION >= 1

    def test_version_reasonable(self):
        assert CURRENT_VERSION <= 100


class TestMigrationType:
    def test_migration_is_callable(self):
        # Migration is a type alias; check it's used correctly
        # The actual migrations are functions matching the signature
        from db.migrate import _m1_add_citations_and_tables
        assert callable(_m1_add_citations_and_tables)


class TestMigrationFunction:
    def test_m1_idempotent(self, tmp_path):
        """Migration 1 should be idempotent (CREATE TABLE IF NOT EXISTS)."""
        from db.migrate import _m1_add_citations_and_tables

        db_path = tmp_path / "test.db"
        conn = sqlite3.connect(db_path)
        # Run twice — should not raise
        _m1_add_citations_and_tables(conn)
        _m1_add_citations_and_tables(conn)
        conn.close()

    def test_m1_creates_citations_table(self, tmp_path):
        from db.migrate import _m1_add_citations_and_tables

        db_path = tmp_path / "test.db"
        conn = sqlite3.connect(db_path)
        _m1_add_citations_and_tables(conn)
        conn.commit()

        # Verify table exists
        cursor = conn.execute(
            "SELECT name FROM sqlite_master WHERE type='table' AND name='citations'"
        )
        row = cursor.fetchone()
        assert row is not None, "citations table should exist after m1"
        conn.close()
