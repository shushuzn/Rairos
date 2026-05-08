# Rairos

<div align="center">

  <img src="logo_hero.svg" width="900" alt="Rairos Demo"/>

</div>

**A Self-Evolving Research Operating System that learns from your feedback to find better research directions over time.**

[![Python](https://img.shields.io/badge/Python-3.10%2B-blue)](https://python.org)
[![PyPI Version](https://img.shields.io/pypi/v/rairos)](https://pypi.org/project/rairos/)
[![Tests](https://github.com/shushuzn/Rairos/actions/workflows/ci.yml/badge.svg?branch=main)](https://github.com/shushuzn/Rairos/actions)
[![Coverage](https://img.shields.io/badge/coverage-75%25-green)](https://github.com/shushuzn/Rairos/actions/workflows/ci.yml)
[![License](https://img.shields.io/badge/License-GPL--3.0--or--later-orange)](#license)

## What It Does

Rairos is an **autonomous research assistant** that:

- **Reads papers** — arXiv, DOI, local PDF, scanned PDF (OCR)
- **Detects gaps** — identifies research opportunities across 36 AI topics
- **Learns your taste** — preference learning + Gene Pool encodes what you find valuable
- **Evolves automatically** — background daemon watches arXiv, analyzes gaps, evolves its knowledge
- **Runs locally** — Ollama support means zero API costs, fully private

```
Feed it a paper → It learns what works → Next search is better
```

## Quick Start

### Option 1: pip

```bash
pip install rairos
rairos import 2601.00155 --tags LLM,Agent
```

### Option 2: Docker (with Ollama for free local LLM)

```bash
git clone https://github.com/shushuzn/Rairos.git
cd Rairos
docker compose up --build
# Open http://localhost:8501
```

### Option 3: From source

```bash
git clone https://github.com/shushuzn/Rairos.git
cd Rairos
pip install -e ".[all]"
rairos --help
```

## Core Commands

```bash
rairos import 2601.00155              # Import papers
rairos gap "reinforcement learning"   # Detect research gaps
rairos research "RLHF alignment"      # Autonomous research loop
rairos daemon start                   # Start background autopilot
rairos daemon status                  # Check daemon status
rairos daemon evolve                  # Run evolution cycle manually
```

### Using Ollama (local, free, no API key)

```bash
# Start Ollama, pull a model, then use it with rairos
ollama pull qwen2.5
rairos gap "transformer efficiency" --model ollama/qwen2.5

# Or set as default
export AIROS_DEFAULT_MODEL_CLI=ollama/qwen2.5
rairos research "attention mechanisms"
```

### Web UI

```bash
# Start the web interface
uvicorn web.app:app --port 8501
# Open http://localhost:8501
```

Or with Docker: `docker compose up` → http://localhost:8501

## Key Features

| Feature | CLI | Web UI |
|---------|-----|--------|
| Paper import (arXiv/DOI/PDF) | `rairos import` | Import page |
| Research gap detection | `rairos gap` | Gap analysis |
| Deep research agent | `rairos agent deep-research` | Research Loop |
| Gene Pool evolution | `rairos daemon evolve` | Evolution Log |
| Credibility scoring | `rairos daemon status` | Credibility page |
| Source trust scores | `rairos daemon status` | Trust Scores |
| Paper2Code pipeline | `rairos paper2code` | Paper2Code page |
| Citation chain analysis | `rairos citation-chain` | Citation Chain |
| Background autopilot | `rairos daemon start` | Daemon dashboard |
| Chat (TUI) | `rairos chat-tui` | Chat page |
| Insight cards | `rairos insight` | Insights page |

## Architecture

```
arXiv Papers → GapAnalyzerV2 → Gene Pool (CapsuleGene encoding)
                                      ↑
DeepResearch Agent ← Gene Pool Guide ← Preference Profile
         ↑
    Search → Extract → Analyze → Reflect → Encode

Daemon (autopilot):
    subscription watch → gap analysis → evolution cycle → credibility scoring
```

The Gene Pool is stored in SQLite (WAL mode) with indexed lookups. Credibility scoring detects trendslop capsules (keyword overlap > 70%). Source trust tracking scores arXiv categories by capsule quality history.

## Documentation

| Doc | Description |
|-----|-------------|
| [Architecture](docs/architecture.md) | System design and module overview |
| [Configuration](docs/configuration.md) | LLM, DB, search, tool configuration |
| [Benchmarks](docs/benchmarks.md) | Performance metrics and test coverage |
| [Roadmap](ROADMAP.md) | Project roadmap |
| [Contributing](CONTRIBUTING.md) | How to contribute |
| [Usage Examples](USAGE_EXAMPLES.md) | Detailed command examples |

## License

GPL-3.0-or-later. See [LICENSE](LICENSE) for details.

---

<div align="center">

<a href="https://www.star-history.com/#shushuzn/Rairos&Date">

 <picture>
   <source media="(prefers-color-scheme: dark)" srcset="https://api.star-history.com/svg?repos=shushuzn/Rairos&type=Date&theme=dark" />
   <source media="(prefers-color-scheme: light)" srcset="https://api.star-history.com/svg?repos=shushuzn/Rairos&type=Date" />
   <img alt="Rairos Star History" src="https://api.star-history.com/svg?repos=shushuzn/Rairos&type=Date" style="width: 80%; height: auto;" />
 </picture>

</a>

</div>

