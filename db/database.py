"""SQLite database layer for AI Research OS.

Schema:
    papers         – one row per unique paper
    parse_history  – audit trail for each parse attempt
    tags           – tag name lookup
    paper_tags     – many-to-many paper ↔ tag
    job_queue      – batch processing queue
    settings       – key-value store
"""
from __future__ import annotations

import json
import logging
import sqlite3
import struct
import threading
import warnings
from contextlib import contextmanager
from dataclasses import dataclass, field
from datetime import datetime, timezone
from functools import lru_cache
from pathlib import Path
from typing import Any, Dict, Generator, List, Literal, Optional, Tuple

from core.exceptions import DatabaseError
from db.migrate import run_migrations

logger = logging.getLogger(__name__)


@lru_cache(maxsize=4096)
def _parse_authors_cached(raw: str) -> List[str]:
    """Parse authors JSON with caching to avoid repeated json.loads in N+1 loops."""
    try:
        return json.loads(raw) if raw else []
    except Exception:
        return []

# ─── Schema ────────────────────────────────────────────────────────────────────

_SCHEMA = """
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
    logged_at   TEXT NOT NULL DEFAULT (datetime('now')),
    FOREIGN KEY (target_id)   REFERENCES papers(id) ON DELETE CASCADE,
    FOREIGN KEY (duplicate_id) REFERENCES papers(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_papers_parse_status ON papers(parse_status);
CREATE INDEX IF NOT EXISTS idx_papers_source ON papers(source);
CREATE INDEX IF NOT EXISTS idx_papers_added_at ON papers(added_at);
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
"""


# ─── Search Data Classes ────────────────────────────────────────────────────────


@dataclass
class ExperimentTableRecord:
    id: int
    paper_id: str
    table_caption: str
    page: int
    headers: List[str]
    rows: List[List[str]]
    bbox_x0: float
    bbox_y0: float
    bbox_x1: float
    bbox_y1: float
    created_at: str


@dataclass
class CitationRecord:
    id: int
    source_id: str
    target_id: str
    created_at: str


@dataclass
class SearchResult:
    paper_id: str
    title: str
    authors: List[str]
    published: str
    primary_category: str
    score: float
    snippet: str
    parse_status: str
    source: str
    abs_url: str
    pdf_url: str


# ─── FTS5 Schema ───────────────────────────────────────────────────────────────


_FTS_SCHEMA = """
CREATE VIRTUAL TABLE IF NOT EXISTS papers_fts USING fts5(
    paper_id UNINDEXED,
    title,
    abstract,
    plain_text,
    tokenize='porter unicode61'
);
"""


# ─── Data Classes ──────────────────────────────────────────────────────────────


@dataclass
class PaperRecord:
    id: str
    source: str
    title: str = ""
    authors: List[str] = field(default_factory=list)
    abstract: str = ""
    published: str = ""
    updated: str = ""
    abs_url: str = ""
    pdf_url: str = ""
    primary_category: str = ""
    journal: str = ""
    volume: str = ""
    issue: str = ""
    page: str = ""
    doi: str = ""
    categories: str = ""
    reference_count: int = 0
    added_at: str = ""
    updated_at: str = ""
    pdf_path: str = ""
    pdf_hash: str = ""
    parse_status: str = "pending"
    parse_error: str = ""
    parse_version: int = 0
    plain_text: str = ""
    latex_blocks: List[Any] = field(default_factory=list)
    table_count: int = 0
    figure_count: int = 0
    word_count: int = 0
    page_count: int = 0
    pnote_path: str = ""
    cnote_path: str = ""
    mnote_path: str = ""
    embed_vector: Any = field(default=None)
    tags: List[str] = field(default_factory=list)

    @classmethod
    def from_row(cls, row: sqlite3.Row) -> PaperRecord:
        d = dict(row)
        authors_raw = d.pop("authors", "[]")
        latex_raw = d.pop("latex_blocks", "[]")
        try:
            authors = _parse_authors_cached(authors_raw)
        except Exception:
            warnings.warn(f"PaperRecord.from_row: failed to parse authors JSON for paper {d.get('id', '?')}: {authors_raw[:50]!r}", stacklevel=2)
            authors = []
        try:
            latex_blocks = json.loads(latex_raw) if latex_raw else []
        except Exception:
            warnings.warn(f"PaperRecord.from_row: failed to parse latex_blocks JSON for paper {d.get('id', '?')}", stacklevel=2)
            latex_blocks = []
        return cls(authors=authors, latex_blocks=latex_blocks, tags=[], **d)


# ─── Database ─────────────────────────────────────────────────────────────────


