# Contributing to Rairos

Thank you for your interest in contributing! This project is a self-evolving research operating system for AI researchers. Here's how you can help.

## Quick Start

```bash
# Clone the repository
git clone https://github.com/shushuzn/Rairos.git
cd Rairos

# Install dependencies (requires Python 3.10+)
uv sync --all-extras

# Run tests
uv run pytest tests/ -q --tb=short -n auto --timeout=60
```

## Development Workflow

### 1. Pick an Issue

- Look for issues labeled [good first issue](https://github.com/shushuzn/Rairos/labels/good%20first%20issue) or [help wanted](https://github.com/shushuzn/Rairos/labels/help%20wanted)
- Comment on the issue to let others know you're working on it
- Fork the repo and create a feature branch

### 2. Branch Naming

```
feature/description      # New features
fix/description          # Bug fixes
docs/description         # Documentation only
test/description         # Test coverage only
refactor/description     # Code refactoring
```

### 3. Making Changes

```bash
# Create your branch
git checkout -b feature/my-feature

# Make your changes, then:
just lint      # Check code style
just test-cov  # Run tests with coverage
just check     # Full lint + format check

# Commit (use conventional commits)
git commit -m "feat(parser): add arXiv ID normalization"
```

### 4. Submit a Pull Request

- Fill out the PR template (title, summary, changes, motivation, verification)
- Link the issue: `Closes #123`
- Ensure CI passes
- Update CHANGELOG.md for user-facing changes

## Code Standards

### Python Style
- Follow PEP 8 (enforced by ruff)
- Add type annotations for new functions
- Keep functions under 100 lines
- Write docstrings for public APIs

### Testing
- All new features need tests
- Aim for meaningful assertions, not just `assert True`
- Use `@pytest.mark.no_freeze` for tests needing real time
- Run `just test FILE` to test specific files

### File Structure
```
Rairos/
├── core/           # Core utilities (retry, rate_limiter, profiler)
├── db/             # Database layer
├── llm/            # LLM integrations and prompts
├── parsers/        # Paper parsers (arXiv, DOI, PDF)
├── pdf/            # PDF extraction and OCR
├── research_loop/  # Autonomous research pipelines
└── tests/          # Test suites (Tier 1-4 by scope)
```

## Labels

| Label | Description |
|-------|-------------|
| `bug` | Something isn't working |
| `enhancement` | New feature or improvement |
| `good first issue` | Good for newcomers |
| `help wanted` | Extra attention needed |
| `docs` | Documentation improvements |
| `test` | Test coverage improvements |
| `refactor` | Code refactoring |
| `research` | Research-related changes |

## Commit Format

```
<type>(<scope>): <subject>

<body>

<footer>
```

Types: `feat`, `fix`, `docs`, `style`, `refactor`, `test`, `chore`

Example:
```
feat(cli): add semantic search command

Add `airos similar` command for finding semantically similar papers
using embedding vectors from Ollama.

Closes #123
```

## Questions?

- Open an issue for bugs or feature requests
- Check the [documentation](https://shushuzn.github.io/Rairos/)
- Review existing PRs for patterns

## License

By contributing, you agree that your contributions will be licensed under the GNU General Public License v3.0.
