# Changelog

All notable changes to this project will be documented in this file.

## v1.5.0 (2026-04-19)

### Test Infrastructure

- Freeze time to 2026-06-15 for all Tier 4 date-dependent tests via conftest.py autouse fixture
- Centralize shared fixtures into tests/conftest.py: mock_research_root, sample_pdf, tmp_db
- Strengthen Tier 4 tmpdir assertions: verify file existence before assertions, check exact sort order, verify exact counts

### Bug Fixes

- Fix TestOcrPage mock_import signature: accept variadic positional args (*args) instead of **kw; save _real_import reference to avoid infinite recursion
- Add paper_exists(), citations(), dedup_log(), delete_paper() test coverage

### Developer Experience

- Add justfile with 10 recipes: test, test-cov, test-tier4, test FILE, lint, lint-fix, fmt, check, install, run, ci
- F841 (unused variable) noqa cleanup across test_ai_research_os.py, test_cli_search.py, test_integration.py, test_sections_segment.py, test_pdf_extract.py

## v1.4.0 (2026-04-18)

### Features

- `citations` — Citation graph CLI for tracking paper reference relationships
  - `--from PAPER_ID`: Show papers cited by PAPER_ID (backward citations / references)
  - `--to PAPER_ID`: Show papers that cite PAPER_ID (forward citations / bibliography)
  - `--format csv`: CSV output with direction, source_id, source_title, target_id, target_title
  - New `citations` table in DB: `(source_id, target_id)` edges with FK to `papers` + CASCADE delete
  - New `CitationRecord` dataclass
  - `add_citation()` / `add_citations_batch()` / `get_citations()` / `get_citation_count()` / `get_paper_title()` DB methods

- `cite-fetch` — Fetch citation data from OpenAlex for papers in the database
  - `cite-fetch [PAPER_ID]`: Fetch for specific paper or all papers in DB
  - `--direction from|to|both`: Which citations to fetch (default: both)
  - `--dry-run`: Preview what would be imported without writing to DB
  - `--skip-external`: Only import citations where both source and target are in local DB
  - `--delay 0.11`: Rate-limit to ~9 req/s (default: 0.11s)
  - Bypasses Windows proxy SSL issues via custom SSL context
  - Resolves arXiv paper → OpenAlex ID via DOI lookup, then fetches `referenced_works` and `cites:` filter

- `cite-import` — Bulk import citation edges from JSON
  - `--file FILE`: Read JSON from file (default: stdin)
  - `--dry-run`: Validate input without writing to DB
  - `--skip-missing`: Skip edges where source or target paper is not in local DB
  - JSON format: `[{"source": "PAPER_ID", "targets": ["TARGET_ID", ...]}, ...]`

- `cite-stats` — Show citation statistics
  - Global view: total edges, unique citing papers, cited papers, avg citations per paper
  - Per-paper view: `--paper PAPER_ID` shows backward (cites) and forward (cited-by) counts
  - CSV output: `--format csv` emits `paper_id,title,cites,cited_by`

- `dedup-semantic` — Find near-duplicate papers using semantic embeddings via Ollama
  - `--stats`: Show embedding coverage (papers with/without embeddings)
  - `--generate`: Generate embeddings for papers missing them via `nomic-embed-text`
  - `--paper PAPER_ID`: Find similar papers for a specific paper
  - `--threshold 0.85`: Cosine similarity threshold (default 0.85)
  - `--limit 20`: Max similar papers returned per query
  - `--format {text,csv}`: Output format — `text` (default) or `csv` with columns `id1,title1,id2,title2,similarity`
  - New DB methods: `set_embedding()`, `get_embedding()`, `get_papers_without_embeddings()`, `find_similar()`, `get_embedding_stats()`
  - Embeddings stored as binary blob in `papers.embed_vector` (768-dim float, nomic-embed-text)

- `cite-graph` — Citation graph visualization subcommand for exploring paper reference networks
  - `--from PAPER_ID`: Show papers cited by PAPER_ID (backward citations / references)
  - `--to PAPER_ID`: Show papers that cite PAPER_ID (forward citations / bibliography)
  - `--format {text,csv,mermaid,ascii}`: Output format — `text` (default), `csv`, `mermaid` (live-reload graph), `ascii` (unicode box art)
  - `--dry-run`: Preview graph without writing to DB
  - `--skip-missing`: Skip edges where source or target paper is not in local DB
  - `--plain-text`: Extract references directly from raw text without DB storage; supports arXiv IDs, DOIs, PMIDs, and ISBNs with regex extraction and DOI-to-arXiv CrossRef fallback
  - `--fetch-metadata`: Fetch and display title, authors, year, venue for each paper in the graph via CrossRef API
  - New DB method: `upsert_citations()` for idempotent citation edge insertion

- `cite-import` — Bulk import citation edges from JSON (extended)
  - `--extract --paper PAPER_ID`: Parse arXiv IDs, DOIs, PMIDs, and ISBNs from the References section of a paper's plain_text and create citation edges for references already in the DB
  - `--dedup`: Use `upsert_citations()` for idempotent insertion, reporting duplicate edge count

### Tests

