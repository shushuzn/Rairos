# Rairos Agents

## Project

Self-Evolving Research OS — two codebases coexist:
- **Python** (~90k lines, primary): `core/`, `llm/`, `cli/`, `db/`, `research_loop/`, `parsers/`, `kg/`, `web/`
- **Rust** (~8.7k lines, rewrite): `crates/` with 8 crates

## Critical: Rust Build

Rust compilation is memory-intensive. **Always use**:
```bash
CARGO_BUILD_JOBS=1 cargo build
CARGO_BUILD_JOBS=1 cargo test
```

## sccache (Compilation Cache)

sccache speeds up Rust builds by caching compilation results.

```bash
# Install (or use prebuilt from /tmp/sccache-v0.11.0-x86_64-unknown-linux-musl/sccache)
cp /tmp/sccache-v0.11.0-x86_64-unknown-linux-musl/sccache /usr/local/bin/

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
| rairos-parser | arXiv/CrossRef/Semantic API, PDF extraction |
| rairos-research | DeepResearchAgent, gap detection |
| rairos-web | REST API + HTML frontend |
| rairos-cli | 48 commands |
| rairos-kg | Knowledge graph, PageRank, communities |
| rairos-memory | Research stance tracking, anomaly detection |

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

- Rust: 8 crates, ~8.8k lines, 49 tests, 48 CLI commands
- Python: ~90k lines, 5156 tests

## Web UI

- Dashboard: stats, recent papers, gene pool diversity
- Papers: search, list, filter
- Gene Pool: add gene, list, feedback
- Knowledge Graph: stats, path finding, rankings
- Memory: research stances, anomaly detection
