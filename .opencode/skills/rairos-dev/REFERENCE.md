# Rairos Project Reference

## Project Overview

- **150 crates** (Rust only, no Python)
- **~73k+ Rust lines**
- **122 CLI commands**
- **67 MCP tools** (27 core + 40 LLM-backed)
- **License**: GPL-3.0

## Key Crates

| Crate | Purpose |
|-------|---------|
| rairos-core | DB, FTS5, subscriptions, tags, constants |
| rairos-cli | 122 commands in main.rs |
| rairos-mcp | MCP protocol server (67 tools) |
| rairos-codegraph | CodeGraph MCP server + CLI |
| rairos-llm | GenePool, Evolution, LLM clients |
| rairos-parser | arXiv/CrossRef/Semantic Scholar API |
| rairos-research | DeepResearchAgent, gap detection |
| rairos-kg | Knowledge graph, PageRank |
| rairos-memory | Research stance tracking |
| rairos-rankers | Paper ranking/scoring |

## CLI Commands (122 total)

Sample commands: Add...

Categories:
- **Papers**: search, add, list, parse, stats, recommend
- **Research**: briefing, litreview, gap, hypothesis
- **Evolution**: gene-pool, crossover, decay, watcher
- **Analysis**: trends, impact, rigor, citations
- **Discovery**: claim-graph, contradictions, paradigm
- **Output**: render, chart, timeline, radar

## MCP Tools (67 total)

**Core Rust (27)**: paper_search, paper_ingest, paper_parse_full, paper_query, paper_chat, paper_recommend, replication_check_simple, github_repo_metadata, huggingface_dataset_metadata, pdf_extract_advanced, tag_add, tag_remove, tag_list, trends_detect_trending, trends_predict_next, trends_top_predictions, trends_compare_tags, citation_graph, kg_paper_subgraph, kg_tag_graph, kg_full_graph, kg_query, pdf_download, pdf_extract_text, pdf_extract_structured, cite_fetch, chart_query

**LLM-backed (40)**: briefing_generate, litreview_generate, slides_generate, gap_detect, gap_submit, gap_evolve, citation_chain_build, citation_chain_families, citation_chain_silent, citation_chain_render, impact_score_paper, impact_rank, replication_check, route_query, trust_scorer_compute, paper_compare, paper_analyze, gene_pool_decay, crossover, tag_all, research_memory_add_stance, research_memory_list_stances, research_memory_check_paper, research_memory_anomalies, leaderboard, impact_leaderboard, claim_graph, review_list, experiment_record, litreview_list, review_simulate, gene_pool_watcher, replication_compare, routeplan_list, routeplan_update_step, routeplan_revise, research_run, hypothesis_generate, hypothesis_list, topic_discovery, orchestrator_run_cycle, deep_research_run, parallel_research_run

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
