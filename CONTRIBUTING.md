# Contributing to Rairos

Thank you for your interest in contributing to Rairos!

## Project Overview

Rairos is a Self-Evolving Research OS built with **100% Rust**:
- 150 crates
- 122 CLI commands
- 67 MCP tools
- GPL-3.0 license

## Development Setup

```bash
# Clone the repository
git clone https://github.com/shushuzn/Rairos.git
cd Rairos

# Build (uses parallel + mold + ccache)
make build

# Or for faster dev builds:
make build-dev

# Test
make test
```

## Making Changes

### Code Style

- Follow Rust standard formatting (`cargo fmt`)
- Run clippy linter (`cargo clippy -- -D warnings`)
- Keep code clear and documented where necessary

### Commit Messages

```
feat(scope): add new feature
fix(scope): fix bug
refactor(scope): code refactoring
test(scope): add or update tests
docs(scope): documentation updates
```

Examples:
```
feat(cli): add paper-search command
fix(mcp): handle missing parameters correctly
refactor(core): extract similarity functions
```

### Building

```bash
# Always use CARGO_BUILD_JOBS=1 to avoid OOM
make build-dev

# Build specific crate
unset RUSTC_WRAPPER && cargo build -p rairos-cli
```

### Testing

```bash
make test

# Test specific crate
unset RUSTC_WRAPPER && cargo test -p rairos-core

# Test with output
unset RUSTC_WRAPPER && cargo test -p rairos-cli -- --nocapture
```

### Memory-Constrained Environments

If you encounter OOM during build, use:

```bash
unset RUSTC_WRAPPER && CARGO_BUILD_JOBS=1 cargo build
```

## Adding New Features

### New CLI Command

1. Add command variant to `Commands` enum in `crates/rairos-cli/src/main.rs`
2. Create handler in `crates/rairos-cli/src/handlers/<feature>.rs`
3. Add match arm in command dispatch

### New MCP Tool

1. Add handler to `crates/rairos-mcp/src/handlers.rs` (core) or `llm_handlers.rs` (LLM-backed)
2. Implement `fn name(&self) -> &str` returning tool name
3. Run `python3 scripts/generate_skills.py` to update documentation

### New Crate

1. Create in `crates/rairos-<name>/`
2. Add to `Cargo.toml` workspace members
3. Update dependencies in relevant crates

## Updating Documentation

After adding CLI commands or MCP tools, regenerate skill files:

```bash
python3 scripts/generate_skills.py
```

This updates:
- `.opencode/skills/rairos-dev/SKILL.md`
- `.opencode/skills/rairos-dev/REFERENCE.md`

### Shell Completions

After adding CLI commands, regenerate shell completions:

```bash
make completions
```

Or manually:
```bash
cargo run -p rairos-cli -- completions bash > completions/bash
cargo run -p rairos-cli -- completions zsh > completions/zsh
cargo run -p rairos-cli -- completions fish > completions/fish
```

## Pull Request Process

1. Fork the repository
2. Create a feature branch (`git checkout -b feat/my-feature`)
3. Make changes with tests
4. Ensure CI passes
5. Submit PR with clear description

### PR Description Template

```markdown
## Summary
Brief description of changes

## Type
- [ ] Bug fix
- [ ] New feature
- [ ] Refactoring
- [ ] Documentation

## Testing
How was this tested?

## Checklist
- [ ] Code follows project style
- [ ] Tests pass
- [ ] Documentation updated (if applicable)
```

## Getting Help

- Open an issue for bugs or feature requests
- Check existing issues before duplicating

## License

By contributing, you agree that your contributions will be licensed under the GPL-3.0 License.
