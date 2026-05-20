.PHONY: help build build-release build-dev test clippy clean install-completions run

# Detect if release binary exists
RELEASE_BIN := target/release/rairos-cli
DEV_BIN := target/debug/rairos-cli

# Default target
help:
	@echo "Rairos - Self-Evolving Research OS"
	@echo ""
	@echo "Usage:"
	@echo "  make build          Build release (optimized, 10-20min)"
	@echo "  make build-dev     Build debug (faster, ~5min)"
	@echo "  make test           Run tests"
	@echo "  make clippy         Run linter"
	@echo "  make run CMD=...    Run CLI (e.g., make run CMD='search \"ML\"')"
	@echo "  make clean          Clean build artifacts"
	@echo ""
	@echo "Direct binary usage:"
	@echo "  ./rairos.sh search \"transformer\"   # Quick search"
	@echo "  ./rairos.sh gap \"LLM\"             # Detect gaps"
	@echo ""
	@echo "Pre-built binary: $(RELEASE_BIN)"

build: $(RELEASE_BIN)
	@echo "Release binary ready: $(RELEASE_BIN)"
	-@./rairos.sh --version 2>/dev/null || true

$(RELEASE_BIN):
	@echo "Building release (this may take 10-20 minutes)..."
	unset RUSTC_WRAPPER && cargo build --release -p rairos-cli

build-dev:
	@echo "Building debug (faster)..."
	unset RUSTC_WRAPPER && cargo build -p rairos-cli

test:
	unset RUSTC_WRAPPER && cargo test --workspace

clippy:
	unset RUSTC_WRAPPER && cargo clippy --workspace -- -D warnings

clean:
	cargo clean
	rm -f rairos.sh

# Run with arguments: make run CMD='gap "transformer"'
run:
	@if [ -f "$(RELEASE_BIN)" ]; then \
		$(RELEASE_BIN) $(CMD); \
	elif [ -f "$(DEV_BIN)" ]; then \
		$(DEV_BIN) $(CMD); \
	else \
		echo "No binary found. Run 'make build' first."; \
	fi

install-completions:
	@echo "Installing shell completions..."
	@# Bash
	cp completions/bash ~/.config/opencode/completions/rairos 2>/dev/null || true
	cp completions/fish ~/.config/fish/completions/rairos.fish 2>/dev/null || true
	cp completions/zsh ~/.config/zsh/completions/_rairos 2>/dev/null || true
	@echo "Completions installed. Restart shell or run: source ~/.bashrc"
