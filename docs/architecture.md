# Architecture

## Overview

```
Input (arXiv/DOI/PDF)
       │
       ▼
  parsers/           # Parse input → structured paper metadata
       │
       ▼
  core/              # Business logic
       │
  ┌───┼───┬─────────┐
  ▼   ▼   ▼         ▼
pdf/  llm/  sections/  renderers/
  │   │       │           │
  ▼   ▼       ▼           ▼
 OCR  AI    P-Note    Output files
    draft  C-Note
           Radar
           Timeline
                │
                ▼
            updaters/     # Write to data stores
```

## Modules

### `ai_research_os.py`
Main entry point. CLI argument parsing and orchestrator.

### `core/`
Shared utilities — date helpers, slug generators, cache.

### `parsers/`
- `arxiv.py` — arXiv ID/URL extraction and API fetching
- `doi.py` — DOI resolution via CrossRef
- `pdf.py` — Local PDF metadata extraction

### `pdf/`
- `extract.py` — MuPDF text extraction
- `ocr.py` — OCR for scanned PDFs via pymupdf

### `sections/`
- `pnote.py` — Paper Note structure
- `cnote.py` — Critical Note structure
- `radar.py` — Radar entry structure
- `timeline.py` — Timeline entry structure

### `llm/`
- `parse.py` — AI-assisted structured drafting

### `renderers/`
- `pnote.py`, `cnote.py`, `radar.py`, `timeline.py` — Output formatting

### `updaters/`
- `radar.py`, `timeline.py` — Append to YAML data stores

## Data Flow

1. **Parse** — Extract paper metadata from input
2. **Fetch** — Get full text via arXiv/DOI/MuPDF
3. **Structure** — Build P-Note + C-Note in memory
4. **Optionally draft** — AI generates structured draft (待核验)
5. **Render** — Write formatted output files
6. **Update** — Append to radar.yaml and timeline.yaml
