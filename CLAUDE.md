# Claude Code Configuration - Rairos

## Behavioral Rules (Always Enforced)

- Do what has been asked; nothing more, nothing less
- NEVER create files unless they're absolutely necessary
- ALWAYS prefer editing an existing file to creating a new one
- NEVER proactively create documentation files (*.md) unless explicitly requested
- NEVER save working files, text/mds, or tests to the root folder
- ALWAYS read a file before editing it
- NEVER commit secrets, credentials, or .env files

## Project: Rairos (ai-research-os)

**Self-Evolving Research OS** — manages papers, detects research gaps, generates insights.

- **Python**: >=3.10, tested on 3.11/3.12/3.13
- **CLI**: 77 commands via `rairos <cmd>`
- **Entry**: `cli:main` (pyproject.toml scripts.rairos)
- **Test**: 5079 tests, pytest with timeout=60s
- **Linter**: ruff (E4/E7/F/W/B/I), mypy (strict=false)
- **CI gate**: 40% coverage

## Build & Test

```bash
# Install dev dependencies
uv sync --all-extras

# Lint
uv run ruff check .
uv run ruff format --check .

# Type check (CI scope: core parsers db llm cli)
uv run mypy core parsers db llm cli scripts notifications

# Tests (fast subset)
uv run pytest tests/test_workflow.py tests/test_cli_dispatch.py -v --timeout=15

# Full test suite (slow — uses pytest-split 4-way sharding)
uv run pytest tests/ -q --tb=short -n auto --timeout=60
```

## Architecture

- **core/**: Utilities — rate_limiter, retry, cache, notifications, observability, profiler
- **db/**: SQLite via database.py (2282 lines, primary key is `id`)
- **llm/**: LLM clients, citation chains, gap detection, evolution, briefings
- **parsers/**: arxiv, cross_search
- **research_loop/**: Deep research, orchestrator, benchmark_runner, claim_graph, paper_parser
- **cli/**: 77 commands in `cli/cmd/`, dispatch registry in `cli/_registry.py`
- **kg/**: Knowledge graph manager
- **web/**: FastAPI routes (web/routes_*.py)

## Key Patterns

- `db.database.Database` is the main DB interface
- `Paper` dataclass: `id`, `arxiv_id`, `title`, `authors`, `published`, `abstract`, `categories`
- `papers` table PK is `id` (NOT arxiv_id)
- MCP tools registered in `mcp/tools_defs.py`
- CLI dispatch: `cli/_registry.py` `_SUBCOMMAND_TABLE` + `_run_<cmd>` in `cli/__init__.py`

## Windows / MSYS2 Notes

- Git push: `GIT_ASKPASS=echo timeout 55 git push` (works 15-55s)
- Python 3.13.12, UV managed
- CRLF: patch tool handles it; mypy CRLF bug on Windows is filtered in pyproject.toml

## GitHub Push

```bash
GIT_ASKPASS=echo timeout 55 git push
```

## Known Issues

- `test_deep_research.py` ignored in pytest (pytest.ini_options addopts)
- `core/basics.py` 95% covered (2 missing lines)
- `db/database.py` low coverage (2282-line file with many branches)
- `mypy 1.20.1 CRLF line-number bug` — filtered by `disable_error_code = ["assignment", "union-attr"]`
- Bandit B608/B310/B311 false positives — acknowledged in pyproject.toml
