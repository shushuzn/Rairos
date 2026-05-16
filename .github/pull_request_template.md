## Summary

<!-- 1-3 sentence description of what this PR does and why -->

## Changes

<!-- List the specific changes made in this PR -->

- [ ] Bug fix (fixes #)
- [ ] New feature
- [ ] Refactoring
- [ ] Documentation update
- [ ] Tests added/updated

## Verification

<!-- How was this tested? -->

```bash
# Test commands run
CARGO_BUILD_JOBS=1 cargo build
CARGO_BUILD_JOBS=1 cargo test -p <affected-crate>
```

## Checklist

- [ ] Code follows Rust style conventions (`cargo fmt`)
- [ ] Clippy passes (`cargo clippy -- -D warnings`)
- [ ] Tests pass (`cargo test --workspace`)
- [ ] Documentation updated if user-facing changes were made
- [ ] CHANGELOG.md updated for user-facing changes

---

**For maintainers:** CI must pass before merge.
