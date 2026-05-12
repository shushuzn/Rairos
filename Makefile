# Rairos Development Makefile
# Self-Evolving Research OS

.PHONY: help
help: ## Show this help
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | sort | awk 'BEGIN {FS = ":.*?## "}; {printf "  \033[36m%-18s\033[0m %s\n", $$1, $$2}'

# ─── Python ───────────────────────────────────────────────────────────────────

py-deps: ## Install Python dependencies
	uv sync --extra dev

py-lint: ## Run ruff linter
	uvx ruff check . --exclude neuraloperator_fork/

py-fmt: ## Format Python code
	uvx ruff format . --exclude neuraloperator_fork/
	uvx ruff check --fix . --exclude neuraloperator_fork/

py-typecheck: ## Run mypy type checker
	uvx mypy core parsers db llm research_loop cli --ignore-missing-imports

py-test: ## Run Python tests
	uv run pytest tests/ -q --timeout=30

py-test-w: py-test ## Run tests with watch (requires pytest-xdist)
	uv run pytest tests/ -q --timeout=30 -n auto

py-all: py-lint py-typecheck py-test ## Run all Python checks

# ─── Rust ─────────────────────────────────────────────────────────────────────

rust-deps: ## Install Rust dependencies (via cargo)
	cargo fetch

rust-build: ## Build Rust crates (single-threaded to avoid OOM)
	CARGO_BUILD_JOBS=1 cargo build

rust-test: ## Run Rust tests
	CARGO_BUILD_JOBS=1 cargo test

rust-fmt: ## Format Rust code
	cargo fmt

rust-clippy: ## Run clippy linter
	CARGO_BUILD_JOBS=1 cargo clippy -- -D warnings

rust-all: rust-fmt rust-clippy rust-test ## Run all Rust checks

# ─── Combined ─────────────────────────────────────────────────────────────────

dev: py-deps ## Setup development environment
dev-verify: py-all rust-build ## Verify full dev setup

# ─── CI ───────────────────────────────────────────────────────────────────────

ci-python: ## Run CI Python checks (lint + typecheck)
	uv sync --extra dev
	uvx ruff check . --exclude neuraloperator_fork/
	uvx ruff format --check . --exclude neuraloperator_fork/
	uvx mypy core parsers db llm research_loop cli --ignore-missing-imports

ci-rust: ## Run CI Rust checks
	cargo fmt --check
	CARGO_BUILD_JOBS=1 cargo clippy -- -D warnings
	CARGO_BUILD_JOBS=1 cargo test

# ─── Utilities ───────────────────────────────────────────────────────────────

clean: ## Remove build artifacts
	rm -rf target/
	rm -rf .venv/
	rm -rf __pycache__/
	find . -type d -name __pycache__ -exec rm -rf {} + 2>/dev/null || true
	find . -type f -name "*.pyc" -delete

sccache-start: ## Start sccache server
	@if command -v sccache >/dev/null 2>&1; then \
		sccache --start-server 2>/dev/null || echo "sccache server already running"; \
	else \
		echo "sccache not installed"; \
	fi

git-commit: ## Stage and commit (usage: make git-commit MSG="your message")
	@if [ -z "$(MSG)" ]; then \
		echo "Usage: make git-commit MSG='Your commit message'"; \
		exit 1; \
	fi
	git add -A && git commit -m "$(MSG)"
