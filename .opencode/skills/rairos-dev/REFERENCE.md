# Rairos Project Reference

## Project Overview

- **150 crates** (Rust only, no Python)
- **~73k+ Rust lines**
- **123 CLI commands**
- **0 MCP tools** (0 core + 0 LLM-backed)
- **License**: GPL-3.0

## Key Crates

| Crate | Purpose |
|-------|---------|
| rairos-core | DB, FTS5, subscriptions, tags, constants |
| rairos-cli | 123 commands in main.rs |
| rairos-mcp | MCP protocol server (0 tools) |
| rairos-codegraph | CodeGraph MCP server + CLI |
| rairos-llm | GenePool, Evolution, LLM clients |
| rairos-parser | arXiv/CrossRef/Semantic Scholar API |
| rairos-research | DeepResearchAgent, gap detection |
| rairos-kg | Knowledge graph, PageRank |
| rairos-memory | Research stance tracking |
| rairos-rankers | Paper ranking/scoring |

## CLI Commands (123 total)

Sample: Add...

## MCP Tools (0 total)

**Core Rust (0)**: 

**LLM-backed (0)**: 

## Data Paths

All data in `~/.ai_research_os/`:

| Path | Content |
|------|---------|
| `papers.db` | SQLite paper database |
| `papers.json` | Papers JSON |
| `evolution/gene_pool.db` | Gene pool SQLite |
| `evolution/gene_pool.jsonl` | Gene pool append log |
| `evolution/events.jsonl` | Evolution events |
| `kg/graph.json` | Knowledge graph |
| `research_memory/` | Research stances |
| `briefings/` | Generated briefings |

## Constants (rairos-core::constants)

Key constants:
- `PAPERS_DB_PATH`: `.ai_research_os/papers.json`
- `GP_DIR_NAME`: `.ai_research_os/evolution`
- `KG_DIR`: `kg`
- `ARXIV_API`: `https://export.arxiv.org/api/query`
- `SEMANTIC_API`: `https://api.semanticscholar.org/graph/v1`
- `LLM_MODEL`: `gpt-4o-mini`

## Known Constraints

| Constraint | Value |
|------------|-------|
| Memory | Build with `CARGO_BUILD_JOBS=1` |
| tree-sitter | Must use 0.23 (not 0.24) |
| Disk space | ~5GB free recommended |
| Database | SQLite with FTS5 |

## Performance Benchmarks

| Operation | Time |
|-----------|------|
| impact_score (1000 papers) | 23 µs |
| impact_rank (1000 papers) | 138 µs |
| citation find_families (100 nodes) | 109 µs |
| citation find_silent (100 nodes) | 49 µs |
| MCP dispatch (cached) | ~10 µs |

## Key Patterns

- `papers` table PK is `id` (NOT arxiv_id)
- Database: SQLite at `rairos.db` or `$RAIROS_DB`
- MCP: `McpServer` with OnceLock cached dispatch
- CLI: `Commands` enum + `handle_*()` functions
