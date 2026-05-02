# Roadmap

> Where is AI Research OS going?

**Last updated:** May 2026
**Maintainer:** @shushuzn

---

## Vision

AI Research OS should be the **last research tool a PhD student or AI researcher ever needs** — a self-evolving partner that learns your research taste, surfaces what matters, and gets smarter every week.

The goal is not to be another PDF manager or reference manager. It's to be an **autonomous research intelligence** that:
- Reads papers so you don't have to read the wrong ones
- Connects ideas across your library automatically
- Surfaces gaps and opportunities in your research domain
- Generates code and experiments from papers

---

## Current State

**v1.7** — Gene Pool ecosystem expanded. FastAPI hand-drawn web UI, Gene/Capsule self-evolution system, CLI unified to subcommands.

Strengths:
- 3839 tests, 129 test files
- 100% pyflakes clean, ruff clean, mypy clean
- arXiv/DOI/PDF import with OCR
- FTS5 + semantic (embedding) search
- Citation graph via OpenAlex
- TUI chat interface
- EvoSkill integration (benchmark-driven skill discovery)
- paper2code pipeline
- Gene Pool + CapsuleGene lifecycle (consumed闭环, capsule merge, auto-archive)
- FastAPI hand-drawn web UI (app_new.py)
- `airos-cli gap list/extract/watch/path/contradictions` for Gene Pool
- `polarity` field + contradiction detection (opposite polarity + same gap_type)
- Citation Pathfinder — trace citation chain from paper to Gene Pool capsule
- arXiv Auto-Import — `gap watch` monitors feed for Gene Pool matches

Gaps:
- No real product demo/showcase
- Minimal community presence
- No mobile/offline access
- LLM provider lock-in (OpenAI/DashScope)

---

## Roadmap

### v2.0 — Community & Polish (Short Term)

Goal: Make the project **contributor-friendly** and **discoverable**.

- [x] **GitHub automation**
  - [x] FUNDING.yml
  - [x] PR template automation
  - [x] Issue templates (bug report, feature request)
  - [x] Stale bot configuration

- [ ] **Documentation**
  - [x] Architecture deep-dive (Gene Pool lifecycle in docs/architecture.md)
  - [x] API reference page with searchable examples
  - [ ] Video demo / GIF showcase
  - [x] Comparisons with Zotero, Notion, Semantic Scholar
  - [x] Benchmark page (what the system measures)

- [x] **First impressions**
  - [x] Professional README banner
  - [x] Logo redesign (hexagonal R, hand-drawn SVG)
  - [x] Social preview images for GitHub links

- [ ] **Good first issues**
  - [ ] Label 10+ issues as `good first issue`
  - [ ] Create `help wanted` label for harder issues

---

### v2.1 — Web UI (Mid Term)

Goal: Make AI Research OS **accessible without CLI**.

- [x] FastAPI web dashboard (app_new.py)
  - [x] Paper library browser
  - [x] Research gap visualizer
  - [x] Hand-drawn aesthetic (SVG sketch filter, Caveat/Lora fonts)
  - [ ] Chat interface (web version of `chat-tui`)
  - [ ] Docker deployment

- [ ] Authentication
  - Optional API key management
  - Session persistence for web

---

### v2.2 — Self-Evolution (Mid-Long Term)

Goal: Make the "self-evolving" part **real and visible**.

- [x] **Gene/Capsule system** — evolution mechanism live
  - [x] Gene Pool dual-store (gene_pool.jsonl + capsules.json)
  - [x] CapsuleGene lifecycle: active → consumed/archived
  - [x] consumed 闭环 (source_cap_id on suggestions)
  - [x] Capsule merge (Jaccard ≥ 0.80)
  - [x] Auto-archive (low_score_streak ≥ 3)
  - [ ] Visual dashboard showing how the system learns
  - [ ] Evolution log: what the system learned this week

- [ ] **Research gap detection** — surface what's missing
  - [x] Gap extraction from papers (LLM-based, paper_gap_extractor.py)
  - [ ] Automatic gap analysis across 3+ papers
  - [ ] Generate research questions from gaps
  - [ ] Trend forecasting: where is the field going?

- [ ] **Weekly research digest**
  - Auto-generated summary of new papers
  - Changes in research landscape
  - Personalized recommendations

---

### v3.0 — Autonomous Research Agent (Long Term)

Goal: A system that does research **with minimal human input**.

- [ ] **Autonomous research loop** — given a research question, the system:
  - Searches literature
  - Downloads and reads papers
  - Generates hypotheses
  - Designs experiments (paper2code)
  - Reports findings

- [ ] **Multi-modal input**
  - Accept paper PDFs via email
  - Slack/Telegram bot interface
  - arXiv RSS feed auto-import

- [ ] **Knowledge graph visualization**
  - D3.js interactive graph
  - Filter by time, topic, citations
  - Path finding between concepts

---

## How to Influence the Roadmap

1. **Star the repo** — tells us what matters to users
2. **Open issues** — bug reports and feature requests drive priorities
3. **Contribute code** — see [CONTRIBUTING.md](CONTRIBUTING.md)
4. **Share your workflow** — how you use the tool helps us understand real needs

---

## Version History

| Version | Status | Key Additions |
|---------|--------|---------------|
| v1.0 | Released | Basic import, search, P-Note generation |
| v1.3 | Released | C-Note, Radar, Timeline, citation graph |
| v1.5 | Released | Chat TUI, semantic search, EvoSkill pipeline |
| v1.6 | Released | Gene Pool, CapsuleGene lifecycle, FastAPI web UI, hand-drawn aesthetic |
| v1.7 | Released | Gene Pool arXiv watch (`gap watch`), Contradiction Detection, Citation Pathfinder |
| v2.0 | Planned | Community polish, Docker, API reference |
| v2.2 | Planned | Self-evolution dashboard, research gap detection |
| v3.0 | Future | Autonomous research agent |

---

*This roadmap is a living document. Priorities shift based on user feedback and contributions.*
