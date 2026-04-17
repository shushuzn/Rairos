# Phase 3 — Citation Graph

## Motivation

Researchers need to trace how ideas evolve: which papers cite paper X (forward citations), and which papers does paper X cite (backward citations). Currently this requires manual Google Scholar lookup. This phase adds it directly to the CLI.

## Data Model

### New Table: `citations`

```sql
CREATE TABLE IF NOT EXISTS citations (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    source_id   TEXT NOT NULL,   -- paper doing the citing
    target_id   TEXT NOT NULL,   -- paper being cited
    created_at  TEXT NOT NULL,
    FOREIGN KEY (source_id)  REFERENCES papers(id)  ON DELETE CASCADE,
    FOREIGN KEY (target_id)  REFERENCES papers(id)  ON DELETE CASCADE,
    UNIQUE(source_id, target_id)
);

CREATE INDEX IF NOT EXISTS idx_citations_source ON citations(source_id);
CREATE INDEX IF NOT EXISTS idx_citations_target ON citations(target_id);
```

- `source_id` → `target_id` = "source cites target"
- One row per unique citation pair
- Parsed from the `references` field of the arXiv/PMID response
- Deduplicated at insert time (UPSERT with `ON CONFLICT DO NOTHING`)

### CitationRecord Dataclass

```python
@dataclass
class CitationRecord:
    id: int
    source_id: str
    target_id: str
    created_at: str
```

## CLI Commands

### `citations --from <paper-id>`

List papers **cited by** `<paper-id>` (backward citations = references).

```
$ python ai_research_os.py citations --from 2301.00001
BACKWARD CITATIONS (5 refs):
  2203.00123  Paper Title Here ...
  2203.00144  Another Paper ...
```

### `citations --to <paper-id>`

List papers **that cite** `<paper-id>` (forward citations = bibliography).

```
$ python ai_research_os.py citations --to 2301.00001
FORWARD CITATIONS (3 citing papers):
  2401.00234  Yet Another Paper ...
```

### `citations --from <paper-id> --to <paper-id>`

Show only papers that are in the local DB (ignore external references).

### `--format csv`

Emit CSV with columns: `direction,source_id,source_title,target_id,target_title`.

## Database Methods

### `get_citations(paper_id: str, direction: Literal["from","to","both"]) -> list[CitationRecord]`

Fetch citations. `direction="from"` → backward (papers this paper cites).
`direction="to"` → forward (papers citing this paper).

### `add_citation(source_id: str, target_id: str) -> bool`

Insert one citation pair. Returns `True` if inserted, `False` if duplicate.

### `add_citations_batch(source_id: str, target_ids: list[str]) -> int`

Bulk insert. Returns count of newly inserted rows.

### `get_citation_count(paper_id: str, direction: str) -> dict`

Return `{"forward": N, "backward": M}` counts.

## Data Sources

Priority order:
1. **Local DB** — if `target_id` or `source_id` already exists in `papers`, resolve title from DB
2. **arXiv API** — for papers not in DB, attempt to fetch metadata via arXiv ID
3. **Crossref** — for DOI-based papers

The citation parsing itself happens during the arXiv metadata fetch. The `fetch_arxiv_metadata` function should be extended to extract the `<arxiv:doi>` and `<arxiv:journal_ref>` sub-elements when available, and store them.

## Implementation Phases

1. Schema migration: add `citations` table to `_SCHEMA`
2. `CitationRecord` dataclass and `add_citation` / `get_citations` methods
3. `citations` CLI subcommand with `--from` / `--to` / `--format` flags
4. Tests

## CLI Parser

New subcommand built with `_build_citations_parser(subparsers)` registered in both legacy and subcommand flows.

Register in `main()` at line ~541 alongside the other `_build_*` calls.

## File Changes

- `db/database.py` — schema, CitationRecord, new methods
- `cli.py` — `_build_citations_parser`, `_run_citations`, register in `main()`
- `tests/test_cli_citations.py` — TBD

## Verification

- `pytest tests/test_cli_citations.py -v` — all pass
- `python ai_research_os.py citations --help` — shows correct usage
- Manual: add a paper, parse its references, run `citations --from <id>` and `citations --to <id>`
