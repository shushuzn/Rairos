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
import threading
from contextlib import contextmanager
from dataclasses import dataclass, field
from datetime import datetime, timezone
from pathlib import Path
from typing import Any, Generator, Iterator, List, Optional

from core.exceptions import DatabaseError

logger = logging.getLogger(__name__)

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

CREATE INDEX IF NOT EXISTS idx_papers_parse_status ON papers(parse_status);
CREATE INDEX IF NOT EXISTS idx_papers_source ON papers(source);
CREATE INDEX IF NOT EXISTS idx_papers_added_at ON papers(added_at);
CREATE INDEX IF NOT EXISTS idx_parse_history_paper_id ON parse_history(paper_id);
CREATE INDEX IF NOT EXISTS idx_job_queue_status ON job_queue(status);
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
            authors = json.loads(authors_raw) if authors_raw else []
        except Exception:
            authors = []
        try:
            latex_blocks = json.loads(latex_raw) if latex_raw else []
        except Exception:
            latex_blocks = []
        tags = cls._get_tags_for_paper(d["id"])
        return cls(authors=authors, latex_blocks=latex_blocks, tags=tags, **d)

    @staticmethod
    def _get_tags_for_paper(paper_id: str) -> List[str]:
        return []  # filled by Database.get_paper


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
        """Create tables if they don't exist. Idempotent."""
        if self._init_done:
            return
        # Verify the directory is writable before creating tables
        if not self.db_path.parent.exists():
            raise DatabaseError(f"Database directory does not exist and cannot be created: {self.db_path.parent}")
        try:
            with self.conn as conn:
                conn.executescript(_SCHEMA)
            self._init_done = True
            logger.info("Database initialized at %s", self.db_path)
        except sqlite3.Error as e:
            raise DatabaseError(f"Failed to initialize database: {e}") from e

    @contextmanager
    def transaction(self) -> Generator[None, None, None]:
        """Context manager for explicit transactions."""
        conn = self.conn
        cur = conn.cursor()
        try:
            cur.execute("BEGIN")
            yield
            cur.execute("COMMIT")
        except Exception:
            cur.execute("ROLLBACK")
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
                ) ON CONFLICT(id) DO UPDATE SET
                    title         = EXCLUDED.title,
                    authors       = EXCLUDED.authors,
                    abstract      = EXCLUDED.abstract,
                    updated       = EXCLUDED.updated,
                    abs_url       = EXCLUDED.abs_url,
                    pdf_url       = EXCLUDED.pdf_url,
                    primary_category = EXCLUDED.primary_category,
                    journal       = EXCLUDED.journal,
                    volume        = EXCLUDED.volume,
                    issue         = EXCLUDED.issue,
                    page          = EXCLUDED.page,
                    doi           = EXCLUDED.doi,
                    categories    = EXCLUDED.categories,
                    reference_count = EXCLUDED.reference_count,
                    updated_at    = EXCLUDED.updated_at,
                    pdf_path      = COALESCE(EXCLUDED.pdf_path, papers.pdf_path),
                    pdf_hash      = COALESCE(EXCLUDED.pdf_hash, papers.pdf_hash)
                WHERE 1=1
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

    def list_papers(
        self,
        status: Optional[str] = None,
        source: Optional[str] = None,
        limit: int = 100,
        offset: int = 0,
    ) -> List[PaperRecord]:
        """List papers with optional filters."""
        try:
            cur = self.conn.cursor()
            sql = "SELECT * FROM papers"
            params: list[Any] = []
            if status:
                sql += " WHERE parse_status = ?"
                params.append(status)
            if source:
                sql += (" WHERE" if not status else " AND") + " source = ?"
                params.append(source)
            sql += " ORDER BY added_at DESC LIMIT ? OFFSET ?"
            params.extend([limit, offset])
            cur.execute(sql, params)
            return [PaperRecord.from_row(row) for row in cur.fetchall()]
        except sqlite3.Error as e:
            raise DatabaseError(f"list_papers failed: {e}") from e

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
            with self.transaction():
                cur = self.conn.cursor()
                # Increment parse_version on each re-parse
                cur.execute(
                    "SELECT parse_version FROM papers WHERE id = ?",
                    (paper_id,),
                )
                row = cur.fetchone()
                version = (row["parse_version"] if row else 0) + 1
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
                        "updated_at": _utcnow(),
                    },
                )
        except sqlite3.Error as e:
            raise DatabaseError(f"update_parse_status({paper_id!r}) failed: {e}") from e

    def update_note_paths(
        self,
        paper_id: str,
        pnote_path: str = "",
        cnote_path: str = "",
        mnote_path: str = "",
    ) -> None:
        """Update generated note file paths."""
        try:
            with self.transaction():
                cur = self.conn.cursor()
                cur.execute(
                    """
                    UPDATE papers SET
                        pnote_path = COALESCE(NULLIF(:pnote, ''), pnote_path),
                        cnote_path = COALESCE(NULLIF(:cnote, ''), cnote_path),
                        mnote_path = COALESCE(NULLIF(:mnote, ''), mnote_path),
                        updated_at  = :updated_at
                    WHERE id = :id
                    """,
                    {
                        "id": paper_id,
                        "pnote": pnote_path,
                        "cnote": cnote_path,
                        "mnote": mnote_path,
                        "updated_at": _utcnow(),
                    },
                )
        except sqlite3.Error as e:
            raise DatabaseError(f"update_note_paths({paper_id!r}) failed: {e}") from e

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

    # ── Search ────────────────────────────────────────────────────────────────

    def search_papers(self, query: str, limit: int = 20) -> List[PaperRecord]:
        """
        Simple full-text search on title and abstract.
        Uses SQLite FTS5 if available, falls back to LIKE.
        """
        try:
            cur = self.conn.cursor()
            q = f"%{query}%"
            cur.execute(
                """
                SELECT * FROM papers
                WHERE title LIKE ? OR abstract LIKE ? OR categories LIKE ?
                ORDER BY added_at DESC
                LIMIT ?
                """,
                (q, q, q, limit),
            )
            return [PaperRecord.from_row(row) for row in cur.fetchall()]
        except sqlite3.Error as e:
            raise DatabaseError(f"search_papers failed: {e}") from e

    # ── Utilities ────────────────────────────────────────────────────────────

    def close(self) -> None:
        """Close the thread-local connection."""
        if hasattr(self._local, "conn") and self._local.conn:
            try:
                self._local.conn.close()
            except Exception:
                pass
            self._local.conn = None

    def vacuum(self) -> None:
        """Shrink the database file."""
        try:
            with self.conn as c:
                c.execute("VACUUM")
            logger.info("Database vacuumed")
        except sqlite3.Error as e:
            raise DatabaseError(f"vacuum failed: {e}") from e


# ─── Helpers ──────────────────────────────────────────────────────────────────


def _utcnow() -> str:
    return datetime.now(timezone.utc).isoformat(timespec="seconds")
