# 🧠 AI Research OS

**A Structured Research Operating System for Serious AI Researchers**

[![Python](https://img.shields.io/badge/Python-3.9%2B-blue)](https://python.org)
[![Platform](https://img.shields.io/badge/Platform-Windows%20%7C%20Mac%20%7C%20Linux-lightgrey)](#)
[![Tests](https://img.shields.io/badge/Tests-372%20passing-brightgreen)](#-testing)
[![LLM](https://img.shields.io/badge/LLM-OpenAI%20Compatible-orange)](#-ai-assisted-draft)
[![License](https://img.shields.io/badge/License-Research--Only-purple)](#-license)

---

## TL;DR

Feed it a paper (arXiv URL, DOI, or PDF). Get back a **P-Note**, **C-Note**, **Radar entry**, and **Timeline entry** — all structured, tagged, and cross-linked. Optionally generate AI-assisted drafts.

```bash
# One paper
python ai_research_os.py https://arxiv.org/abs/2601.00155 --tags LLM,Agent

# With AI draft (requires API key)
python ai_research_os.py https://arxiv.org/abs/2601.00155 --tags LLM --ai
```

This is **not a PDF manager**. It is a **Cognitive Upgrade System** that enforces structured thinking, explicit reasoning, and long-term research tracking.

---

## What It Does

| Input | Output |
|-------|--------|
| arXiv URL/ID | P-Note + C-Note + Radar + Timeline |
| DOI | P-Note + C-Note + Radar + Timeline |
| Local PDF | P-Note + C-Note + Radar + Timeline |
| Scanned PDF | Same (via OCR) |
| `--ai` flag | + AI-structured draft (待核验) |

---

## Installation

### Dependencies

```bash
pip install requests feedparser pymupdf
```

### OCR (optional, for scanned PDFs)

```bash
pip install pytesseract pillow
```

**Windows**: Download Tesseract from [UB-Mannheim/tesseract](https://github.com/UB-Mannheim/tesseract/wiki) — add to PATH and install Chinese (`chi_sim`).

### AI Draft (optional)

Requires an OpenAI-compatible API key. See [API_CONFIG.md](API_CONFIG.md) for full configuration.

```bash
# Set environment variables
export OPENAI_API_KEY="sk-..."
export OPENAI_BASE_URL="https://dashscope.aliyuncs.com/compatible-mode/v1"

# Or pass directly as arguments (highest priority)
python ai_research_os.py <input> --api-key "sk-..." --base-url "https://..." --model "qwen3.5-plus"
```

---

## Usage Examples

### arXiv Paper

```bash
python ai_research_os.py https://arxiv.org/abs/2601.00155 --tags LLM,Agent
python ai_research_os.py 2601.00155 --tags LLM
```

### DOI

```bash
python ai_research_os.py 10.48550/arXiv.2601.00155 --tags LLM
```

### Local PDF

```bash
python ai_research_os.py test --pdf "paper.pdf" --tags RAG
python ai_research_os.py test --pdf "scanned.pdf" --ocr --ocr-lang chi_sim+eng
```

### With AI Draft

```bash
python ai_research_os.py https://arxiv.org/abs/2601.00155 --tags LLM --ai
```

---

## Project Structure

```
ai_research_os/
├── core/              # Paper dataclass, constants, today_iso
├── parsers/           # arXiv fetch, Crossref fetch, DOI/arXiv detection
├── pdf/               # download, extract (PyMuPDF + OCR + pdfminer)
├── sections/          # section segmentation + formatting
├── llm/               # OpenAI-compatible client, AI draft generation
├── renderers/         # P-Note, C-Note, M-Note rendering
├── notes/             # frontmatter, tag inference, note collection
├── updaters/          # Radar heat tracking, Timeline
├── cli.py             # CLI entry point + argparse
└── tests/             # 372 tests
```

---

## Research Tree Output

Papers are organized into a 12-directory structure:

```
00-Radar/          # Topic heat tracking
01-Foundations/    # Foundational papers
02-Models/         # Model papers
03-Applications/   # Application papers
04-Evaluation/     # Evaluation methods
05-Tools/          # Tools & libraries
06-Theory/         # Theory papers
07-Architecture/   # System architecture
08-Training/       # Training methods
09-Data/           # Datasets
10-Applications/   # Applied research
11-Future-Directions/
```

---

## Knowledge Evolution Logic

```
Paper → P-Note (paper note)
  → C-Note (concept note, per tag)
  → M-Note (comparison note, when 3+ papers share same tag)
  → Radar (topic frequency heat score)
  → Timeline (year-based evolution)
```

---

## CLI Reference

| Argument | Description | Default |
|----------|-------------|---------|
| `--pdf <path>` | Use local PDF | - |
| `--ocr` | Enable OCR fallback | off |
| `--ocr-lang <lang>` | OCR language | `chi_sim+eng` |
| `--max-pages <n>` | Limit parsed pages | unlimited |
| `--ai` | Enable AI draft generation | off |
| `--model <name>` | LLM model name | `qwen3.5-plus` |
| `--base-url <url>` | API endpoint | DashScope compatible |
| `--api-key <key>` | API key | env `OPENAI_API_KEY` |
| `--tags <t1,t2>` | Comma-separated tags | auto-inferred |

---

## Testing

```bash
PYTHONHOME=/c/Users/adm/AppData/Local/Programs/Python/Python312 \
  .venv/Scripts/python.exe -m pytest tests/ -q
```

**Current**: 372 tests passing, 1 skipped.

---

## Research Philosophy

This system enforces:

- Structured thinking (P/C/M-Notes)
- Explicit reasoning (frontmatter fields)
- Comparison-based insight (M-Notes)
- Long-term tracking (Radar + Timeline)
- Decision logging (evolution logs)
- Cognitive iteration (periodic M-Note revision)

---

## Recommended Workflow

1. Read 1 paper daily
2. Assign 1–3 tags
3. Weekly check Radar
4. Auto-trigger M-Notes when 3+ papers share a tag
5. Quarterly review Timeline
6. Periodically revise M-Notes

---

## Roadmap

- Citation graph extraction
- Auto experiment table parsing
- Embedding-based search
- Knowledge graph building
- Trend prediction
- Research momentum scoring

---

## License

Research & educational use only.
