#!/bin/bash
#
# Rairos CLI Runner - Simplifies calling rairos-cli
#
# Usage:
#   ./rairos.sh search "transformer"
#   ./rairos.sh gap "LLM efficiency"
#   ./rairos.sh init
#   ./rairos.sh --help
#
# Auto-detects: release binary > debug binary > cargo run

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
RELEASE_BIN="$SCRIPT_DIR/target/release/rairos-cli"
DEV_BIN="$SCRIPT_DIR/target/debug/rairos-cli"

# Colors
if [[ -t 1 && -z "${NO_COLOR:-}" ]]; then
    C_GREEN='\033[0;32m'
    C_YELLOW='\033[1;33m'
    C_RESET='\033[0m'
else
    C_GREEN=''; C_YELLOW=''; C_RESET=''
fi

# Find binary
if [[ -x "$RELEASE_BIN" ]]; then
    BIN="$RELEASE_BIN"
elif [[ -x "$DEV_BIN" ]]; then
    BIN="$DEV_BIN"
    echo -e "${C_YELLOW}Using debug build (run 'make build' for release)${C_RESET}"
else
    echo "No binary found. Building..."
    cd "$SCRIPT_DIR"
    unset RUSTC_WRAPPER
    cargo build --release -p rairos-cli
    BIN="$RELEASE_BIN"
fi

# Run
exec "$BIN" "$@"
