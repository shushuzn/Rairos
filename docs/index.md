# AI Research OS — CLI Reference

**Local-first paper management CLI with SQLite FTS5 search, semantic deduplication, and citation graph.**

## What It Does

Manage a local database of AI/ML papers. Import from arXiv/DOI, search with full-text search, find near-duplicates with semantic embeddings, and build a citation graph.

## Quick Start

```bash
pip install -e .
python -m ai_research_os init          # create ~/.ai_research_os/papers.db
python -m ai_research_os import 2601.00155
python -m ai_research_os list
python -m ai_research_os search "attention mechanism"
python -m ai_research_os status
```

## CLI Subcommands

| Command | Description |
|---------|-------------|
| `import [IDS...]` | Import papers by arXiv ID or DOI |
| `list` | List papers in database |
| `search [QUERY]` | Full-text search with FTS5 BM25 ranking |
| `stats` | Show database statistics |
| `export` | Export papers as BibTeX or JSON |
| `queue` | Show or clear pending papers |
| `merge` | Merge duplicate papers |
| `dedup-semantic` | Find near-duplicates via Ollama embeddings |
| `cite-fetch` | Fetch citation data from OpenAlex |
| `cite-import` | Bulk import citation edges from JSON |
| `cite-stats` | Show citation statistics |

## Project Status

- **Tests**: 795 passing, 1 skipped
- **Version**: 1.3.0
- **Python**: 3.9+

## Links

- [Installation](installation.md)
- [Usage Reference](usage.md)
- [Changelog](../CHANGELOG.md)
- [GitHub](https://github.com/shushuzn/ai_research_os)
