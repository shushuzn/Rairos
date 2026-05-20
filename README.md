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
[![Stars](https://img.shields.io/github/stars/shushuzn/Rairos?style=social)](https://github.com/shushuzn/Rairos/stargazers)
[![Forks](https://img.shields.io/github/forks/shushuzn/Rairos?style=social)](https://github.com/shushuzn/Rairos/network/members)
[![Downloads](https://img.shields.io/github/releases-downloads/shushuzn/Rairos/total?style=social)](https://github.com/shushuzn/Rairos/releases)

## Why Rairos?

| Feature | Zotero | Mendeley | **Rairos** |
|---------|--------|----------|-------------|
| PDF storage | ✅ | ✅ | ✅ |
| Reference management | ✅ | ✅ | ✅ |
| Research gap detection | ❌ | ❌ | ✅ |
| Self-evolving Gene Pool | ❌ | ❌ | ✅ |
| Local LLM support | ❌ | ❌ | ✅ |
| MCP tools for AI agents | ❌ | ❌ | ✅ |
| 105 CLI commands | ❌ | ❌ | ✅ |

Rairos is the **first research tool that evolves with you** — it learns what you find valuable and improves future searches automatically.

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

## Performance

Built in **100% Rust** for maximum performance:

| Metric | Value | Comparison |
|--------|-------|------------|
| Startup time | **< 50ms** | Python: 500ms+, Node.js: 80ms+ |
| Memory usage | **~10 MB** | Python: 100MB+, Node.js: 190MB+ |
| Binary size | **Single ~15MB** | Python/Node: node_modules 200MB+ |
| Search latency | **12ms** | FTS5 keyword search |
| Parallel processing | **Rayon** | Full CPU utilization |

Run `./rairos.sh benchmark` to see full performance metrics.

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

## Installation

### From Source

```bash
git clone https://github.com/shushuzn/Rairos.git
cd Rairos
make build
```

### Via cargo install

```bash
# Install from crates.io (when published)
cargo install rairos

# Or install from source
cargo install --path crates/rairos-cli
```

### Via cargo-binstall (fast, no compile)

```bash
# Install cargo-binstall first: https://github.com/cargo-bins/cargo-binstall
cargo binstall rairos-cli
```

### Pre-built Binaries

Download from the [latest release](https://github.com/shushuzn/Rairos/releases/latest):

| Platform | Download |
|----------|----------|
| Linux x86_64 | `rairos-cli-x86_64-unknown-linux-musl.tar.gz` |
| macOS Apple Silicon | `rairos-cli-aarch64-apple-darwin.tar.gz` |
| macOS Intel | `rairos-cli-x86_64-apple-darwin.tar.gz` |
| Windows x86_64 | `rairos-cli-x86_64-pc-windows-msvc.zip` |

```bash
# Extract and install
tar -xzf rairos-cli-*.tar.gz
sudo mv rairos /usr/local/bin/
```

### Via Package Managers

```bash
# Homebrew (macOS/Linux)
brew install rairos

# Arch Linux (AUR)
paru -S rairos-cli

# Nix/NixOS
nix-env -iA nixpkgs.rairos

# Guix
guix install rairos

# FreeBSD
pkg install rairos
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
# Install Python SDK
pip install rairos

# Or Node.js SDK
npm install rairos
```

**Note**: For development, install from source:
```bash
cd sdks/python && pip install -e . && cd ../..
cd sdks/js && npm install && cd ../..
```

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

## Key Features

| Feature | Description |
|---------|-------------|
| 🔍 **Paper Import** | arXiv, DOI, PDF, OCR support |
| 🧬 **Gene Pool** | Self-evolving research patterns |
| 🎯 **Gap Detection** | 36 AI research topics |
| 💬 **RAG Chat** | Chat with your papers |
| 📊 **Knowledge Graph** | Visualize paper relationships |
| 🤖 **69 MCP Tools** | AI agent integration |
| 🖥️ **105 CLI Commands** | Full terminal interface |

## Ecosystem

Core crates that power Rairos:

| Crate | Description |
|-------|-------------|
| [rairos-core](crates/rairos-core/) | Database, FTS5 search, subscriptions, tags |
| [rairos-llm](crates/rairos-llm/) | GenePool evolution, LLM clients |
| [rairos-cli](crates/rairos-cli/) | 105 CLI commands |
| [rairos-mcp](crates/rairos-mcp/) | 69 MCP tools (JSON-RPC 2.0) |
| [rairos-kg](crates/rairos-kg/) | Knowledge graph, PageRank |
| [rairos-research](crates/rairos-research/) | DeepResearchAgent, gap detection |

## FAQ

### How does Rairos differ from Zotero or Mendeley?

Rairos is not a PDF manager — it's a **self-evolving research partner** that learns from your research patterns and improves over time. It focuses on detecting research gaps and generating insights, not just storing papers.

### Does Rairos require an internet connection?

Rairos works **fully offline** with local LLM support (Ollama). Cloud features (OpenAI, DashScope) are optional.

### What's the minimum Rust version?

**Rust 1.85+** is required. Rairos uses modern Rust features for performance and safety.

### How does Gene Pool work?

Gene Pool encodes successful research patterns as "genes" that evolve over time. When you mark papers as useful, the system learns what matters to you and prioritizes similar findings in future searches.

### Can I use Rairos programmatically?

Yes! Rairos provides:
- **CLI**: 105 commands via `./rairos.sh`
- **MCP**: 69 tools for AI agent integration
- **REST API**: Built-in web server with OpenAPI docs
- **SDKs**: Python (`pip install rairos`) and Node.js (`npm install rairos`)

## Troubleshooting

### Build fails with "memory allocation failed"

```bash
# Reduce parallelism
unset RUSTC_WRAPPER && CARGO_BUILD_JOBS=1 cargo build

# Or use make with reduced jobs
make build CARGO_BUILD_JOBS=1
```

### Database locked errors

```bash
# Ensure only one Rairos process is running
pkill rairos  # Kill any running instances
./rairos.sh init  # Reinitialize if needed
```

### Ollama connection fails

```bash
# Check Ollama is running
ollama list

# Pull a model if needed
ollama pull qwen2.5

# Set explicit base URL
export OLLAMA_BASE_URL=http://localhost:11434
```

### arXiv paper not found

Some papers require authentication or are not on arXiv. Try:
- DOI import: `./rairos.sh add --doi 10.xxxx/xxxxx`
- Direct PDF: `./rairos.sh import /path/to/paper.pdf`

### Rust version mismatch

```bash
# Rairos requires Rust 1.85+
rustc --version
rustup update  # Update to latest stable
```

### Need help?

- Run `./rairos.sh --help` for all commands
- Run `./rairos.sh <command> --help` for command-specific help
- Check [docs/](docs/) for detailed documentation
- Open an issue on GitHub for bugs or feature requests

## Similar Programs

Looking for alternatives? Compare with other research tools:

| Tool | Type | Focus |
|------|------|-------|
| [Zotero](https://www.zotero.org/) | Reference manager | PDF storage, citations |
| [Mendeley](https://www.mendeley.com/) | Reference manager | PDF annotation, sync |
| [Semantic Scholar](https://www.semanticscholar.org/) | Search engine | AI-powered paper discovery |
| [Consensus](https://consensus.app/) | Search engine | Q&A over papers |
| [Elicit](https://elicit.org/) | Research assistant | AI analysis |

**Rairos differs** by being **self-hosted** with **self-evolving** Gene Pool — it learns from your feedback to improve future searches.

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