class Database:
    """
    Thread-safe SQLite wrapper for AI Research OS.

    The database file is created automatically on first access.
    """

    def __init__(self, db_path: str | Path = "~/.cache/ai_research_os/research.db"):
        self.db_path = Path(db_path).expanduser()
        self.db_path.parent.mkdir(parents=True, exist_ok=True)
        self._local = threading.local()
        self._init_done = False

    # ── Connection management ──────────────────────────────────────────────────

    @property
    def conn(self) -> sqlite3.Connection:
        """Return a thread-local connection."""
        if not hasattr(self._local, "conn") or self._local.conn is None:
            try:
                conn = sqlite3.connect(str(self.db_path), timeout=30.0)
                conn.row_factory = sqlite3.Row
                conn.execute("PRAGMA journal_mode=WAL")
                conn.execute("PRAGMA foreign_keys=ON")
                conn.execute("PRAGMA busy_timeout=30000")
                self._local.conn = conn
            except sqlite3.Error as e:
                raise DatabaseError(f"Failed to connect to database: {e}") from e
        return self._local.conn

    def init(self) -> None:
        """Create tables and run migrations. Idempotent."""
        if self._init_done:
            return
        if not self.db_path.parent.exists():
            raise DatabaseError(f"Database directory does not exist and cannot be created: {self.db_path.parent}")
        try:
            with self.conn as conn:
                conn.executescript(_SCHEMA)
                conn.executescript(_FTS_SCHEMA)
                run_migrations(conn)
            self._init_done = True
            logger.info("Database initialized at %s", self.db_path)
        except sqlite3.Error as e:
            raise DatabaseError(f"Failed to initialize database: {e}") from e

    @contextmanager
    def transaction(self) -> Generator[None, None, None]:
        """Context manager for explicit transactions.

        Uses Python's implicit transaction management (with conn:) which handles
        nesting correctly under WAL mode. Safe to call within operations that
        have already started an implicit transaction.
        """
        try:
            with self.conn as _conn:
                yield
        except Exception as e:
            warnings.warn(f"Transaction failed, rolling back: {e}", stacklevel=2)
            self.conn.rollback()
            raise

    # ── Papers ────────────────────────────────────────────────────────────────

    def upsert_paper(
        self,
        paper_id: str,
        source: str,
        title: str = "",
        authors: List[str] | str = "",
        abstract: str = "",
        published: str = "",
        updated: str = "",
        abs_url: str = "",
        pdf_url: str = "",
        primary_category: str = "",
        journal: str = "",
        volume: str = "",
        issue: str = "",
        page: str = "",
        doi: str = "",
        categories: str = "",
        reference_count: int = 0,
        pdf_path: str = "",
        pdf_hash: str = "",
        extra: Optional[dict] = None,
    ) -> PaperRecord:
        """Insert or update a paper. Returns the record."""
        now = _utcnow()
        if isinstance(authors, list):
            authors_json = json.dumps(authors, ensure_ascii=False)
        else:
            authors_json = authors

        with self.transaction():
            cur = self.conn.cursor()
            # SELECT first — skip UPDATE if paper already exists with identical fields
            cur.execute("SELECT id FROM papers WHERE id = ?", (paper_id,))
            row = cur.fetchone()

            if row is None:
                cur.execute(
                    """
                    INSERT INTO papers (
                        id, source, title, authors, abstract, published, updated,
                        abs_url, pdf_url, primary_category, journal, volume, issue,
                        page, doi, categories, reference_count, added_at, updated_at,
                        pdf_path, pdf_hash
                    ) VALUES (
                        :id, :source, :title, :authors, :abstract, :published, :updated,
                        :abs_url, :pdf_url, :primary_category, :journal, :volume, :issue,
                        :page, :doi, :categories, :reference_count, :added_at, :updated_at,
                        :pdf_path, :pdf_hash
                    )
                    """,
                    {
                        "id": paper_id,
                        "source": source,
                        "title": title,
                        "authors": authors_json,
                        "abstract": abstract,
                        "published": published,
                        "updated": updated,
                        "abs_url": abs_url,
                        "pdf_url": pdf_url,
                        "primary_category": primary_category,
                        "journal": journal,
                        "volume": volume,
                        "issue": issue,
                        "page": page,
                        "doi": doi,
                        "categories": categories,
                        "reference_count": reference_count,
                        "added_at": now,
                        "updated_at": now,
                        "pdf_path": pdf_path,
                        "pdf_hash": pdf_hash,
                    },
                )
            else:
                cur.execute(
                    """
                    UPDATE papers SET
                        title         = :title,
                        authors       = :authors,
                        abstract      = :abstract,
                        updated       = :updated,
                        abs_url       = :abs_url,
                        pdf_url       = :pdf_url,
                        primary_category = :primary_category,
                        journal       = :journal,
                        volume        = :volume,
                        issue         = :issue,
                        page          = :page,
                        doi           = :doi,
                        categories    = :categories,
                        reference_count = :reference_count,
                        updated_at    = :updated_at,
                        pdf_path      = COALESCE(:pdf_path, pdf_path),
                        pdf_hash      = COALESCE(:pdf_hash, pdf_hash)
                    WHERE id = :id
                    """,
                    {
                        "id": paper_id,
                        "title": title,
                        "authors": authors_json,
                        "abstract": abstract,
                        "updated": updated,
                        "abs_url": abs_url,
                        "pdf_url": pdf_url,
                        "primary_category": primary_category,
                        "journal": journal,
                        "volume": volume,
                        "issue": issue,
                        "page": page,
                        "doi": doi,
                        "categories": categories,
                        "reference_count": reference_count,
                        "updated_at": now,
                        "pdf_path": pdf_path,
                        "pdf_hash": pdf_hash,
                    },
                )
            # Sync FTS index after insert/update
            self._sync_fts(paper_id, title, abstract)
        return self.get_paper(paper_id)

    def get_paper(self, paper_id: str) -> Optional[PaperRecord]:
        """Return a paper record or None."""
        try:
            cur = self.conn.cursor()
            cur.execute("SELECT * FROM papers WHERE id = ?", (paper_id,))
            row = cur.fetchone()
            if row is None:
                return None
            return PaperRecord.from_row(row)
        except sqlite3.Error as e:
            raise DatabaseError(f"get_paper({paper_id!r}) failed: {e}") from e

    def get_papers_bulk(self, paper_ids: list[str]) -> Dict[str, PaperRecord]:
        """Return a dict of {paper_id: PaperRecord} for the given IDs. O(n) query."""
        if not paper_ids:
            return {}
        try:
            placeholders = ",".join("?" * len(paper_ids))
            cur = self.conn.cursor()
            cur.execute(f"SELECT * FROM papers WHERE id IN ({placeholders})", paper_ids)
            return {row["id"]: PaperRecord.from_row(row) for row in cur.fetchall()}
        except sqlite3.Error as e:
            raise DatabaseError(f"get_papers_bulk failed: {e}") from e

    # list_papers is defined in the Search section above (with filters + sort)

    def update_parse_status(
        self,
        paper_id: str,
        status: str,
        error: str = "",
        plain_text: str = "",
        latex_blocks: List[Any] | str = "",
        table_count: int = 0,
        figure_count: int = 0,
        word_count: int = 0,
        page_count: int = 0,
    ) -> None:
        """Update parse result fields."""
        try:
            latex_json = json.dumps(latex_blocks) if not isinstance(latex_blocks, str) else latex_blocks
            now = _utcnow()
            with self.transaction():
                cur = self.conn.cursor()
                # Single SELECT to get parse_version + title/abstract (was 2 queries)
                cur.execute(
                    "SELECT parse_version, title, abstract FROM papers WHERE id = ?",
                    (paper_id,),
                )
                row = cur.fetchone()
                version = (row["parse_version"] if row else 0) + 1
                title_val = row["title"] if row else ""
                abstract_val = row["abstract"] if row else ""
                cur.execute(
                    """
                    UPDATE papers SET
                        parse_status  = :status,
                        parse_error   = :error,
                        plain_text    = :plain_text,
                        latex_blocks  = :latex_blocks,
                        table_count   = :table_count,
                        figure_count  = :figure_count,
                        word_count    = :word_count,
                        page_count    = :page_count,
                        parse_version = :parse_version,
                        updated_at    = :updated_at
                    WHERE id = :id
                    """,
                    {
                        "id": paper_id,
                        "status": status,
                        "error": error,
                        "plain_text": plain_text,
                        "latex_blocks": latex_json,
                        "table_count": table_count,
                        "figure_count": figure_count,
                        "word_count": word_count,
                        "page_count": page_count,
                        "parse_version": version,
                        "updated_at": now,
                    },
                )
                # Sync FTS — pass title/abstract directly (was re-fetching after UPDATE)
                self._sync_fts(paper_id, title_val or "", abstract_val or "")
        except sqlite3.Error as e:
            raise DatabaseError(f"update_parse_status({paper_id!r}) failed: {e}") from e

    def paper_count(self, status: Optional[str] = None) -> int:
        """Return total paper count, optionally filtered by parse status."""
        try:
            cur = self.conn.cursor()
            if status:
                cur.execute(
                    "SELECT COUNT(*) FROM papers WHERE parse_status = ?",
                    (status,),
                )
            else:
                cur.execute("SELECT COUNT(*) FROM papers")
            return cur.fetchone()[0]
        except sqlite3.Error as e:
            raise DatabaseError(f"paper_count failed: {e}") from e

    # ── Tags ──────────────────────────────────────────────────────────────────

    def add_tag(self, paper_id: str, tag: str) -> None:
        """Add a tag to a paper. Creates tag if it doesn't exist."""
        try:
            with self.transaction():
                cur = self.conn.cursor()
                cur.execute(
                    "INSERT OR IGNORE INTO tags (name) VALUES (?)",
                    (tag.lower().strip(),),
                )
                cur.execute(
                    "SELECT id FROM tags WHERE name = ?",
                    (tag.lower().strip(),),
                )
                tag_row = cur.fetchone()
                if tag_row:
                    cur.execute(
                        "INSERT OR IGNORE INTO paper_tags (paper_id, tag_id) VALUES (?, ?)",
                        (paper_id, tag_row["id"]),
                    )
        except sqlite3.Error as e:
            raise DatabaseError(f"add_tag({paper_id!r}, {tag!r}) failed: {e}") from e

    def remove_tag(self, paper_id: str, tag: str) -> None:
        """Remove a tag from a paper."""
        try:
            with self.transaction():
                cur = self.conn.cursor()
                cur.execute(
                    "DELETE FROM paper_tags WHERE paper_id = ? AND tag_id = (SELECT id FROM tags WHERE name = ?)",
                    (paper_id, tag.lower().strip()),
                )
        except sqlite3.Error as e:
            raise DatabaseError(f"remove_tag failed: {e}") from e

    def get_tags(self, paper_id: str) -> List[str]:
        """Get all tags for a paper."""
        try:
            cur = self.conn.cursor()
            cur.execute(
                """
                SELECT t.name FROM tags t
                JOIN paper_tags pt ON pt.tag_id = t.id
                WHERE pt.paper_id = ?
                ORDER BY t.name
                """,
                (paper_id,),
            )
            return [row["name"] for row in cur.fetchall()]
        except sqlite3.Error as e:
            raise DatabaseError(f"get_tags({paper_id!r}) failed: {e}") from e

    def list_all_tags(self) -> List[str]:
        """List all tag names."""
        try:
            cur = self.conn.cursor()
            cur.execute("SELECT name FROM tags ORDER BY name")
            return [row["name"] for row in cur.fetchall()]
        except sqlite3.Error as e:
            raise DatabaseError(f"list_all_tags failed: {e}") from e

    def papers_by_tag(self, tag: str) -> List[PaperRecord]:
        """Return all papers with a given tag."""
        try:
            cur = self.conn.cursor()
            cur.execute(
                """
                SELECT p.* FROM papers p
                JOIN paper_tags pt ON pt.paper_id = p.id
                JOIN tags t ON t.id = pt.tag_id
                WHERE t.name = ?
                ORDER BY p.added_at DESC
                """,
                (tag.lower().strip(),),
            )
            return [PaperRecord.from_row(row) for row in cur.fetchall()]
        except sqlite3.Error as e:
            raise DatabaseError(f"papers_by_tag({tag!r}) failed: {e}") from e

    # ── Parse History ─────────────────────────────────────────────────────────

    def record_parse_attempt(
        self,
        paper_id: str,
        duration_sec: float,
        status: str,
        error: str = "",
        pdf_hash: str = "",
        file_size: int = 0,
    ) -> None:
        """Append a parse history entry."""
        try:
            cur = self.conn.cursor()
            cur.execute(
                """
                INSERT INTO parse_history
                    (paper_id, attempted_at, duration_sec, status, error, pdf_hash, file_size)
                VALUES (?, ?, ?, ?, ?, ?, ?)
                """,
                (paper_id, _utcnow(), duration_sec, status, error, pdf_hash, file_size),
            )
        except sqlite3.Error as e:
            raise DatabaseError(f"record_parse_attempt failed: {e}") from e

    def get_parse_history(self, paper_id: str, limit: int = 10) -> List[sqlite3.Row]:
        """Return recent parse attempts for a paper."""
        try:
            cur = self.conn.cursor()
            cur.execute(
                """
                SELECT * FROM parse_history
                WHERE paper_id = ?
                ORDER BY attempted_at DESC
                LIMIT ?
                """,
                (paper_id, limit),
            )
            return list(cur.fetchall())
        except sqlite3.Error as e:
            raise DatabaseError(f"get_parse_history failed: {e}") from e

    # ── Job Queue ────────────────────────────────────────────────────────────

    def enqueue_job(self, paper_id: str, job_type: str, priority: int = 5) -> int:
        """Add a job to the queue. Returns the job rowid."""
        try:
            cur = self.conn.cursor()
            cur.execute(
                """
                INSERT INTO job_queue (paper_id, job_type, priority, created_at)
                VALUES (?, ?, ?, ?)
                """,
                (paper_id, job_type, priority, _utcnow()),
            )
            return cur.lastrowid or 0
        except sqlite3.Error as e:
            raise DatabaseError(f"enqueue_job failed: {e}") from e

    def dequeue_job(self, job_type: Optional[str] = None) -> Optional[sqlite3.Row]:
        """Atomically pop the highest-priority queued job. Returns the row or None."""
        try:
            cur = self.conn.cursor()
            sql = """
                SELECT * FROM job_queue
                WHERE status = 'queued'
            """
            params: list[Any] = []
            if job_type:
                sql += " AND job_type = ?"
                params.append(job_type)
            sql += " ORDER BY priority DESC, created_at ASC LIMIT 1"
            cur.execute(sql, params)
            row = cur.fetchone()
            if row:
                cur.execute(
                    "UPDATE job_queue SET status='running', started_at=? WHERE id=?",
                    (_utcnow(), row["id"]),
                )
            return row
        except sqlite3.Error as e:
            raise DatabaseError(f"dequeue_job failed: {e}") from e

    def complete_job(self, job_id: int, status: str = "done", error: str = "") -> None:
        """Mark a job as complete or failed."""
        try:
            cur = self.conn.cursor()
            cur.execute(
                """
                UPDATE job_queue
                SET status=?, completed_at=?, error=?
                WHERE id=?
                """,
                (status, _utcnow(), error, job_id),
            )
        except sqlite3.Error as e:
            raise DatabaseError(f"complete_job({job_id}) failed: {e}") from e

    def queue_depth(self, status: str = "queued") -> int:
        """Return the number of jobs in the queue with the given status."""
        try:
            cur = self.conn.cursor()
            cur.execute(
                "SELECT COUNT(*) FROM job_queue WHERE status = ?",
                (status,),
            )
            return cur.fetchone()[0]
        except sqlite3.Error as e:
            raise DatabaseError(f"queue_depth failed: {e}") from e

    def cancel_job(self, job_id: int) -> bool:
        """Remove a job from the queue by job id. Returns True if a row was deleted."""
        try:
            cur = self.conn.cursor()
            cur.execute("DELETE FROM job_queue WHERE id = ?", (job_id,))
            return cur.rowcount > 0
        except sqlite3.Error as e:
            raise DatabaseError(f"cancel_job({job_id}) failed: {e}") from e

    # ── Settings ─────────────────────────────────────────────────────────────

    def set_setting(self, key: str, value: str) -> None:
        """Set a key-value setting."""
        try:
            cur = self.conn.cursor()
            cur.execute(
                "INSERT OR REPLACE INTO settings (key, value) VALUES (?, ?)",
                (key, value),
            )
        except sqlite3.Error as e:
            raise DatabaseError(f"set_setting failed: {e}") from e

    def get_setting(self, key: str, default: str = "") -> str:
        """Get a setting value or default."""
        try:
            cur = self.conn.cursor()
            cur.execute("SELECT value FROM settings WHERE key = ?", (key,))
            row = cur.fetchone()
            return row["value"] if row else default
        except sqlite3.Error as e:
            raise DatabaseError(f"get_setting failed: {e}") from e

    # ── Delete ────────────────────────────────────────────────────────────────

    def delete_paper(self, paper_id: str) -> bool:
        """Delete a paper and its FTS entry. Returns True if a row was deleted."""
        try:
            with self.transaction():
                cur = self.conn.cursor()
                cur.execute("DELETE FROM papers_fts WHERE paper_id = ?", (paper_id,))
                cur.execute("DELETE FROM papers WHERE id = ?", (paper_id,))
            return cur.rowcount > 0
        except sqlite3.Error as e:
            raise DatabaseError(f"delete_paper({paper_id!r}) failed: {e}") from e

    # ── FTS Sync ─────────────────────────────────────────────────────────────

    def _sync_fts(self, paper_id: str, title: str, abstract: str) -> None:
        """Insert or replace a paper's FTS entry. Idempotent — safe to call on every upsert."""
        try:
            cur = self.conn.cursor()
            cur.execute("DELETE FROM papers_fts WHERE paper_id = ?", (paper_id,))
            cur.execute(
                "INSERT INTO papers_fts(paper_id, title, abstract, plain_text) VALUES (?, ?, ?, '')",
                (paper_id, title or "", abstract or ""),
            )
        except sqlite3.Error:
            pass  # FTS sync is best-effort

    # ── Search ────────────────────────────────────────────────────────────────

    def search_papers(
        self,
        query: str,
        limit: int = 20,
        offset: int = 0,
        source: Optional[str] = None,
        category: Optional[str] = None,
        date_from: Optional[str] = None,
        date_to: Optional[str] = None,
        parse_status: Optional[str] = None,
    ) -> Tuple[List[SearchResult], int]:
        """
        Full-text search with BM25 ranking.
        FTS returns paper_ids; full paper data is looked up from papers table.
        Returns (results, total_count).
        Falls back to LIKE when FTS5 is not available.
        """
        try:
            cur = self.conn.cursor()

            # Build WHERE conditions for papers table join
            join_where = ""
            count_where = ""
            params: List[Any] = []
            count_params: List[Any] = []

            if source:
                join_where += " AND p.source = ?"
                count_where += " AND p.source = ?"
                params.append(source)
                count_params.append(source)
            if category:
                join_where += " AND p.primary_category = ?"
                count_where += " AND p.primary_category = ?"
                params.append(category)
                count_params.append(category)
            if parse_status:
                join_where += " AND p.parse_status = ?"
                count_where += " AND p.parse_status = ?"
                params.append(parse_status)
                count_params.append(parse_status)
            if date_from:
                join_where += " AND p.published >= ?"
                count_where += " AND p.published >= ?"
                params.append(date_from)
                count_params.append(date_from)
            if date_to:
                join_where += " AND p.published <= ?"
                count_where += " AND p.published <= ?"
                params.append(date_to)
                count_params.append(date_to)

            fts_query = f'"{query}"'

            # Count total matches (after all filters)
            count_sql = f"""
                SELECT COUNT(*) FROM papers_fts fts
                JOIN papers p ON p.id = fts.paper_id
                WHERE papers_fts MATCH ?{count_where}
            """
            cur.execute(count_sql, [fts_query] + count_params)
            total = cur.fetchone()[0]

            # Search: FTS returns paper_id, title, abstract, score, snippet
            search_sql = f"""
                SELECT
                    fts.paper_id,
                    fts.title,
                    fts.abstract,
                    bm25(papers_fts) AS score,
                    snippet(papers_fts, 0, '**', '**', '...', 30) AS snippet
                FROM papers_fts
                JOIN papers p ON p.id = papers_fts.paper_id
                WHERE papers_fts MATCH ?{join_where}
                ORDER BY score
                LIMIT ? OFFSET ?
            """
            cur.execute(search_sql, [fts_query] + params + [limit, offset])
            fts_rows = cur.fetchall()

            if not fts_rows:
                return [], total

            # Look up full paper data from papers table
            paper_ids = [r["paper_id"] for r in fts_rows]
            placeholders = ",".join("?" * len(paper_ids))
            cur.execute(f"SELECT * FROM papers WHERE id IN ({placeholders})", paper_ids)
            paper_map = {row["id"]: row for row in cur.fetchall()}

            results = []
            # Batch-parse all author JSONs at once to avoid N individual calls.
            raw_author_list: List[Optional[str]] = [
                paper_map.get(r["paper_id"])["authors"]
                if paper_map.get(r["paper_id"]) is not None else None
                for r in fts_rows
            ]
            for fts_row, raw_authors in zip(fts_rows, raw_author_list):
                pid = fts_row["paper_id"]
                paper = paper_map.get(pid)
                if paper is None:
                    continue
                if raw_authors is not None:
                    try:
                        authors = _parse_authors_cached(raw_authors)
                    except Exception:
                        warnings.warn(f"search_papers: failed to parse authors JSON for paper {paper['id']}", stacklevel=2)
                        authors = []
                else:
                    authors = []
                results.append(
                    SearchResult(
                        paper_id=paper["id"],
                        title=paper["title"] or "",
                        authors=authors,
                        published=paper["published"] or "",
                        primary_category=paper["primary_category"] or "",
                        score=float(fts_row["score"]),
                        snippet=fts_row["snippet"] or "",
                        parse_status=paper["parse_status"] or "",
                        source=paper["source"] or "",
                        abs_url=paper["abs_url"] or "",
                        pdf_url=paper["pdf_url"] or "",
                    )
                )

            return results, total

        except sqlite3.OperationalError:
            return self._search_like(query, limit, offset, source, category, date_from, date_to, parse_status)
        except sqlite3.Error as e:
            raise DatabaseError(f"search_papers failed: {e}") from e

    def _search_like(
        self,
        query: str,
        limit: int = 20,
        offset: int = 0,
        source: Optional[str] = None,
        category: Optional[str] = None,
        date_from: Optional[str] = None,
        date_to: Optional[str] = None,
        parse_status: Optional[str] = None,
    ) -> Tuple[List[SearchResult], int]:
        """Fallback LIKE-based search when FTS5 is unavailable."""
        try:
            cur = self.conn.cursor()
            q = f"%{query}%"

            where = "WHERE (title LIKE ? OR abstract LIKE ? OR plain_text LIKE ?)"
            params: list[Any] = [q, q, q]

            if source:
                where += " AND source = ?"
                params.append(source)
            if category:
                where += " AND primary_category = ?"
                params.append(category)
            if date_from:
                where += " AND published >= ?"
                params.append(date_from)
            if date_to:
                where += " AND published <= ?"
                params.append(date_to)
            if parse_status:
                where += " AND parse_status = ?"
                params.append(parse_status)

            # Count
            cur.execute(f"SELECT COUNT(*) FROM papers {where}", params)
            total = cur.fetchone()[0]

            # Search
            sql = f"""
                SELECT id, title, authors, published, primary_category,
                       source, parse_status, abs_url, pdf_url
                FROM papers
                {where}
                ORDER BY added_at DESC
                LIMIT ? OFFSET ?
            """
            params.extend([limit, offset])
            cur.execute(sql, params)

            results = []
            for row in cur.fetchall():
                try:
                    authors = _parse_authors_cached(row["authors"])
                except Exception:
                    warnings.warn(f"_search_like: failed to parse authors JSON for paper {row['id']}", stacklevel=2)
                    authors = []
                results.append(
                    SearchResult(
                        paper_id=row["id"],
                        title=row["title"] or "",
                        authors=authors,
                        published=row["published"] or "",
                        primary_category=row["primary_category"] or "",
                        score=0.0,
                        snippet=f"...{query}...",
                        parse_status=row["parse_status"] or "",
                        source=row["source"] or "",
                        abs_url=row["abs_url"] or "",
                        pdf_url=row["pdf_url"] or "",
                    )
                )
            return results, total

        except sqlite3.Error as e:
            raise DatabaseError(f"_search_like failed: {e}") from e

    def list_papers(
        self,
        limit: int = 20,
        offset: int = 0,
        source: Optional[str] = None,
        category: Optional[str] = None,
        date_from: Optional[str] = None,
        date_to: Optional[str] = None,
        parse_status: Optional[str] = None,
        sort_by: str = "added_at",
        sort_order: str = "desc",
    ) -> Tuple[List[PaperRecord], int]:
        """Filtered list with sort. Returns (papers, total_count)."""
        try:
            cur = self.conn.cursor()
            where_parts: list[str] = []
            params: list[Any] = []

            if source:
                where_parts.append("source = ?")
                params.append(source)
            if category:
                where_parts.append("primary_category = ?")
                params.append(category)
            if date_from:
                where_parts.append("published >= ?")
                params.append(date_from)
            if date_to:
                where_parts.append("published <= ?")
                params.append(date_to)
            if parse_status:
                where_parts.append("parse_status = ?")
                params.append(parse_status)

            where = "WHERE " + " AND ".join(where_parts) if where_parts else ""

            # Validate sort
            allowed_sort = {"added_at", "published", "title"}
            if sort_by not in allowed_sort:
                sort_by = "added_at"
            sort_order = "DESC" if sort_order.lower() == "desc" else "ASC"

            # Count
            cur.execute(f"SELECT COUNT(*) FROM papers {where}", params)
            total = cur.fetchone()[0]

            # List
            sql = f"SELECT * FROM papers {where} ORDER BY {sort_by} {sort_order} LIMIT ? OFFSET ?"
            params.extend([limit, offset])
            cur.execute(sql, params)

            return [PaperRecord.from_row(row) for row in cur.fetchall()], total

        except sqlite3.Error as e:
            raise DatabaseError(f"list_papers failed: {e}") from e

    def rebuild_fts_index(self) -> int:
        """Rebuild FTS index from all existing papers. Returns count of indexed papers."""
        try:
            cur = self.conn.cursor()
            cur.execute("DELETE FROM papers_fts")
            cur.execute("""
                INSERT INTO papers_fts(paper_id, title, abstract, plain_text)
                SELECT id, title, abstract, COALESCE(plain_text, '') FROM papers
            """)
            count = cur.rowcount
            logger.info("FTS index rebuilt: %d papers indexed", count)
            return count
        except sqlite3.Error as e:
            raise DatabaseError(f"rebuild_fts_index failed: {e}") from e

    def get_cached_paper(self, uid: str) -> Optional[Any]:
        """Get a cached paper by UID, or a stats counter if uid=='__stats__'."""
        try:
            cur = self.conn.cursor()
            if uid == "__stats__":
                cur.execute("SELECT COUNT(*) FROM paper_cache")
                return cur.fetchone()[0]
            cur.execute("SELECT data FROM paper_cache WHERE uid = ?", (uid,))
            row = cur.fetchone()
            return json.loads(row[0]) if row else None
        except sqlite3.Error as e:
            raise DatabaseError(f"get_cached_paper failed: {e}") from e

    def set_cached_paper(self, uid: str, data: Any) -> None:
        """Cache a paper JSON blob by UID."""
        try:
            with self.conn as c:
                c.execute(
                    "INSERT OR REPLACE INTO paper_cache (uid, data, cached_at) VALUES (?, ?, ?)",
                    (uid, json.dumps(data), _utcnow()),
                )
        except sqlite3.Error as e:
            raise DatabaseError(f"set_cached_paper failed: {e}") from e

    def clear_cache(self) -> int:
        """Clear all cache entries. Returns count deleted."""
        try:
            with self.conn as c:
                c.execute("DELETE FROM paper_cache")
            return c.total_changes
        except sqlite3.Error as e:
            raise DatabaseError(f"clear_cache failed: {e}") from e

    def clear_jobs(self, status: str = "queued") -> int:
        """Delete all jobs in the queue with the given status. Returns count deleted."""
        try:
            cur = self.conn.cursor()
            cur.execute("DELETE FROM job_queue WHERE status = ?", (status,))
            return cur.rowcount
        except sqlite3.Error as e:
            raise DatabaseError(f"clear_jobs failed: {e}") from e

    def clear_pending_papers(self) -> int:
        """Reset parse_status of all pending papers to 'idle'. Returns count cleared."""
        try:
            cur = self.conn.cursor()
            cur.execute(
                "UPDATE papers SET parse_status = 'idle' WHERE parse_status = 'pending'"
            )
            self.conn.commit()
            return cur.rowcount
        except sqlite3.Error as e:
            raise DatabaseError(f"clear_pending_papers failed: {e}") from e

    def get_papers(self, limit: int = 10000, offset: int = 0) -> List[PaperRecord]:
        """Return all papers (no filters), newest first."""
        try:
            cur = self.conn.cursor()
            cur.execute(
                "SELECT * FROM papers ORDER BY added_at DESC LIMIT ? OFFSET ?",
                (limit, offset),
            )
            return [PaperRecord.from_row(row) for row in cur.fetchall()]
        except sqlite3.Error as e:
            raise DatabaseError(f"get_papers failed: {e}") from e

    def get_stats(self) -> dict[str, Any]:
        """Return a summary dict of database statistics."""
        try:
            cur = self.conn.cursor()
            stats: dict[str, Any] = {}
            # Papers
            cur.execute("SELECT COUNT(*) FROM papers")
            stats["total_papers"] = cur.fetchone()[0]
            # By source
            cur.execute("SELECT source, COUNT(*) FROM papers GROUP BY source")
            stats["by_source"] = {r[0]: r[1] for r in cur.fetchall()}
            # By status
            cur.execute("SELECT parse_status, COUNT(*) FROM papers GROUP BY parse_status")
            stats["by_status"] = {r[0] or "none": r[1] for r in cur.fetchall()}
            # Queue
            cur.execute("SELECT COUNT(*) FROM job_queue WHERE status = 'queued'")
            stats["queue_queued"] = cur.fetchone()[0]
            cur.execute("SELECT COUNT(*) FROM job_queue WHERE status = 'running'")
            stats["queue_running"] = cur.fetchone()[0]
            # Cache
            cur.execute("SELECT COUNT(*) FROM paper_cache")
            stats["cache_entries"] = cur.fetchone()[0]
            # Dedup log
            cur.execute("SELECT COUNT(*) FROM dedup_log")
            stats["dedup_records"] = cur.fetchone()[0]
            return stats
        except sqlite3.Error as e:
            raise DatabaseError(f"get_stats failed: {e}") from e

    def export_papers(self, format: str = "csv", limit: int = 0) -> Tuple[str, List[Dict[str, Any]]]:
        """Export all papers as CSV or JSON. Returns (header_row_or_fields, rows)."""
        try:
            cur = self.conn.cursor()
            fields = [
                "id", "source", "title", "authors", "abstract", "published",
                "doi", "primary_category", "parse_status", "added_at",
            ]
            sql = f"SELECT {','.join(fields)} FROM papers ORDER BY added_at DESC"
            if limit > 0:
                sql += f" LIMIT {limit}"
            cur.execute(sql)
            rows = []
            for row in cur.fetchall():
                rows.append({f: row[i] for i, f in enumerate(fields)})
            return (fields, rows)
        except sqlite3.Error as e:
            raise DatabaseError(f"export_papers failed: {e}") from e

    # ── Utilities ────────────────────────────────────────────────────────────

    def close(self) -> None:
        """Close the thread-local connection."""
        if hasattr(self._local, "conn") and self._local.conn:
            try:
                self._local.conn.close()
            except Exception:
                warnings.warn("Database.close: failed to close connection", stacklevel=2)
            self._local.conn = None

    def vacuum(self) -> None:
        """Shrink the database file."""
        try:
            with self.conn as c:
                c.execute("VACUUM")
            logger.info("Database vacuumed")
        except sqlite3.Error as e:
            raise DatabaseError(f"vacuum failed: {e}") from e

    # ── Deduplication & Merge ─────────────────────────────────────────────────

    def find_duplicates(self, since: str | None = None) -> List[Tuple[PaperRecord, PaperRecord]]:
        """
        Find pairs of papers that are likely duplicates.
        Two papers are duplicates if:
          - They share the same DOI (non-empty), OR
          - Their titles are identical and neither title is empty
        If `since` is given (YYYY-MM-DD), only papers added on or after that date are considered.
        Returns a list of (older, newer) PaperRecord pairs.
        """
        try:
            cur = self.conn.cursor()
            where_since = " AND a.added_at >= ? " if since else " "
            # Single query: join papers with itself and fetch all needed columns at once.
            cur.execute(
                f"""
                SELECT a.id      AS a_id,
                       b.id      AS b_id,
                       a.source  AS a_source,  a.title      AS a_title,
                       a.authors AS a_authors, a.abstract   AS a_abstract,
                       a.published    AS a_published,    a.updated      AS a_updated,
                       a.abs_url      AS a_abs_url,      a.pdf_url      AS a_pdf_url,
                       a.primary_category AS a_primary_category,
                       a.journal  AS a_journal,  a.volume    AS a_volume,
                       a.issue    AS a_issue,    a.page      AS a_page,
                       a.doi      AS a_doi,      a.categories AS a_categories,
                       a.parse_status   AS a_parse_status,
                       a.embed_vector  AS a_embed_vector,
                       a.added_at AS a_added_at, a.updated_at AS a_updated_at,
                       b.source  AS b_source,  b.title      AS b_title,
                       b.authors AS b_authors, b.abstract   AS b_abstract,
                       b.published    AS b_published,    b.updated      AS b_updated,
                       b.abs_url      AS b_abs_url,      b.pdf_url      AS b_pdf_url,
                       b.primary_category AS b_primary_category,
                       b.journal  AS b_journal,  b.volume    AS b_volume,
                       b.issue    AS b_issue,    b.page      AS b_page,
                       b.doi      AS b_doi,      b.categories AS b_categories,
                       b.parse_status   AS b_parse_status,
                       b.embed_vector  AS b_embed_vector,
                       b.added_at AS b_added_at, b.updated_at AS b_updated_at
                FROM papers a
                JOIN papers b ON
                    a.id < b.id
                    AND (
                        (a.doi IS NOT NULL AND a.doi = b.doi AND a.doi != '')
                        OR
                        (a.title IS NOT NULL AND a.title != '' AND a.title = b.title)
                    )
                WHERE 1=1
                {where_since}
                ORDER BY a.added_at DESC
                """,
                ([since] if since else []),
            )
            pairs: List[Tuple[PaperRecord, PaperRecord]] = []
            for row in cur.fetchall():
                a_record = PaperRecord(
                    id=row["a_id"], source=row["a_source"], title=row["a_title"],
                    authors=row["a_authors"], abstract=row["a_abstract"],
                    published=row["a_published"], updated=row["a_updated"],
                    abs_url=row["a_abs_url"], pdf_url=row["a_pdf_url"],
                    primary_category=row["a_primary_category"],
                    journal=row["a_journal"], volume=row["a_volume"],
                    issue=row["a_issue"], page=row["a_page"],
                    doi=row["a_doi"], categories=row["a_categories"],
                    parse_status=row["a_parse_status"],
                    embed_vector=row["a_embed_vector"],
                    added_at=row["a_added_at"], updated_at=row["a_updated_at"],
                )
                b_record = PaperRecord(
                    id=row["b_id"], source=row["b_source"], title=row["b_title"],
                    authors=row["b_authors"], abstract=row["b_abstract"],
                    published=row["b_published"], updated=row["b_updated"],
                    abs_url=row["b_abs_url"], pdf_url=row["b_pdf_url"],
                    primary_category=row["b_primary_category"],
                    journal=row["b_journal"], volume=row["b_volume"],
                    issue=row["b_issue"], page=row["b_page"],
                    doi=row["b_doi"], categories=row["b_categories"],
                    parse_status=row["b_parse_status"],
                    embed_vector=row["b_embed_vector"],
                    added_at=row["b_added_at"], updated_at=row["b_updated_at"],
                )
                pairs.append((a_record, b_record))
            return pairs
        except sqlite3.Error as e:
            raise DatabaseError(f"find_duplicates failed: {e}") from e

    def merge_papers(self, target_id: str, duplicate_id: str) -> bool:
        """
        Merge duplicate_id into target_id:
          - Copies paper_tags from duplicate to target (skipping conflicts)
          - Copies parse data (plain_text, latex_blocks, table_count, figure_count,
            word_count, page_count, pnote_path, cnote_path, mnote_path) if target is empty
          - Transfers pending/running jobs from duplicate to target
          - Deletes the duplicate paper and its FTS entry
        Returns True if the duplicate was found and deleted.
        """
        try:
            with self.transaction():
                cur = self.conn.cursor()

                # Make sure both papers exist
                cur.execute("SELECT id FROM papers WHERE id = ?", (duplicate_id,))
                if cur.fetchone() is None:
                    return False

                # Transfer tags (insert ignore — skip if target already has the tag)
                cur.execute(
                    """
                    INSERT OR IGNORE INTO paper_tags (paper_id, tag_id)
                    SELECT ?, tag_id FROM paper_tags WHERE paper_id = ?
                    """,
                    (target_id, duplicate_id),
                )

                # Fill empty parse fields on target from duplicate
                parse_fields = [
                    ("plain_text", "papers.plain_text = COALESCE(NULLIF(papers.plain_text, ''), excluded.plain_text)"),
                    ("latex_blocks", "papers.latex_blocks = CASE WHEN papers.latex_blocks = '[]' THEN excluded.latex_blocks ELSE papers.latex_blocks END"),
                    ("table_count", "papers.table_count = CASE WHEN papers.table_count = 0 THEN excluded.table_count ELSE papers.table_count END"),
                    ("figure_count", "papers.figure_count = CASE WHEN papers.figure_count = 0 THEN excluded.figure_count ELSE papers.figure_count END"),
                    ("word_count", "papers.word_count = CASE WHEN papers.word_count = 0 THEN excluded.word_count ELSE papers.word_count END"),
                    ("page_count", "papers.page_count = CASE WHEN papers.page_count = 0 THEN excluded.page_count ELSE papers.page_count END"),
                    ("pnote_path", "papers.pnote_path = CASE WHEN papers.pnote_path = '' THEN excluded.pnote_path ELSE papers.pnote_path END"),
                    ("cnote_path", "papers.cnote_path = CASE WHEN papers.cnote_path = '' THEN excluded.cnote_path ELSE papers.cnote_path END"),
                    ("mnote_path", "papers.mnote_path = CASE WHEN papers.mnote_path = '' THEN excluded.mnote_path ELSE papers.mnote_path END"),
                ]
                for field, _ in parse_fields:
                    cur.execute(f"SELECT {field} FROM papers WHERE id = ?", (duplicate_id,))
                    row = cur.fetchone()
                    val = row[field] if row else None
                    if val is not None and val != "" and val != 0 and val != "[]":
                        cur.execute(
                            f"UPDATE papers SET {field} = ? WHERE id = ? AND ({field} = '' OR {field} = '[]' OR {field} = 0)",
                            (val, target_id),
                        )

                # Transfer jobs from duplicate to target (only queued jobs)
                cur.execute(
                    "UPDATE job_queue SET paper_id = ? WHERE paper_id = ? AND status = 'queued'",
                    (target_id, duplicate_id),
                )

                # Update updated_at on target
                cur.execute(
                    "UPDATE papers SET updated_at = ? WHERE id = ?",
                    (_utcnow(), target_id),
                )

                # Delete duplicate FTS entry
                cur.execute("DELETE FROM papers_fts WHERE paper_id = ?", (duplicate_id,))
                # Delete duplicate paper
                cur.execute("DELETE FROM papers WHERE id = ?", (duplicate_id,))

            return True
        except sqlite3.Error as e:
            raise DatabaseError(f"merge_papers({target_id!r}, {duplicate_id!r}) failed: {e}") from e

    def log_dedup(self, target_id: str, duplicate_id: str, keep_policy: str) -> None:
        """Record a dedup merge in the dedup_log table."""
        try:
            with self.transaction():
                cur = self.conn.cursor()
                cur.execute(
                    "INSERT INTO dedup_log (target_id, duplicate_id, keep_policy) VALUES (?, ?, ?)",
                    (target_id, duplicate_id, keep_policy),
                )
        except sqlite3.Error as e:
            raise DatabaseError(f"log_dedup failed: {e}") from e

    def get_dedup_log(self) -> List[Dict[str, Any]]:
        """Return dedup history ordered by most recent first."""
        cur = self.conn.cursor()
        cur.execute(
            """
            SELECT d.id, d.target_id, d.duplicate_id, d.keep_policy, d.logged_at,
                   p1.title AS target_title, p2.title AS duplicate_title
            FROM dedup_log d
            JOIN papers p1 ON p1.id = d.target_id
            JOIN papers p2 ON p2.id = d.duplicate_id
            ORDER BY d.id DESC
            """,
        )
        return [dict(row) for row in cur.fetchall()]

    # ── Citations ────────────────────────────────────────────────────────────────

    def add_citation(self, source_id: str, target_id: str) -> bool:
        """Insert one citation pair. Returns True if inserted, False if duplicate."""
        try:
            cur = self.conn.cursor()
            cur.execute(
                "INSERT OR IGNORE INTO citations (source_id, target_id, created_at) VALUES (?, ?, ?)",
                (source_id, target_id, _utcnow()),
            )
            self.conn.commit()
            return cur.rowcount > 0
        except sqlite3.Error as e:
            raise DatabaseError(f"add_citation failed: {e}") from e

    def add_citations_batch(self, source_id: str, target_ids: list[str]) -> int:
        """Bulk insert citation pairs. Returns count of newly inserted rows."""
        if not target_ids:
            return 0
        try:
            cur = self.conn.cursor()
            now = _utcnow()
            rows = [(source_id, tid, now) for tid in target_ids]
            cur.executemany(
                "INSERT OR IGNORE INTO citations (source_id, target_id, created_at) VALUES (?, ?, ?)",
                rows,
            )
            self.conn.commit()
            return cur.rowcount
        except sqlite3.Error as e:
            raise DatabaseError(f"add_citations_batch failed: {e}") from e

    def upsert_citations(self, source_id: str, target_ids: list[str]) -> Tuple[int, int]:
        """Bulk upsert citation pairs. Returns (new_count, duplicate_count)."""
        if not target_ids:
            return 0, 0
        try:
            cur = self.conn.cursor()
            now = _utcnow()
            rows = [(source_id, tid, now) for tid in target_ids]
            cur.executemany(
                "INSERT OR IGNORE INTO citations (source_id, target_id, created_at) VALUES (?, ?, ?)",
                rows,
            )
            new_count = cur.rowcount
            dup_count = len(target_ids) - new_count
            self.conn.commit()
            return new_count, dup_count
        except sqlite3.Error as e:
            raise DatabaseError(f"upsert_citations failed: {e}") from e

    # ── Experiment Tables ─────────────────────────────────────────────────────

    def upsert_experiment_tables(
        self,
        paper_id: str,
        tables: List[ExperimentTableRecord],
    ) -> int:
        """
        Replace all experiment tables for a paper with the given list.
        Returns the count of tables stored.
        """
        try:
            with self.transaction():
                cur = self.conn.cursor()
                # Clear existing tables for this paper
                cur.execute(
                    "DELETE FROM experiment_tables WHERE paper_id = ?",
                    (paper_id,),
                )
                for tbl in tables:
                    headers_json = json.dumps(tbl.headers, ensure_ascii=False)
                    rows_json = json.dumps(tbl.rows, ensure_ascii=False)
                    cur.execute(
                        """
                        INSERT INTO experiment_tables
                            (paper_id, table_caption, page, headers, rows,
                             bbox_x0, bbox_y0, bbox_x1, bbox_y1, created_at)
                        VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
                        """,
                        (
                            paper_id,
                            tbl.table_caption,
                            tbl.page,
                            headers_json,
                            rows_json,
                            tbl.bbox_x0,
                            tbl.bbox_y0,
                            tbl.bbox_x1,
                            tbl.bbox_y1,
                            tbl.created_at,
                        ),
                    )
                self.conn.commit()
            return len(tables)
        except sqlite3.Error as e:
            raise DatabaseError(f"upsert_experiment_tables({paper_id!r}) failed: {e}") from e

    def get_experiment_tables(self, paper_id: str) -> List[ExperimentTableRecord]:
        """Return all experiment tables for a paper."""
        try:
            cur = self.conn.cursor()
            cur.execute(
                "SELECT * FROM experiment_tables WHERE paper_id = ? ORDER BY page, id",
                (paper_id,),
            )
            results = []
            for row in cur.fetchall():
                try:
                    headers = json.loads(row["headers"] or "[]")
                except Exception:
                    headers = []
                try:
                    rows_data = json.loads(row["rows"] or "[]")
                except Exception:
                    rows_data = []
                results.append(
                    ExperimentTableRecord(
                        id=row["id"],
                        paper_id=row["paper_id"],
                        table_caption=row["table_caption"] or "",
                        page=row["page"] or 0,
                        headers=headers,
                        rows=rows_data,
                        bbox_x0=row["bbox_x0"] or 0.0,
                        bbox_y0=row["bbox_y0"] or 0.0,
                        bbox_x1=row["bbox_x1"] or 0.0,
                        bbox_y1=row["bbox_y1"] or 0.0,
                        created_at=row["created_at"] or "",
                    )
                )
            return results
        except sqlite3.Error as e:
            raise DatabaseError(f"get_experiment_tables({paper_id!r}) failed: {e}") from e

    def get_citations(
        self, paper_id: str, direction: Literal["from", "to", "both"] = "both"
    ) -> List[CitationRecord]:
        """Fetch citations for a paper.

        direction='from'  — papers cited by paper_id (backward citations / references)
        direction='to'    — papers that cite paper_id (forward citations)
        direction='both'  — union of the above
        """
        try:
            cur = self.conn.cursor()
            if direction == "from":
                cur.execute(
                    "SELECT id, source_id, target_id, created_at FROM citations WHERE source_id = ?",
                    (paper_id,),
                )
            elif direction == "to":
                cur.execute(
                    "SELECT id, source_id, target_id, created_at FROM citations WHERE target_id = ?",
                    (paper_id,),
                )
            else:
                cur.execute(
                    "SELECT id, source_id, target_id, created_at FROM citations "
                    "WHERE source_id = ? OR target_id = ?",
                    (paper_id, paper_id),
                )
            return [CitationRecord(**dict(row)) for row in cur.fetchall()]
        except sqlite3.Error as e:
            raise DatabaseError(f"get_citations failed: {e}") from e

    def get_citation_count(self, paper_id: str) -> dict[str, int]:
        """Return {'forward': N, 'backward': M} counts."""
        try:
            cur = self.conn.cursor()
            cur.execute(
                "SELECT COUNT(*) FROM citations WHERE target_id = ?", (paper_id,)
            )
            forward = cur.fetchone()[0]
            cur.execute(
                "SELECT COUNT(*) FROM citations WHERE source_id = ?", (paper_id,)
            )
            backward = cur.fetchone()[0]
            return {"forward": forward, "backward": backward}
        except sqlite3.Error as e:
            raise DatabaseError(f"get_citation_count failed: {e}") from e

    def get_paper_title(self, paper_id: str) -> str:
        """Return title for a paper, or '' if not found."""
        try:
            cur = self.conn.cursor()
            cur.execute("SELECT title FROM papers WHERE id = ?", (paper_id,))
            row = cur.fetchone()
            return row[0] if row else ""
        except sqlite3.Error as e:
            raise DatabaseError(f"get_paper_title failed: {e}") from e

    def paper_exists(self, paper_id: str) -> bool:
        """Return True if paper_id exists in the papers table."""
        try:
            cur = self.conn.cursor()
            cur.execute("SELECT 1 FROM papers WHERE id = ?", (paper_id,))
            return cur.fetchone() is not None
        except sqlite3.Error:
            return False

    # ── Embeddings (semantic dedup) ────────────────────────────────────────────

    # Provide EMBEDDING_DIM as a class-level alias so existing call sites
    # (e.g. code that reads cls.EMBEDDING_DIM) keep working.
    # Actual value is loaded from the config module at runtime.
    @property
    def EMBEDDING_DIM(self) -> int:  # type: ignore[override]
        from config import EMBEDDING_DIM as _d
        return _d

    def set_embedding(self, paper_id: str, vector: List[float]) -> bool:
        """Store an embedding vector (list of floats) for a paper."""
        try:
            blob = struct.pack(f"{len(vector)}f", *vector)
            cur = self.conn.cursor()
            cur.execute(
                "UPDATE papers SET embed_vector = ? WHERE id = ?",
                (blob, paper_id),
            )
            self.conn.commit()
            return cur.rowcount > 0
        except Exception as e:
            raise DatabaseError(f"set_embedding({paper_id!r}) failed: {e}") from e

    def get_embedding(self, paper_id: str) -> Optional[List[float]]:
        """Retrieve the embedding vector for a paper, or None if not set."""
        try:
            cur = self.conn.cursor()
            cur.execute("SELECT embed_vector FROM papers WHERE id = ?", (paper_id,))
            row = cur.fetchone()
            if not row or row[0] is None:
                return None
            blob = row[0]
            count = len(blob) // 4
            return list(struct.unpack(f"{count}f", blob))
        except Exception as e:
            raise DatabaseError(f"get_embedding({paper_id!r}) failed: {e}") from e

    def get_papers_without_embeddings(self, limit: int = 1000) -> List["PaperRecord"]:
        """Return papers that have a title but no embedding yet."""
        try:
            cur = self.conn.cursor()
            cur.execute(
                "SELECT * FROM papers WHERE embed_vector IS NULL AND title != '' LIMIT ?",
                (limit,),
            )
            return [PaperRecord.from_row(r) for r in cur.fetchall()]
        except sqlite3.Error as e:
            raise DatabaseError(f"get_papers_without_embeddings failed: {e}") from e

    def find_similar(
        self, paper_id: str, threshold: float = 0.85, limit: int = 20
    ) -> List[Tuple["PaperRecord", float]]:
        """
        Find papers similar to paper_id using cosine similarity of embeddings.

        Returns list of (PaperRecord, similarity_score) sorted by score descending.
        Only papers with similarity >= threshold are returned.
        """
        try:
            from rankers.cosine import CosineSimilarityRanker

            ranker = CosineSimilarityRanker(self)
            return ranker.rank(paper_id, threshold=threshold, limit=limit)
        except Exception as e:
            raise DatabaseError(f"find_similar failed: {e}") from e

    def get_similarity(self, paper_id1: str, paper_id2: str) -> float | None:
        """
        Compute cosine similarity between two papers by their embeddings.
        Returns None if either paper lacks an embedding.
        """
        try:
            emb1 = self.get_embedding(paper_id1)
            emb2 = self.get_embedding(paper_id2)
            if emb1 is None or emb2 is None:
                return None
            norm1 = sum(x * x for x in emb1) ** 0.5
            norm2 = sum(x * x for x in emb2) ** 0.5
            if norm1 == 0 or norm2 == 0:
                return None
            dot = sum(a * b for a, b in zip(emb1, emb2))
            return dot / (norm1 * norm2)
        except Exception as e:
            raise DatabaseError(f"get_similarity failed: {e}") from e

    def get_embedding_stats(self) -> dict:
        """Return embedding coverage stats."""
        try:
            cur = self.conn.cursor()
            cur.execute("SELECT COUNT(*) FROM papers WHERE embed_vector IS NOT NULL")
            with_emb = cur.fetchone()[0]
            cur.execute("SELECT COUNT(*) FROM papers WHERE title != ''")
            total_with_text = cur.fetchone()[0]
            return {"with_embedding": with_emb, "total_with_text": total_with_text}
        except sqlite3.Error as e:
            raise DatabaseError(f"get_embedding_stats failed: {e}") from e

    # ── Helpers ────────────────────────────────────────────────────────────────


def _utcnow() -> str:
    return datetime.now(timezone.utc).isoformat(timespec="seconds")
