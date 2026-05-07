# CLI Reference

All commands follow the pattern `python -m cli <subcommand> [options]`.

## Core Research

| Command | Description |
|---------|-------------|
| `research` | Automatically search arXiv, download PDFs, extract text, and generate AI summaries |
| `import` | Import papers by arXiv ID, DOI, or paper UID. Supports batch import from file |
| `ingest` | Full import pipeline: fetch metadata, run deep analysis, generate embeddings, and sync the knowledge graph |
| `search` | Search papers by query, year, source, tag, or parse status |
| `list` | List papers with filtering, sorting, and multiple output formats |
| `status` | Show database and queue status |

## Knowledge Graph

| Command | Description |
|---------|-------------|
| `kg` | Query, visualize, and manage the research knowledge graph |
| `cite-graph` | Build and visualize citation relationships |
| `citation-chain` | Build and visualize citation relationships |
| `citations` | Display papers cited by or citing a given paper |
| `cite-backfill` | Find papers without citation records and fetch forward citations from OpenAlex |
| `cite-import` | Import citations from file |
| `cite-fetch` | Fetch citation data for papers |
| `cite-stats` | Show citation statistics |

## Analysis

| Command | Description |
|---------|-------------|
| `gap` | Analyze papers to identify research gaps and suggest questions |
| `hypothesize` | Generate research hypotheses with experiment designs and risk assessments |
| `trend` | Analyze paper distribution, keyword trends, and citation velocity |
| `benchmark` | Detect and compare benchmark results across papers |
| `compare` | Compare multiple papers side-by-side |
| `analyze` | Orchestrate gap→trend→story→hypothesis analysis |
| `insight` | Extract and manage key insights from papers |
| `review` | Generate structured literature review for a research topic |
| `litreview` | Generate and maintain living literature reviews from subscription papers |
| `route` | Classify a research query and execute routed commands using LLM or embedding similarity |

## Chat & Interaction

| Command | Description |
|---------|-------------|
| `chat` | Ask questions about papers in your library with source citations |
| `chat-tui` | Launch a full-screen terminal chat interface for RAG-powered Q&A |
| `session` | Start, list, and manage research sessions for context-aware conversations |
| `ask` | Ask research questions with context awareness |
| `repl` | Interactive REPL for chained KG/paper exploration |

## Research Workflow

| Command | Description |
|---------|-------------|
| `roadmap` | Generate structured research roadmaps from questions and hypotheses |
| `story` | Generate narrative understanding from research papers |
| `pipeline` | Run gap analysis, generate hypotheses, and optionally create experiment records |
| `question` | Track and manage research questions from gap detection and manual entry |
| `journal` | Track research activities and thoughts |
| `narrative` | Aggregate all research state into narrative threads with phase tracking |
| `subscribe` | Subscribe to research topics and get AI-scored paper recommendations |

## Evolution & Gene Pool

| Command | Description |
|---------|-------------|
| `evoskill` | EvoSkill benchmark-driven skill discovery |
| `paper2code` | Generate code from paper |
| `rag` | RAG pipeline: paper → code → tests → benchmark |
| `lean` | Translate research hypotheses into Lean 4 code and verify them |
| `replicate` | Track paper replication attempts and results |
| `experiment` | Run and track experiments |

## Agents & Autonomy

| Command | Description |
|---------|-------------|
| `agent` | Run autonomous deep research agent with gap detection and session persistence |
| `daemon` | Start/stop autonomous research daemon that watches arXiv, runs gap detection, and evolves Gene Pool |
| `scout` | Search ArXiv for recent papers, score against Gene Pool capsules, recommend relevant ones |
| `signal` | Match live event keyword against historical capsules, cross-reference market data |
| `discover` | Cross-reference Gene Pool capsules, market data, and events to find correlations |

## Data Management

| Command | Description |
|---------|-------------|
| `queue` | Show and manage pending papers |
| `cache` | Manage paper metadata cache |
| `dedup` | Deduplicate papers by exact match |
| `dedup-semantic` | Deduplicate papers by semantic similarity |
| `merge` | Merge duplicate papers |
| `export` | Export papers in BibTeX or JSON format |
| `path` | Plan research reading order based on citation graph and relevance |
| `read-queue` | Manage paper reading queue |

## Financial & News

| Command | Description |
|---------|-------------|
| `jin10` | Real-time financial data via Jin10 MCP service |
| `intel` | Aggregate Jin10 news, market quotes, Gene Pool state, and academic papers |
| `digest` | Generate weekly research summary |
| `report` | Generate fresh report with latest Gene Pool data |
| `watch` | Watch papers.json and auto-rebuild KG on changes |

## Utilities

| Command | Description |
|---------|-------------|
| `stats` | Show database statistics |
| `validate` | Analyze research question novelty and feasibility |
| `influence` | Compute citation velocity = forward_citations / age_years |
| `slides` | Generate presentation slides from papers |
| `postprocess` | Run 6-stage research deep dive pipeline on a paper |
| `visual` | Visualize papers and knowledge graphs |
| `demo` | Load sample papers and Gene Pool capsules to explore Rairos |
| `friction` | Detect research friction points — failed commands, abandoned workflows |
| `argue` | Build structured arguments from evidence |

## Web & Dashboard

| Command | Description |
|---------|-------------|
| `dashboard` | Launch FastAPI web interface and open browser |
| `web` | Alias for dashboard |

## Quick Reference

```bash
# Install
pip install -e ".[all]"
rairos --help                    # Show all commands

# Paper operations
python -m cli import 2601.00155
python -m cli search "attention"
python -m cli list --status done

# Research
python -m cli research "RLHF alignment" --limit 5
python -m cli gap --papers "2601.00155"
python -m cli chat-tui

# Knowledge graph
python -m cli kg --query "transformer"
python -m cli cite-graph 2601.00155

# Gene Pool
python -m cli scout --topic "mixture of experts"
python -m cli daemon start
```
