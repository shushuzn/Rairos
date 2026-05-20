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

**Self-Evolving Research OS (100% Rust, 149 crates, ~73k lines, 104 CLI commands, 68 MCP tools)**

- **Rust CLI**: 104 commands via `cargo run -p rairos-cli -- <cmd>`
- **Rust CLI main.rs**: `crates/rairos-cli/src/main.rs` (955 lines, Commands enum + handle_*())
- **Rust MCP**: 68 pure-Rust tools in `crates/rairos-mcp/src/` — zero Python fallback
- **Python**: Fully migrated to Rust. All Python source removed.
- **Test**: `CARGO_BUILD_JOBS=1 cargo test --workspace`
- **Linter**: `cargo clippy --workspace -- -D warnings`
- **CI gate**: `cargo build + cargo test + clippy` ( `.github/workflows/rust.yml`)

## Build & Test

```bash
# Use Makefile (auto-detects optimal settings)
make build          # Release build with optimizations
make build-dev      # Debug build (faster)
make test           # Run tests
make clippy         # Run linter

# Or use rairos.sh directly
./rairos.sh --help

# Manual (for CI/memory-constrained environments)
unset RUSTC_WRAPPER && cargo build --release -p rairos-cli
```

## Architecture

**CLI**: `crates/rairos-cli/src/main.rs` (`Commands` enum at line 80 + `handle_*()` functions).
**MCP**: `crates/rairos-mcp/src/` (68 pure-Rust `ToolHandler` implementations, no Python fallback).

## Key Patterns

- `papers` table PK is `id` (NOT arxiv_id)
- `Paper` struct: `id`, `arxiv_id`, `title`, `authors`, `published`, `abstract`, `categories`
- MCP dispatch: `McpServer` (OnceLock cached) → pure-Rust `ToolHandler` dispatch
- CLI dispatch: `Commands` enum at line 80 of `crates/rairos-cli/src/main.rs`

## GitHub Push

```bash
git push
```

## Known Issues

- MCP server runs in OnceLock-cached tokio runtime; 2+ concurrent `call_tool` invocations compete for the lock
