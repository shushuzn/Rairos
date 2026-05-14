# Rust CLI: Complete `parse` + `repl` Handlers

## Motivation

The Rust CLI (`rairos-cli`) has 46 implemented handlers but 2 remain stubs:
`handle_parse` (PDF text extraction) and `handle_repl` (interactive REPL).
Completing them eliminates the last "not yet implemented" messages in the Rust CLI.

## Design

### 1. `rairos parse <id>` — PDF Parse

**Flow:**
1. Lookup paper by `id` or `arxiv_id` from DB
2. Construct arXiv PDF URL: `https://arxiv.org/pdf/{arxiv_id}.pdf`
3. Download to `~/.ai_research_os/pdfs/{arxiv_id}.pdf` via `rairos_pdf::download_pdf()`
4. Extract text using `lopdf` (add dep to `rairos-pdf`)
5. Update DB: `parse_status = Done`
6. Print preview (first 500 chars)

**Files:**
- `crates/rairos-pdf/Cargo.toml` — add `lopdf`
- `crates/rairos-pdf/src/lib.rs` — implement `extract_pdf_text()`
- `crates/rairos-cli/src/main.rs` — replace `handle_parse` stub

### 2. `rairos repl` — Interactive REPL

**Commands:**

| Command | Implementation |
|---------|---------------|
| `help` | Print command list |
| `exit` / `quit` | `break` loop |
| `search <query>` | `db.search_papers()` → table |
| `stats` | Inline stats display |
| `show <id>` | Paper detail |
| `list [--status N]` | List with optional status filter |
| `gap <topic>` | Gap detection |
| `add <arxiv_id>` | arXiv import |

**Files:**
- `crates/rairos-cli/src/main.rs` — replace `handle_repl` stub with stdin loop

## Testing

- `cargo test` passes
- Manual: `cargo run -p rairos-cli -- parse <arxiv_id>` and `cargo run -p rairos-cli -- repl`
