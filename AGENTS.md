# Rairos Agents

## Project

Self-Evolving Research OS — **100% Rust**: 154 crates, ~73k+ Rust lines, 68 MCP tools, 104 CLI commands.
Python CLI fully migrated to Rust. All Python source code removed.

## Rust Build

Memory-intensive. **Always use**: `CARGO_BUILD_JOBS=1 cargo build`

```bash
CARGO_BUILD_JOBS=1 cargo build
CARGO_BUILD_JOBS=1 cargo test
```

### sccache (if available)

```bash
sccache --start-server
CARGO_BUILD_JOBS=1 cargo build
```

### Key Dependencies

| Dependency | Version | Notes |
|------------|---------|-------|
| ratatui | 0.30 | Terminal UI framework (updated from 0.28 for lru security fix) |
| indicatif | 0.18 | Progress bars |
| tokio | 1.x | Async runtime |

## Key Crates (22 of 154)

| Crate | Purpose |
|-------|---------|
| rairos-core | DB, FTS5, subscriptions, tags, experiment_tables, db_optimize |
| rairos-llm | GenePool, Evolution, LLM clients |
| rairos-parser | arXiv/CrossRef/Semantic Scholar API, PDF extraction |
| rairos-research | DeepResearchAgent, gap detection |
| rairos-web | REST API + HTML frontend |
| rairos-cli | 104 commands (2721 lines, main.rs) |
| rairos-kg | Knowledge graph, PageRank, communities |
| rairos-memory | Research stance tracking, anomaly detection |
| rairos-rankers | Paper ranking and scoring |
| rairos-notify | Desktop/system notifications |
| rairos-updaters | Paper metadata updaters |
| rairos-notes | Note-taking with frontmatter |
| rairos-pdf | Advanced PDF processing |
| rairos-viz | Chart and visualization generation |
| rairos-trends | Research trend analysis |
| rairos-render | Lite review, paper rendering |
| rairos-mcp | MCP protocol server (68 Rust tools, JSON-RPC 2.0) |
| rairos-insight-types | Insight types (actions, profiles, events, trust) |
| rairos-insight-credibility | Credibility scoring & trendslop detection |
| rairos-insight-storage | Capsule storage — gene_pool.db (SQLite) |
| rairos-insight-tracker | EvolutionTracker — event recording, profile management |
| rairos-insight-evolution | Evolution engine — audit, propose, evaluate, apply |

## CLI (104 commands)

```bash
CARGO_BUILD_JOBS=1 cargo build
cargo run -p rairos-cli -- --help
cargo run -p rairos-cli -- daemon --foreground
CARGO_BUILD_JOBS=1 cargo test
```

## MCP Tools (68 Rust, zero Python)

| Source | Count | Tools |
|--------|-------|-------|
| Core Rust | 25 | paper_search/ingest/parse_full/query/chat/recommend, replication_check_simple, pdf_extract_advanced, tag_add/remove/list, trends_detect_trending/predict_next/top_predictions/compare_tags, citation_graph, kg_paper_subgraph/kg_tag_graph/kg_full_graph/kg_query, pdf_download/extract_text/extract_structured, cite_fetch, chart_query |
| LLM-backed | 43 | briefing_generate, litreview_generate, slides_generate, gap_detect/submit/evolve, citation_chain_build/families/silent/render, impact_score_paper/rank, replication_check, route_query, trust_scorer_compute, paper_compare/analyze, gene_pool_decay, crossover, tag_all, research_memory_add_stance/list_stances/check_paper/anomalies, leaderboard/impact_leaderboard, claim_graph, review_list, experiment_record, litreview_list, review_simulate, gene_pool_watcher, replication_compare, routeplan_list/update_step/revise, research_run, hypothesis_generate/list, topic_discovery, orchestrator_run_cycle, deep_research_run, parallel_research_run |

## Persistence Paths

- GenePool: `~/.ai_research_os/evolution/gene_pool.db` (SQLite, WAL mode) + `gene_pool.jsonl` (append log)
- Knowledge Graph: `~/.ai_research_os/kg/graph.json`
- Research Memory: `~/.ai_research_os/research_memory/`
- Evolution events: `~/.ai_research_os/evolution/events.jsonl`

## Shared Utilities

### Similarity Functions (rairos-core)

These functions are centralized in `rairos-core` for reuse across crates:

