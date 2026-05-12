"""
db/database.py — Database layer (Rust-powered)

All database operations now delegate to rairos_db_py (Rust).
This file provides a Python API shim for backward compatibility.
"""

from __future__ import annotations

import atexit
import json
import os
import sqlite3
import struct
import tempfile
from dataclasses import dataclass
from pathlib import Path
from typing import Any, Dict, List, Optional, Tuple

from rairos_db_py import PyDatabase


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


# Schema constants (retained for backward compatibility with tests)
_SCHEMA = """
CREATE TABLE IF NOT EXISTS papers (
    id TEXT PRIMARY KEY, source TEXT, title TEXT, authors TEXT,
    abstract TEXT, published TEXT, updated TEXT, abs_url TEXT, pdf_url TEXT,
    primary_category TEXT, journal TEXT, volume TEXT, issue TEXT, page TEXT,
    doi TEXT, categories TEXT, reference_count INTEGER, added_at TEXT,
    updated_at TEXT, parse_status TEXT DEFAULT '', parse_error TEXT DEFAULT '',
    parse_version INTEGER DEFAULT 0,
    plain_text TEXT DEFAULT '', latex_blocks TEXT DEFAULT '',
    table_count INTEGER DEFAULT 0, figure_count INTEGER DEFAULT 0,
    word_count INTEGER DEFAULT 0, page_count INTEGER DEFAULT 0,
    embed_vector BLOB,
    pdf_path TEXT, pdf_hash TEXT
);
CREATE TABLE IF NOT EXISTS embeddings (
    paper_id TEXT PRIMARY KEY, vector BLOB, updated_at TEXT
);
CREATE TABLE IF NOT EXISTS arxiv_search_cache (
    query_hash TEXT PRIMARY KEY, query TEXT, results_json TEXT,
    created_at TEXT, hit_count INTEGER DEFAULT 0
);
CREATE TABLE IF NOT EXISTS tags (
    paper_id TEXT, tag TEXT, PRIMARY KEY (paper_id, tag)
);
CREATE TABLE IF NOT EXISTS parse_history (
    id INTEGER PRIMARY KEY AUTOINCREMENT, paper_id TEXT, status TEXT,
    error TEXT, duration_sec REAL DEFAULT 0, pdf_hash TEXT DEFAULT '',
    file_size INTEGER DEFAULT 0, attempted_at TEXT DEFAULT CURRENT_TIMESTAMP
);
CREATE TABLE IF NOT EXISTS settings (
    key TEXT PRIMARY KEY, value TEXT
);
"""

_FTS_SCHEMA = """
CREATE VIRTUAL TABLE IF NOT EXISTS papers_fts USING fts5(
    paper_id UNINDEXED, title, abstract, plain_text,
    tokenize='porter unicode61'
);
"""


class PaperRecord:
    """Thin wrapper around paper dict returned from Rust."""

    __slots__ = ("_data",)

    def __init__(self, data: dict):
        self._data = data

    @classmethod
    def from_row(cls, row: Any) -> "PaperRecord":
        """Create PaperRecord from a sqlite3.Row (or any dict-like)."""
        if isinstance(row, dict):
            return cls(dict(row))
        if hasattr(row, "cursor_description"):
            # standard sqlite3.Row
            return cls(dict(zip([c[0] for c in row.cursor_description], row)))
        # _SqliteRow: use keys()
        return cls(dict(zip(row.keys(), row)))

    @property
    def id(self) -> str:
        return self._data.get("id", "")  # type: ignore[no-any-return]

    @property
    def paper_id(self) -> str:
        return self._data.get("paper_id") or self._data.get("id", "")  # type: ignore[no-any-return]

    @property
    def title(self) -> str:
        return self._data.get("title", "")  # type: ignore[no-any-return]

    @property
    def authors(self) -> List[str]:
        return self._data.get("authors", [])  # type: ignore[no-any-return]

    @property
    def abstract(self) -> str:
        return self._data.get("abstract", "")  # type: ignore[no-any-return]

    @property
    def published(self) -> str:
        return self._data.get("published", "")  # type: ignore[no-any-return]

    @property
    def updated(self) -> str:
        return self._data.get("updated", "")  # type: ignore[no-any-return]

    @property
    def source(self) -> str:
        return self._data.get("source", "")  # type: ignore[no-any-return]

    @property
    def parse_status(self) -> str:
        return self._data.get("parse_status", "")  # type: ignore[no-any-return]

    @property
    def primary_category(self) -> str:
        return self._data.get("primary_category", "")  # type: ignore[no-any-return]

    @property
    def added_at(self) -> str:
        return self._data.get("added_at", "")  # type: ignore[no-any-return]

    @property
    def abs_url(self) -> str:
        return self._data.get("abs_url", "")  # type: ignore[no-any-return]

    @property
    def pdf_url(self) -> str:
        return self._data.get("pdf_url", "")  # type: ignore[no-any-return]

    def __getitem__(self, key: str) -> Any:
        return self._data.get(key)

    def __repr__(self) -> str:
        return f"PaperRecord({self.id!r}: {self.title!r})"

    def __getattr__(self, name: str) -> Any:
        return self._data.get(name, "")


class SearchResult:
    """Search result wrapper."""

    __slots__ = ("_data",)

    def __init__(self, data: Optional[dict] = None, **kwargs):
        if data is not None:
            self._data: dict = dict(data)
        else:
            self._data = dict(kwargs)

    @property
    def paper_id(self) -> str:
        return self._data.get("paper_id", "")  # type: ignore[no-any-return]

    @property
    def id(self) -> str:
        return self._data.get("paper_id", "")  # type: ignore[no-any-return]

    @property
    def title(self) -> str:
        return self._data.get("title", "")  # type: ignore[no-any-return]

    @property
    def score(self) -> float:
        return self._data.get("score", 0.0)  # type: ignore[no-any-return]

    @property
    def snippet(self) -> str:
        return self._data.get("snippet", "")  # type: ignore[no-any-return]

    def __getattr__(self, name: str) -> Any:
        return self._data.get(name)


