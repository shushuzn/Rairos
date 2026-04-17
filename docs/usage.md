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

## Output Files

Each run generates:

- `P-NOTE-*.md` — Paper note
- `C-NOTE-*.md` — Critical note  
- `radar.yaml` — Radar entry
- `timeline.yaml` — Timeline entry
