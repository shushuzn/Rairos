# Architecture

## Overview

```
CLI (rairos-cli) → crates/* (154 crates)
  ├── rairos-core        # Core data structures + database (SQLite, FTS5)
  ├── rairos-cli         # 105 CLI commands
  ├── rairos-mcp         # MCP protocol server (69 tools)
  ├── rairos-codegraph   # Code knowledge graph with MCP tools
  ├── rairos-llm         # LLM clients, gene pool, evolution
  ├── rairos-parser      # arXiv/CrossRef/Semantic Scholar API
  ├── rairos-research    # Deep research agent, gap detection
  ├── rairos-web         # REST API + HTML frontend
  ├── rairos-kg          # Knowledge graph, PageRank, communities
  └── ... (143 more crates)
```

AI Research OS is a **local-first** research tool. No cloud dependency — all data stays in `~/.ai_research_os/`.

## Key Modules

### ### CLI Entry Point

`rairos-cli` — 105 CLI commands registered via `clap` derives. All commands dispatch to pure Rust handlers:

```bash
./rairos.sh <command> [options]
```

### CodeGraph

`rairos-codegraph` — Pre-indexed code knowledge graph for Claude Code. Provides fast code exploration via MCP tools (codegraph_search, codegraph_context, codegraph_callers, etc.) without expensive file scanning.

```bash
./rairos.sh <command> [options]
```

### Database Layer

`rairos-core` — SQLite with FTS5 full-text search. Configurable via `RAIROS_DB` env var or `--db` CLI flag.

### LLM Integration

`rairos-llm` — Multi-provider LLM client supporting OpenAI, Anthropic, Ollama, etc. Used for gap detection, Gene Pool evolution, and research agent tasks.

### MCP Protocol

`rairos-mcp` — 69 pure-Rust tools exposing all Rairos functionality via JSON-RPC 2.0 MCP.

## Data Flow

```
arXiv API / DOI / PDF → rairos-parser → rairos-core (DB)
  → rairos-llm (analysis, gap detection)
    → rairos-kg (knowledge graph)
      → rairos-research (deep research cycles)
        → rairos-mcp / rairos-cli (user-facing)
```

## Persistence

All data persists in `~/.ai_research_os/`:

| Path | Contents |
|------|----------|
| `~/.ai_research_os/rairos.db` | Main database (papers, gaps, experiments) |
| `~/.ai_research_os/kg/graph.json` | Knowledge graph |
| `~/.ai_research_os/evolution/gene_pool.db` | Gene Pool (evolution state) |
| `~/.ai_research_os/research_memory/` | Research memory (stances, anomalies) |
