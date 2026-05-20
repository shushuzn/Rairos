# Contributing to Rairos

Thank you for your interest in contributing! Rairos is a self-evolving research operating system built in Rust. Here's how you can help.

## Quick Start

```bash
# Clone the repository
git clone https://github.com/shushuzn/Rairos.git
cd Rairos

# Build (memory-intensive — use single job)
make build-dev

# Run tests
make test
```

## Prerequisites

- **Rust toolchain** — install via `rustup` (latest stable)
- **SQLite** — `libsqlite3-dev` (Linux) or bundled via `rusqlite`

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
cargo fmt            # Format code
cargo clippy         # Lint checks
make build-dev  # Compile

# Commit (use conventional commits)
git commit -m "feat(parser): add arXiv ID normalization"
```

### 4. Submit a Pull Request

- Fill out the PR template (title, summary, changes, motivation, verification)
- Link the issue: `Closes #123`
- Ensure CI passes
- Update CHANGELOG.md for user-facing changes

## Code Standards

### Rust Style
- Follow Rust 2024 edition idioms
- Run `cargo fmt` before committing
- Run `cargo clippy` — aim for zero warnings
- Add doc comments (`///`) for public APIs
- Keep functions focused and well-named

### Testing
- All new features need tests
- Use `#[cfg(test)] mod tests { ... }` pattern
- Run `make test -p CRATE_NAME` for fast iteration
- Use `cargo test --doc` for doc tests

### Crate Structure

```
crates/
├── rairos-core/         # Database layer (SQLite, FTS5, migrations)
├── rairos-cli/          # CLI (105 commands, clap-based)
├── rairos-llm/          # LLM integrations (GenePool, evolution)
├── rairos-parser/       # Paper parsers (arXiv, DOI, PDF)
├── rairos-pdf/          # PDF extraction (lopdf)
├── rairos-research/     # Deep research agent, gap detection
├── rairos-citations/    # Citation graph (OpenAlex)
├── rairos-rankers/      # Paper impact scoring
├── rairos-kg/           # Knowledge graph (PageRank, communities)
├── rairos-mcp/          # MCP protocol server (68 tools)
├── rairos-memory/       # Research stance tracking, anomalies
└── ... (154 crates total)
```

## Build Tips

- **For memory issues: unset RUSTC_WRAPPER && CARGO_BUILD_JOBS=1 cargo build
- To build a specific crate: `cargo build -p rairos-core`
- For incremental compilation: `make build-dev`
- Install [sccache](https://github.com/mozilla/sccache) for faster rebuilds:

```bash
# On Linux/macOS
sccache --start-server
make build-dev

# Check stats
sccache --show-stats
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

Add `./rairos.sh similar` command for finding semantically similar papers
using embedding vectors from Ollama.

Closes #123
```

## Questions?

- Open an issue for bugs or feature requests
- Check the [documentation](https://shushuzn.github.io/Rairos/)
- Review existing PRs for patterns

## License

By contributing, you agree that your contributions will be licensed under the GNU General Public License v3.0.
