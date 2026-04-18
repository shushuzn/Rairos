# Installation

## Requirements

- Python 3.9+
- pip

## Install

```bash
pip install -e .
```

This installs the `ai_research-os` package and the `ai-research-os` CLI entry point.

## Initialize Database

```bash
python -m ai_research_os init
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
python -m ai_research_os --help
python -m ai_research_os status   # shows database stats
```

## Uninstall

```bash
pip uninstall ai-research-os
rm -rf ~/.ai_research_os   # remove database and cache
```
