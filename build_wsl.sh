#!/bin/bash
# Build Rairos Rust crates in WSL2
set -e

# Install Rust if not present
if ! command -v rustc &>/dev/null; then
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
    source "$HOME/.cargo/env"
fi

cd /mnt/d/OpenClaw/workspace/80-PROJECTS/ai_research_os

# Build all crates
cargo check --workspace

# Or build release
# cargo build --release --workspace
