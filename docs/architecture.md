# Architecture

## System Diagram

![AI Research OS Architecture](../assets/architecture.html)

*Open [architecture.html](../assets/architecture.html) in a browser for the interactive/zoomable version.*

## Overview

```
CLI (python -m cli)
    │
    ├── parsers/           # arXiv, DOI, OpenAlex, PDF
    ├── database.py        # SQLite + FTS5
    ├── embed.py           # Ollama embedding client
    └── citation.py        # OpenAlex citation API
```

AI Research OS is a **local-first** research tool. No cloud dependency — all data stays in `~/.ai_research_os/`.

## Modules

### CLI Entry Point

`cli/` — 23 subcommands registered via `argparse`. Two invocation styles:

```bash
python -m cli <subcommand> [args]   # recommended
airos-cli <subcommand> [args]       # via entry point
```

### `parsers/`

| Module | Input | Output |
|--------|-------|--------|
| `arxiv.py` | arXiv ID or URL | `Paper` object |
| `crossref.py` | DOI | `Paper` + optional arXiv ID |
| `pdf.py` | Local PDF path | `Paper` with full text |
| `openalex.py` | OpenAlex work ID | Citation graph data |
| `input_detection.py` | Arbitrary string | Detects arXiv ID, DOI, or file path |

### `database.py`

SQLite wrapper with FTS5 full-text search.

**Schema:**

| Table | Purpose |
|-------|---------|
| `papers` | One row per unique paper |
| `papers_fts` | FTS5 virtual table — BM25 full-text search |
| `papers_embeddings` | 768-dim vectors via Ollama `nomic-embed-text` |
| `citations` | (source_id, target_id) directed edges |

**Key methods:**
- `upsert_paper()` — insert or update, handles duplicates
- `search_papers(query)` — FTS5 with BM25 ranking, category/date filters
- `set_embedding()` / `find_similar()` — semantic deduplication
- `add_citation()` / `get_citations()` — citation graph traversal

### `embed.py`

Ollama embedding client at `http://localhost:11434/api/embeddings`.

- Model: `nomic-embed-text` (768-dim)
- Used for: semantic deduplication, similarity search

### `citation.py`

OpenAlex API client. Bypasses Windows proxy SSL issues via custom SSL context.

```
GET /works?filter=doi:{doi}              → resolve → OpenAlex ID
GET /works/{id}/references               → backward citations
GET /works?filter=cites:{id}            → forward citations
```

### `notes/`

Structured knowledge output from paper processing:

| Structure | Scope | Description |
|-----------|-------|-------------|
| P-Note | Per paper | One key insight per paper |
| C-Note | Per concept/tag | Aggregated notes across papers sharing a tag |
| M-Note | 3+ papers | Comparison when papers share a tag |
| Radar | Per tag | Topic frequency heat score |
| Timeline | Per tag | Year-based research evolution |

### `kg/`

Knowledge graph — paper-level and concept-level nodes with citation edges.

### `research_loop/`

Autonomous research loop (Plan → Act → Observe → Learn):

- `paper2code_integration/` — generate code from paper
- `rag_pipeline/` — RAG: paper → code → tests → benchmark
- `evoskill_integration/` — feedback-driven skill discovery

## Data Flow

```
import PAPER_ID
    │
    ▼
parse (arXiv | DOI | PDF)
    │
    ▼
upsert_paper() → papers table
    │
    ├──► index_paper()     → papers_fts (FTS5)
    └──► embed.py          → papers_embeddings (Ollama)

search "query"
    │
    ▼
FTS5 BM25(query) → ranked results

cite-fetch PAPER_ID
    │
    ▼
OpenAlex API → add_citation() → citations table

research "topic"
    │
    ▼
Plan → Act → Observe → Learn → (repeat)
```

## Database Location

Default: `~/.ai_research_os/papers.db`

Override with:
```bash
export AI_RESEARCH_OS_DB=/path/to/papers.db
```
