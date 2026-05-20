---
name: rairos-dev
description: Rairos project development workflow - Rust build, test, debug, and code analysis. Use when working on Rairos codebase (100% Rust, 150 crates). Triggers: cargo build, cargo test, flamegraph, CodeGraph, adding CLI commands, adding MCP tools.
---

# Rairos Development

## Quick Start

```bash
make build-dev   # Debug build (faster for iteration)
make test        # Run tests
./rairos.sh --help  # CLI help
```

## Build Variants

| Command | Use Case |
|---------|----------|
| `make build` | Release build (parallel + mold + ccache) |
| `make build-dev` | Debug build (faster) |
| `unset RUSTC_WRAPPER && cargo build -p rairos-cli` | Single crate |
| `unset RUSTC_WRAPPER && cargo build --release -p rairos-cli` | Release single crate |

## Testing

```bash
make test                    # All tests
unset RUSTC_WRAPPER && cargo test -p rairos-codegraph  # Specific crate
unset RUSTC_WRAPPER && cargo test -p rairos-cli -- --nocapture  # With output
```

## Debugging

```bash
# Debug logs
RUST_LOG=debug ./rairos.sh <command>

# Or directly:
RUST_LOG=debug ./rairos.sh <command>
```

## Code Analysis (CodeGraph MCP)

Use CodeGraph MCP tools for code exploration:

| Tool | Purpose |
|------|---------|
| `codegraph_files` | Project structure (start here) |
| `codegraph_search` | Find symbols by name |
| `codegraph_context` | Build context for a task |
| `codegraph_callers` | Find who calls a function |
| `codegraph_callees` | Find what a function calls |
| `codegraph_impact` | Analyze change impact |

See `CODERIEF.md` for detailed usage.

## Code Quality

```bash
# Lint
cargo clippy -p rairos-cli

# Format
cargo fmt -- --check

# Security audit
cargo audit
```

## Adding New Features

### New CLI Command

1. Add command variant to `Commands` enum in `main.rs`
2. Create handler in `handlers/<feature>.rs`
3. Add match arm in command dispatch

Template:
```rust
Commands::NewCommand { arg } => {
    handle_new_command(arg)?;
}
```

### New MCP Tool

1. Add tool definition in `rairos-mcp/src/handlers.rs`
2. Implement handler function with `fn name(&self) -> &str`
3. Register in tool list

## Common Issues

| Issue | Solution |
|-------|----------|
| Memory | unset RUSTC_WRAPPER && CARGO_BUILD_JOBS=1 cargo build |
| Disk full | `rm -rf target/` |
| Tree-sitter ABI error | Use tree-sitter 0.23, not 0.24 |
| Slow tests | Test single crate: `-p <name>` |

## Project Structure

```
crates/
├── rairos-cli/       # 148 CLI commands
├── rairos-core/      # DB, FTS5, constants
├── rairos-mcp/       # 0 MCP tools
├── rairos-codegraph/ # CodeGraph MCP server
└── ... (140 other crates)
```

See `REFERENCE.md` for full architecture details.
