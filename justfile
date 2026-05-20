# Justfile — developer commands for Rairos
# NOTE: For Rust development, use Makefile instead (make build, make test, etc.)
# Install just: cargo install just

default:
    @just --list

# Run full test suite (Rust)
test:
    unset RUSTC_WRAPPER && cargo test --workspace

# Lint (Rust)
lint:
    unset RUSTC_WRAPPER && cargo clippy --workspace -- -D warnings

# Build release
build:
    unset RUSTC_WRAPPER && cargo build --release -p rairos-cli

# Build debug
build-dev:
    unset RUSTC_WRAPPER && cargo build -p rairos-cli

# Run the CLI
run *ARGS:
    ./rairos.sh {{ARGS}}

# Show help
help:
    @echo "Use 'make' for most commands:"
    @echo "  make build, make test, make run CMD='...'"
    @echo "Or use ./rairos.sh directly"
    @just --list
