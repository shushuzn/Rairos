# Installation

## Requirements

- Rust 1.85+ (tested on 1.85–1.86)
- SQLite 3.x (bundled via `rusqlite`)

## Install from Source

```bash
git clone https://github.com/shushuzn/Rairos.git
cd Rairos
make build
```

## Initialize Database

```bash
./rairos.sh init
```

## Run a Paper Search

```bash
./rairos.sh add <arxiv-id>
./rairos.sh list
```

## For Developers

```bash
make build-dev    # Debug build (faster for iterative development)
make test         # Run tests
make clippy       # Run linter
make clean        # Clean build artifacts
```

For faster repeated builds, ccache is automatically configured. For sccache:

```bash
cargo install sccache
sccache --start-server
unset RUSTC_WRAPPER && cargo build --release -p rairos-cli
```

## Environment Variables

| Variable | Default | Description |
|----------|---------|-------------|
| `RAIROS_DB` | `rairos.db` | Database file path |
| `RAIROS_DATA_DIR` | `~/.ai_research_os/` | Data storage root |
