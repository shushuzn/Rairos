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

**v1.5.2** — Core infrastructure is solid. The CLI is functional with 23 subcommands. Research loop and RAG pipeline exist but need polish.

Strengths:
- 3839 tests, 129 test files
- 100% pyflakes clean, ruff clean, mypy clean
- arXiv/DOI/PDF import with OCR
- FTS5 + semantic (embedding) search
- Citation graph via OpenAlex
- TUI chat interface
- EvoSkill integration (benchmark-driven skill discovery)
- paper2code pipeline

Gaps:
- No web UI (only CLI)
- No real product demo/showcase
- Minimal community presence (2 GitHub stars)
- No mobile/offline access
- LLM provider lock-in (OpenAI/DashScope)

---

## Roadmap

### v2.0 — Community & Polish (Short Term)

Goal: Make the project **contributor-friendly** and **discoverable**.

- [ ] **GitHub automation**
  - Issue templates (bug report, feature request)
  - PR template automation
  - Stale bot configuration
  - Auto-label PRs based on files changed

- [ ] **Documentation**
  - [ ] API reference page with searchable examples
  - [ ] Architecture deep-dive (how the self-evolution works)
  - [ ] Video demo / GIF showcase
  - [ ] Comparisons with Zotero, Notion, Semantic Scholar
  - [ ] Benchmark page (what the system measures)

- [ ] **First impressions**
  - [ ] Professional README banner
  - [ ] Logo redesign
  - [ ] Social preview images for GitHub links

- [ ] **Good first issues**
  - [ ] Label 10+ issues as `good first issue`
  - [ ] Create `help wanted` label for harder issues

---

### v2.1 — Web UI (Mid Term)

Goal: Make AI Research OS **accessible without CLI**.

- [ ] Streamlit-based web dashboard
  - Paper library browser
  - Search interface
  - Research gap visualizer
  - Chat interface (web version of `chat-tui`)

- [ ] Docker deployment
  - `docker-compose.yml` with Ollama, Milvus, and the app
  - One-command setup for non-technical users

- [ ] Authentication
  - Optional API key management
  - Session persistence for web

---

### v2.2 — Self-Evolution (Mid-Long Term)

Goal: Make the "self-evolving" part **real and visible**.

- [ ] **Gene/Capsule system** — make the evolution mechanism understandable
  - Visual dashboard showing how the system learns
  - User feedback loop: thumbs up/down on suggestions
  - Evolution log: what the system learned this week

- [ ] **Research gap detection** — surface what's missing
  - Automatic gap analysis across 3+ papers
  - Generate research questions from gaps
  - Trend forecasting: where is the field going?

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
| v2.0 | Planned | Community polish, web UI, Docker |
| v2.2 | Planned | Self-evolution dashboard, research gap detection |
| v3.0 | Future | Autonomous research agent |

---

*This roadmap is a living document. Priorities shift based on user feedback and contributions.*
