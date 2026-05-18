# Rairos Project Reference

## Project Overview

- **150 crates** (Rust only, no Python)
- **~73k+ Rust lines**
- **123 CLI commands**
- **87 MCP tools** (47 core + 40 LLM-backed)
- **License**: GPL-3.0

## Key Crates

| Crate | Purpose |
|-------|---------|
| rairos-core | DB, FTS5, subscriptions, tags, constants |
| rairos-cli | 123 commands in main.rs |
| rairos-mcp | MCP protocol server (87 tools) |
| rairos-codegraph | CodeGraph MCP server + CLI |
| rairos-llm | GenePool, Evolution, LLM clients |
| rairos-parser | arXiv/CrossRef/Semantic Scholar API |
| rairos-research | DeepResearchAgent, gap detection |
| rairos-kg | Knowledge graph, PageRank |
| rairos-memory | Research stance tracking |
| rairos-rankers | Paper ranking/scoring |

## CLI Commands (123 total)

Sample: Add...

## MCP Tools (87 total)

**Core Rust (47)**: paper_search, paper_ingest, tag_add, tag_remove, tag_list, trends_detect_trending, paper_recommend, citation_graph, paper_query, paper_chat, kg_paper_subgraph, kg_tag_graph, kg_full_graph, kg_query, pdf_download, pdf_extract_text, pdf_extract_structured, trends_predict_next, trends_top_predictions, trends_compare_tags, cite_fetch, paper_search_multi, paper_lookup_doi, paper_citations, paper_verify_citations, paper_visualize_trends, paper_visualize_radar, paper_critical_analysis, paper_generate_review_pdf, paper_hypothesis_report, paper_generate_schematic, paper_science_discovery, paper_database_lookup, paper_peer_review, paper_format_citation, paper_literature_review, chart_query, paper_parse_full, replication_check_simple, github_repo_metadata, huggingface_dataset_metadata, pdf_extract_advanced, paper_generate, what_if_oracle, paper_docx_export, paper_slides_generate, paper_grant_proposal

**LLM-backed (40)**: briefing_generate, litreview_generate, slides_generate, gap_detect, citation_chain_build, citation_chain_families, citation_chain_silent, citation_chain_render, impact_score_paper, impact_rank, replication_check, paper_compare, paper_analyze, gap_submit, gap_evolve, gene_pool_decay, crossover, leaderboard, impact_leaderboard, claim_graph, hypothesis_generate, topic_discovery, orchestrator_run_cycle, deep_research_run, parallel_research_run, trust_scorer_compute, routeplan_create, tag_all, review_list, hypothesis_list, experiment_record, litreview_list, review_simulate, gene_pool_watcher, replication_compare, routeplan_list, routeplan_update_step, routeplan_revise, research_run

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
