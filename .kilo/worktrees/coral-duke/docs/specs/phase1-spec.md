# Phase 1 Specification: Infrastructure Rebuild

## Status: PLANNED

## Motivation
33 defects identified. Phase 1 fixes the foundation — making the system robust, extensible, and capable of error recovery.

## Goals
1. **Database layer** — replace ad-hoc file system with SQLite
2. **Error handling** — global retry/circuit-breaker, graceful degradation
3. **PDF parsing** — proper LaTeX extraction + table structure + cache
4. **Cache mechanism** — avoid repeated parsing of same PDF

---

## 1. Database Layer

### Schema

```sql
-- papers: one row per unique paper
CREATE TABLE papers (
    id              TEXT PRIMARY KEY,       -- arXiv ID or DOI (normalized)
    source          TEXT NOT NULL,           -- 'arxiv' or 'doi'
    title           TEXT,
    authors         TEXT,                   -- JSON array as text
    abstract        TEXT,
    published       TEXT,                   -- YYYY-MM-DD
    updated         TEXT,                   -- YYYY-MM-DD
    abs_url         TEXT,
    pdf_url         TEXT,
    primary_category TEXT,
    journal         TEXT,
    volume          TEXT,
    issue           TEXT,
    page            TEXT,
    doi             TEXT,
    categories      TEXT,                   -- comma-separated
    reference_count INTEGER DEFAULT 0,
    -- metadata
    added_at        TEXT NOT NULL,          -- ISO timestamp
    updated_at      TEXT NOT NULL,
    -- processing state
    pdf_path        TEXT,                   -- local PDF file path
    pdf_hash        TEXT,                   -- SHA256 of PDF for cache
    parse_status    TEXT DEFAULT 'pending', -- pending|parsing|done|failed
    parse_error     TEXT,                   -- last error message
    parse_version   INTEGER DEFAULT 0,       -- increment on re-parse
    -- structured content (populated after parsing)
    plain_text      TEXT,                   -- full text extracted
    latex_blocks     TEXT,                   -- JSON array of LaTeX strings
    table_count     INTEGER DEFAULT 0,
    figure_count    INTEGER DEFAULT 0,
    word_count      INTEGER DEFAULT 0,
    page_count      INTEGER DEFAULT 0,
    -- AI summaries
    pnote_path      TEXT,
    cnote_path      TEXT,
    mnote_path      TEXT,
    -- search
    embed_vector    BLOB                    -- future: pgvector-compatible
);

-- parse_history: audit trail for each parse attempt
CREATE TABLE parse_history (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    paper_id    TEXT NOT NULL,
    attempted_at TEXT NOT NULL,
    duration_sec REAL,
    status      TEXT NOT NULL,           -- success|failed|partial
    error       TEXT,
    parse_version INTEGER,
    pdf_hash    TEXT,
    file_size   INTEGER,
    FOREIGN KEY (paper_id) REFERENCES papers(id)
);

-- tags: many-to-many
CREATE TABLE tags (
    id      INTEGER PRIMARY KEY AUTOINCREMENT,
    name    TEXT UNIQUE NOT NULL
);

CREATE TABLE paper_tags (
    paper_id TEXT NOT NULL,
    tag_id  INTEGER NOT NULL,
    PRIMARY KEY (paper_id, tag_id),
    FOREIGN KEY (paper_id) REFERENCES papers(id),
    FOREIGN KEY (tag_id) REFERENCES tags(id)
);

-- processing queue (for batch operations)
CREATE TABLE job_queue (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    paper_id    TEXT NOT NULL,
    job_type    TEXT NOT NULL,           -- 'parse'|'summarize'|'embed'
    priority    INTEGER DEFAULT 5,
    status      TEXT DEFAULT 'queued',   -- queued|running|done|failed
    created_at  TEXT NOT NULL,
    started_at  TEXT,
    completed_at TEXT,
    error       TEXT,
    FOREIGN KEY (paper_id) REFERENCES papers(id)
);

-- settings / key-value store
CREATE TABLE settings (
    key    TEXT PRIMARY KEY,
    value  TEXT
);
```

### API

