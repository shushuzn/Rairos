# Rairos

<div align="center">

  <img src="logo_hero.svg" width="900" alt="Rairos Demo"/>

</div>

**A Self-Evolving Research Operating System** — 100% Rust (154 crates), 104 CLI commands, 68 MCP tools.

[![Build](https://github.com/shushuzn/Rairos/actions/workflows/rust.yml/badge.svg?branch=main)](https://github.com/shushuzn/Rairos/actions)
![Rust](https://img.shields.io/badge/rust-1.85+-orange.svg?logo=rust)
![Crates](https://img.shields.io/badge/crates-154-blue.svg)
![Lines](https://img.shields.io/badge/lines-73k%2B-green.svg)
![MCP](https://img.shields.io/badge/mcp_tools-68-blue.svg?logo=robot)
![CLI](https://img.shields.io/badge/cli_commands-104-blue.svg?logo=terminal)

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
| rairos-cli | 104 CLI commands |
| rairos-mcp | 68 MCP tools (JSON-RPC 2.0) |
| rairos-web | REST API + HTML frontend |
| rairos-kg | Knowledge graph, PageRank |
| rairos-research | DeepResearchAgent, gap detection |
| rairos-memory | Research stance tracking |
| rairos-insight-* (5 crates) | Evolution tracking, credibility scoring, storage |

Build: `CARGO_BUILD_JOBS=1 cargo build`

## Quick Start (Rust CLI)

```bash
git clone https://github.com/shushuzn/Rairos.git
cd Rairos
CARGO_BUILD_JOBS=1 cargo build
cargo run -p rairos-cli -- --help
```

### With Ollama (local, free)

```bash
ollama pull qwen2.5
cargo run -p rairos-cli -- gap "transformer efficiency"
```

## Documentation

| Doc | Description |
|-----|-------------|
| [Architecture](docs/architecture.md) | System design and module overview |
| [Configuration](docs/configuration.md) | LLM, DB, search, tool configuration |
| [Benchmarks](docs/benchmarks.md) | Performance metrics |
| [AGENTS.md](AGENTS.md) | Agent context for AI tools |

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
