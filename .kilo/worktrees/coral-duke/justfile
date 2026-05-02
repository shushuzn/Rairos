# Justfile — developer commands for ai_research_os
# Install just: winget install just | cargo install just | scoop install just

# Default recipe — show help
default:
    @just --list

# Run full test suite (quiet output)
test:
    python -B -m pytest tests/ -q

# Run tests with coverage report
test-cov:
    python -B -m pytest tests/ --cov=ai_research_os --cov-report=term-missing:skip-covered

# Run only Tier 4 unit tests
test-tier4:
    python -B -m pytest tests/test_unit_notes.py -v

# Run specific test file
test FILE:
    python -B -m pytest tests/{{FILE}} -v

# Lint all Python files (check only, no auto-fix)
lint:
    ruff check .

# Auto-fix lint issues
lint-fix:
    ruff check --fix .

# Format code
fmt:
    ruff format .

# Run full lint + format pipeline
check: lint fmt
    @echo "Lint + format OK"

# Install all dependencies
install:
    pip install -e ".[all]"

# Run the CLI
run URL *TAGS:
    python ai_research_os.py {{URL}} {{TAGS}}

# Openai-compatible API test
chat:
    python -c "import ai_research_os as airo; print(airo.__version__ if hasattr(airo, '__version__') else 'ok')"

# Show test count
test-count:
    python -B -m pytest tests/ --collect-only -q

# Run CI pipeline locally (what GitHub Actions does)
ci: lint test-cov
