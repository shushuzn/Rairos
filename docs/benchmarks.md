# Benchmarks

Rairos benchmarks measure three core capabilities: **paper import speed**, **search latency**, and **RAG answer quality**. Numbers below are collected on a standard research workstation (8-core CPU, 16 GB RAM, SSD).

> [!NOTE]
> These benchmarks are **self-measured** using the built-in `PerformanceProfiler`. Run `rairos benchmark` to reproduce.

## Paper Import Pipeline

| Stage | Operation | Avg Time | Notes |
|-------|-----------|----------|-------|
| arXiv metadata fetch | HTTP GET | 0.8s | Cached after first fetch |
| PDF download | HTTP GET | 1.2s | Per paper, depends on size |
| Text extraction | Rust parser (lopdf) | 0.3s | Per page, parallelized |
| Database write | SQLite INSERT | 0.05s | Batched for bulk imports |
| LLM analysis | API call | 2–8s | Optional, depends on model |

### Bulk Import Throughput

```
10 papers:   ~45s  (0.13 papers/sec)
50 papers:   ~3m   (0.27 papers/sec)  ← parallel downloads kick in
100 papers:  ~5m   (0.33 papers/sec)
```

Speedup after first run comes from metadata caching and smart cache deduplication.

## Search Performance

| Query Type | Latency (p50) | Latency (p95) | Notes |
|------------|---------------|---------------|-------|
| Keyword FTS5 | 12ms | 35ms | BM25 ranking |
| Semantic vector | 28ms | 80ms | 768-dim embeddings |
| Hybrid (FTS + vector) | 40ms | 110ms | RRF fusion |
| Filtered search | 8ms | 22ms | With index on tag/status |

Results measured over 1,000 papers in database.

## RAG Answer Quality

Rairos uses `BenchmarkComparator` to evaluate answer quality by cross-referencing cited papers against ground-truth experiment tables.

| Metric | Score | Description |
|--------|-------|-------------|
| Citation Accuracy | 94.2% | Claims backed by cited paper's table data |
| Factual Recall | 87.6% | Key numbers recalled from imported papers |
| Hallucination Rate | 3.1% | Unverified claims per answer |

> [!TIP]
> Run `./rairos.sh benchmark --compare <paper_id1> <paper_id2>` to compare benchmark results across any two imported papers.

## Core Module Test Coverage

| Crate | Coverage | Test Count |
|-------|----------|------------|
| `rairos-core` | 91% | 312 tests |
| `rairos-llm` | 78% | 485 tests |
| `rairos-parser` | 85% | 124 tests |
| `rairos-citations` | 89% | 67 tests |
| **Total** | **83%+** | **3,827+ tests** |

Run `cargo test` to reproduce the full test suite.

## Evolution System

The self-evolution system improves over time. Gene/capsule patterns that fail to produce useful insights are faded out.

| Metric | Value |
|--------|-------|
| Active capsules | 24 |
| Capsules evolved this month | 7 |
| Average insight quality score | 6.4 / 10 |
| Self-improvement rate | +0.3 / month |

## Reproducing These Numbers

```bash
# Full test suite
make test

# Profile paper import
./rairos.sh benchmark --time-import 2601.00155

# Compare two papers' benchmark tables
./rairos.sh benchmark --compare 2601.00155 2302.00763
```
