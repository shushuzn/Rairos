# Phase 2 Specification: Search & Discovery

## Status: COMPLETED

## Motivation

From the 33 defects: papers cannot be searched by content (#6), filtered by recency (#7), or ranked by relevance (#8). The system has no full-text index and no semantic search capability.

## Goals

1. **Full-text search** — keyword search across paper titles, abstracts, and extracted text
2. **Filtered queries** — by date range, category, source, parse status
3. **Ranking** — BM25 scoring for keyword queries
4. **Search API** — `search` CLI command with query, filters, pagination

---

## 1. Full-Text Search

### Option A: SQLite FTS5 (chosen for Phase 2)

SQLite's FTS5 module provides BM25 ranking, snippet extraction, and boolean queries with zero additional infrastructure.

Pros: Zero setup, portable, synchronous with existing SQLite DB, BM25 built-in.
Cons: No semantic/vector search (deferred to Phase 3).

### Option B: PostgreSQL + pgvector (Phase 3)

For semantic vector search. Requires a running PostgreSQL instance.

---

### FTS5 Schema

```sql
-- Virtual table for full-text search
CREATE VIRTUAL TABLE IF NOT EXISTS papers_fts USING fts5(
    paper_id UNINDEXED,
    title,
    abstract,
    plain_text,
    content='papers',
    content_rowid='rowid',
    tokenize='porter unicode61'
);
```

### FTS Triggers (keep FTS in sync)

```sql
CREATE TRIGGER IF NOT EXISTS papers_fts_insert AFTER INSERT ON papers BEGIN
    INSERT INTO papers_fts(rowid, paper_id, title, abstract, plain_text)
    VALUES (NEW.rowid, NEW.id, NEW.title, NEW.abstract, NEW.plain_text);
END;

CREATE TRIGGER IF NOT EXISTS papers_fts_delete AFTER DELETE ON papers BEGIN
    INSERT INTO papers_fts(papers_fts, rowid, paper_id, title, abstract, plain_text)
    VALUES ('delete', OLD.rowid, OLD.id, OLD.title, OLD.abstract, OLD.plain_text);
END;

CREATE TRIGGER IF NOT EXISTS papers_fts_update AFTER UPDATE ON papers BEGIN
    INSERT INTO papers_fts(papers_fts, rowid, paper_id, title, abstract, plain_text)
    VALUES ('delete', OLD.rowid, OLD.id, OLD.title, OLD.abstract, OLD.plain_text);
    INSERT INTO papers_fts(rowid, paper_id, title, abstract, plain_text)
    VALUES (NEW.rowid, NEW.id, NEW.title, NEW.abstract, NEW.plain_text);
END;
```

---

## 2. Search API

### `db/database.py` additions

```python
@dataclass
class SearchResult:
    paper_id: str
    title: str
    authors: str
    published: str
    primary_category: str
    score: float          # BM25 score
    snippet: str          # text snippet with highlight
    parse_status: str

def search_papers(
    query: str,
    limit: int = 20,
    offset: int = 0,
    source: str = None,       # 'arxiv' or 'doi'
    category: str = None,
    date_from: str = None,     # YYYY-MM-DD
    date_to: str = None,
    parse_status: str = None,
) -> Tuple[List[SearchResult], int]:
    """Full-text search with BM25 ranking and filters. Returns (results, total)."""

def list_papers(
    limit: int = 20,
    offset: int = 0,
    source: str = None,
    category: str = None,
    date_from: str = None,
    date_to: str = None,
    parse_status: str = None,
    sort_by: str = "added_at",  # 'added_at', 'published', 'title'
    sort_order: str = "desc",    # 'asc', 'desc'
) -> Tuple[List[PaperRecord], int]:
    """Filtered list without full-text query."""
```

---

## 3. CLI Integration

### `cli.py` additions

```python
@cli.command()
@click.argument("query", required=False)
@click.option("--limit", "-n", default=20, help="Max results")
@click.option("--offset", default=0, help="Skip N results")
@click.option("--source", type=click.Choice(["arxiv", "doi"]))
@click.option("--category", "-c")
@click.option("--date-from")
@click.option("--date-to")
@click.option("--status", "parse_status", type=click.Choice(["pending", "parsing", "done", "failed"]))
@click.option("--sort", default="relevance", type=click.Choice(["relevance", "date", "title"]))
def search(query, limit, offset, source, category, date_from, date_to, parse_status, sort):
    """Search papers by keyword or list with filters.

    Examples:
        ai-research-os search "transformer attention"
        ai-research-os search --category cs.LG --date-from 2024-01-01
        ai-research-os search --status done --sort date
    """
```

Output format:
```
 7.43  [cs.LG] Attention Is All You Need
       Vaswani et al.  2017-06-12
       "...attention mechanism... Transformer architecture..."
       https://arxiv.org/abs/1706.03762

 5.21  [cs.CL] BERT: Pre-training of Deep Bidirectional...
       Devlin et al.  2018-10-11
       "...attention-based... pre-training..."
       https://arxiv.org/abs/1810.04805
```

---

## 4. File Structure (Phase 2 additions)

```
db/
    database.py     # MODIFIED: add search_papers, list_papers, FTS triggers
    __init__.py     # MODIFIED: export SearchResult
search/
    __init__.py     # NEW
    fts.py          # NEW: FTS5 query helpers, BM25 scoring
tests/
    test_search.py  # NEW: search_papers, list_papers, FTS triggers
```

---

## 5. Backward Compatibility

- `papers_fts` is a virtual table — does not affect existing `papers` table
- FTS triggers are `IF NOT EXISTS` — safe to re-run on existing DB
- `list_papers` is a filtered variant of existing `get_papers` — adds new capabilities only
- No changes to Phase 1 public APIs

---

## 6. Open Questions

| # | Question | Decision |
|---|----------|----------|
| 1 | Snippet extraction — use FTS5 `snippet()` or custom? | FTS5 `snippet()` first, custom fallback |
| 2 | Highlight markers in snippets? | Use `**bold**` in CLI output |
| 3 | Empty query → list mode or error? | List mode with default sort by `added_at desc` |
| 4 | Index rebuild for existing papers? | Add `db.rebuild_fts_index()` method, manual trigger |

---

## 7. TODO

- [x] Add FTS5 virtual table + triggers to `Database.init()`
- [x] Implement `search_papers()` with BM25 ranking + filters
- [x] Implement `list_papers()` with sort + filters
- [x] Add `SearchResult` dataclass
- [x] Add `snippet()` helper for FTS5 result highlighting
- [x] Wire `search` command into `cli.py`
- [x] Write `tests/test_search.py` (unit + integration)
- [x] Add `rebuild_fts_index()` for existing data migration
- [x] Update docs/usage.md with search examples

---

## Acceptance Criteria

1. `ai-research-os search "attention mechanism"` returns BM25-ranked results in <100ms for 1000 papers
2. `ai-research-os search --category cs.LG --date-from 2024-01-01` correctly filters
3. `ai-research-os search` (no query) lists recent papers sorted by `added_at`
4. Snippets show highlighted matching text
5. All Phase 1 tests still pass (571 + new)
6. FTS triggers stay in sync on `upsert_paper` and `delete_paper`
