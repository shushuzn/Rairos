# Changelog

All notable changes to this project will be documented in this file.

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
