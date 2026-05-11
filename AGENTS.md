# Rairos Agents

## Project

Self-Evolving Research OS — two codebases coexist:
- **Python** (~90k lines, primary): `core/`, `llm/`, `cli/`, `db/`, `research_loop/`, `parsers/`, `kg/`, `web/`
- **Rust** (~8k lines, rewrite): `crates/` with 8 crates

## Critical: Rust Build

Rust compilation is memory-intensive. **Always use**:
```bash
CARGO_BUILD_JOBS=1 cargo build
CARGO_BUILD_JOBS=1 cargo test
```

## Rust Crates

| Crate | Purpose |
|-------|---------|
| rairos-core | DB, FTS5, subscriptions, tags |
| rairos-llm | GenePool, Evolution, LLM clients |
| rairos-parser | arXiv/CrossRef/Semantic API, PDF extraction |
| rairos-research | DeepResearchAgent, gap detection |
| rairos-web | REST API + HTML frontend |
| rairos-cli | 37 commands (daemon, paper-*, gene-*, etc.) |
| rairos-kg | Knowledge graph, PageRank, communities |
| rairos-memory | Research stance tracking, anomaly detection |

## Rust Commands

```bash
# Build all
CARGO_BUILD_JOBS=1 cargo build

# Build specific crate
CARGO_BUILD_JOBS=1 cargo build -p rairos-cli

# Run CLI
cargo run -p rairos-cli -- paper-list
cargo run -p rairos-cli -- gene-list
cargo run -p rairos-cli -- daemon --foreground

# Run tests (all or specific)
CARGO_BUILD_JOBS=1 cargo test
CARGO_BUILD_JOBS=1 cargo test -p rairos-llm

# Run web server
cargo run -p rairos-cli -- daemon --foreground
```

## Python Commands

```bash
# Install deps
uv sync --all-extras

# Lint
uv run ruff check .
uv run ruff format --check .

# Type check
uv run mypy core parsers db llm research_loop cli

# Tests (fast)
uv run pytest tests/test_workflow.py tests/test_cli_dispatch.py -v --timeout=15

# Full tests (slow)
uv run pytest tests/ -q --tb=short -n auto --timeout=60
```

## Git Push

GitHub push requires token authentication:
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

## Existing Docs

- `CLAUDE.md` — Python project details (original)
- `pyproject.toml` — Python dependencies
- `Cargo.toml` — Rust workspace config