class Database:
    """
    Python API shim for rairos-db (Rust).

    Local SQLite mirror keeps papers/embeddings in sync so that
    legacy code using raw SQL (via conn) continues to work.
    """

    def __init__(self, path: Optional[str | "Path"] = None):
        path_str: Optional[str] = None
        if path is not None:
            path_str = str(path) if not isinstance(path, str) else path
        if path_str and path_str != ":memory:":
            self._inner = PyDatabase(path_str)
        elif path_str == ":memory:":
            self._inner = PyDatabase(":memory:")
        else:
            # No path given — create an isolated temp file for the Rust DB
            # to avoid polluting the shared default (~/.rairos/rairos.db)
            fd, tmp_path = tempfile.mkstemp(suffix=".db")
            os.close(fd)
            self._inner = PyDatabase(tmp_path)
        self._inner.init_()
        self._conn: Optional[sqlite3.Connection] = None  # Local mirror
        self._dedup_log: list = []
        self._citations: list = []

    def init(self) -> None:
        self._inner.init_()
        try:
            self._inner.clear_all()
        except Exception:
            pass
        self._init_mirror()

    def _init_mirror(self):
        """Initialize local SQLite mirror from _SCHEMA + _FTS_SCHEMA."""
        fd, path = tempfile.mkstemp(suffix=".db")
        os.close(fd)
        conn = sqlite3.connect(path)
        conn.execute("PRAGMA journal_mode=WAL")
        # Execute full schema (regular tables only; FTS5 needs special handling)
        conn.executescript(_SCHEMA)
        # FTS5 virtual table must be created outside transaction
        try:
            conn.execute(_FTS_SCHEMA.strip())
        except Exception:
            conn.rollback()
        conn.execute(
            "CREATE TABLE IF NOT EXISTS job_queue("
            "id INTEGER PRIMARY KEY AUTOINCREMENT, paper_id TEXT, job_type TEXT, "
            "status TEXT DEFAULT 'queued', priority INTEGER DEFAULT 5, "
            "result TEXT, error TEXT, created_at TEXT, started_at TEXT, completed_at TEXT)"
        )
        conn.execute("CREATE INDEX IF NOT EXISTS idx_job_queue_status ON job_queue(status)")
        conn.execute("CREATE TABLE IF NOT EXISTS settings(key TEXT PRIMARY KEY, value TEXT)")
        conn.commit()

        def cleanup():
            try:
                conn.close()
                os.unlink(path)
            except Exception:
                pass

        atexit.register(cleanup)
        self._conn = conn

    def close(self) -> None:
        if self._conn:
            self._conn.close()
            self._conn = None

    @property
    def conn(self):
        """
        Legacy cursor for backward compatibility.
        Supports SELECT queries on papers (local mirror) and
        embeddings reads/writes.
        """
        return _LegacyCursor(self._conn)

    # -------------------------------------------------------------------------
    # Paper CRUD
    # -------------------------------------------------------------------------

    def upsert_paper(
        self,
        paper_id: str,
        source: str = "",
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
        """Insert or update a paper. Returns a PaperRecord."""
        if isinstance(authors, list):
            authors_list: List[str] = authors
        else:
            authors_list = [authors] if authors else []

        # Write to Rust
        data = {
            "paper_id": paper_id,
            "source": source,
            "title": title,
            "authors": authors_list,
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
            "pdf_path": pdf_path,
            "pdf_hash": pdf_hash,
        }
        result = self._inner.upsert_paper(data)

        # Also write to local mirror (for filtered search / list_papers)
        import datetime

        now = datetime.datetime.now().isoformat()
        authors_json = json.dumps(authors_list)
        self._conn.execute(
            "INSERT OR REPLACE INTO papers (id, source, title, authors, abstract, "
            "published, updated, abs_url, pdf_url, primary_category, journal, volume, "
            "issue, page, doi, categories, reference_count, added_at, updated_at, "
            "parse_status, pdf_path, pdf_hash) "
            "VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, '', ?, ?)",
            (
                paper_id,
                source,
                title,
                authors_json,
                abstract,
                published,
                updated,
                abs_url,
                pdf_url,
                primary_category,
                journal,
                volume,
                issue,
                page,
                doi,
                categories,
                reference_count,
                now,
                now,
                pdf_path,
                pdf_hash,
            ),
        )
        # Also populate FTS
        try:
            self._conn.execute(
                "INSERT OR REPLACE INTO papers_fts (paper_id, title, abstract, plain_text) "
                "VALUES (?, ?, ?, '')",
                (paper_id, title, abstract),
            )
            self._conn.commit()
        except Exception:
            self._conn.rollback()

        if result is None:
            return PaperRecord({})
        return PaperRecord(json.loads(result))

    def get_paper(self, paper_id: str) -> Optional[PaperRecord]:
        """Get a paper by ID."""
        result = self._inner.get_paper(paper_id)
        if result is None:
            return None
        return PaperRecord(json.loads(result))

    def delete_paper(self, paper_id: str) -> bool:
        """Delete a paper. Returns True if a row was deleted."""
        self._conn.execute("DELETE FROM papers WHERE id = ?", (paper_id,))
        self._conn.execute("DELETE FROM embeddings WHERE paper_id = ?", (paper_id,))
        try:
            self._conn.execute("DELETE FROM papers_fts WHERE paper_id = ?", (paper_id,))
        except Exception:
            pass  # FTS virtual table may not exist
        self._conn.commit()
        return self._inner.delete_paper(paper_id)  # type: ignore[no-any-return]

    def paper_exists(self, paper_id: str) -> bool:
        """Check if a paper exists."""
        return self._inner.paper_exists(paper_id)  # type: ignore[no-any-return]

    # -------------------------------------------------------------------------
    # Search
    # -------------------------------------------------------------------------

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
    ) -> Tuple[List[PaperRecord], int]:
        """Full-text search. Returns (results, total_count)."""
        needs_filter = source or category or parse_status or date_from or date_to

        if needs_filter:
            # Use local FTS to get full paper records for filtering
            try:
                rows = self._conn.execute(
                    "SELECT paper_id FROM papers_fts WHERE papers_fts MATCH ?",
                    (query,),
                ).fetchall()
            except Exception:
                rows = []
            candidate_ids = [r[0] for r in rows]

            # Build full PaperRecord objects from local mirror for filtering
            filtered: List[PaperRecord] = []
            for pid in candidate_ids:
                row = self._conn.execute("SELECT * FROM papers WHERE id = ?", (pid,)).fetchone()
                if not row:
                    continue
                cols = [d[1] for d in self._conn.execute("PRAGMA table_info(papers)").fetchall()]
                d = dict(zip(cols, row))
                for field in ("authors", "latex_blocks"):
                    if d.get(field):
                        try:
                            d[field] = json.loads(d[field])
                        except Exception:
                            pass
                record = PaperRecord(d)

                # Apply filters
                if source and record.source != source:
                    continue
                if category and record.primary_category != category:
                    continue
                if parse_status and record.parse_status != parse_status:
                    continue
                pub = getattr(record, "published", "") or ""
                if date_from and pub < date_from:
                    continue
                if date_to and pub > date_to:
                    continue
                filtered.append(record)

            total = len(filtered)
            sliced = filtered[offset : offset + limit]
            return sliced, total

        # No filtering — use Rust FTS directly
        result = self._inner.search_papers(query, limit, offset, None, None, None)
        data = json.loads(result)
        records = [PaperRecord(r) for r in data.get("results", [])]
        return records, data.get("total", 0)

    def rebuild_fts_index(self) -> None:
        """Rebuild the mirror FTS index from current papers table (removes orphans)."""
        conn = self._conn
        conn.execute("DELETE FROM papers_fts")
        conn.execute(
            "INSERT INTO papers_fts(paper_id, title, abstract, plain_text) "
            "SELECT id, title, abstract, plain_text FROM papers"
        )
        conn.commit()

    def list_papers(
        self,
        limit: int = 100,
        offset: int = 0,
        source: Optional[str] = None,
        category: Optional[str] = None,
        date_from: Optional[str] = None,
        date_to: Optional[str] = None,
        parse_status: Optional[str] = None,
        sort_by: str = "added_at",
        sort_order: str = "desc",
    ) -> Tuple[List[PaperRecord], int]:
        """List papers from local mirror."""
        conn = self._conn
        sql = "SELECT * FROM papers WHERE 1=1"
        params: List[Any] = []
        if source:
            sql += " AND source = ?"
            params.append(source)
        if category:
            sql += " AND primary_category = ?"
            params.append(category)
        if parse_status:
            sql += " AND parse_status = ?"
            params.append(parse_status)
        order_col = (
            sort_by
            if sort_by in ("added_at", "published", "updated", "title", "reference_count")
            else "added_at"
        )
        order_dir = "DESC" if sort_order.lower() == "desc" else "ASC"
        sql += f" ORDER BY {order_col} {order_dir} LIMIT ? OFFSET ?"
        params.extend([limit, offset])

        rows = conn.execute(sql, tuple(params)).fetchall()
        cols = [d[1] for d in conn.execute("PRAGMA table_info(papers)").fetchall()]
        records = []
        for row in rows:
            d = dict(zip(cols, row))
            if d.get("authors"):
                try:
                    d["authors"] = json.loads(d["authors"])
                except Exception:
                    d["authors"] = []
            records.append(PaperRecord(d))

        count_sql = "SELECT COUNT(*) FROM papers WHERE 1=1"
        count_params: List[Any] = []
        if source:
            count_sql += " AND source = ?"
            count_params.append(source)
        if category:
            count_sql += " AND primary_category = ?"
            count_params.append(category)
        if parse_status:
            count_sql += " AND parse_status = ?"
            count_params.append(parse_status)
        total_row = conn.execute(count_sql, tuple(count_params)).fetchone()
        total = total_row[0] if total_row else 0
        return records, total

    # -------------------------------------------------------------------------
    # Bulk operations
    # -------------------------------------------------------------------------

    def upsert_papers_bulk(self, papers: List[dict], source: str = "bulk") -> Tuple[int, int]:
        """Bulk upsert. Returns (inserted, updated)."""
        inserted = 0
        for p in papers:
            result = self.upsert_paper(
                paper_id=p.get("paper_id", ""),
                source=source,
                title=p.get("title", ""),
                authors=p.get("authors", []),
                abstract=p.get("abstract", ""),
                published=p.get("published", ""),
                abs_url=p.get("abs_url", ""),
                pdf_url=p.get("pdf_url", ""),
                primary_category=p.get("primary_category", ""),
            )
            if result.id:
                inserted += 1
        return inserted, 0

    # -------------------------------------------------------------------------
    # Embeddings (EmbeddingMixin)
    # -------------------------------------------------------------------------

    def set_embedding(self, paper_id: str, vector: List[float]) -> bool:
        """Store embedding vector for a paper."""
        if not self._inner.paper_exists(paper_id):
            return False
        blob = struct.pack(f"{len(vector)}f", *vector)
        import datetime

        now = datetime.datetime.now().isoformat()
        try:
            self._conn.execute(
                "INSERT OR REPLACE INTO embeddings (paper_id, vector, updated_at) VALUES (?, ?, ?)",
                (paper_id, blob, now),
            )
            self._conn.commit()
            return True
        except Exception:
            return False

    def get_embedding(self, paper_id: str) -> Optional[List[float]]:
        """Retrieve embedding vector for a paper."""
        try:
            row = self._conn.execute(
                "SELECT vector FROM embeddings WHERE paper_id = ?", (paper_id,)
            ).fetchone()
            if row and row[0]:
                count = len(row[0]) // 4
                return list(struct.unpack(f"{count}f", row[0]))
            return None
        except Exception:
            return None

    def get_embeddings_bulk(self, paper_ids: List[str]) -> dict:
        """Bulk retrieve embeddings."""
        result = {}
        try:
            placeholders = ",".join("?" * len(paper_ids))
            rows = self._conn.execute(
                f"SELECT paper_id, vector FROM embeddings WHERE paper_id IN ({placeholders})",
                paper_ids,
            ).fetchall()
            for paper_id, blob in rows:
                if blob:
                    count = len(blob) // 4
                    result[paper_id] = list(struct.unpack(f"{count}f", blob))
        except Exception:
            pass
        return result

    def find_similar(
        self, paper_id: str, top_k: int = 10, threshold: float = 0.0, limit: int = 0
    ) -> List[Tuple[str, float]]:
        """Find similar papers based on embedding cosine similarity."""
        import numpy as np

        row = self.get_embedding(paper_id)
        if not row:
            return []
        target = np.array(row, dtype=np.float32)
        rows = self._conn.execute(
            "SELECT paper_id, vector FROM embeddings WHERE paper_id != ?",
            (paper_id,),
        ).fetchall()
        scored = []
        for pid, blob in rows:
            count = len(blob) // 4
            vec = np.array(list(struct.unpack(f"{count}f", blob)), dtype=np.float32)
            norm_target = np.linalg.norm(target)
            norm_vec = np.linalg.norm(vec)
            if norm_target > 0 and norm_vec > 0:
                sim = float(np.dot(target, vec) / (norm_target * norm_vec))
                if sim >= threshold:
                    scored.append((pid, sim))
        scored.sort(key=lambda x: x[1], reverse=True)
        return scored[:limit] if limit else scored[:top_k]

    def get_similarity(self, paper_id1: str, paper_id2: str) -> Optional[float]:
        """Compute cosine similarity between two papers."""
        import numpy as np

        e1 = self.get_embedding(paper_id1)
        e2 = self.get_embedding(paper_id2)
        if e1 and e2:
            v1, v2 = np.array(e1, dtype=np.float32), np.array(e2, dtype=np.float32)
            norm1, norm2 = np.linalg.norm(v1), np.linalg.norm(v2)
            if norm1 > 0 and norm2 > 0:
                return float(np.dot(v1, v2) / (norm1 * norm2))
        return None

    def get_embedding_stats(self) -> dict:
        """Return stats about embeddings."""
        row = self._conn.execute("SELECT COUNT(*) FROM embeddings").fetchone()
        with_embedding = row[0] if row else 0
        row2 = self._conn.execute(
            "SELECT COUNT(*) FROM papers WHERE title IS NOT NULL AND title != ''"
        ).fetchone()
        total_with_text = row2[0] if row2 else 0
        return {
            "with_embedding": with_embedding,
            "total_with_text": total_with_text,
        }

    # -------------------------------------------------------------------------
    # Stats
    # -------------------------------------------------------------------------

    def paper_count(self, status: Optional[str] = None) -> int:
        """Return count of papers, optionally filtered by parse_status."""
        if status:
            # Query both Rust (papers without local mirror entry) and local mirror
            # Local mirror has the authoritative parse_status
            rust_total = self._inner.get_stats().get("total_papers", 0)
            # pending: count empty parse_status (implicitly pending) + missing from mirror
            if status == "pending":
                mirror_rows = self._conn.execute(
                    "SELECT COUNT(*) FROM papers WHERE parse_status IN (?, '')",
                    (status,),
                ).fetchall()
                mirror_count = sum(r[0] for r in mirror_rows)
            else:
                mirror_rows = self._conn.execute(
                    "SELECT COUNT(*) FROM papers WHERE parse_status = ?",
                    (status,),
                ).fetchall()
                mirror_count = sum(r[0] for r in mirror_rows)
            # Papers not yet in mirror are implicitly pending
            missing = max(
                0, rust_total - self._conn.execute("SELECT COUNT(*) FROM papers").fetchone()[0]
            )
            if status == "pending":
                return mirror_count + missing  # type: ignore[no-any-return]
            return mirror_count  # type: ignore[no-any-return]
        else:
            rust_total = self._inner.get_stats().get("total_papers", 0)
            return rust_total  # type: ignore[no-any-return]

    def get_stats(self) -> dict:
        """Return database statistics."""
        rust_stats = self._inner.get_stats()
        # Add by_source breakdown from local mirror
        source_rows = self._conn.execute(
            "SELECT source, COUNT(*) FROM papers GROUP BY source"
        ).fetchall()
        by_source = {row[0]: row[1] for row in source_rows}
        # by_status breakdown
        status_rows = self._conn.execute(
            "SELECT parse_status, COUNT(*) FROM papers GROUP BY parse_status"
        ).fetchall()
        by_status = {}
        for row in status_rows:
            key = row[0] if row[0] else "pending"
            by_status[key] = row[1]
        # cache entries
        cache_row = self._conn.execute("SELECT COUNT(*) FROM arxiv_search_cache").fetchone()
        cache_count = cache_row[0] if cache_row else 0
        # queue size
        queue_row = self._conn.execute(
            "SELECT COUNT(*) FROM job_queue WHERE status = 'queued'"
        ).fetchone()
        queue_queued = queue_row[0] if queue_row else 0
        running_row = self._conn.execute(
            "SELECT COUNT(*) FROM job_queue WHERE status = 'running'"
        ).fetchone()
        queue_running = running_row[0] if running_row else 0
        return {
            **rust_stats,
            "total_papers": rust_stats.get("total_papers", 0),
            "by_source": by_source,
            "by_status": by_status,
            "cache_entries": cache_count,
            "cache_count": cache_count,
            "dedup_records": len(self._dedup_log),
            "queue_size": queue_queued,
            "queue_queued": queue_queued,
            "queue_running": queue_running,
        }

    # -------------------------------------------------------------------------
    # Legacy / compatibility
    # -------------------------------------------------------------------------

    @property
    def db(self) -> "Database":
        """Alias for backward compatibility."""
        return self

    # Papers

    def get_papers(self, limit: int = 100, offset: int = 0, **kwargs) -> List[dict]:
        """Legacy get_papers — delegates to list_papers."""
        papers, _ = self.list_papers(limit=limit, offset=offset, **kwargs)
        return [r._data for r in papers]

    def get_papers_bulk(self, paper_ids: List[str]) -> Dict[str, PaperRecord]:
        """Bulk get papers by IDs, returning a dict keyed by paper_id."""
        result: Dict[str, PaperRecord] = {}
        for pid in paper_ids:
            p = self.get_paper(pid)
            if p:
                result[pid] = p
        return result

    def get_paper_title(self, paper_id: str) -> str:
        """Get just the title of a paper."""
        p = self.get_paper(paper_id)
        return p.title if p else ""

    def get_papers_by_reading_status(
        self, status: str, limit: int = 100, offset: int = 0
    ) -> List[PaperRecord]:
        """Get papers filtered by reading status."""
        # reading_status stored in parse_status or extra field
        papers, _ = self.list_papers(limit=limit, offset=offset)
        return [p for p in papers if (p.parse_status or "").startswith(status)]

    def find_duplicates(self, paper_id: str) -> List[str]:
        """Find duplicate paper IDs (stub — always returns empty)."""
        return []

    def merge_papers(self, primary_id: str, duplicate_ids: str | List[str]) -> bool:
        """Merge duplicate papers into primary. Returns True if any papers were merged."""
        if isinstance(duplicate_ids, str):
            duplicate_ids = [duplicate_ids]
        primary = self.get_paper(primary_id)
        if primary is None:
            return False

        merged = False
        for dup_id in duplicate_ids:
            dup = self.get_paper(dup_id)
            if dup is None:
                continue

            # Copy non-empty fields from duplicate to primary
            fields_to_copy = [
                "title",
                "authors",
                "abstract",
                "published",
                "updated",
                "abs_url",
                "pdf_url",
                "primary_category",
                "journal",
                "volume",
                "issue",
                "page",
                "doi",
                "categories",
                "reference_count",
                "pdf_path",
                "pdf_hash",
            ]
            for field in fields_to_copy:
                primary_val = getattr(primary, field, "") or ""
                dup_val = getattr(dup, field, "") or ""
                if not primary_val and dup_val:
                    # Use raw SQL to update in both mirrors
                    self._conn.execute(
                        f"UPDATE papers SET {field} = ? WHERE id = ?", (dup_val, primary_id)
                    )

            # Copy parse_status if primary's is empty and duplicate's is filled
            if not (primary.parse_status or ""):
                dup_parse_status = dup.parse_status or ""
                if dup_parse_status:
                    self._conn.execute(
                        "UPDATE papers SET parse_status = ? WHERE id = ?",
                        (dup_parse_status, primary_id),
                    )
                    # Also copy parse-related fields
                    for f in [
                        "plain_text",
                        "latex_blocks",
                        "table_count",
                        "figure_count",
                        "word_count",
                        "page_count",
                        "parse_error",
                        "parse_version",
                    ]:
                        v = getattr(dup, f, None)
                        if v and v != "":
                            self._conn.execute(
                                f"UPDATE papers SET {f} = ? WHERE id = ?", (v, primary_id)
                            )

            # Transfer tags from duplicate to primary
            for tag in self.get_tags(dup_id):
                self.add_tag(primary_id, tag)

            # Transfer enqueued jobs from duplicate to primary
            self._conn.execute(
                "UPDATE job_queue SET paper_id = ? WHERE paper_id = ? AND status = 'queued'",
                (primary_id, dup_id),
            )

            # Delete duplicate
            self.delete_paper(dup_id)
            merged = True

        if merged:
            self._conn.commit()
        return merged

    def log_dedup(self, target_id: str, duplicate_id: str, keep_policy: str) -> None:
        """Log a deduplication event."""
        self._dedup_log.append(
            {
                "target_id": target_id,
                "duplicate_id": duplicate_id,
                "keep_policy": keep_policy,
            }
        )

    def get_dedup_log(self, limit: int = 100) -> List[dict]:
        """Get deduplication log."""
        return list(self._dedup_log[:limit])

    def clear_pending_papers(self) -> int:
        """Clear all papers with parse_status='pending'. Returns count deleted."""
        cur = self._conn.execute("SELECT id FROM papers WHERE parse_status = 'pending'").fetchall()
        count = len(cur)
        self._conn.execute("DELETE FROM papers WHERE parse_status = 'pending'")
        self._conn.commit()
        return count

    def export_papers(self, paper_ids: List[str], format: str = "json", **kwargs) -> str:
        """Export papers in specified format (stub JSON)."""
        papers = self.get_papers_bulk(paper_ids)
        return json.dumps([papers[pid]._data for pid in paper_ids], ensure_ascii=False)

    # Chat sessions

    def create_chat_session(self, session_id: str, title: str = "") -> bool:
        """Create a chat session (stub — no-op)."""
        return True

    def get_chat_sessions(self, limit: int = 20, **kwargs) -> List[dict]:
        """Get chat sessions (stub — empty list)."""
        return []

    def get_chat_messages(self, session_id: str, limit: int = 100) -> List[dict]:
        """Get chat messages for a session (stub — empty list)."""
        return []

    def add_chat_message(
        self,
        session_id: str,
        role: str,
        content: str,
        citations: Optional[List[dict]] = None,
    ) -> bool:
        """Add a chat message (stub)."""
        return True

    def delete_chat_session(self, session_id: str) -> bool:
        """Delete a chat session (stub)."""
        return True

    def search_chat_sessions(self, query: str, limit: int = 15) -> List[dict]:
        """Search chat sessions (stub — empty list)."""
        return []

    def update_chat_session_title(self, session_id: str, title: str) -> bool:
        """Update chat session title (stub)."""
        return True

    # Subscriptions

    def add_arxiv_subscription(self, query: str, name: str = "", max_results: int = 10) -> dict:
        """Add an arXiv subscription (stub)."""
        return {"query": query, "name": name, "max_results": max_results}

    def get_arxiv_subscription(self, subscription_id: str) -> Optional[dict]:
        """Get an arXiv subscription (stub)."""
        return None

    def delete_arxiv_subscription(self, subscription_id: str) -> bool:
        """Delete an arXiv subscription (stub)."""
        return True

    def list_arxiv_subscriptions(self, **kwargs) -> List[dict]:
        """List arXiv subscriptions (stub — empty list)."""
        return []

    def get_active_subscriptions(self) -> List[dict]:
        """Get active arXiv subscriptions (stub — empty list)."""
        return []

    def get_subscription_papers(self, subscription_id: str, limit: int = 20) -> List[PaperRecord]:
        """Get papers from a subscription (stub — empty list)."""
        return []

    # Jobs / queue

    def dequeue_job(self, **kwargs) -> Optional[dict]:
        """Dequeue a queued job. Marks it as running and returns it."""
        import datetime

        now = datetime.datetime.now().isoformat()
        row = self._conn.execute(
            "SELECT id, paper_id, job_type, status, priority, created_at "
            "FROM job_queue WHERE status = 'queued' ORDER BY priority DESC, created_at ASC LIMIT 1"
        ).fetchone()
        if row is None:
            return None
        self._conn.execute(
            "UPDATE job_queue SET status = 'running', started_at = ? WHERE id = ?", (now, row[0])
        )
        self._conn.commit()
        return {
            "id": row[0],
            "paper_id": row[1],
            "job_type": row[2],
            "status": "running",
            "priority": row[4],
            "created_at": row[5],
            "started_at": now,
        }

    def get_queue_jobs(self, status: Optional[str] = None, **kwargs) -> List[dict]:
        """Get jobs in queue (stub — empty list)."""
        return []

    def cancel_job(self, job_id: str) -> bool:
        """Cancel a background job (stub)."""
        return True

    # Cache

    def get_cached_paper(self, paper_id: str) -> Optional[dict]:
        """Get cached paper metadata (stub — always miss)."""
        return None

    def set_cached_paper(self, paper_id: str, data: dict) -> bool:
        """Store cached paper metadata in local mirror."""
        import datetime

        now = datetime.datetime.now().isoformat()
        self._conn.execute(
            "INSERT OR REPLACE INTO arxiv_search_cache(query_hash, query, results_json, created_at) VALUES(?, ?, ?, ?)",
            (paper_id, paper_id, json.dumps(data), now),
        )
        self._conn.commit()
        return True

    def clear_cache(self) -> int:
        """Clear all caches (stub)."""
        return 0

    # Literature reviews

    def get_literature_review(self, review_id: str) -> Optional[dict]:
        """Get a literature review (stub)."""
        return None

    def add_literature_review(
        self, review_id: str, title: str, paper_ids: List[str], **kwargs
    ) -> bool:
        """Add a literature review (stub)."""
        return True

    def update_literature_review(self, review_id: str, **kwargs) -> bool:
        """Update a literature review (stub)."""
        return True

    def delete_literature_review(self, review_id: str) -> bool:
        """Delete a literature review (stub)."""
        return True

    def list_literature_reviews(self, **kwargs) -> List[dict]:
        """List literature reviews (stub — empty list)."""
        return []

    # Citations

    def add_citation(self, source_id: str, target_id: str, context: str = "", **kwargs) -> bool:
        """Add a citation edge. Returns False if already exists."""
        # Check for duplicate
        for c in self._citations:
            if c["source_id"] == source_id and c["target_id"] == target_id:
                return False
        self._citations.append(
            {
                "source_id": source_id,
                "target_id": target_id,
            }
        )
        return True

    def add_citations_batch(self, source_id: str, target_ids: list[str]) -> int:
        """Batch add citations."""
        for target_id in target_ids:
            self._citations.append(
                {
                    "source_id": source_id,
                    "target_id": target_id,
                }
            )
        return len(target_ids)

    def get_citations(
        self, paper_id: str, direction: str = "outgoing", **kwargs
    ) -> List[CitationRecord]:
        """Get citations for a paper."""
        if direction == "from":
            return [
                CitationRecord(
                    id=i, source_id=c["source_id"], target_id=c["target_id"], created_at=""
                )  # type: ignore[misc]
                for i, c in enumerate(self._citations)
                if c["source_id"] == paper_id
            ]
        elif direction == "to":
            return [
                CitationRecord(
                    id=i, source_id=c["source_id"], target_id=c["target_id"], created_at=""
                )  # type: ignore[misc]
                for i, c in enumerate(self._citations)
                if c["target_id"] == paper_id
            ]
        else:
            return [
                CitationRecord(
                    id=i, source_id=c["source_id"], target_id=c["target_id"], created_at=""
                )  # type: ignore[misc]
                for i, c in enumerate(self._citations)
                if c["source_id"] == paper_id or c["target_id"] == paper_id
            ]

    def get_citation_count(self, paper_id: str) -> dict[str, int]:
        """Get citation count for a paper."""
        forward = sum(1 for c in self._citations if c["source_id"] == paper_id)
        backward = sum(1 for c in self._citations if c["target_id"] == paper_id)
        return {"forward": forward, "backward": backward}

    def upsert_citations(self, paper_id: str, citations: List[dict]) -> Tuple[int, int]:
        """Upsert citations for a paper. Returns (new_count, dup_count)."""
        new_count = 0
        dup_count = 0
        for c in citations:
            # Handle both dict and string formats
            if isinstance(c, str):
                target_id = c
            else:
                target_id = c.get("target_id") or c.get("target") or c.get("id", "")
            if not target_id:
                continue
            existed = any(
                x["source_id"] == paper_id and x["target_id"] == target_id for x in self._citations
            )
            if existed:
                dup_count += 1
            else:
                self._citations.append({"source_id": paper_id, "target_id": target_id})
                new_count += 1
        return new_count, dup_count

    def get_edges_by_node(self, node_id: str, **kwargs) -> List[dict]:
        """Get citation edges for a node (stub — empty list)."""
        return []

    # Experiment tables

    def get_experiment_tables(self, paper_id: str, **kwargs) -> List[dict]:
        """Get experiment tables for a paper (stub — empty list)."""
        return []

    def get_all_experiment_tables(self, **kwargs) -> List[dict]:
        """Get all experiment tables (stub — empty list)."""
        return []

    def upsert_experiment_tables(self, paper_id: str, tables: List[ExperimentTableRecord]) -> bool:
        """Upsert experiment tables (stub)."""
        return True

    # Reading status

    def get_reading_status(self, paper_id: str) -> str:
        """Get reading status of a paper (stub — always 'unread')."""
        return "unread"

    def update_reading_status(self, paper_id: str, status: str, **kwargs) -> bool:
        """Update reading status (stub)."""
        return True

    # Code traces

    def get_paper_code_trace(self, paper_id: str, **kwargs) -> Optional[dict]:
        """Get code trace for a paper (stub)."""
        return None

    def list_paper_code_traces(self, limit: int = 50, **kwargs) -> List[dict]:
        """List code traces (stub — empty list)."""
        return []

    def upsert_cache(self, *args, **kwargs):
        """Cache writes no longer needed."""
        return

    def get_cache(self, *args, **kwargs):
        """Cache reads no longer needed."""
        return None

    def get_papers_without_embeddings(self, limit: int = 100) -> List[PaperRecord]:
        """Return papers that don't have embedding vectors (excluding empty-titled)."""
        rows = self._conn.execute(
            "SELECT id FROM papers WHERE title IS NOT NULL AND title != '' "
            "AND id NOT IN (SELECT paper_id FROM embeddings) LIMIT ?",
            (limit,),
        ).fetchall()
        result = []
        for (pid,) in rows:
            full = self.get_paper(pid)
            if full:
                result.append(full)
        return result

    def get_all_papers(self, *args, **kwargs) -> List[dict]:
        """Legacy. Use list_papers instead."""
        papers, _ = self.list_papers(limit=1000)
        return [r._data for r in papers]

    # -------------------------------------------------------------------------
    # Tags
    # -------------------------------------------------------------------------

    def add_tag(self, paper_id: str, tag: str) -> None:
        """Add a tag to a paper."""
        self._conn.execute(
            "INSERT OR IGNORE INTO tags(paper_id, tag) VALUES(?, ?)", (paper_id, tag)
        )
        self._conn.commit()

    def remove_tag(self, paper_id: str, tag: str) -> bool:
        """Remove a tag from a paper. Returns True if tag existed."""
        cur = self._conn.execute(
            "DELETE FROM tags WHERE paper_id=? AND tag=? RETURNING tag", (paper_id, tag)
        )
        deleted = cur.fetchone() is not None
        self._conn.commit()
        return deleted

    def get_tags(self, paper_id: str) -> List[str]:
        """Get tags for a paper."""
        cur = self._conn.execute("SELECT tag FROM tags WHERE paper_id=? ORDER BY tag", (paper_id,))
        return [r[0] for r in cur.fetchall()]

    def list_all_tags(self, **kwargs) -> List[str]:
        """List all unique tags."""
        cur = self._conn.execute("SELECT DISTINCT tag FROM tags ORDER BY tag")
        return [r[0] for r in cur.fetchall()]

    def papers_by_tag(self, tag: str, **kwargs) -> List[PaperRecord]:
        """Get all papers with a given tag."""
        limit = kwargs.get("limit", 100)
        offset = kwargs.get("offset", 0)
        cur = self._conn.execute(
            "SELECT paper_id FROM tags WHERE tag=? ORDER BY paper_id LIMIT ? OFFSET ?",
            (tag, limit, offset),
        )
        result = []
        for (pid,) in cur.fetchall():
            p = self.get_paper(pid)
            if p:
                result.append(p)
        return result

    def update_parse_status(
        self,
        paper_id: str,
        status: str,
        error: str = "",
        plain_text: str = "",
        latex_blocks: Any = "",
        table_count: int = 0,
        figure_count: int = 0,
        word_count: int = 0,
        page_count: int = 0,
    ) -> None:
        """Update parse status for a paper in the local mirror."""
        import datetime

        now = datetime.datetime.now().isoformat()

        # Get current version
        cur = self._conn.execute(
            "SELECT parse_version FROM papers WHERE id = ?", (paper_id,)
        ).fetchone()
        version = (cur[0] or 0) + 1 if cur else 1

        # Serialize latex_blocks
        if isinstance(latex_blocks, list):
            latex_blocks = json.dumps(latex_blocks)
        elif latex_blocks is None:
            latex_blocks = ""

        self._conn.execute(
            "UPDATE papers SET parse_status = ?, parse_error = ?, plain_text = ?, "
            "latex_blocks = ?, table_count = ?, figure_count = ?, word_count = ?, "
            "page_count = ?, parse_version = ?, updated_at = ? WHERE id = ?",
            (
                status,
                error,
                plain_text,
                latex_blocks,
                table_count,
                figure_count,
                word_count,
                page_count,
                version,
                now,
                paper_id,
            ),
        )
        self._conn.commit()

    def record_parse_attempt(
        self,
        paper_id: str,
        duration_sec: float,
        status: str,
        error: str = "",
        pdf_hash: str = "",
        file_size: int = 0,
    ) -> None:
        """Record a parse attempt in the history table."""
        import datetime

        now = datetime.datetime.now().isoformat()
        self._conn.execute(
            "INSERT INTO parse_history(paper_id, status, error, duration_sec, pdf_hash, file_size, attempted_at) VALUES(?, ?, ?, ?, ?, ?, ?)",
            (paper_id, status, error, duration_sec, pdf_hash, file_size, now),
        )
        self._conn.commit()

    def get_parse_history(self, paper_id: str) -> List[dict]:
        """Get parse history for a paper."""
        cur = self._conn.execute(
            "SELECT id, paper_id, status, error, duration_sec, pdf_hash, file_size, attempted_at FROM parse_history WHERE paper_id=? ORDER BY attempted_at DESC",
            (paper_id,),
        )
        return [
            {
                "id": r[0],
                "paper_id": r[1],
                "status": r[2],
                "error": r[3],
                "duration_sec": r[4],
                "pdf_hash": r[5],
                "file_size": r[6],
                "attempted_at": r[7],
            }
            for r in cur.fetchall()
        ]

    # -------------------------------------------------------------------------
    # Settings
    # -------------------------------------------------------------------------

    def set_setting(self, key: str, value: str) -> None:
        """Set a key-value setting."""
        self._conn.execute("INSERT OR REPLACE INTO settings(key, value) VALUES(?, ?)", (key, value))
        self._conn.commit()

    def get_setting(self, key: str, default: Optional[str] = None) -> Optional[str]:
        """Get a key-value setting."""
        cur = self._conn.execute("SELECT value FROM settings WHERE key = ?", (key,))
        row = cur.fetchone()
        return row[0] if row else default

    # -------------------------------------------------------------------------
    # Job queue
    # -------------------------------------------------------------------------

    def enqueue_job(self, paper_id: str, job_type: str, priority: int = 5) -> int:
        """Add a job to the queue. Returns the job ID (rowid)."""
        import datetime

        now = datetime.datetime.now().isoformat()
        cursor = self._conn.execute(
            "INSERT INTO job_queue (paper_id, job_type, status, priority, created_at) "
            "VALUES (?, ?, 'queued', ?, ?)",
            (paper_id, job_type, priority, now),
        )
        self._conn.commit()
        return cursor.lastrowid if cursor.lastrowid else 0

    def complete_job(self, job_id: int, status: str = "done", error: str = "") -> None:
        """Mark a job as complete or failed."""
        import datetime

        now = datetime.datetime.now().isoformat()
        self._conn.execute(
            "UPDATE job_queue SET status = ?, error = ?, completed_at = ? WHERE id = ?",
            (status, error, now, job_id),
        )
        self._conn.commit()

    def queue_depth(self, status: str = "queued") -> int:
        """Return the number of jobs in the queue with the given status."""
        row = self._conn.execute(
            "SELECT COUNT(*) FROM job_queue WHERE status = ?", (status,)
        ).fetchone()
        return row[0] if row else 0

    # -------------------------------------------------------------------------
    # Vacuum
    # -------------------------------------------------------------------------

    def vacuum(self) -> None:
        """Shrink the database file (stub — no-op)."""
        pass


