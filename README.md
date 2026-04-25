# AI Research OS

**A Structured Research Operating System for Serious AI Researchers**

[![Python](https://img.shields.io/badge/Python-3.9%2B-blue)](https://python.org)
[![Tests](https://github.com/shushuzn/ai_research_os/actions/workflows/ci.yml/badge.svg?branch=main)](https://github.com/shushuzn/ai_research_os/actions)
[![License](https://img.shields.io/badge/License-Research--Only-purple)](#license)

## What It Does

Feed it a paper (arXiv URL, DOI, or PDF). Get back a **P-Note**, **C-Note**, **Radar entry**, and **Timeline entry** — all structured, tagged, and cross-linked.

| Input | Output |
|---|---|
| arXiv URL/ID | P-Note + C-Note + Radar + Timeline |
| DOI | P-Note + C-Note + Radar + Timeline |
| Local PDF | P-Note + C-Note + Radar + Timeline |
| Scanned PDF | Same (via OCR) |

This is **not a PDF manager**. It is a **Cognitive Upgrade System** that enforces structured thinking, explicit reasoning, and long-term research tracking.

## Quick Start

```bash
pip install requests feedparser pymupdf
```

### One paper

```bash
# arXiv URL
python -m cli 2601.00155 --tags LLM,Agent

# DOI
python -m cli 10.48550/arXiv.2601.00155 --tags LLM

# Local PDF (with OCR for scanned PDFs)
python -m cli test --pdf "paper.pdf" --tags RAG
python -m cli test --pdf "scanned.pdf" --ocr --ocr-lang chi_sim+eng
```

### Three core commands

```bash
python -m cli import 2601.00155 10.1038/nature12373   # Add papers to DB
python -m cli search "attention mechanism" --tag LLM     # Search papers
python -m cli research "RLHF alignment" --limit 5        # Autonomous research loop
```

### AI draft (optional)

```bash
export OPENAI_API_KEY="sk-..."
export OPENAI_BASE_URL="https://dashscope.aliyuncs.com/compatible-mode/v1"

python -m cli 2601.00155 --tags LLM --ai
```

For full configuration, see [API_CONFIG.md](API_CONFIG.md).

## Research Tree

Papers are organized into 12 directories:

```
00-Radar/            Topic heat tracking
01-Foundations/      Foundational papers
02-Models/           Model papers
03-Training/         Training methods
04-Scaling/         Scaling laws
05-Alignment/        Alignment research
06-Agents/           Agent systems
07-Infrastructure/    Infrastructure
08-Optimization/     Optimization techniques
09-Evaluation/       Evaluation methods
10-Applications/     Applied research
11-Future-Directions/
```

## Knowledge Evolution

```
Paper → P-Note (paper note, per paper)
      → C-Note (concept note, per tag)
      → M-Note (comparison note, when 3+ papers share same tag)
      → Radar (topic frequency heat score)
      → Timeline (year-based evolution)
```

## Recommended Workflow

1. Read 1 paper daily
2. Assign 1–3 tags
3. Weekly check Radar
4. Auto-trigger M-Notes when 3+ papers share a tag
5. Quarterly review Timeline

## All Commands

Run `python -m cli <command> --help` for any command. Key commands:

| Command | Description |
|---|---|
| `import` | Add papers by arXiv ID / DOI / UID |
| `search` | Full-text search with filters |
| `list` | List papers with sort/filter/export |
| `research` | Autonomous research loop (search → download → extract → AI summarize) |
| `stats` | DB overview |
| `export` | Export DB to CSV or JSON |
| `citations` | Show citation relationships |
| `kg` | Knowledge graph query and rebuild |
| `similar` | Find semantically similar papers (requires Ollama) |
| `dedup` | Find exact duplicates |
| `dedup-semantic` | Semantic deduplication (requires Ollama) |
| `paper2code` | Generate code from paper |
| `rag` | RAG pipeline (paper2code + tests + benchmark) |

For the complete command reference, see [ADVANCED_COMMANDS.md](ADVANCED_COMMANDS.md).
For Chinese documentation, see [README.zh-CN.md](README.zh-CN.md).

## Shell Completions

Install tab completion for your shell:

```bash
# Fish
cp completions/fish ~/.config/fish/completions/airos.fish

# Zsh
cp completions/zsh _airos ~/.zsh/completion/_airos

# Bash
source completions/bash
# Or: cp completions/bash /etc/bash_completion.d/airos
```

After installation, `airos <Tab>` and `python -m cli <Tab>` will show all 23 subcommands with descriptions.

## Testing

```bash
python -B -m pytest tests/ -q
```

## License {#license}

Research & educational use only.
