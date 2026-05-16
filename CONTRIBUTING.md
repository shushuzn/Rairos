# Contributing to AI Research OS

Thank you for your interest in contributing! This project is a self-evolving research operating system for AI researchers, built 100% in Rust (154 crates). Here's how you can help.

## Quick Start

```bash
# Clone the repository
git clone https://github.com/shushuzn/Rairos.git
cd Rairos

# Build the project
CARGO_BUILD_JOBS=1 cargo build --workspace

# Run tests
CARGO_BUILD_JOBS=1 cargo test --workspace
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
cargo build -p <crate-name>    # Build a specific crate
CARGO_BUILD_JOBS=1 cargo test  # Run all tests

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

- Follow Rust 2021 edition conventions
- Run `cargo clippy --workspace -- -D warnings` before committing
- Use `cargo fmt` for formatting
- Add doc comments (`///`) for public APIs and structs
- Keep functions focused and well-named

### Testing

- All new features need tests
- Use `#[cfg(test)]` modules, not a separate `tests/` directory
- Run `CARGO_BUILD_JOBS=1 cargo test --workspace` to verify

### Crate Naming

- All crates prefixed with `rairos-` (e.g., `rairos-core`, `rairos-cli`)
- Library crates: `rairos-<name>` exposes a `<name>`-related functionality
- CLI command crates: placed in `crates/rairos-<command-name>/`

## Project Structure

```
ai_research_os/
├── Cargo.toml            # Workspace root (154 members)
├── crates/
│   ├── rairos-core/      # Core data structures + database
│   ├── rairos-cli/       # CLI entry point (clap derives)
│   ├── rairos-mcp/       # MCP protocol server
│   ├── rairos-llm/       # LLM clients
│   └── ...               # 150 more crates
├── AGENTS.md             # Full crate list + CLI reference
├── docs/                 # Architecture, installation docs
└── .github/workflows/    # CI configuration
```

## Getting Help

- Open a [Discussion](https://github.com/shushuzn/Rairos/discussions) for questions
- File an [Issue](https://github.com/shushuzn/Rairos/issues) for bugs or feature requests
