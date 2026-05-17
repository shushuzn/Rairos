#!/usr/bin/env python3
"""
Generate Rairos skill files from source code.
Single source of truth - run on demand or via pre-commit hook.

Usage:
    python3 scripts/generate_skills.py
    python3 scripts/generate_skills.py --watch  # watch mode
"""

import re
import json
import os
import sys
from pathlib import Path
from datetime import datetime
from typing import List, Tuple, Dict

PROJECT_ROOT = Path(__file__).parent.parent
SKILL_DIR = PROJECT_ROOT / ".opencode" / "skills" / "rairos-dev"

# ---------------------------------------------------------------------------
# Extraction functions
# ---------------------------------------------------------------------------

def count_cli_commands() -> int:
    """Count Commands enum variants in main.rs"""
    main_rs = PROJECT_ROOT / "crates" / "rairos-cli" / "src" / "main.rs"
    content = main_rs.read_text()
    pattern = r"^\s+Commands::"
    return len(re.findall(pattern, content, re.MULTILINE))

def count_crates() -> int:
    """Count rairos-* crates"""
    crates_dir = PROJECT_ROOT / "crates"
    return len(list(crates_dir.glob("rairos-*")))

def count_mcp_tools() -> Tuple[int, int, int]:
    """Count MCP tools: (core, llm, total)"""
    handlers = PROJECT_ROOT / "crates" / "rairos-mcp" / "src" / "handlers.rs"
    llm_handlers = PROJECT_ROOT / "crates" / "rairos-mcp" / "src" / "llm_handlers.rs"
    pattern = r'fn name\(&self\) -> &str'

    core = 0
    llm = 0
    if handlers.exists():
        core = len(re.findall(pattern, handlers.read_text()))
    if llm_handlers.exists():
        llm = len(re.findall(pattern, llm_handlers.read_text()))

    return core, llm, core + llm

def get_project_stats() -> Dict:
    """Get all project stats"""
    core, llm, total = count_mcp_tools()
    return {
        "crates": count_crates(),
        "cli_commands": count_cli_commands(),
        "mcp_core": core,
        "mcp_llm": llm,
        "mcp_total": total,
    }

def get_crates_list() -> List[Tuple[str, str]]:
    """Get list of key crates with descriptions"""
    crates_meta = {
        "rairos-core": "DB, FTS5, subscriptions, tags, constants",
        "rairos-cli": "CLI commands (main.rs dispatch)",
        "rairos-mcp": "MCP protocol server",
        "rairos-codegraph": "CodeGraph MCP + CLI",
        "rairos-llm": "GenePool, Evolution, LLM clients",
        "rairos-parser": "arXiv/CrossRef/Semantic Scholar API",
        "rairos-research": "DeepResearchAgent, gap detection",
        "rairos-kg": "Knowledge graph, PageRank",
        "rairos-memory": "Research stance tracking",
        "rairos-rankers": "Paper ranking/scoring",
    }
    crates_dir = PROJECT_ROOT / "crates"
    result = []
    for crate, desc in crates_meta.items():
        if (crates_dir / crate).exists():
            result.append((crate, desc))
    return result

def get_all_cli_commands() -> List[str]:
    """Get all CLI commands from Commands enum"""
    main_rs = PROJECT_ROOT / "crates" / "rairos-cli" / "src" / "main.rs"
    content = main_rs.read_text()
    enum_match = re.search(r'enum Commands \{(.+?)\n    \}', content, re.DOTALL)
    if not enum_match:
        return []
    enum_body = enum_match.group(1)
    variants = re.findall(r'^\s+(\w+)\s*\{', enum_body, re.MULTILINE)
    return variants

def get_cli_commands_sample() -> List[str]:
    """Get sample of CLI commands"""
    return get_all_cli_commands()[:20]

