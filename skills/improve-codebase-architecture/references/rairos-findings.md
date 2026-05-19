# Rairos Architecture Findings

Session: 2025-05-19 — Rairos codebase exploration (149-crate Rust monorepo, ~73k LOC, 68 MCP tools, 104 CLI commands).

## Candidates (from exploration)

### 1. Citation Domain — Triple Implementation

**Modules:** `rairos-citations`, `rairos-citation-chain`, `rairos-llm/src/citation_chain.rs`

**Problem:** Three independent implementations of the same domain with zero cross-crate reuse. `rairos-citations` is dead code (zero importers). `rairos-citation-chain` is the richer implementation (builder pattern, graphviz/mermaid renderers, silent citation detection). `rairos-llm/citation_chain.rs` is a thin Semantic Scholar API wrapper used by MCP.

**Key findings:**
- `rairos-citations` (741 LOC) — dead, no downstream importers
- `rairos-citation-chain` (807 LOC) — CLI uses it; 12+ builder methods; graphviz/mermaid renderers; `CitationChainBuilder` with HashMap-based nodes
- `rairos-llm/citation_chain.rs` — MCP uses it; 3 structs, 4 async functions; Semantic Scholar API wrapper; less capable than `rairos-citation-chain`
- `rairos-citation-pathfinder-web` (292 LOC) — visualization, should be kept separate

**Proposed consolidation:**
- Delete `rairos-citations` (dead code, deletion test passes — complexity vanishes)
- Merge `rairos-llm/citation_chain.rs` into `rairos-citation-chain/src/fetch.rs` as `fetch` submodule
- Keep `rairos-citation-chain` as the single source of truth for citation types

**Benefits:**
- `CitationNode` / `CitationChain` / `CitationChainBuilder` defined once, not three times
- CLI and MCP both import from the same types
- `PaperInfo` and `SilentCitation` structs from dead crate can be preserved/merged into main types

---

### 2. CLI Flat Dispatch — 104 Commands, No Hierarchy

**Modules:** `rairos-cli/src/main.rs` + `handlers/` (20+ modules, 80+ handlers)

**Problem:** Single-layer `match &cli.command` dispatch to 80+ `handle_*` functions. No intermediate abstraction. Comments in source group commands into batches but compiler does not enforce grouping.

**Key findings:**
- `Commands` enum in `handlers/commands.rs` (line ~285), derived with `#[derive(Subcommand)]`
- `match &cli.command` in `main.rs` (~100 lines) — direct dispatch, no sub-dispatch
- Two handler signatures: stateless (`Result<()>`) and DB-backed (`fn(&Database, ...) -> Result<()>`)
- No trait objects, no registry, no closure tables
- Tight coupling via `use handlers::*;` glob re-export

**Proposed deepening:**
- Introduce command group traits (`PaperCommands`, `GeneCommands`, etc.)
- Compress the `match` from 80 arms to 5-6 sub-dispatch calls
- Reduce bounce between `commands.rs`, `handlers/mod.rs`, `main.rs`, and handler modules

---

### 3. MCP OnceLock Runtime Contention

**Modules:** `rairos-mcp/src/protocol.rs`, `handlers.rs`, `llm_handlers/`

**Problem:** Single `RwLock<HashMap<String, Box<dyn ToolHandler>>>` for all 68 tools. CLAUDE.md already documents: "2+ concurrent call_tool invocations compete for the lock."

**Key findings:**
- `register_all()` calls `register()` 48 times sequentially
- `register_llm_handlers()` calls `register()` 45 times
- No module-level grouping — flat HashMap lookup for all tools
- Handler trait is fully object-safe via `async_trait` — all 68 tools are `Box<dyn ToolHandler>`

**Proposed deepening:**
- Per-category sub-locks (`DashMap` or multiple `RwLock`s) to reduce contention
- Consider `#[derive(Tool)]` macro to auto-generate `name()`, `description()`, `input_schema()` from struct definitions, eliminating repetitive boilerplate
- Per-call task spawn model to eliminate shared-state concurrency

---

### 4. rairos-llm Crate Overload — 18+ Modules in One Crate

**Modules:** `rairos-llm/src/lib.rs` (70KB, 18+ public submodules)

**Problem:** All submodules (briefing, citation_chain, gap_detector, impact, lit_review, slides, trust_scorer, replication, route_planner, paper_analyzer, paper_comparison...)堆在 `rairos-llm` with no internal hierarchy. Also re-exports from `rairos-insight-*` crates.

**Key findings:**
- Interface = sum of all submodules, no unified entry point
- `GenePool` / `Evolution` are data containers (shallow), but the crate also provides the LLM client implementations (deep)
- A bug in `paper_analyzer` could silently affect MCP LLM tools AND CLI gene commands

**Proposed deepening:**
- Split into domain crates: `rairos-briefing`, `rairos-route-planner`, `rairos-paper-analyzer`
- Keep `rairos-llm` for trait definitions and client implementations only

---

## Generalizable Patterns

### Pattern: Multi-Implementation Domain
When the same domain (e.g., citation chains) appears in 3+ places with different capabilities, apply the deletion test to determine which is alive:
1. Find all importers via `grep -r "use .*citation_chain"` across workspace
2. Any crate with zero importers is dead code — delete it
3. For live implementations, identify capability gaps and merge upward into the richest implementation

### Pattern: Flat Dispatch at Scale
Commands > 50 warrant a hierarchy. Use trait objects or an enum-of-enums to create intermediate grouping. The compiler should enforce grouping, not comments.

### Pattern: Shared-State Concurrency at Handler Count
68+ handlers sharing one lock is a contention problem. Use `DashMap` or sharded maps instead of a single `RwLock`.
