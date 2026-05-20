# Usage

## Initialize

```bash
./rairos.sh init
```

## Import Papers

```bash
# Single paper by arXiv ID
./rairos.sh import 2601.00155

# Multiple papers
./rairos.sh import 2601.00155 2301.12345 10.1038/nature12373

# Batch from file (one ID per line)
./rairos.sh import --file ids.txt
```

## List Papers

```bash
# List all papers
./rairos.sh list

# Filter by status
./rairos.sh list --status pending
./rairos.sh list --status done

# Filter by source
./rairos.sh list --source arxiv
```

## Search

```bash
# Full-text search (FTS5 with BM25 ranking)
./rairos.sh search "transformer attention"

# Filter by category and date
./rairos.sh search --category cs.LG --date-from 2024-01-01

# Filter by parse status and sort by date
./rairos.sh search --status done --sort date

# Limit results
./rairos.sh search "LLM" --limit 50
```

## Database Statistics

```bash
./rairos.sh status
./rairos.sh stats
```

## Export

```bash
# BibTeX (default)
./rairos.sh export > papers.bib

# JSON
./rairos.sh export --format json > papers.json
```

## Queue

```bash
# Show pending papers
./rairos.sh queue --list

# Clear pending papers
./rairos.sh queue --clear
```

## Merge Duplicates

```bash
# Dry run — preview what would merge
./rairos.sh merge --dry-run

# Auto-merge high-similarity pairs (>= 0.95)
./rairos.sh merge --auto --dry-run

# Auto-merge for real
./rairos.sh merge --auto

# Keep specific paper when merging
./rairos.sh merge --keep newer 2301.00001 --dry-run

# Auto with semantic preference (0.8+ sim + matching titles)
./rairos.sh merge --keep semantic --auto --dry-run
```

## Semantic Deduplication

Requires Ollama with `nomic-embed-text` model.

```bash
# Check embedding coverage
./rairos.sh dedup-semantic --stats

# Generate embeddings for papers missing them
./rairos.sh dedup-semantic --generate

# Find similar papers for a specific paper
./rairos.sh dedup-semantic --paper 2601.00155

# Custom similarity threshold (higher = stricter match, default: 0.85)
./rairos.sh dedup-semantic --paper 2601.00155 --threshold 0.90

# Limit number of similar papers returned
./rairos.sh dedup-semantic --paper 2601.00155 --limit 5

# CSV output for pipeline integration
./rairos.sh dedup-semantic --generate --format csv
```

## Citation Graph

### Fetch Citations from OpenAlex

```bash
# Fetch for all papers in DB
./rairos.sh cite-fetch

# Fetch for specific paper
./rairos.sh cite-fetch 2601.00155

# Dry run — preview what would be imported
./rairos.sh cite-fetch --dry-run

# Only import citations where both papers are in local DB
./rairos.sh cite-fetch --skip-external

# Fetch only backward citations (papers cited by this paper)
./rairos.sh cite-fetch 2601.00155 --direction from

# Fetch only forward citations (papers citing this paper)
./rairos.sh cite-fetch 2601.00155 --direction to

# Rate limit (~9 req/s)
./rairos.sh cite-fetch --delay 0.11
```

### Extract References from Paper (plain-text)
```bash
# Extract references from a paper's plain text and print them
./rairos.sh cite-import --extract --paper 2601.00155

# Same, plus import citation edges into DB (arXiv IDs that exist in DB)
./rairos.sh cite-import --extract --paper 2601.00155 --dry-run

# Import with duplicate reporting (uses upsert mode)
./rairos.sh cite-import --extract --paper 2601.00155 --dedup
```
Extract mode finds arXiv IDs, DOIs, PMIDs, and ISBNs in the paper's plain text and prints them. PMIDs and ISBNs are shown as-is; DOIs are resolved to titles via CrossRef. Only arXiv IDs can be linked as citation edges.

### Bulk Import Citations from JSON

```bash
# From stdin
cat citations.json | ./rairos.sh cite-import

# From file
./rairos.sh cite-import --file citations.json

# Dry run
./rairos.sh cite-import --file citations.json --dry-run

# Skip edges where source/target is not in DB
./rairos.sh cite-import --file citations.json --skip-missing
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
./rairos.sh cite-stats

# Per-paper stats
./rairos.sh cite-stats --paper 2601.00155

# Sort by citing papers (papers that cite most others)
./rairos.sh cite-stats --by citing

# Sort by cited-by (most cited papers)
./rairos.sh cite-stats --by cited

# CSV output
./rairos.sh cite-stats --format csv
```

### Citation Graph (plain-text)
```bash
# Extract references from a plain-text file and print as citation graph
./rairos.sh cite-graph --plain-text --paper 2601.00155
./rairos.sh cite-graph --plain-text --file ./paper.txt
./rairos.sh cite-graph --plain-text --file ./paper.txt --verbose
```

Plain-text mode reads a paper's plain text, extracts all arXiv IDs, DOIs, PMIDs, and ISBNs, and prints them as a citation list. Verbose mode shows the context around each identifier. DOI/PMID/ISBN are shown but cannot be linked as citation edges (only arXiv IDs are linked).

### Citation Graph (database)
```bash
# Graph centered on a paper (depth=1 by default)
./rairos.sh cite-graph --paper 2601.00155
./rairos.sh cite-graph --paper 2601.00155 --depth 2
./rairos.sh cite-graph --paper 2601.00155 --max-nodes 50

# Output formats
./rairos.sh cite-graph --paper 2601.00155 --format json
./rairos.sh cite-graph --paper 2601.00155 --format mermaid
./rairos.sh cite-graph --paper 2601.00155 --format text
```

### Deduplicate Papers
```bash
# Deduplicate by exact arXiv ID match
./rairos.sh dedup

# Deduplicate by semantic similarity (embedding-based)
./rairos.sh dedup-semantic
./rairos.sh dedup-semantic --paper 2601.00155
./rairos.sh dedup-semantic --paper 2601.00155 --threshold 0.85 --limit 5
```
`dedup` removes duplicate papers from the database using exact match. `dedup-semantic` finds papers with similar abstracts using embeddings; `--threshold` controls similarity cutoff (0.0-1.0, default 0.8), `--limit` caps results per paper.