def get_mcp_tool_names() -> Tuple[List[str], List[str]]:
    """Get MCP tool names from handlers"""
    handlers = PROJECT_ROOT / "crates" / "rairos-mcp" / "src" / "handlers.rs"
    llm_handlers = PROJECT_ROOT / "crates" / "rairos-mcp" / "src" / "llm_handlers.rs"

    core_tools = []
    llm_tools = []

    if handlers.exists():
        content = handlers.read_text()
        # Find patterns like: fn name(&self) -> &str { "tool_name" }
        matches = re.findall(r'fn name\(&self\) -> &str\s*\{\s*"(\w+)"', content)
        core_tools = matches

    if llm_handlers.exists():
        content = llm_handlers.read_text()
        matches = re.findall(r'fn name\(&self\) -> &str\s*\{\s*"(\w+)"', content)
        llm_tools = matches

    return core_tools, llm_tools

# ---------------------------------------------------------------------------
# Generators
# ---------------------------------------------------------------------------

def generate_skill_md(stats: Dict) -> str:
    """Generate SKILL.md"""
    crates = stats["crates"]
    cli = stats["cli_commands"]
    mcp = stats["mcp_total"]
    crates_other = crates - 10

    content = """---
name: rairos-dev
description: Rairos project development workflow - Rust build, test, debug, and code analysis. Use when working on Rairos codebase (100% Rust, {crates} crates). Triggers: cargo build, cargo test, flamegraph, CodeGraph, adding CLI commands, adding MCP tools.
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
Commands::NewCommand {{ arg }} => {{
    handle_new_command(arg)?;
}}
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
├── rairos-cli/       # {cli} CLI commands
├── rairos-core/      # DB, FTS5, constants
├── rairos-mcp/       # {mcp} MCP tools
├── rairos-codegraph/ # CodeGraph MCP server
└── ... ({other} other crates)
```

See `REFERENCE.md` for full architecture details.
""".format(
        crates=crates,
        cli=cli,
        mcp=mcp,
        other=crates_other
    )
    return content

def generate_reference_md(stats: Dict, crates_list: List[Tuple[str, str]], cli_sample: List[str], core_tools: List[str], llm_tools: List[str]) -> str:
    """Generate REFERENCE.md"""
    return f"""# Rairos Project Reference

## Project Overview

- **{stats["crates"]} crates** (Rust only, no Python)
- **~73k+ Rust lines**
- **{stats["cli_commands"]} CLI commands**
- **{stats["mcp_total"]} MCP tools** ({stats["mcp_core"]} core + {stats["mcp_llm"]} LLM-backed)
- **License**: GPL-3.0

## Key Crates

| Crate | Purpose |
|-------|---------|
| rairos-core | DB, FTS5, subscriptions, tags, constants |
| rairos-cli | {stats["cli_commands"]} commands in main.rs |
| rairos-mcp | MCP protocol server ({stats["mcp_total"]} tools) |
| rairos-codegraph | CodeGraph MCP server + CLI |
| rairos-llm | GenePool, Evolution, LLM clients |
| rairos-parser | arXiv/CrossRef/Semantic Scholar API |
| rairos-research | DeepResearchAgent, gap detection |
| rairos-kg | Knowledge graph, PageRank |
| rairos-memory | Research stance tracking |
| rairos-rankers | Paper ranking/scoring |

## CLI Commands ({stats["cli_commands"]} total)

Sample: {', '.join(cli_sample[:15])}...

## MCP Tools ({stats["mcp_total"]} total)

**Core Rust ({stats["mcp_core"]})**: {', '.join(core_tools)}

**LLM-backed ({stats["mcp_llm"]})**: {', '.join(llm_tools)}

## Data Paths

All data in `~/.ai_research_os/`:

| Path | Content |
|------|---------|
| `papers.db` | SQLite paper database |
| `papers.json` | Papers JSON |
| `evolution/gene_pool.db` | Gene pool SQLite |
| `evolution/gene_pool.jsonl` | Gene pool append log |
| `evolution/events.jsonl` | Evolution events |
| `kg/graph.json` | Knowledge graph |
| `research_memory/` | Research stances |
| `briefings/` | Generated briefings |

## Constants (rairos-core::constants)

Key constants:
- `PAPERS_DB_PATH`: `.ai_research_os/papers.json`
- `GP_DIR_NAME`: `.ai_research_os/evolution`
- `KG_DIR`: `kg`
- `ARXIV_API`: `https://export.arxiv.org/api/query`
- `SEMANTIC_API`: `https://api.semanticscholar.org/graph/v1`
- `LLM_MODEL`: `gpt-4o-mini`

## Known Constraints

| Constraint | Value |
|------------|-------|
| Memory | Build with `CARGO_BUILD_JOBS=1` |
| tree-sitter | Must use 0.23 (not 0.24) |
| Disk space | ~5GB free recommended |
| Database | SQLite with FTS5 |

## Performance Benchmarks

| Operation | Time |
|-----------|------|
| impact_score (1000 papers) | 23 µs |
| impact_rank (1000 papers) | 138 µs |
| citation find_families (100 nodes) | 109 µs |
| citation find_silent (100 nodes) | 49 µs |
| MCP dispatch (cached) | ~10 µs |

## Key Patterns

- `papers` table PK is `id` (NOT arxiv_id)
- Database: SQLite at `rairos.db` or `$RAIROS_DB`
- MCP: `McpServer` with OnceLock cached dispatch
- CLI: `Commands` enum + `handle_*()` functions
"""

