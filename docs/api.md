# API Reference

## CLI (rairos-cli)

All commands via `./rairos.sh <command>`.

```bash
./rairos.sh --help               # Full command list
./rairos.sh <command> --help    # Per-command help
```

## Core Data Structures

Defined in `crates/rairos-core/src/lib.rs`:

### `Paper`

```rust
pub struct Paper {
    pub id: String,
    pub arxiv_id: Option<String>,
    pub title: String,
    pub authors: Vec<String>,
    pub published: DateTime<Utc>,
    pub abstract_text: String,
    pub categories: Vec<String>,
    pub parse_status: ParseStatus,
    pub metadata: PaperMetadata,
}
```

### `PaperMetadata`

```rust
pub struct PaperMetadata {
    pub cited_by: usize,
    pub references: usize,
    pub doi: Option<String>,
    pub pdf_url: Option<String>,
}
```

### `ParseStatus`

```rust
pub enum ParseStatus {
    Pending,
    Parsing,
    Done,
    Failed,
}
```

### `SearchResult`

```rust
pub struct SearchResult {
    pub paper_id: String,
    pub title: String,
    pub authors: Vec<String>,
    pub published: String,
    pub primary_category: String,
    pub score: f64,
    pub snippet: String,
    pub parse_status: String,
    pub source: String,
    pub abs_url: String,
    pub pdf_url: String,
}
```

## Database

All database operations go through `rairos-core::Database`:

```rust
// Open database
let db = Database::open("rairos.db")?;

// Search papers
let (results, total) = db.search_papers_ext("query", 10, 0, None, None)?;

// Paper CRUD
db.upsert_paper(&paper)?;
let paper = db.get_paper("id")?;
db.delete_paper("id")?;

// List papers
let (papers, total) = db.list_papers(50, 0, None, None)?;
```

For CLI access, all operations are available as subcommands:

```bash
./rairos.sh search "query"
./rairos.sh show <paper-id>
./rairos.sh list
./rairos.sh init
./rairos.sh stats
./rairos.sh status
```

## MCP Tools (68 Rust)

Rairos exposes 68 MCP tools via JSON-RPC 2.0 at `crates/rairos-mcp/src/`.

Tool categories:
- **Paper**: `paper_search`, `paper_ingest`, `paper_parse_full`, `paper_query`, `paper_recommend`
- **Citations**: `citation_chain_build`, `citation_families`, `citation_silent`, `citation_render`, `cite_fetch`
- **Knowledge Graph**: `kg_paper_subgraph`, `kg_tag_graph`, `kg_full_graph`, `kg_query`
- **Research**: `deep_research_run`, `parallel_research_run`, `hypothesis_generate`, `gap_detect`, `gap_submit`
- **Gene Pool**: `crossover`, `gene_pool_decay`, `gene_pool_watcher`
- **Analysis**: `impact_score_paper`, `replication_check`, `trust_scorer_compute`, `paper_compare`
- **System**: `chart_query`, `tag_list`, `route_query`, `briefing_generate`, `review_list`

See `AGENTS.md` for the full list.

## Persistence

| Path | Contents |
|------|----------|
| `rairos.db` | Main database (configurable via `RAIROS_DB` env var or `--db` flag) |
| `~/.ai_research_os/kg/graph.json` | Knowledge graph |
| `~/.ai_research_os/evolution/gene_pool.db` | Gene Pool (evolution state) |
| `~/.ai_research_os/research_memory/` | Research memory |
