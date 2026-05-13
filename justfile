# Justfile — developer commands for Rairos
# Install just: winget install just | cargo install just | scoop install just

default:
    @just --list

# Run full test suite (quiet, skip neuraloperator ns shadowing)
test:
    python -B -m pytest tests/ -q

# Run tests with coverage
test-cov:
    python -B -m pytest tests/ --cov=. --cov-report=term-missing:skip-covered

# Run only a specific test file
test FILE:
    python -B -m pytest tests/{{FILE}} -v

# Show test count
test-count:
    python -B -m pytest tests/ --collect-only -q

# Lint check only
lint:
    ruff check .

# Lint auto-fix
lint-fix:
    ruff check --fix .

# Format code
fmt:
    ruff format .

# Format check (CI gate)
fmt-check:
    ruff format --check .

# Type check (mypy on core modules)
typecheck:
    python -m mypy core parsers db llm research_loop --ignore-missing-imports

# Full CI pipeline (what GitHub Actions runs)
ci: lint fmt-check typecheck
    @echo "CI checks passed"

# Full CI + tests
ci-full: ci test-cov
    @echo "Full CI + tests passed"

# Install all deps
install:
    pip install -e ".[all]"

# Install pre-commit hooks
hooks:
    pre-commit install

# Run the CLI
run *ARGS:
    python -m cli {{ARGS}}
