# Comparisons

> How AI Research OS stacks up against popular research tools.

**Last updated:** May 2026

---

## vs. Zotero

| Feature | Zotero | AI Research OS |
|---------|--------|----------------|
| Paper import | arXiv, DOI, PDF | arXiv, DOI, PDF, scanned PDF (OCR) |
| Metadata | Manual or DOI lookup | Automatic via OpenAlex |
| Full-text search | Plugin-dependent | FTS5 + semantic (Ollama embeddings) |
| Citation graph | Limited | OpenAlex citation graph with depth-2 traversal |
| Note structure | Free-form | P-Note, C-Note, M-Note, Radar, Timeline |
| Research memory | None | Gene Pool — self-evolving gap tracker |
| Gap detection | None | LLM-powered gap extraction + contradiction detection |
| Citation paths | None | Citation Pathfinder to Gene Pool |
| arXiv watch | None | `gap watch` with Gene Pool matching |
| Evolution system | None | CapsuleGene lifecycle with consumed闭环 |
| Web UI | None | FastAPI hand-drawn web UI |
| CLI | None | 105 commands |

**Zotero wins:** Mature ecosystem, browser connector, large community, mobile apps.
**AI Research OS wins:** Autonomous gap detection, self-evolving Gene Pool, citation pathfinding.

---

## vs. Notion

| Feature | Notion | AI Research OS |
|---------|--------|----------------|
| Paper management | Manual page creation | Automatic from arXiv/DOI/PDF |
| Citation graph | None | OpenAlex-backed citation network |
| Research memory | Manual tagging | Gene Pool with auto-archival |
| Gap detection | None | LLM-powered |
| Semantic search | Basic AI blocks | Ollama embedding similarity |
| Structured output | Templates | P-Note / C-Note / M-Note / Radar / Timeline |
| arXiv import | Manual paste | One-command import |
| Evolution system | None | CapsuleGene lifecycle |
| Web UI | Notion-hosted | Self-hosted FastAPI |

**Notion wins:** Beautiful UI, collaboration, flexible databases, API integrations.
**AI Research OS wins:** Purpose-built for research loops, autonomous gap detection, no manual curation.

---

## vs. Semantic Scholar

| Feature | Semantic Scholar | AI Research OS |
|---------|-----------------|----------------|
| Paper corpus | 200M+ papers | Your local library |
| Citation graph | Global | Local + OpenAlex for references |
| Gap detection | None | LLM-powered gap extraction |
| Research memory | None | Gene Pool — persistent across sessions |
| Self-evolution | None | CapsuleGene lifecycle + merge + auto-archive |
| arXiv watch | RSS only | Gene Pool-aware matching |
| Contradiction detection | None | Polarity + gap_type pair detection |
| Citation pathfinding | Paper pages | Citation Pathfinder to Gene Pool |
| Local data | None | All data in `~/.ai_research_os/` |
| Extensible | No | Open architecture, Rust-based pipeline |

**Semantic Scholar wins:** Massive corpus, web accessibility, free.
**AI Research OS wins:** Local-first, autonomous gap detection, self-evolving memory, Gene Pool.

---

## vs. Obsidian

| Feature | Obsidian | AI Research OS |
|---------|----------|----------------|
| Paper import | Manual | Automatic from arXiv/DOI/PDF |
| Citation graph | Plugin-dependent | OpenAlex-backed |
| Note structure | Free-form vault | P-Note / C-Note / M-Note / Radar / Timeline |
| Research memory | Manual linking | Gene Pool with autonomous tracking |
| Semantic search | Local REST API plugin | Ollama embeddings |
| Gap detection | None | LLM-powered |
| arXiv watch | Manual | `gap watch` with daemon mode |
| Evolution system | None | CapsuleGene lifecycle |
| Web UI | None | FastAPI hand-drawn UI |

**Obsidian wins:** Mature PKM ecosystem, graph view, plugins, mobile support.
**AI Research OS wins:** Purpose-built for research, autonomous gap detection, no manual linking.

---

## vs. Connected Papers

| Feature | Connected Papers | AI Research OS |
|---------|-----------------|----------------|
| Visualization | Origin papers + prior/subsequent | Full citation graph via OpenAlex |
| Gap detection | None | LLM-powered |
| Research memory | None | Gene Pool |
| Pathfinding | Visual exploration | Citation Pathfinder to Gene Pool |
| arXiv watch | None | Gene Pool-aware monitoring |
| Local data | None | `~/.ai_research_os/` |

**Connected Papers wins:** Beautiful visual graph, instant web access.
**AI Research OS wins:** Deeper analysis, Gene Pool integration, local-first.

---

## Summary

AI Research OS is most differentiated on:

1. **Self-evolving Gene Pool** — no other tool tracks research gaps persistently and evolves them
2. **Contradiction Detection** — surfaces opposing findings across your library
3. **Citation Pathfinder** — traces citation chains to Gene Pool entries, not just "papers like this"
4. **Autonomous gap extraction** — LLM-powered, no manual labeling required
5. **arXiv Auto-Import** — `gap watch` monitors new uploads against Gene Pool in daemon mode

If you need a mature, collaborative reference manager → **Zotero**
If you need a beautiful PKM with full control → **Obsidian**
If you need massive global citation data → **Semantic Scholar**
If you need an autonomous research partner that learns → **AI Research OS**

---

*To update this page, edit `docs/comparisons.md`.*
