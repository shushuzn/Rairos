# Usage

## Initialize

```bash
python -m ai_research_os init
```

## Import Papers

```bash
# Single paper by arXiv ID
python -m ai_research_os import 2601.00155

# Multiple papers
python -m ai_research_os import 2601.00155 2301.12345 10.1038/nature12373

# Batch from file (one ID per line)
python -m ai_research_os import --file ids.txt
```

## List Papers

```bash
# List all papers
python -m ai_research_os list

# Filter by status
python -m ai_research_os list --status pending
python -m ai_research_os list --status done

# Filter by source
python -m ai_research_os list --source arxiv
```

## Search

```bash
# Full-text search (FTS5 with BM25 ranking)
python -m ai_research_os search "transformer attention"

# Filter by category and date
python -m ai_research_os search --category cs.LG --date-from 2024-01-01

# Filter by parse status and sort by date
python -m ai_research_os search --status done --sort date

# Limit results
python -m ai_research_os search "LLM" --limit 50
```

## Database Statistics

```bash
python -m ai_research_os status
python -m ai_research_os stats
```

## Export

```bash
# BibTeX (default)
python -m ai_research_os export > papers.bib

# JSON
python -m ai_research_os export --format json > papers.json
```

## Queue

```bash
# Show pending papers
python -m ai_research_os queue --list

# Clear pending papers
python -m ai_research_os queue --clear
```

## Merge Duplicates

```bash
# Dry run — preview what would merge
python -m ai_research_os merge --dry-run

# Auto-merge high-similarity pairs (>= 0.95)
python -m ai_research_os merge --auto --dry-run

# Auto-merge for real
python -m ai_research_os merge --auto

# Keep specific paper when merging
python -m ai_research_os merge --keep newer 2301.00001 --dry-run

# Auto with semantic preference (0.8+ sim + matching titles)
python -m ai_research_os merge --keep semantic --auto --dry-run
```

## Semantic Deduplication

Requires Ollama with `nomic-embed-text` model.

```bash
# Check embedding coverage
python -m ai_research_os dedup-semantic --stats

# Generate embeddings for papers missing them
python -m ai_research_os dedup-semantic --generate

# Find similar papers for a specific paper
python -m ai_research_os dedup-semantic --paper 2601.00155

# Custom similarity threshold (higher = stricter match, default: 0.85)
python -m ai_research_os dedup-semantic --paper 2601.00155 --threshold 0.90

# Limit number of similar papers returned
python -m ai_research_os dedup-semantic --paper 2601.00155 --limit 5

# CSV output for pipeline integration
python -m ai_research_os dedup-semantic --generate --format csv
```

## Citation Graph

### Fetch Citations from OpenAlex

```bash
# Fetch for all papers in DB
python -m ai_research_os cite-fetch

# Fetch for specific paper
python -m ai_research_os cite-fetch 2601.00155

# Dry run — preview what would be imported
python -m ai_research_os cite-fetch --dry-run

# Only import citations where both papers are in local DB
python -m ai_research_os cite-fetch --skip-external

# Fetch only backward citations (papers cited by this paper)
python -m ai_research_os cite-fetch 2601.00155 --direction from

# Fetch only forward citations (papers citing this paper)
python -m ai_research_os cite-fetch 2601.00155 --direction to

# Rate limit (~9 req/s)
python -m ai_research_os cite-fetch --delay 0.11
```

### Extract References from Paper (plain-text)
```bash
# Extract references from a paper's plain text and print them
python -m ai_research_os cite-import --extract --paper 2601.00155

# Same, plus import citation edges into DB (arXiv IDs that exist in DB)
python -m ai_research_os cite-import --extract --paper 2601.00155 --dry-run

# Import with duplicate reporting (uses upsert mode)
python -m ai_research_os cite-import --extract --paper 2601.00155 --dedup
```
Extract mode finds arXiv IDs, DOIs, PMIDs, and ISBNs in the paper's plain text and prints them. PMIDs and ISBNs are shown as-is; DOIs are resolved to titles via CrossRef. Only arXiv IDs can be linked as citation edges.

### Bulk Import Citations from JSON

```bash
# From stdin
cat citations.json | python -m ai_research_os cite-import

# From file
python -m ai_research_os cite-import --file citations.json

# Dry run
python -m ai_research_os cite-import --file citations.json --dry-run

# Skip edges where source/target is not in DB
python -m ai_research_os cite-import --file citations.json --skip-missing
```

JSON format:
```json
[
  {
    "source": "2601.00155",
    "targets": ["2301.09876", "2305.12345"]
  }
]
```

### Citation Statistics

```bash
# Global stats — total edges, unique citing/cited, avg per paper
python -m ai_research_os cite-stats

# Per-paper stats
python -m ai_research_os cite-stats --paper 2601.00155

# Sort by citing papers (papers that cite most others)
python -m ai_research_os cite-stats --by citing

# Sort by cited-by (most cited papers)
python -m ai_research_os cite-stats --by cited

# CSV output
python -m ai_research_os cite-stats --format csv
```

### Citation Graph (plain-text)
```bash
# Extract references from a plain-text file and print as citation graph
python -m ai_research_os cite-graph --plain-text --paper 2601.00155
python -m ai_research_os cite-graph --plain-text --file ./paper.txt
python -m ai_research_os cite-graph --plain-text --file ./paper.txt --verbose
```

Plain-text mode reads a paper's plain text, extracts all arXiv IDs, DOIs, PMIDs, and ISBNs, and prints them as a citation list. Verbose mode shows the context around each identifier. DOI/PMID/ISBN are shown but cannot be linked as citation edges (only arXiv IDs are linked).

### Citation Graph (database)
```bash
# Graph centered on a paper (depth=1 by default)
python -m ai_research_os cite-graph --paper 2601.00155
python -m ai_research_os cite-graph --paper 2601.00155 --depth 2
python -m ai_research_os cite-graph --paper 2601.00155 --max-nodes 50

# Output formats
python -m ai_research_os cite-graph --paper 2601.00155 --format json
python -m ai_research_os cite-graph --paper 2601.00155 --format mermaid
python -m ai_research_os cite-graph --paper 2601.00155 --format text
```

### Deduplicate Papers
```bash
# Deduplicate by exact arXiv ID match
python -m ai_research_os dedup

# Deduplicate by semantic similarity (embedding-based)
python -m ai_research_os dedup-semantic
python -m ai_research_os dedup-semantic --paper 2601.00155
python -m ai_research_os dedup-semantic --paper 2601.00155 --threshold 0.85 --limit 5
```
`dedup` removes duplicate papers from the database using exact match. `dedup-semantic` finds papers with similar abstracts using embeddings; `--threshold` controls similarity cutoff (0.0-1.0, default 0.8), `--limit` caps results per paper.
