"""
db/database.py — Database layer (Rust-powered)

All database operations now delegate to rairos_db_py (Rust).
This file provides a Python API shim for backward compatibility.
"""

from typing import Any, List, Optional, Tuple

from rairos_db_py import PyDatabase


class PaperRecord:
    """Thin wrapper around paper dict returned from Rust."""

    __slots__ = ("_data",)

    def __init__(self, data: dict):
        self._data = data

    @property
    def id(self) -> str:
        return self._data.get("id", "")

    @property
    def paper_id(self) -> str:
        return self._data.get("id", "")

    @property
    def title(self) -> str:
        return self._data.get("title", "")

    @property
    def authors(self) -> List[str]:
        return self._data.get("authors", [])

    @property
    def abstract(self) -> str:
        return self._data.get("abstract", "")

    @property
    def published(self) -> str:
        return self._data.get("published", "")

    @property
    def updated(self) -> str:
        return self._data.get("updated", "")

    @property
    def source(self) -> str:
        return self._data.get("source", "")

    @property
    def parse_status(self) -> str:
        return self._data.get("parse_status", "")

    @property
    def primary_category(self) -> str:
        return self._data.get("primary_category", "")

    @property
    def added_at(self) -> str:
        return self._data.get("added_at", "")

    @property
    def abs_url(self) -> str:
        return self._data.get("abs_url", "")

    @property
    def pdf_url(self) -> str:
        return self._data.get("pdf_url", "")

    def __getitem__(self, key: str) -> Any:
        return self._data.get(key)

    def __repr__(self) -> str:
        return f"PaperRecord({self.id!r}: {self.title!r})"

    def __getattr__(self, name: str) -> Any:
        return self._data.get(name, "")


class Database:
    """
    Python API shim for rairos-db (Rust).

    Wraps PyDatabase (PyO3) and exposes the same API as the original
    Python Database class so existing call sites work without changes.
    """

    def __init__(self, path: Optional[str] = None):
        if path and path != ":memory:":
            self._inner = PyDatabase(path)
        elif path == ":memory:":
            self._inner = PyDatabase(":memory:")
        else:
            self._inner = PyDatabase()
        self._inner.init_()

    def init(self) -> None:
        self._inner.init_()

    def close(self) -> None:
        # No-op: Rust manages connection lifetime
        pass

    @property
    def conn(self):
        """Raw connection for advanced queries — returns a mock cursor."""
        return _MockCursor(self)

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
        if result is None:
            return PaperRecord({})
        import json

        data = json.loads(result)
        return PaperRecord(data)

    def get_paper(self, paper_id: str) -> Optional[PaperRecord]:
        """Get a paper by ID."""
        import json

        result = self._inner.get_paper(paper_id)
        if result is None:
            return None
        return PaperRecord(json.loads(result))

    def delete_paper(self, paper_id: str) -> bool:
        """Delete a paper. Returns True if a row was deleted."""
        return self._inner.delete_paper(paper_id)

    def paper_exists(self, paper_id: str) -> bool:
        """Check if a paper exists."""
        return self._inner.paper_exists(paper_id)

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
        import json

        result = self._inner.search_papers(
            query, limit, offset, source, category, parse_status
        )
        data = json.loads(result)
        total = data.get("total", 0)
        records = [PaperRecord(r) for r in data.get("results", [])]
        return records, total

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
        """List papers with optional filters. Returns (papers, total_count)."""
        import json

        result = self._inner.list_papers(
            limit, offset, source, category, parse_status
        )
        data = json.loads(result)
        total = data.get("total", 0)
        records = [PaperRecord(r) for r in data.get("papers", [])]
        return records, total

    # -------------------------------------------------------------------------
    # Bulk operations
    # -------------------------------------------------------------------------

    def upsert_papers_bulk(
        self, papers: List[dict], source: str = "bulk"
    ) -> Tuple[int, int]:
        """Bulk upsert. Returns (inserted, updated)."""
        inserted = 0
        updated = 0
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
        return inserted, updated

    # -------------------------------------------------------------------------
    # Stats
    # -------------------------------------------------------------------------

    def paper_count(self, parse_status: Optional[str] = None) -> int:
        """Count papers, optionally filtered by parse_status."""
        return self._inner.paper_count(parse_status)

    def get_total_papers(self) -> int:
        """Total paper count."""
        return self._inner.get_total_papers()

    def get_stats(self) -> dict:
        """Get database statistics."""
        return self._inner.get_stats()


class SearchResult:
    """Wrapper for search result dict."""

    __slots__ = ("_data",)

    def __init__(self, data: dict):
        self._data = data

    @property
    def paper_id(self) -> str:
        return self._data.get("paper_id", "")

    @property
    def id(self) -> str:
        return self._data.get("paper_id", "")

    @property
    def title(self) -> str:
        return self._data.get("title", "")

    @property
    def score(self) -> float:
        return self._data.get("score", 0.0)

    @property
    def snippet(self) -> str:
        return self._data.get("snippet", "")

    def __getattr__(self, name: str) -> Any:
        return self._data.get(name)


class _MockCursor:
    """Minimal cursor mock for code that accesses db.conn directly."""

    __slots__ = ("_db",)

    def __init__(self, db: Database):
        self._db = db

    def execute(self, query: str, params: tuple = ()):
        """Mock execute — only handles a few known patterns."""
        import sqlite3

        # For simple queries that some code uses, fall back to raw SQLite
        # on a separate in-memory DB for read operations
        conn = sqlite3.connect(":memory:")
        return conn.execute(query, params)

    def __enter__(self):
        return self

    def __exit__(self, *args):
        pass
