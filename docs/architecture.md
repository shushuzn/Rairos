# Architecture

## Overview

```
CLI (cli.py)
    │
    ├── parsers/        # arXiv, DOI, OpenAlex API
    ├── database.py     # SQLite + FTS5
    ├── embed.py        # Ollama embedding client
    └── citation.py     # OpenAlex citation API
```

## Modules

### `cli.py`
Main entry point. 13 subcommands registered in two parser flows:
- **Legacy flow**: `main()` dispatches on `sys.argv[0]` (used by direct invocation)
- **Subcommand flow**: `main(subcommand, args)` (used by `python -m ai_research_os`)

### `database.py`
SQLite wrapper with FTS5 full-text search.

Schema:
- `papers` — one row per unique paper (id, title, authors, abstract, published, source, category, parse_status, etc.)
- `papers_fts` — FTS5 virtual table for full-text search
- `papers_embeddings` — paper_id → 768-dim embedding vector (for semantic dedup)
- `citations` — (source_id, target_id) citation edges

Key methods:
- `upsert_paper(paper_id, source, ...)` — insert or update
- `search_papers(query, category, ...)` — FTS5 with BM25 ranking
- `set_embedding(paper_id, embedding)` / `find_similar(paper_id, threshold, limit)` — semantic dedup
- `add_citation(source_id, target_id)` / `get_citations(paper_id, direction)` — citation graph

### `embed.py`
Ollama embedding client. Uses `http://localhost:11434/api/embeddings` with `nomic-embed-text` model. Returns 768-dim vectors.

### `citation.py`
OpenAlex API client. Fetches citation data via:
- `https://api.openalex.org/works?filter=doi:{doi}` — resolve arXiv ID → OpenAlex work ID
- `https://api.openalex.org/works/{openalex_id}/references` — backward citations (cited works)
- `https://api.openalex.org/works?filter=cites:{openalex_id}` — forward citations (citing works)

Bypasses Windows proxy SSL issues via custom SSL context.

## Data Flow

```
import PAPER_ID
    │
    ▼
parse_arXiv_or_DOI(paper_id)
    │
    ▼
upsert_paper() → papers table
    │
    ▼
index_paper() → papers_fts (FTS5)

search QUERY
    │
    ▼
FTS5 BM25(query) → ranked results

dedup-semantic --generate
    │
    ▼
embed.py → Ollama /api/embeddings → papers_embeddings

cite-fetch PAPER_ID
    │
    ▼
citation.py → OpenAlex API → add_citation() → citations table
```

## Database Location

Default: `~/.ai_research_os/papers.db`

Override with `AI_RESEARCH_OS_DB` environment variable.
