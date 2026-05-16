# Installation

## Requirements

- Rust 1.81+ (tested on 1.81–1.86)
- SQLite 3.x (bundled via `rusqlite`)

## Install from Source

```bash
git clone https://github.com/shushuzn/Rairos.git
cd Rairos
CARGO_BUILD_JOBS=1 cargo build --workspace
```

## Initialize Database

```bash
cargo run -p rairos-cli -- init
```

## Run a Paper Search

```bash
cargo run -p rairos-cli -- add <arxiv-id>
cargo run -p rairos-cli -- list
```

## Optimize for Memory

The Rust workspace has 154 crates. For builds on memory-constrained machines:

```bash
CARGO_BUILD_JOBS=1 cargo build --workspace
```

For incremental dev builds:

```bash
cargo build -p rairos-cli
```

## Environment Variables

| Variable | Default | Description |
|----------|---------|-------------|
| `RAIROS_DB` | `rairos.db` | Database file path |
| `RAIROS_DATA_DIR` | `~/.ai_research_os/` | Data storage root |
