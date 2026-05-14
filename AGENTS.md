# Rairos Agents

## Superpowers

Superpowers (obra/superpowers) local config at `Rairos/.opencode/opencode.json` (gitignored).
Plugin: `/root/superpowers/.opencode/plugins/superpowers.js`
Skills: `/root/superpowers/skills/` — 15 skills auto-registered on OpenCode startup.

## Project

Self-Evolving Research OS — two codebases coexist:
- **Python** (~90k lines, primary): `core/`, `llm/`, `cli/`, `db/`, `research_loop/`, `parsers/`, `kg/`, `web/`
- **Rust** (~50k+ lines, rewrite): `crates/` with **120 crates**

## Critical: Rust Build

Rust compilation is memory-intensive. **Always use**:
```bash
CARGO_BUILD_JOBS=1 cargo build
CARGO_BUILD_JOBS=1 cargo test
```

## sccache (Compilation Cache)

sccache speeds up Rust builds by caching compilation results.

```bash
# Download prebuilt sccache binary (avoid compiling from source)
SCCACHE_VERSION="v0.11.0"
curl -sL "https://github.com/mozilla/sccache/releases/download/${SCCACHE_VERSION}/sccache-${SCCACHE_VERSION}-x86_64-unknown-linux-musl.tar.gz" \
  | tar xz -C /tmp
cp "/tmp/sccache-${SCCACHE_VERSION}-x86_64-unknown-linux-musl/sccache" /usr/local/bin/

# Start server
sccache --start-server

# Build (sccache is configured in ~/.cargo/config.toml)
CARGO_BUILD_JOBS=1 cargo build

# Check cache stats
sccache --show-stats
```

## Rust Crates

| Crate | Purpose |
|-------|---------|
| rairos-core | DB, FTS5, subscriptions, tags |
| rairos-llm | GenePool, Evolution, LLM clients |
| rairos-parser | arXiv/CrossRef/Semantic Scholar API, PDF extraction |
| rairos-research | DeepResearchAgent, gap detection |
| rairos-web | REST API + HTML frontend |
| rairos-cli | 48 commands |
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
| rairos-mcp | MCP protocol server (25 Rust tools, JSON-RPC 2.0) |

## Rust Commands

```bash
# Build all
CARGO_BUILD_JOBS=1 cargo build

# Run CLI (48 commands)
cargo run -p rairos-cli -- help
cargo run -p rairos-cli -- paper-list
cargo run -p rairos-cli -- gene-list
cargo run -p rairos-cli -- stance-list
cargo run -p rairos-cli -- cite-stats
cargo run -p rairos-cli -- kg-stats
cargo run -p rairos-cli -- daemon --foreground

# Run tests
CARGO_BUILD_JOBS=1 cargo test
CARGO_BUILD_JOBS=1 cargo test -p rairos-llm
```

## Python Commands

```bash
uv sync --all-extras
uv run ruff check .
uv run pytest tests/test_workflow.py -v --timeout=15
```

## Git Push

```bash
GIT_ASKPASS=echo timeout 55 git push
```

## Persistence Paths (Rust)

- GenePool: `~/.ai_research_os/evolution/gene_pool.jsonl`
- Knowledge Graph: `~/.ai_research_os/kg/graph.json`
- Research Memory: `~/.ai_research_os/research_memory/`

## Key Patterns

- `papers` table PK is `id` (NOT arxiv_id)
- `Paper` struct: `id`, `arxiv_id`, `title`, `authors`, `abstract`, `categories`
- Database: SQLite at `rairos.db` (CLI default) or `$RAIROS_DB`

## Stats

- Rust: **153 crates**, ~55k+ lines, 47 CLI commands, 207+ test files
- Python: ~90k lines, 5079 tests collected

## Web UI

- Dashboard: stats, recent papers, gene pool diversity
- Papers: search, list, filter
- Gene Pool: add gene, list, feedback
- Knowledge Graph: stats, path finding, rankings
- Memory: research stances, anomaly detection

## Rust MCP Tools (25 Rust + 39 Python fallback)

| Source | Count | Tools |
|--------|-------|-------|
| Original Rust | 14 | paper_search/ingest/query/chat/recommend, tag_add/remove/list, trends_detect_trending, citation_graph, kg_paper_subgraph/kg_tag_graph/kg_full_graph/kg_query |
| Mapped Rust (Phase 4) | 11 | briefing_generate, litreview_generate, slides_generate, gap_detect, citation_chain_build/families/silent/render, impact_score_paper/rank, replication_check |
| Python fallback | 39 | pdf_download/extract_*/extract_structured, trends_predict_next/top_predictions/compare_tags, chart_query, research_run, cite_fetch, paper_analyze, paper2code_run, gap_submit/evolve, research_agent_*, hypothesis_generate/list, experiment_record, litreview_list, research_memory_*, review_*, routeplan_*, impact_leaderboard, replication_compare, tag_all, crossover, leaderboard, gene_pool_decay/watcher, claim_graph |

### Performance Benchmarks

| Benchmark | Result |
|-----------|--------|
| cargo bench impact_score (1000 papers) | 23 µs (23 ns/paper) |
| cargo bench impact_rank (1000 papers) | 138 µs |
| cargo bench citation find_families (100 nodes) | 109 µs |
| cargo bench citation find_silent (100 nodes) | 49 µs |
| MCP dispatch (OnceLock cached) | ~10 µs baseline |
| MCP dispatch (before caching) | ~130 µs |

### Test Structure

`tests/test_mcp_handler.py`: 27 tests, 5 classes
- `TestProtocolHandlers` (7): initialize, list_tools, request routing
- `TestToolRouting` (4): every routed tool has impl/schema, unknown tool returns error
- `TestMcpJsonRpc` (5): JSON-RPC round-trip, serialization
- `TestRustToolIntegration` (10): 11 Rust tools return expected keys, ranking ordering, graceful LLM-key handling
- `TestPythonFallback` (1): Python-only tools still work via fallback

### Key Decisions

- `OnceLock` caches `McpServer` + tokio `Runtime` — avoids rebuilding both on every call (17-37x speedup)
- Rust-first dispatch: `handle_call_tool` tries `call_tool_rs()` first, falls back to Python on `None`
- Backward-compatible params: Rust handlers accept both `paper_id` and `arxiv_id`
- Schema validation in Python runs before Rust dispatch, so Rust handler params must match tools_defs.py schemas

### Remaining High-ROI Modules for Rust Porting

| Module | Lines | Why |
|--------|-------|-----|
| `semantic_router.py` | 499 | Per-call routing, self-contained logic |
| `route_planner.py` | 578 | LLM call chain planning |
| `trust_scorer.py` | 211 | Pure computation, no deps |

Note: `postprocess.py` (561 lines) is a pipeline orchestrator calling 7+ other modules — NOT suitable for standalone port.