# ---------------------------------------------------------------------------
# Main
# ---------------------------------------------------------------------------

def generate_all():
    """Generate all skill files"""
    print("Generating Rairos skill files...")

    stats = get_project_stats()
    crates_list = get_crates_list()
    cli_sample = get_all_cli_commands()
    core_tools, llm_tools = get_mcp_tool_names()

    print(f"  Stats: {stats['crates']} crates, {stats['cli_commands']} CLI, {stats['mcp_total']} MCP")
    print(f"  MCP: {len(core_tools)} core, {len(llm_tools)} LLM")

    SKILL_DIR.mkdir(parents=True, exist_ok=True)

    # Generate files
    (SKILL_DIR / "SKILL.md").write_text(generate_skill_md(stats))
    (SKILL_DIR / "REFERENCE.md").write_text(generate_reference_md(stats, crates_list, cli_sample, core_tools, llm_tools))

    # CODERIEF.md is static - just copy if needed
    coderief = SKILL_DIR / "CODERIEF.md"
    if not coderief.exists():
        coderief.write_text("""# CodeGraph MCP Tools

CodeGraph provides code exploration via MCP protocol. Use for understanding Rairos codebase structure, finding symbols, and analyzing dependencies.

## Tool Overview

| Tool | When to Use |
|------|-------------|
| `codegraph_files` | **Start here** - explore project structure |
| `codegraph_search` | Find symbols by name |
| `codegraph_context` | Build context for a task |
| `codegraph_explore` | Deep exploration of a topic |
| `codegraph_callers` | Find who calls a function |
| `codegraph_callees` | Find what a function calls |
| `codegraph_impact` | Analyze change impact |
| `codegraph_node` | Get symbol details |
| `codegraph_status` | Check index status |

## Common Workflows

### 1. Explore Project Structure

```
codegraph_files {{ path: "crates", format: "tree", maxDepth: 2 }}
```

### 2. Find a Symbol

```
codegraph_search {{ query: "paper_search", limit: 5 }}
```

### 3. Build Context for a Task

```
codegraph_context {{ task: "add new CLI command to rairos-cli", maxNodes: 20 }}
```

### 4. Find Function Callers

```
codegraph_callers {{ symbol: "handle_paper_search", limit: 10 }}
```

### 5. Analyze Change Impact

```
codegraph_impact {{ symbol: "PapersDatabase", depth: 2 }}
```

### 6. Deep Exploration

```
codegraph_explore {{ query: "McpServer dispatch protocol", maxFiles: 10 }}
```

## Tips

1. Use `codegraph_search` first to find symbol names
2. Use `codegraph_context` for task planning
3. Use `codegraph_explore` for deep investigation
4. Use `codegraph_callers/callees` for refactoring
5. Use `codegraph_impact` before making changes
""")

    print(f"Generated {SKILL_DIR / 'SKILL.md'}")
    print(f"Generated {SKILL_DIR / 'REFERENCE.md'}")
    print(f"Generated {SKILL_DIR / 'CODERIEF.md'}")

if __name__ == "__main__":
    if "--watch" in sys.argv:
        print("Watch mode not implemented. Run manually after changes.")
    generate_all()
