# Changelog

All notable changes to this project will be documented in this file.

## [2026-05-16] — Python→Rust Migration Complete

### BUG FIXES

- Eliminate all 7 compiler warnings


### CONTINUOUS INTEGRATION

- Clean up Rust workflow (remove Python/pytest/PyO3 steps)

- Remove Python-based workflows, simplify release/docs


### DOCUMENTATION

- Remove Python badge, fix CI badge to rust.yml

- Update AGENTS.md (157 crates, rairos-core db_optimize) and CLAUDE.md (Rust CI, remove Python test refs)

- CLAUDE.md - remove stale warning list, fix formatting


### OTHER

- Remove dead rairos-db crate (not in workspace, zero dependents)

- Remove stale pyproject.toml (Python fully migrated to Rust)

- Remove stale tests/ directory and all Python pycache artifacts

- Remove Python config/CI, replace commitizen with git-cliff

