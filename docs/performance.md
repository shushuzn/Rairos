# Performance Benchmarks

Generated: 2026-05-13
Environment: Python 3.12, Windows

## claim_graph.py — ClaimGraph operations

| Operation | Input | Iterations | Total Time | Per Call |
|-----------|-------|------------|------------|----------|
| `find_contradictions()` | 50 claims (100 nodes) | 1000 | 0.187s | 0.19ms |
| `find_bidirectional_contradictions()` | 50 claims, 25 edges | 1000 | 0.002s | 2μs |

> Note: `find_contradictions` is O(n²) on claim count per type. Scaling to 500 claims would be ~19ms/call.

## Adding pytest-benchmark

To track regression, add `pytest-benchmark` to dev dependencies and run:
```bash
pytest tests/test_claim_graph.py --benchmark-only
```

Expected baseline (commit `93c810c`):
- `test_find_contradictions_50_claims`: ~0.19ms
- `test_find_bidirectional_25_edges`: ~2μs
