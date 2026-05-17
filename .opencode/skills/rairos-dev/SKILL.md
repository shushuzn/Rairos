---
name: rairos-dev
description: Rairos project development workflow - Rust build, test, debug, and code analysis. Use when working on Rairos codebase (100% Rust, 150 crates). Triggers: cargo build, cargo test, flamegraph, CodeGraph, adding CLI commands, adding MCP tools.
---

# Rairos Development

## Quick Start

```bash
# Build (ALWAYS use CARGO_BUILD_JOBS=1 due to memory limits)
CARGO_BUILD_JOBS=1 cargo build

# Test
CARGO_BUILD_JOBS=1 cargo test

# Run CLI
cargo run -p rairos-cli -- --help
```

## Build Variants

| Command | Use Case |
|---------|----------|
| `CARGO_BUILD_JOBS=1 cargo build` | Default dev build |
| `cargo build --release -p rairos-cli` | Release build |
| `sccache --start-server && CARGO_BUILD_JOBS=1 cargo build` | With compile cache |
| `cargo build -p <crate-name>` | Single crate |

## Testing

```bash
# Test specific crate
CARGO_BUILD_JOBS=1 cargo test -p rairos-codegraph

# Test with output
CARGO_BUILD_JOBS=1 cargo test -p rairos-cli -- --nocapture

# All tests (slow)
CARGO_BUILD_JOBS=1 cargo test
```

## Debugging

```bash
# CPU flamegraph (requires: cargo install flamegraph)
CARGO_BUILD_JOBS=1 cargo flamegraph --bin rairos-cli -- <command>

# Perf stat (requires sudo)
sudo perf stat -e cycles cargo run -p rairos-cli -- <command>

# Debug logs
RUST_LOG=debug cargo run -p rairos-cli -- <command>
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
| OOM during build | Use `CARGO_BUILD_JOBS=1` |
| Disk full | `rm -rf target/` |
| Tree-sitter ABI error | Use tree-sitter 0.23, not 0.24 |
| Slow tests | Test single crate: `-p <name>` |

## Project Structure

```
crates/
├── rairos-cli/       # 123 CLI commands
├── rairos-core/      # DB, FTS5, constants
├── rairos-mcp/       # 67 MCP tools
├── rairos-codegraph/ # CodeGraph MCP server
└── ... (140 other crates)
```

See `REFERENCE.md` for full architecture details.