- Add `test_cli_cite_graph.py` with 16+ tests covering cite-graph --from/--to, --format, --dry-run, --skip-missing, --plain-text, --fetch-metadata
- Add 2 tests for `dedup-semantic --threshold` and `--limit` coverage

### Documentation

- Update cite-* toolchain usage guide in `docs/usage.md` with cite-graph, --extract, --dedup documentation
- Add PMID/ISBN extraction and dedup-semantic --threshold/--limit to `docs/index.md` and `docs/usage.md`

## v1.3.0 (2026-04-18)

### Features

- `stats` — DB overview: total papers, breakdown by source, parse_status, job_queue size, dedup_log count, HTTP cache size
- `import` — Batch add papers by arXiv UID / DOI / URL with `--source` flag and `--skip-existing` flag
- `export` — Full DB export to CSV or JSON with `--out FILE`; includes all 11 Paper fields
- `queue --list` — List pending papers with their IDs, arXiv IDs, titles, and added_at timestamps
- `queue --clear` — Reset all papers with parse_status=pending back to idle, clearing the queue
- `list --sort` — Sort papers by `published` / `title` / `parse_status` / `added_at` with `--order asc|desc`
- `list --format csv` — Emit CSV with 8 columns: id, arxiv_id, title, published, source, parse_status, added_at, cached_at
- `dedup --since YYYY-MM-DD` — Limit deduplication scan to papers added on or after the given date
- `merge --dry-run` — Preview keep/drop decisions for each duplicate pair before executing the merge

### Bug Fixes

- Fix `queue --list` and `--clear` semantics: `--list` reads `papers.parse_status`, `--clear` resets `papers.parse_status` (not `job_queue` table)
- Fix CLI subcommand parser: `stats`, `import`, `export` now correctly registered in the new subcommand parser flow
- Fix `PaperRecord` attribute: use `.id` instead of non-existent `.uid` throughout `_run_queue`

### CI

- Fix PyPI release: add `skip_existing: true` to prevent 400 "File already exists" errors
- Fix GitHub Release: use `github.ref_name || inputs.tag` so workflow_dispatch creates correct tag; add `contents: write` permission for `softprops/action-gh-release`

### Refactor

- Add `clear_pending_papers()` to `db/database.py` — resets `parse_status` to idle for all pending papers
- Split CLI into dual parser flow: legacy flow (positional arXiv ID/DOI) and new subcommand flow (no positional args)

## v1.0.2 (2026-04-17)

### Bug Fixes
- Correct pyproject.toml after web UI corruption ([#49](https://github.com/shushuzn/ai_research_os/pull/49))
- Add get_cached/set_cached mocks to prevent test cache pollution

### CI
- Upload pytest output as artifact for debugging
- Add workflow_dispatch trigger for manual CI runs

### Documentation
- Bilingual README (English + Chinese)

## v1.0.1 (2026-04-17)

### Features
- Add GitHub Actions PyPI release workflow
- Add pyproject.toml for PyPI packaging

### CI
- Fix CI artifact upload after web UI corruption

### Documentation
- Add CHANGELOG.md for v1.0.0

## v1.0.0 (2026-04-16)

### Features
- `--ai-cnote` standalone CLI mode for AI-generating C-Notes from P-Notes
- Parse and structure LLM P-note draft output with rubric extraction
- `--structured` flag for PDF structured extraction (tables/math separated)
- HTTP response cache for arXiv and Crossref
- `fetch_arxiv_metadata_batch` for single-request multi-paper fetch
- LLM prompt cost optimization
- Improve P-note AI prompt with scoring rubric and citation requirements
- Extend Paper dataclass with journal, volume, issue, page, doi, comment, journal_ref, categories, reference_count
- PDF structured extraction + C-note AI draft + P-note metadata reader
- CLI shell completions (bash, zsh, fish)

### Bug Fixes
- Rubric extraction: key-anchored regex + multi-format support
- Mnote filename collision + preserve manual bullets
- Escape `#` in render_cnote, warn on bad date, fix `short()` hash collision
- Handle None/empty input in `normalize_doi` and `normalize_arxiv_id`
- Add missing `system_prompt` param to `call_llm_chat_completions`
- Correct Paper constructor in `ai_generate_pnote_draft` tests
- 17 test failures in ai_research_os
- Remove hardcoded proxy, fix docstring escape sequences
- Remove pip cache from test-pytest (no requirements.txt)

### Tests
- Add 68 Tier 2 parser unit tests
- Add 23 new tests for Tier 3 coverage
- Add 29 new tests for parsing, utils, and note functions
- Add coverage for all bug fixes
- Fix Tier 4 test assertions to match actual function behaviors
- Rename duplicate test classes with Tier1/Tier2/Tier4 suffixes
- Add integration tests covering CLI full flows
- Add pytest job to run all 255 tests

### Refactor
- Split monolithic `ai_research_os.py` into modular package

### CI
- Add GitHub Actions workflow with lint, test-arxiv, test-crossref, and test-pytest jobs
- Rewrite ci.yml with correct branch triggers
- Use ubuntu-22.04 instead of ubuntu-latest

### Documentation
- Improve README: better structure, usage examples, CLI reference table