| Function | Signature | Purpose |
|----------|-----------|---------|
| `cosine_similarity` | `fn(a: &[f32], b: &[f32]) -> f32` | Vector similarity |
| `jaccard_similarity` | `fn(a: &[String], b: &[String]) -> f64` | Set overlap |

**Used by:** `rairos-rankers`, `rairos-rankers-score`, `rairos-vault`, `rairos-credibility`, `rairos-bold-vault`, `rairos-credibility-scorer`

### Constants (rairos-core)

Constants centralized in `rairos-core::constants` for reuse:

| Constant | Value | Purpose |
|----------|-------|---------|
| `CAPSULE_PATH` | `.ai_research_os/gene_pool/capsules.json` | Gene Pool capsule path |
| `CAPSULES_JSON` | `capsules.json` | Capsule filename |
| `GP_DIR_NAME` | `.ai_research_os/evolution` | Evolution directory |
| `GENE_POOL_JSONL` | `gene_pool.jsonl` | Gene pool log file |
| `PAPERS_DB_PATH` | `.ai_research_os/papers.json` | Papers database path |
| `AIROS_DIR_NAME` | `.ai_research_os` | Root data directory |
| `KG_DIR` | `kg` | Knowledge graph directory |
| `TAGS_FILE` | `tags.jsonl` | Tags file |
| `BRIEFINGS_DIR` | `briefings` | Briefings directory |
| `CLIMATE_CATS` | `&["cs.AI", "cs.LG", "cs.ET", "physics.ao-ph", "atm.ph"]` | Climate-related categories |
| `ARXIV_API` | `https://export.arxiv.org/api/query` | arXiv API endpoint |
| `SEMANTIC_API` | `https://api.semanticscholar.org/graph/v1` | Semantic Scholar API |
| `CROSSREF_WORKS` | `https://api.crossref.org/works/{doi}` | CrossRef API |
| `DOI_RESOLVER` | `https://doi.org/` | DOI resolution URL |
| `LLM_BASE_URL` | `https://api.openai.com/v1` | OpenAI API base |
| `LLM_MODEL` | `gpt-4o-mini` | Default LLM model |
| `OLLAMA_BASE_URL` | `http://localhost:11434` | Ollama API endpoint |
| `OLLAMA_EMBEDDING_MODEL` | `nomic-embed-text` | Ollama embedding model |
| `RADAR_FILE` | `Radar.md` | Radar filename |
| `TIMELINE_FILE` | `Timeline.md` | Timeline filename |

**Used by:** `rairos-bold-vault`, `rairos-vault`, `rairos-game-mode`, `rairos-gene-pool-watcher`, `rairos-gene-pool-io`, `rairos-climate`, `rairos-climate-ai-monitor`, `rairos-mcp`, `rairos-research`, `rairos-parser`, `rairos-parsers`, `rairos-llm`, `rairos-updaters`, `rairos-chat`, `rairos-labor-displacement-tracker`, `rairos-eval-gap-monitor`, `rairos-credibility`, `rairos-kg`

## Key Patterns

- `papers` table PK is `id` (NOT arxiv_id)
- `Paper` struct: `id`, `arxiv_id`, `title`, `authors`, `abstract`, `categories`
- Database: SQLite at `rairos.db` (CLI default) or `$RAIROS_DB`

## Performance Benchmarks

| Benchmark | Result |
|-----------|--------|
| cargo bench impact_score (1000 papers) | 23 µs (23 ns/paper) |
| cargo bench impact_rank (1000 papers) | 138 µs |
| cargo bench citation find_families (100 nodes) | 109 µs |
| cargo bench citation find_silent (100 nodes) | 49 µs |
| MCP dispatch (OnceLock cached) | ~10 µs baseline |

## Rust Dispatch Architecture

- MCP: `McpServer` (OnceLock cached) dispatches to pure Rust trait handlers — **no Python fallback**
- CLI: `Commands` enum + `handle_*()` functions in `crates/rairos-cli/src/main.rs`
- Backward-compatible params: Rust handlers accept both `paper_id` and `arxiv_id`

## Security

- Run `cargo audit` to check for vulnerabilities in dependencies
- **Known fixed:** RUSTSEC-2026-0002 (lru crate) — fixed via ratatui 0.30
- **Note:** urllib3 alerts (#3, #2) reference non-existent `uv.lock` — dismiss as inaccurate
