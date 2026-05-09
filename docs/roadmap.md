# Roadmap

> Where is Rairos going?

**Last updated:** May 2026
**Maintainer:** @shushuzn

---

## Vision

Rairos should be the **last research tool a PhD student or AI researcher ever needs** — a self-evolving partner that learns your research taste, surfaces what matters, and gets smarter every week.

The goal is not to be another PDF manager or reference manager. It's to be an **autonomous research intelligence** that:
- Reads papers so you don't have to read the wrong ones
- Connects ideas across your library automatically
- Surfaces gaps and opportunities in your research domain
- Generates code and experiments from papers

---

## Current State

**v1.5.4** — Core infrastructure is solid. The CLI is functional with 23 subcommands. Research loop, gap analysis, and RAG pipeline are operational.

Strengths:
- 73 tests, CI gate at 40% coverage
- arXiv/DOI/PDF import with OCR
- FTS5 + semantic (embedding) search
- Citation graph via OpenAlex
- TUI chat interface
- EvoSkill integration (benchmark-driven skill discovery)
- paper2code pipeline
- Gap clustering with hotspot trend analysis
- Contradiction timeline for paradigm shift detection
- Parallel multi-agent research with result merging
- GenePool saturation-aware adaptive scheduling
- Rich webhook notifications (Discord embeds, Feishu cards)
- Structured observability (JSON logging, correlation IDs, metrics)
- MCP tool fuzzing for regression testing

Gaps:
- Web UI is minimal (Streamlit dashboard exists but limited)
- No real product demo/showcase video
- No mobile/offline access
- Community presence limited

---

## Roadmap

### v2.0 — Community & Polish (Short Term)

Goal: Make the project **contributor-friendly** and **discoverable**.

- [x] **GitHub automation** (partial — CI working, issue templates needed)
- [ ] **Documentation** (ongoing)
  - [x] API reference page with examples
  - [x] Architecture overview
  - [ ] Video demo / GIF showcase
  - [x] Comparisons with Zotero, Notion, Semantic Scholar
  - [ ] Benchmark page (what the system measures)
- [ ] **First impressions**
  - [x] Professional README banner
  - [x] Logo redesign
  - [ ] Social preview images for GitHub links

### v2.1 — Web UI (Mid Term)

Goal: Make Rairos **accessible without CLI**.

- [x] Streamlit-based web dashboard (partial)
  - [x] Paper library browser
  - [x] Search interface
  - [x] Research gap visualizer with heatmap
  - [x] Chat interface (web version of `chat-tui`)
  - [x] Review generation endpoints
- [ ] Docker deployment
  - [ ] `docker-compose.yml` with Ollama, Milvus, and the app
  - [ ] One-command setup for non-technical users
- [ ] Authentication
  - [ ] Optional API key management
  - [ ] Session persistence for web

### v2.2 — Self-Evolution (Mid-Long Term)

Goal: Make the "self-evolving" part **real and visible**.

- [x] **Gene/Capsule system** — evolution mechanism operational
  - [x] Visual dashboard showing how the system learns (family tree)
  - [x] User feedback loop: thumbs up/down on suggestions
  - [x] Evolution log: what the system learned this week
- [x] **Research gap detection** — surface what's missing
  - [x] Automatic gap analysis across papers
  - [x] Gap clustering with hotspot trend analysis
  - [x] Generate research questions from gaps
  - [x] Paradigm shift detection via contradiction timeline
- [ ] **Weekly research digest**
  - [ ] Auto-generated summary of new papers
  - [ ] Changes in research landscape
  - [ ] Personalized recommendations

### v3.0 — Autonomous Research Agent (Long Term)

Goal: A system that does research **with minimal human input**.

- [x] **Autonomous research loop** — given a research question, the system:
  - [x] Searches literature
  - [x] Downloads and reads papers
  - [x] Generates hypotheses
  - [x] Designs experiments (paper2code)
  - [x] Reports findings
- [ ] **Multi-modal input**
  - [ ] Accept paper PDFs via email
  - [ ] Slack/Telegram bot interface
  - [x] arXiv RSS feed auto-import (subscribe command)
- [ ] **Knowledge graph visualization**
  - [ ] D3.js interactive graph
  - [ ] Filter by time, topic, citations
  - [ ] Path finding between concepts

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
| v1.5.4 | Released | Gap clustering, contradiction timeline, parallel research, GenePool saturation, rich webhooks, observability |
| v2.0 | In Progress | Community polish, web UI improvements, Docker |
| v2.2 | In Progress | Self-evolution dashboard, research digest |
| v3.0 | Future | Autonomous research agent, multi-modal input |

---

*This roadmap is a living document. Priorities shift based on user feedback and contributions.*
