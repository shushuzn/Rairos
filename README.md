# Rairos

<div align="center">
  <img src="logo_hero.svg" width="900" alt="Rairos Demo"/>
</div>

**A Self-Evolving Research Operating System that learns from your feedback to find better research directions over time.**

[![Python](https://img.shields.io/badge/Python-3.9%2B-blue)](https://python.org)
[![PyPI Version](https://img.shields.io/pypi/v/rairos)](https://pypi.org/project/rairos/)
[![Coverage](https://img.shields.io/codecov/c/github/shushuzn/Rairos/main?logo=codecov)](https://app.codecov.io/gh/shushuzn/Rairos)
[![Tests](https://github.com/shushuzn/Rairos/actions/workflows/ci.yml/badge.svg?branch=main)](https://github.com/shushuzn/Rairos/actions)
[![License](https://img.shields.io/badge/License-GPL--3.0--or--later-orange)](#license)

## What It Does

Rairos detects research gaps from arXiv papers, encodes successful patterns in a **Gene Pool**, and uses **preference learning** to rank gaps you're most likely to find valuable. The more you use it, the better it gets.

```
Feed it a paper ¡ú It learns what works ¡ú Next search is better
```

| Input | Output |
|---|---|
| arXiv URL/ID | CapsuleGene + Briefing + Gene Pool |
| DOI | CapsuleGene + Briefing + Gene Pool |
| Local PDF | CapsuleGene + Briefing + Gene Pool |
| Scanned PDF | Same (via OCR) |

## How It Learns

1. **Gap Detection** ¡ª scans papers for research gaps across 36 AI topics
2. **Gene Pool** ¡ª encodes successful patterns as retrievable capsules (116 capsules, avg score 0.71)
3. **Preference Learning** ¡ª your accept/reject feedback shifts gap rankings (40% weight)
4. **Deep Research Agent** ¡ª queries Gene Pool to reformulate search queries automatically

## Quick Start

```bash
pip install rairos
airos-cli 2601.00155 --tags LLM,Agent
```

### Core Commands

```bash
airos-cli import 2601.00155 10.1038/nature12373   # Import papers
airos-cli gap "reinforcement learning" --limit 5   # Detect research gaps
airos-cli research "RLHF alignment" --limit 5     # Autonomous research loop
airos-cli paper2code 2106.09685                    # Paper ¡ú code ¡ú tests
```

### Preference Learning

```bash
airos-cli gap accept --topic "state space models" --gap-type "method_limitation"
airos-cli insight rate --card <id> --stars 5
airos-cli gene-pool --stats   # See your learned patterns
```

## Architecture

```
arXiv Papers ¡ú GapAnalyzerV2 ¡ú Gene Pool (CapsuleGene encoding)
                                      ¡ý
DeepResearch Agent ¡û GenePoolGuide ¡û Preference Profile
         ¡ý
    Search ¡ú Extract ¡ú Analyze ¡ú Reflect ¡ú Encode
```

## Installation

```bash
pip install rairos
```

Or from source:

```bash
git clone https://github.com/shushuzn/Rairos.git
cd Rairos
pip install -e .
```

## Documentation

| Doc | Description |
|-----|-------------|
| [Architecture](docs/architecture.md) | System design and module overview |
| [Configuration](docs/configuration.md) | LLM, DB, Search, Tool configuration |
| [Benchmarks](docs/benchmarks.md) | Performance metrics and test coverage |
| [Contributing](CONTRIBUTING.md) | How to contribute |
| [Roadmap](ROADMAP.md) | Project roadmap |

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
