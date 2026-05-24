CARGO := cargo
NPROCS := $(shell nproc 2>/dev/null || echo 1)
MAKEFLAGS += --jobs=$(NPROCS)

.PHONY: help build build-release build-dev test clippy clean run install-completions check-audit

# Detect if release binary exists
RELEASE_BIN := target/release/rairos-cli
DEV_BIN := target/debug/rairos-cli

# Auto-detect optimal linker: mold > lld > default
LINKER := $(shell command -v mold 2>/dev/null && echo "-C link-arg=-fuse-ld=mold" || (command -v ld.lld 2>/dev/null && echo "-C link-arg=-fuse-ld=lld" || echo ""))
# Auto-detect sccache
SCCACHE := $(shell command -v sccache 2>/dev/null && echo "sccache" || echo "")
ifneq ($(SCCACHE),)
	CARGO := RUSTC_WRAPPER=sccache cargo
endif

# Common RUSTFLAGS for release: parallel codegen, native CPU tuning
RELEASE_RUSTFLAGS := $(LINKER) -C target-cpu=native

# Default target
help:
	@echo "Rairos - Self-Evolving Research OS"
	@echo ""
	@echo "Usage:"
	@echo "  make build          Build release (optimized, ~5-10min with sccache)"
	@echo "  make build-dev      Build debug (faster)"
	@echo "  make test           Run tests (parallel)"
	@echo "  make clippy         Run linter"
	@echo "  make run CMD=...    Run CLI (e.g., make run CMD='search \"ML\"')"
	@echo "  make check-audit    Security audit"
	@echo "  make clean          Clean build artifacts"
	@echo ""
	@echo "Accelerators detected:"
ifneq ($(SCCACHE),)
	@echo "  ✓ sccache (build cache)"
endif
ifneq ($(LINKER),)
	@echo "  ✓ mold/lld (fast linker)"
endif
	@echo "  ✓ $(NPROCS)-core parallel build"
	@echo ""
	@echo "Direct binary usage:"
	@echo "  ./rairos.sh search \"transformer\"   # Quick search"
	@echo "  ./rairos.sh gap \"LLM\"             # Detect gaps"
	@echo ""

build: $(RELEASE_BIN)
	@echo "Release binary ready: $(RELEASE_BIN)"
	-@./rairos.sh --version 2>/dev/null || true

$(RELEASE_BIN):
	@echo "Building release ($(shell date +%T))..."
	RUSTFLAGS="$(RELEASE_RUSTFLAGS)" $(CARGO) build --release -p rairos-cli
	@echo "Done ($(shell date +%T))"

# PGO: Profile-Guided Optimization (requires llvm-tools component)
# 1. make pgo-generate    — build with instrumentation
# 2. run the binary with typical workloads
# 3. make pgo-use         — rebuild with profile data
PGO_DATA_DIR := /tmp/rairos-pgo
pgo-generate:
	@echo "Building instrumented binary for PGO..."
	mkdir -p $(PGO_DATA_DIR)
	RUSTC_WRAPPER= RUSTFLAGS="-Cprofile-generate=$(PGO_DATA_DIR)" \
		cargo build --release -p rairos-cli
	@echo "Run './target/release/rairos-cli <typical-commands>' to collect profiles"
	@echo "Then run 'make pgo-use'"

pgo-use:
	@echo "Merging PGO profiles..."
	LLVM_PROFDATA=$$(find ~/.rustup -name llvm-profdata -type f | head -1) && \
	$$LLVM_PROFDATA merge -o $(PGO_DATA_DIR)/merged.profdata $(PGO_DATA_DIR)/*.profraw && \
	echo "Building with PGO..." && \
	RUSTC_WRAPPER= RUSTFLAGS="-Cprofile-use=$(PGO_DATA_DIR)/merged.profdata" \
		cargo build --release -p rairos-cli
	@echo "PGO build complete"

build-dev:
	@echo "Building debug ($(shell date +%T))..."
	$(CARGO) build -p rairos-cli
	@echo "Done ($(shell date +%T))"

test:
	$(CARGO) test --workspace -- --nocapture

clippy:
	$(CARGO) clippy --workspace -- -D warnings

check-audit:
	$(CARGO) audit

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
	cp completions/bash ~/.config/opencode/completions/rairos 2>/dev/null || true
	cp completions/fish ~/.config/fish/completions/rairos.fish 2>/dev/null || true
	cp completions/zsh ~/.config/zsh/completions/_rairos 2>/dev/null || true
	@echo "Completions installed. Restart shell or run: source ~/.bashrc"
