# AI Research OS

**A Structured Research Operating System for Serious AI Researchers**

## What It Does

Feed it a paper (arXiv URL, DOI, or PDF). Get back a **P-Note**, **C-Note**, **Radar entry**, and **Timeline entry** — all structured, tagged, and cross-linked.

```bash
# One paper
python ai_research_os.py https://arxiv.org/abs/2601.00155 --tags LLM,Agent

# With AI draft (requires API key)
python ai_research_os.py https://arxiv.org/abs/2601.00155 --tags LLM --ai
```

This is **not a PDF manager**. It is a **Cognitive Upgrade System** that enforces structured thinking, explicit reasoning, and long-term research tracking.

## Input / Output

| Input | Output |
|-------|--------|
| arXiv URL/ID | P-Note + C-Note + Radar + Timeline |
| DOI | P-Note + C-Note + Radar + Timeline |
| Local PDF | P-Note + C-Note + Radar + Timeline |
| Scanned PDF | Same (via OCR) |
| `--ai` flag | + AI-structured draft (待核验) |

## Quick Start

```bash
pip install requests feedparser pymupdf
python ai_research_os.py https://arxiv.org/abs/2601.00155 --tags LLM
```

## Project Structure

```
ai_research_os/
  ai_research_os.py      # Main entry point
  core/                   # Core logic
  parsers/                # Input parsing (arXiv, DOI, PDF)
  pdf/                    # PDF extraction + OCR
  sections/               # Output sections (pnote, cnote, radar, timeline)
  llm/                    # AI-assisted drafting
  renderers/              # Output rendering
  updaters/               # Data store updates
  docs/                   # Documentation (MkDocs)
  tests/                  # Test suite
```

## License

Research-Only. See [LICENSE](https://github.com/shushuzn/ai_research_os/blob/main/LICENSE).
