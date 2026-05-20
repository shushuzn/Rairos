#!/bin/bash
# setup-dev.sh - Setup Rairos development environment
# Run this once after cloning or when setting up a new environment

set -e

echo "=========================================="
echo "Rairos Development Environment Setup"
echo "=========================================="

# Get script directory
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"

cd "$PROJECT_ROOT"

echo ""
echo "1. Checking environment..."
echo "   Rust version: $(rustc --version 2>/dev/null || echo 'NOT FOUND')"
echo "   Cargo version: $(cargo --version 2>/dev/null || echo 'NOT FOUND')"

# Check disk space
DISK_AVAIL=$(df -h . | tail -1 | awk '{print $4}')
echo "   Disk space: $DISK_AVAIL available"

# Check memory
MEM_TOTAL=$(free -h 2>/dev/null | grep Mem | awk '{print $2}' || echo "unknown")
echo "   Memory: $MEM_TOTAL total"

echo ""
echo "2. Installing git hooks..."
if [ -f ".git/hooks/pre-commit" ]; then
    cp .git/hooks/pre-commit .git/hooks/pre-commit.bak 2>/dev/null || true
    echo "   Backed up existing hook"
fi
if [ -f "$SCRIPT_DIR/.git/hooks/pre-commit" ]; then
    cp "$SCRIPT_DIR/.git/hooks/pre-commit" .git/hooks/pre-commit
    chmod +x .git/hooks/pre-commit
    echo "   Installed pre-commit hook"
else
    echo "   WARNING: pre-commit hook not found"
fi

echo ""
echo "3. Checking installed tools..."
check_tool() {
    if command -v "$1" &> /dev/null; then
        echo "   $1: $(command -v $1)"
    else
        echo "   $1: NOT INSTALLED (optional)"
    fi
}

check_tool flamegraph
check_tool cargo-audit
check_tool cargo-nextest
check_tool sccache

echo ""
echo "4. Generating skill files..."
if [ -f "scripts/generate_skills.py" ]; then
    python3 scripts/generate_skills.py
    echo "   Skills generated"
else
    echo "   WARNING: generate_skills.py not found"
fi

echo ""
echo "4. Verifying build..."
echo "   Building rairos-cli (this may take a few minutes)..."
if unset RUSTC_WRAPPER && cargo build -p rairos-cli 2>&1 | grep -q "error"; then
    echo "   WARNING: Build had errors"
else
    echo "   Build check passed"
fi

echo ""
echo "=========================================="
echo "Setup complete!"
echo "=========================================="
echo ""
echo "Next steps:"
echo "  make help           # Show available commands"
echo "  make build          # Build project"
echo "  make test          # Run tests"
echo "  make setup          # Re-run this setup"
echo ""
echo "For AI coding assistants (opencode, Claude Code):"
echo "  - opencode.json is already configured"
echo "  - Skill files in .opencode/skills/rairos-dev/"
echo "  - Restart opencode to load new skills"
echo ""
