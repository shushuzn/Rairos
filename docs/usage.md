# Usage

## Basic Usage

```bash
python ai_research_os.py <input> [options]
```

## Input Types

### arXiv URL
```bash
python ai_research_os.py https://arxiv.org/abs/2601.00155
python ai_research_os.py https://arxiv.org/pdf/2601.00155.pdf
python ai_research_os.py 2601.00155
```

### DOI
```bash
python ai_research_os.py 10.1038/nature12373
```

### Local PDF
```bash
python ai_research_os.py ./papers/my-paper.pdf
```

## Options

| Option | Description |
|--------|-------------|
| `--tags` | Comma-separated research tags |
| `--ai` | Generate AI-assisted draft |
| `--api-key` | API key for AI drafting |
| `--output-dir` | Output directory (default: current) |

## Examples

```bash
# With tags
python ai_research_os.py https://arxiv.org/abs/2601.00155 --tags LLM,Agent

# With AI draft
python ai_research_os.py https://arxiv.org/abs/2601.00155 --tags LLM --ai

# Custom output directory
python ai_research_os.py 10.1038/nature12373 --tags NLP --output-dir ./research
```

## Search Commands

The `search` and `list` subcommands provide full-text search across indexed papers using SQLite FTS5 with BM25 ranking.

```bash
# Full-text search
python ai_research_os.py search "transformer attention"

# Filter by category and date
python ai_research_os.py search --category cs.LG --date-from 2024-01-01

# Filter by parse status and sort by date
python ai_research_os.py search --status done --sort date

# List recent papers (no query = list mode)
python ai_research_os.py list

# List papers by status
python ai_research_os.py list --status pending

# Show database statistics
python ai_research_os.py status
```

### Search Options

| Option | Description |
|--------|-------------|
| `--category` | Filter by primary category (e.g. `cs.LG`, `cs.CL`) |
| `--source` | Filter by source (e.g. `arxiv`, `doi`) |
| `--status` | Filter by parse status (`pending`, `done`, `failed`) |
| `--date-from` | Filter papers published on or after this date |
| `--date-to` | Filter papers published on or before this date |
| `--sort` | Sort by `relevance` (default for search) or `date` |
| `--limit` | Max results to return (default: 20) |
| `--offset` | Pagination offset |

## Output Files

Each run generates:

- `P-NOTE-*.md` — Paper note
- `C-NOTE-*.md` — Critical note
- `radar.yaml` — Radar entry
- `timeline.yaml` — Timeline entry
