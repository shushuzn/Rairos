# Usage

## Initialize

```bash
rairos init
```

## Import Papers

```bash
# Single paper by arXiv ID
rairos import 2601.00155

# Multiple papers
rairos import 2601.00155 2301.12345 10.1038/nature12373

# Batch from file (one ID per line)
rairos import --file ids.txt
```

## List Papers

```bash
# List all papers
rairos list

# Filter by status
rairos list --status pending
rairos list --status done

# Filter by source
rairos list --source arxiv
```

## Search

```bash
# Full-text search (FTS5 with BM25 ranking)
rairos search "transformer attention"

# Filter by category and date
rairos search --category cs.LG --date-from 2024-01-01

# Filter by parse status and sort by date
rairos search --status done --sort date

# Limit results
rairos search "LLM" --limit 50
```

## Database Statistics

```bash
rairos status
rairos stats
```

## Export

```bash
# BibTeX (default)
rairos export > papers.bib

# JSON
rairos export --format json > papers.json
```

## Queue

```bash
# Show pending papers
rairos queue --list

# Clear pending papers
rairos queue --clear
```

## Merge Duplicates

```bash
# Dry run — preview what would merge
rairos merge --dry-run

# Auto-merge high-similarity pairs (>= 0.95)
rairos merge --auto --dry-run

# Auto-merge for real
rairos merge --auto

# Keep specific paper when merging
rairos merge --keep newer 2301.00001 --dry-run

# Auto with semantic preference (0.8+ sim + matching titles)
rairos merge --keep semantic --auto --dry-run
```

## Semantic Deduplication

Requires Ollama with `nomic-embed-text` model.

```bash
# Check embedding coverage
rairos dedup-semantic --stats

# Generate embeddings for papers missing them
rairos dedup-semantic --generate

# Find similar papers for a specific paper
rairos dedup-semantic --paper 2601.00155

# Custom similarity threshold (higher = stricter match, default: 0.85)
rairos dedup-semantic --paper 2601.00155 --threshold 0.90

# Limit number of similar papers returned
rairos dedup-semantic --paper 2601.00155 --limit 5

# CSV output for pipeline integration
rairos dedup-semantic --generate --format csv
```

## Citation Graph

### Fetch Citations from OpenAlex

```bash
# Fetch for all papers in DB
rairos cite-fetch

# Fetch for specific paper
rairos cite-fetch 2601.00155

# Dry run — preview what would be imported
rairos cite-fetch --dry-run

# Only import citations where both papers are in local DB
rairos cite-fetch --skip-external

# Fetch only backward citations (papers cited by this paper)
rairos cite-fetch 2601.00155 --direction from

# Fetch only forward citations (papers citing this paper)
rairos cite-fetch 2601.00155 --direction to

# Rate limit (~9 req/s)
rairos cite-fetch --delay 0.11
```

### Extract References from Paper (plain-text)
```bash
# Extract references from a paper's plain text and print them
rairos cite-import --extract --paper 2601.00155

# Same, plus import citation edges into DB (arXiv IDs that exist in DB)
rairos cite-import --extract --paper 2601.00155 --dry-run

# Import with duplicate reporting (uses upsert mode)
rairos cite-import --extract --paper 2601.00155 --dedup
```
Extract mode finds arXiv IDs, DOIs, PMIDs, and ISBNs in the paper's plain text and prints them. PMIDs and ISBNs are shown as-is; DOIs are resolved to titles via CrossRef. Only arXiv IDs can be linked as citation edges.

### Bulk Import Citations from JSON

```bash
# From stdin
cat citations.json | rairos cite-import

# From file
rairos cite-import --file citations.json

# Dry run
rairos cite-import --file citations.json --dry-run

# Skip edges where source/target is not in DB
rairos cite-import --file citations.json --skip-missing
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
rairos cite-stats

# Per-paper stats
rairos cite-stats --paper 2601.00155

# Sort by citing papers (papers that cite most others)
rairos cite-stats --by citing

# Sort by cited-by (most cited papers)
rairos cite-stats --by cited

# CSV output
rairos cite-stats --format csv
```

### Citation Graph (plain-text)
```bash
# Extract references from a plain-text file and print as citation graph
rairos cite-graph --plain-text --paper 2601.00155
rairos cite-graph --plain-text --file ./paper.txt
rairos cite-graph --plain-text --file ./paper.txt --verbose
```

Plain-text mode reads a paper's plain text, extracts all arXiv IDs, DOIs, PMIDs, and ISBNs, and prints them as a citation list. Verbose mode shows the context around each identifier. DOI/PMID/ISBN are shown but cannot be linked as citation edges (only arXiv IDs are linked).

### Citation Graph (database)
```bash
# Graph centered on a paper (depth=1 by default)
rairos cite-graph --paper 2601.00155
rairos cite-graph --paper 2601.00155 --depth 2
rairos cite-graph --paper 2601.00155 --max-nodes 50

# Output formats
rairos cite-graph --paper 2601.00155 --format json
rairos cite-graph --paper 2601.00155 --format mermaid
rairos cite-graph --paper 2601.00155 --format text
```

### Deduplicate Papers
```bash
# Deduplicate by exact arXiv ID match
rairos dedup

# Deduplicate by semantic similarity (embedding-based)
rairos dedup-semantic
rairos dedup-semantic --paper 2601.00155
rairos dedup-semantic --paper 2601.00155 --threshold 0.85 --limit 5
```
`dedup` removes duplicate papers from the database using exact match. `dedup-semantic` finds papers with similar abstracts using embeddings; `--threshold` controls similarity cutoff (0.0-1.0, default 0.8), `--limit` caps results per paper.
