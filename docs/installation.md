# Installation

## Requirements

- Python 3.10+ (tested on 3.11/3.12/3.13)
- [uv](https://github.com/astral-sh/uv) (recommended) or pip

## Install with uv (recommended)

```bash
git clone https://github.com/shushuzn/Rairos.git
cd Rairos
uv sync --all-extras
rairos --help
```

## Install with pip

```bash
pip install -e ".[all]"
```

Both install the `rairos` CLI entry point.

## Initialize Database

```bash
rairos init
```

Creates the SQLite database at `~/.ai_research_os/papers.db`.

## Ollama (Optional — for Semantic Dedup)

For `dedup-semantic` command, install [Ollama](https://ollama.com) and pull the embedding model:

```bash
ollama pull nomic-embed-text
```

The CLI will automatically use `http://localhost:11434` for embeddings.

## Verify

```bash
rairos --help
rairos status   # shows database stats
```

## Uninstall

```bash
pip uninstall ai-research-os
rm -rf ~/.ai_research_os   # remove database and cache
```