class _SqliteRow:
    """Minimal row-like object with both index and name access."""

    __slots__ = ("_data", "_keys")

    def __init__(self, data: tuple, keys: List[str]):
        self._data = data
        self._keys = keys

    def __getitem__(self, key):
        if isinstance(key, int):
            return self._data[key]
        return self._data[self._keys.index(key)]

    def __iter__(self):
        return iter(self._data)

    def keys(self):
        return self._keys

    def values(self):
        return self._data

    def __len__(self):
        return len(self._data)


class _LegacyCursor:
    """
    Minimal cursor shim for backward-compatible raw SQL access.

    SELECT queries on papers/embeddings hit the local mirror SQLite.
    INSERT/UPDATE/DELETE on papers hit both Rust and local mirror.
    FTS queries return empty (not supported in shim).
    """

    __slots__ = ("_conn", "_last", "_cols")

    def __init__(self, conn: sqlite3.Connection):
        self._conn = conn
        self._last = None  # last sqlite3.Cursor result
        self._cols: List[str] = []  # column names of last SELECT

    def execute(self, query: str, params: tuple = ()):
        q = query.strip().upper()
        if q.startswith("SELECT") or q.startswith("PRAGMA"):
            if "papers_fts" in query.lower() and "FROM papers_fts" in query.upper():
                # Only mock actual FTS virtual table searches, not existence checks
                self._last = None
                return _MockResult()
            self._last = self._conn.execute(query, params)
            self._cols = (
                [d[0] for d in self._last.description]
                if self._last and self._last.description
                else []
            )
            return self
        else:
            # INSERT/UPDATE/DELETE — route through mirror only
            if "papers" in query.lower() and "arxiv_search_cache" not in query.lower():
                if (
                    "INSERT OR REPLACE INTO papers" in query
                    or "INSERT OR IGNORE" in query.upper()
                    or "UPDATE" in q
                ):
                    self._last = self._conn.execute(query, params)
                    return self
            if "embeddings" in query.lower():
                self._last = self._conn.execute(query, params)
                return self
            self._last = None
            return _MockResult()

    def executemany(self, query: str, params_list):
        for params in params_list:
            self.execute(query, params)

    def commit(self):
        self._conn.commit()

    def close(self):
        pass

    @property
    def description(self):
        return self._cols

    def cursor(self):
        return self

    def fetchone(self):
        if self._last is not None:
            row = self._last.fetchone()
            if row is not None:
                return _SqliteRow(tuple(row), self._cols)
            return None
        return None

    def fetchall(self):
        if self._last is not None:
            rows = self._last.fetchall()
            return [_SqliteRow(tuple(r), self._cols) for r in rows]
        return []

    def fetchmany(self, n=None):
        if self._last is not None:
            return self._last.fetchmany(n)
        return []

    def __iter__(self):
        if self._last is not None:
            return iter(self._last)
        return iter([])

    def __enter__(self):
        return self

    def __exit__(self, *args):
        pass


class _MockResult:
    """Fake cursor result for write operations or unsupported queries."""

    def fetchone(self):
        return None

    def fetchall(self):
        return []

    def fetchmany(self, n=None):
        return []

    def __iter__(self):
        return iter([])
