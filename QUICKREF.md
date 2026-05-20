# Rairos Quick Reference

## Quick Start
```bash
make build                    # Build release (first time: 10-20min)
./rairos.sh --help           # Show all commands
```

## Common Commands
```bash
./rairos.sh init              # Initialize database
./rairos.sh add 2301.001     # Add paper by arXiv ID
./rairos.sh search "ML"       # Search papers
./rairos.sh gap "LLM"         # Detect research gaps
./rairos.sh list --status pending  # List papers by status
./rairos.sh stats             # Show statistics
./rairos.sh radar             # View research radar
./raros.sh trend --topic "AI" # Analyze trends
```

## Development
```bash
make build-dev    # Debug build (faster)
make test         # Run tests
make clippy       # Run linter
make clean        # Clean build
make run CMD='...'  # Run with args
```

## Makefile Targets
| Target | Description |
|--------|-------------|
| `make help` | Show this help |
| `make build` | Release build |
| `make build-dev` | Debug build |
| `make test` | Run tests |
| `make clippy` | Lint code |
| `make run CMD='...'` | Run CLI |
| `make clean` | Clean artifacts |

## Tips
- Use `./rairos.sh <cmd> --help` for command-specific help
- Pre-built binary: `target/release/rairos-cli`
- Config: `~/.ai_research_os/` or `$RAIROS_DATA_DIR`
- Database: `rairos.db` or `$RAIROS_DB`
