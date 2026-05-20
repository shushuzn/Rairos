# Rairos

<div align="center">

  <img src="logo_hero.svg" width="900" alt="Rairos Demo"/>

</div>

**A Self-Evolving Research Operating System** — 100% Rust (154 crates), 105 CLI commands, 69 MCP tools.

[![Build](https://github.com/shushuzn/Rairos/actions/workflows/rust.yml/badge.svg?branch=main)](https://github.com/shushuzn/Rairos/actions)
![Rust](https://img.shields.io/badge/rust-1.85+-orange.svg?logo=rust)
![Crates](https://img.shields.io/badge/crates-154-blue.svg)
![Lines](https://img.shields.io/badge/lines-116k%2B-green.svg)
![MCP](https://img.shields.io/badge/mcp_tools-69-blue.svg?logo=robot)
![CLI](https://img.shields.io/badge/cli_commands-105-blue.svg?logo=terminal)

## What It Does

Rairos is an **autonomous research assistant** that:

- **Reads papers** — arXiv, DOI, local PDF, scanned PDF (OCR)
- **Detects gaps** — identifies research opportunities across 36 AI topics
- **Learns your taste** — Gene Pool with credibility scoring encodes what you find valuable
- **Evolves automatically** — background daemon watches arXiv, analyzes gaps, evolves Gene Pool
- **Runs locally** — full local LLM support (Ollama), zero API costs

```
Feed it a paper → It learns what works → Next search is better
```

## Rust Stack (154 crates)

| Crate | Purpose |
|-------|---------|
| rairos-core | DB, FTS5, subscriptions, tags |
| rairos-llm | GenePool, Evolution, LLM clients |
| rairos-cli | 105 CLI commands |
| rairos-mcp | 69 MCP tools (JSON-RPC 2.0) |
| rairos-web | REST API + HTML frontend |
| rairos-kg | Knowledge graph, PageRank |
| rairos-research | DeepResearchAgent, gap detection |
| rairos-memory | Research stance tracking |
| rairos-insight-* (5 crates) | Evolution tracking, credibility scoring, storage |

Build: `make build` (uses parallel jobs + mold linker + ccache)

## Quick Start (Rust CLI)

```bash
git clone https://github.com/shushuzn/Rairos.git
cd Rairos

# First-time build (10-20 min)
make build

# Usage (pick one)
./rairos.sh search "transformer"     # Quick search
./rairos.sh gap "LLM efficiency"    # Detect research gaps
./rairos.sh --help                  # All 105 commands

# Or use make
make run CMD='list --status pending'
```

### With Ollama (local, free)

```bash
ollama pull qwen2.5
./rairos.sh gap "transformer efficiency"
```

## Documentation

| Doc | Description |
|-----|-------------|
| [Architecture](docs/architecture.md) | System design and module overview |
| [Configuration](docs/configuration.md) | LLM, DB, search, tool configuration |
| [Benchmarks](docs/benchmarks.md) | Performance metrics |
| [AGENTS.md](AGENTS.md) | Agent context for AI tools |

## API Gateway (Commercial)

The **Rairos API** provides programmatic access to research intelligence.

### Pricing

| Tier | Price | Requests/day |
|------|-------|--------------|
| Free | $0 | 100 |
| Pro | $29/mo | 10,000 |
| Team | $99/mo | 100,000 |
| Enterprise | $499/mo | Unlimited |

### Quick Start

```bash
# Install Python SDK (from source)
cd sdks/python && pip install -e . && cd ../..

# Or Node.js SDK (from source)
cd sdks/js && npm install && cd ../..
```

**Note**: SDK packages are available on PyPI and npm.

```python
from rairos import RairosClient

client = RairosClient(api_key="your-api-key")

# Search papers
papers = client.search_papers("machine learning")

# Detect research gaps
gaps = client.detect_gap("transformer efficiency")

# Check usage
usage = client.get_usage()
```

### API Documentation

| Resource | URL |
|----------|-----|
| Swagger UI | `http://localhost:8081/docs` |
| OpenAPI JSON | `http://localhost:8081/docs/openapi.json` |
| Full Docs | [docs/api/index.md](docs/api/index.md) |

See [API Documentation](docs/api/index.md) for full API reference.

### Deployment

See [Deployment Checklist](deploy/CHECKLIST.md) for production setup.

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
