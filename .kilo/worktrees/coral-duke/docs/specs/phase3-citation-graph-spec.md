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

### `cite list --from <paper-id>`

List papers **cited by** `<paper-id>` (backward citations = references).

### `cite list --to <paper-id>`

List papers **that cite** `<paper-id>` (forward citations = bibliography).

### `cite list --from <paper-id> --to <paper-id>`

Show only papers that are in the local DB (ignore external references).

### `cite graph <paper-id>`

Render citation graph as a tree/forest showing forward and backward citation chains.

Supported formats:
- `--format text` (default): plain-text tree view
- `--format json`: JSON graph representation
- `--format mermaid`: Mermaid flowchart

Supported modes:
- `--plaintext` (default): shows inline titles
- `--db`: fetches from local DB only (no external API calls)

Traversal options:
- `--depth N` (default: 2): how many levels deep to traverse
- `--no-forward`: only backward citations
- `--no-backward`: only forward citations

### `cite fetch <paper-id-or-URL>`

Fetch paper metadata and citation data from OpenAlex. Pulls:
- Paper title, authors, year, abstract
- Forward citations (papers citing this paper)
- Backward citations (references in this paper)

Supports: arXiv ID, DOI, PMID, arXiv URL

### `cite import <source>`

Import papers and their citations from various sources:
- `arxiv:<arXiv-id>` — fetch from arXiv API
- `pmid:<pubmed-id>` — fetch from PubMed/NCBI
- `isbn:<isbn>` — fetch from Open Library / ISBNDB

### `cite dedup [--strategy exact|semantic]`

Deduplicate the local papers database:
- `exact` (default): exact match deduplication by PDF hash and DOI
- `semantic`: uses embeddings to find near-duplicate papers

### `cite merge <source-db> [--dry-run]`

Merge another database into the local one. Supports dry-run mode.

### `cite stats [--paper <paper-id>]`

Show citation statistics:
- For a specific paper: forward/backward citation counts
- Global: total papers, total citations, papers with most citations

### `--format csv`

For `cite list`: emit CSV with columns `direction,source_id,source_title,target_id,target_title`.

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
1. **OpenAlex API** (primary) — comprehensive paper and citation data
2. **Local DB** — if paper already exists, resolve title from DB
3. **arXiv API** — for arXiv-based papers
4. **Crossref** — for DOI-based papers
5. **Semantic Scholar** — fallback for additional citation data

## Implementation Phases

COMPLETED:
1. Schema migration: add `citations` table to `_SCHEMA`
2. `CitationRecord` dataclass and `add_citation` / `get_citations` / `add_citations_batch` / `get_citation_count` methods
3. `cite list` CLI subcommand with `--from` / `--to` / `--format` flags
4. `cite graph` with text/json/mermaid formats and DB/plaintext modes
5. `cite fetch` via OpenAlex API
6. `cite import` for arXiv/PMID/ISBN
7. `--from <id> --to <id>` bidirectional bridge mode for `citations` command (PR #172)

PLANNED / IN PROGRESS:
- Crossref priority boost in data source fallback chain
- arXiv/Crossref metadata extraction improvements (`<arxiv:doi>`, `<arxiv:journal_ref>`)

## CLI Parser

Subcommand built with `_build_cite_parser(subparsers)` registered in both legacy and subcommand flows.

Register in `main()` alongside the other `_build_*` calls.

## File Changes

COMPLETED:
- `db/database.py` — schema, CitationRecord, citation methods
- `cli.py` — `_build_cite_parser`, `_run_cite_*`, `cite-*` commands
- `tests/test_cli_cite_*.py` — comprehensive test coverage

## Verification

- `pytest tests/test_cli_cite_graph.py -v` — all pass
- `python ai_research_os.py cite --help` — shows correct usage
- Manual: add a paper, parse its references, run `cite list --from <id>` and `cite list --to <id>`
- `cite graph <id> --format mermaid` — generates valid Mermaid diagram
