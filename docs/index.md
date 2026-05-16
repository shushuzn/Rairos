# AI Research OS

**A Self-Evolving Research Operating System for AI Researchers** · 100% Rust (154 crates)

AI Research OS is a local-first research tool that grows smarter over time. It learns your research patterns, surfaces what matters, and generates insights from your paper library.


![Rust](https://img.shields.io/badge/Rust-100%25-orange)
[![Build](https://github.com/shushuzn/Rairos/actions/workflows/rust.yml/badge.svg?branch=main)](https://github.com/shushuzn/Rairos/actions)
[![License: GPL v3](https://img.shields.io/badge/License-GPL%20v3-blue.svg)](https://www.gnu.org/licenses/gpl-3.0)

## What It Does

Feed it a paper — get back structured, cross-linked research knowledge:

| Input | Output |
|-------|--------|
| arXiv URL/ID | P-Note + C-Note + Radar + Timeline |
| DOI | P-Note + C-Note + Radar + Timeline |
| Local PDF | P-Note + C-Note + Radar + Timeline |
| Scanned PDF | Same (via OCR) |

## Core Philosophy

**Not a PDF manager.** A *self-evolving research partner* that:

- Learns from your research patterns
- Improves answers over time
- Adapts to your specific domain
- Surfaces gaps and opportunities

## Quick Start

```bash
CARGO_BUILD_JOBS=1 cargo build --workspace

# Import a paper
rairos import 2601.00155 --tags LLM,Agent

# Search your library
rairos search "attention mechanism" --tag LLM

# Autonomous research
rairos research "RLHF alignment" --limit 5

# Chat with your papers
rairos chat-tui
```

See [Installation](installation.md) for full setup instructions.

## Key Features

### Key New Capabilities

- **Adaptive Scheduling** — GenePool saturation-aware research interval
- **Gap Clustering** — Semantic clustering of research gaps with hotspot trend analysis
- **Contradiction Timeline** — Detect paradigm shifts from conflicting paper claims
- **Impact Tracking** — Quantified research impact: novelty × depth × strength × speed
- **Parallel Research** — Multi-agent concurrent gap analysis with result merging
- **Topic Discovery** — Gap-density-based intelligent subscription suggestions
- **Rich Webhooks** — Discord embeds + Feishu cards for gap and paradigm shift alerts
- **Structured Observability** — JSON logging with correlation IDs and event tracking

### 104 CLI Commands

- `import` — Bulk import from arXiv, DOI, PDF
- `search` — Full-text search with BM25 ranking
- `chat-tui` — Full-screen TUI chat with paper context
- `kg` — Knowledge graph query and rebuild
- `gap` — Detect research gaps, generate research questions
- `rag` — RAG pipeline: paper analysis → code generation → testing
- `benchmark` — Cross-paper benchmark with charts
- `pipeline` — Full research pipeline: gap → hypothesis → experiment
- `subscribe` — RSS-style paper feed by tag/query

### Research Knowledge Structure

```
Paper → P-Note (per paper)
      → C-Note (per concept/tag)
      → M-Note (comparison when 3+ papers share a tag)
      → Radar (topic frequency heat score)
      → Timeline (year-based evolution)
```

### Integrations

- **arXiv** — Direct import by ID or URL
- **OpenAlex** — Citation graph (forward + backward)
- **Ollama** — Local embeddings (nomic-embed-text, 768-dim)
- **DashScope / OpenAI** — AI draft generation
- **EvoSkill** — Benchmark-driven skill discovery
- **Streamlit** — Optional web dashboard

## Project Status

| Metric | Value |
|--------|-------|
| Tests | 3800+ across 154 crates |
| Rust | 1.81+ |
| License | GPL v3 |
| Version | 1.5.4 |

## Resources

- [Usage Guide](usage.md) — Full command reference
- [Architecture](architecture.md) — System design
- [Configuration](configuration.md) — Environment variables
- [Contributing](../contributing.md) — How to contribute
- [Roadmap](../roadmap.md) — Where we're going
- [GitHub](https://github.com/shushuzn/Rairos)

## License

GNU General Public License v3.0 — see [LICENSE](../LICENSE) for details.