```python
# db/__init__.py
from db.database import Database, PaperRecord, ParseHistoryRecord

db = Database("~/.cache/ai_research_os/research.db")
db.init()  # creates tables if not exist

# CRUD
paper = db.upsert_paper(paper_id, source, title=..., abstract=..., ...)
paper = db.get_paper(paper_id)
papers = db.list_papers(status="done", limit=100)
db.update_parse_status(paper_id, status, error=None)
db.add_tag(paper_id, "transformer")
tags = db.get_tags(paper_id)
db.enqueue_job(paper_id, "parse")
job = db.dequeue_job()  # atomic pop
db.record_parse_attempt(paper_id, duration_sec, status, error=None)
```

---

## 2. Error Handling

### Retry Decorator

```python
# core/retry.py
from functools import wraps
import time
import logging

logger = logging.getLogger(__name__)

def retry(
    max_attempts: int = 3,
    base_delay: float = 1.0,
    max_delay: float = 30.0,
    exceptions: tuple = (Exception,),
    on_retry: callable = None,
):
    """Exponential backoff retry decorator."""
    def decorator(fn):
        @wraps(fn)
        def wrapper(*args, **kwargs):
            last_exc = None
            for attempt in range(1, max_attempts + 1):
                try:
                    return fn(*args, **kwargs)
                except exceptions as e:
                    last_exc = e
                    if attempt == max_attempts:
                        break
                    delay = min(base_delay * (2 ** (attempt - 1)), max_delay)
                    logger.warning(f"[retry] {fn.__name__} attempt {attempt}/{max_attempts} failed: {e}. Waiting {delay:.1f}s")
                    if on_retry:
                        on_retry(e, attempt)
                    time.sleep(delay)
            raise last_exc
        return wrapper
    return decorator


def circuit_breaker(
    failure_threshold: int = 5,
    recovery_timeout: float = 60.0,
    expected_exception: type = Exception,
):
    """Circuit breaker decorator. Opens after failure_threshold failures."""
    def decorator(fn):
        @wraps(fn)
        def wrapper(*args, **kwargs):
            # implementation in core/retry.py
            ...
        return wrapper
    return decorator
```

### Global Exception Classes

```python
# core/exceptions.py
class AIResearchOSError(Exception):
    """Base exception."""
    pass

class PDFParseError(AIResearchOSError):
    """PDF extraction failed."""
    pass

class APIClientError(AIResearchOSError):
    """External API call failed after retries."""
    pass

class PaperNotFoundError(AIResearchOSError):
    """Paper ID not in database."""
    pass

class DatabaseError(AIResearchOSError):
    """Database operation failed."""
    pass
```

### Usage
- `download_pdf`: retry on HTTP 5xx, timeout, connection error
- `extract_pdf_text`: retry on corruption, fallback to empty string + log
- `call_llm_chat_completions`: retry on 429 rate limit (with backoff), 5xx errors
- All public functions: never raise bare `Exception`, always wrap in typed exception

---

## 3. PDF Parsing Enhancement

### Requirements
- Extract actual LaTeX source from PDF (not just Unicode approximations)
- Table structure preservation (not just text dump)
- Figure/caption detection
- Robust fallback chain: text → pdfminer → OCR

### Implementation: `pdf/parser.py` (new)

Replaces `pdf/extract.py` for structured extraction.

