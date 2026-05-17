# CodeGraph MCP Tools

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
codegraph_files { path: "crates", format: "tree", maxDepth: 2 }
```

### 2. Find a Symbol

```
codegraph_search { query: "paper_search", limit: 5 }
```

### 3. Build Context for a Task

```
codegraph_context { task: "add new CLI command to rairos-cli", maxNodes: 20 }
```

### 4. Find Function Callers

```
codegraph_callers { symbol: "handle_paper_search", limit: 10 }
```

### 5. Analyze Change Impact

```
codegraph_impact { symbol: "PapersDatabase", depth: 2 }
```

### 6. Deep Exploration

```
codegraph_explore { query: "McpServer dispatch protocol", maxFiles: 10 }
```

## Input Schema Notes

All tools accept `projectPath` to query other codebases. Default uses current workspace.

| Parameter | Type | Default |
|-----------|------|---------|
| `projectPath` | string | current workspace |
| `maxNodes` / `maxFiles` | number | varies |
| `limit` | number | 10-20 |

## Example Queries

**Find CLI handlers:**
```
codegraph_search { query: "handle_", limit: 10 }
```

**Find MCP tools:**
```
codegraph_search { query: "paper_", kind: "function", limit: 15 }
```

**Find database code:**
```
codegraph_search { query: "Database", kind: "struct" }
```

**Explore error handling:**
```
codegraph_explore { query: "map_err anyhow Result", maxFiles: 8 }
```

## Index Status

Check if index is available:
```
codegraph_status {}
```

Returns: indexed files, nodes, edges count.

## Tips

1. Use `codegraph_search` first to find symbol names
2. Use `codegraph_context` for task planning
3. Use `codegraph_explore` for deep investigation
4. Use `codegraph_callers/callees` for refactoring
5. Use `codegraph_impact` before making changes
