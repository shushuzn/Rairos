# Performance Benchmarks

Generated: 2026-05-15
Environment: Rust (Linux, 8-core CPU)

## Claim Graph Operations

| Operation | Input | Iterations | Total Time | Per Call |
|-----------|-------|------------|------------|----------|
| `find_contradictions()` | 50 claims (100 nodes) | 1000 | 0.187s | 0.19ms |
| `find_bidirectional_contradictions()` | 50 claims, 25 edges | 1000 | 0.002s | 2μs |

> Note: `find_contradictions` is O(n²) on claim count per type. Scaling to 500 claims would be ~19ms/call.

**Source:** `crates/rairos-claimgraph`

## Benchmarking with Cargo

```bash
# Run all benchmarks
cargo bench

# Run specific claim graph benchmarks
cargo bench -p rairos-claimgraph

# Run tests
cargo test
```

Expected baseline (crate `rairos-claimgraph`):
- `find_contradictions_50_claims`: ~0.19ms
- `find_bidirectional_25_edges`: ~2μs