```python
# pdf/parser.py
from dataclasses import dataclass, field
from pathlib import Path
from typing import List, Optional
import hashlib

from core.exceptions import PDFParseError
from core.retry import retry

@dataclass
class LaTeXBlock:
    source: str           # actual LaTeX source
    is_display: bool      # standalone equation vs inline
    page: int
    bbox: tuple = None

@dataclass
class TableData:
    headers: List[str]
    rows: List[List[str]]
    page: int
    bbox: tuple = None
    caption: str = ""

@dataclass
class FigureData:
    caption: str
    page: int
    bbox: tuple
    alt_text: str = ""

@dataclass
class ParsedPaper:
    paper_id: str
    text: str                    # cleaned plain text
    latex_blocks: List[LaTeXBlock]
    tables: List[TableData]
    figures: List[FigureData]
    page_count: int
    word_count: int
    parse_version: int
    pdf_hash: str
    # metadata
    title: str = ""
    authors: List[str] = field(default_factory=list)
    abstract: str = ""
    published: str = ""
    # processing info
    warnings: List[str] = field(default_factory=list)
    errors: List[str] = field(default_factory=list)

class PDFParser:
    def __init__(self, cache_db: Database = None):
        self.cache_db = cache_db

    @retry(max_attempts=2, exceptions=(OSError, IOError))
    def parse(self, pdf_path: Path, paper_id: str) -> ParsedPaper:
        """Main entry point. Uses cache if available."""
        pdf_hash = self._hash_file(pdf_path)
        
        # Check cache
        if self.cache_db:
            cached = self._check_cache(paper_id, pdf_hash)
            if cached:
                return cached
        
        # Parse fresh
        paper = self._do_parse(pdf_path, paper_id, pdf_hash)
        
        # Save to cache
        if self.cache_db:
            self._save_to_cache(paper)
        
        return paper

    def _do_parse(self, pdf_path: Path, paper_id: str, pdf_hash: str) -> ParsedPaper:
        """Actual parsing logic with fallback chain."""
        # Step 1: PyMuPDF structured extraction (tables, blocks)
        content = self._extract_structured(pdf_path)
        
        # Step 2: If text is too short or garbled, try pdfminer
        if len(content.get("text", "")) < 500:
            content = self._extract_pdfminer_fallback(pdf_path)
        
        # Step 3: Latex extraction — try to get actual LaTeX source
        latex = self._extract_latex(pdf_path)
        
        # Step 4: Table structure (improved)
        tables = self._extract_tables_improved(pdf_path)
        
        # Step 5: Figure detection
        figures = self._extract_figures(pdf_path)
        
        return ParsedPaper(
            paper_id=paper_id,
            text=content.get("text", ""),
            latex_blocks=latex,
            tables=tables,
            figures=figures,
            page_count=content.get("page_count", 0),
            word_count=len(content.get("text", "").split()),
            parse_version=1,
            pdf_hash=pdf_hash,
            warnings=content.get("warnings", []),
            errors=content.get("errors", []),
        )
```

### Fallback Chain
1. **PyMuPDF** `get_text("text")` — fast, most PDFs
2. **pdfminer.six** — for complex encodings / security-restricted PDFs
3. **Tesseract OCR** — last resort for scanned/image-only PDFs

---

## 4. Cache Mechanism

### PDF Parse Cache
- Key: `paper_id + pdf_hash`
- Stored in: SQLite `papers` table (`parse_status`, `plain_text`, `latex_blocks`, `pdf_hash`)
- Invalidation: if `pdf_hash` changes → re-parse
- TTL: no TTL for parsed content (content doesn't expire)

### HTTP API Cache
- arXiv API: 24h TTL (already exists in `core/cache.py`)
- Crossref API: 24h TTL (extend existing cache)
- LLM API responses: 7d TTL for identical prompts (new)

### File-based cache
```
~/.cache/ai_research_os/
├── arxiv/          # HTTP cache (existing)
├── crossref/       # HTTP cache (existing)
├── pdf/            # NEW: downloaded PDFs (avoid re-fetch)
├── parsed/         # NEW: parsed JSON per paper_id
└── research.db     # NEW: SQLite database
```

---

## 5. Backward Compatibility

- `pdf/extract.py` stays as-is during Phase 1 (thin wrapper around new `pdf/parser.py`)
- `core/cache.py` stays as-is
- All existing `__init__.py` re-exports unchanged
- `cli.py` unchanged until Phase 2

### Migration Path
- New code in `db/`, `pdf/parser.py`, `core/retry.py`, `core/exceptions.py`
- Existing code untouched until Phase 1 is stable
- Phase 2 will wire new modules into CLI

---

## File Structure (Phase 1 additions)

```
core/
    exceptions.py    # NEW
    retry.py        # NEW
db/
    __init__.py     # NEW
    database.py     # NEW
    schema.sql      # NEW (embedded in database.py)
pdf/
    parser.py       # NEW (replaces extract.py for structured extraction)
    extract.py      # UNCHANGED (wrapper)
```

---

## Acceptance Criteria

1. `Database` class: CRUD for papers, tags, job queue, parse history
2. `PDFParser.parse()` returns `ParsedPaper` with LaTeX blocks, tables, figures
3. Cache hit returns same result without re-parsing
4. `retry` decorator handles HTTP failures with exponential backoff
5. No bare `Exception` propagation from public functions
6. All existing tests pass (no regression)
7. `python -m pytest` passes in new venv
