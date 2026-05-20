# Benchmark Reference

> What Rairos measures — and how.

**Last updated:** May 2026

---

## Paper Impact Scoring

### Research Momentum Score

`ResearchMomentum` computes a [0–100] score per paper from five weighted components:

```
score = citation_score × 0.30
      + tag_popularity  × 0.25
      + recency_boost  × 0.20
      + novelty_factor  × 0.15
      + radar_heat     × 0.10
```

| Component | Weight | Description |
|-----------|--------|-------------|
| `citation_score` | 30% | Raw citation count, normalised to [0,100] |
| `tag_popularity` | 25% | How many papers share this paper's tags |
| `recency_boost` | 20% | Newer papers score higher; exponential decay over 5 years |
| `novelty_factor` | 15% | Rarity of this paper's tag combination vs. the library |
| `radar_heat` | 10% | Tag frequency velocity — tags that are accelerating |

All components are independently min-max normalised to [0, 100] before weighting.

**Source:** `crates/rairos-rankers`

---

### Composite Impact Score

`ImpactScorer` produces a richer per-paper profile, used by `./rairos.sh citation-chain --impact`:

```
composite_score = normalized_score × 0.30
                + pagerank_score  × 0.30
                + momentum_score  × 0.25
                + author_h_index  × 0.15
```

| Component | Weight | Description |
|-----------|--------|-------------|
| `normalized_score` | 30% | Raw citations ÷ years_since_publication |
| `pagerank_score` | 30% | PageRank over the citation graph — influence propagation |
| `momentum_score` | 25% | Citation velocity: citations in last 2 years ÷ total |
| `author_h_index` | 15% | Aggregate h-index of paper's authors |

**Tier bands:**

| Tier | Composite Score | Percentile |
|------|----------------|-----------|
| S | ≥ 80 | top 5% |
| A | 60–79 | top 20% |
| B | 40–59 | top 50% |
| C | 20–39 | top 80% |
| D | < 20 | bottom 20% |

**Source:** `crates/rairos-rankers`

---

## Gene Pool Health

### Capsule Lifecycle Metrics

Each `CapsuleGene` tracks:

| Metric | Field | Healthy Range |
|--------|-------|---------------|
| Activity | `status == "active"` | majority of pool |
| Validation rate | `outcome_success_score` | moving average > 0.50 |
| Consumption speed | `feedback_count` growth | > 0 within 2 weeks |
| Low-score streak | `low_score_streak` | auto-archive triggers at ≥ 3 |

**Auto-archive condition:**
```
status == "active"
AND low_score_streak >= 3
AND outcome_success_score < 0.30
→ archived
```

**Capsule merge condition (Jaccard):**
```
same gap_type
AND Jaccard(keywords_A, keywords_B) >= 0.80
→ merge into higher-score capsule
```

**Source:** `crates/rairos-insight-evolution`

---

## Research Gap Detection

### Gap Type Taxonomy

Eight gap types tracked:

| Type | When to use |
|------|-------------|
| `unexplored_application` | Known methods applied to new domains |
| `method_limitation` | Current methods have specific drawbacks |
| `contradiction` | Paper challenges or refutes existing findings |
| `evaluation_gap` | Lack of proper benchmarks |
| `scalability_issue` | Methods don't scale to real-world settings |
| `theoretical_gap` | Lack of theoretical foundations |
| `dataset_gap` | No suitable dataset for the problem |
| `generalization_gap` | Methods fail on out-of-distribution data |

### Polarity

Each extracted gap carries a `polarity`:

| Value | Meaning | Gene Pool effect |
|-------|---------|----------------|
| `positive` | Paper ADVANCES the gap | candidate for `consumed` |
| `negative` | Paper FAILS or CHALLENGES the gap | contradiction signal |

### Contradiction Detection

Rule-based (no extra LLM call). Finds pairs where:

```
gap_type_A == gap_type_B
AND polarity_A != polarity_B
AND shared_keywords >= 1
```

**Source:** `crates/rairos-research`

---

## Citation Pathfinding

### BFS Path Resolution

`CitationChainBuilder.find_paths_to_gene_pool()` traces citation chains from a seed paper:

```
BFS directions: both "cites" (backward) and "cited_by" (forward)
Max depth: configurable (default 3)
Target: any Gene Pool capsule's source_paper_id
Stopping: first capsule reached per path (shortest path per target)
```

### Citation Graph Metrics

| Metric | Source |
|--------|--------|
| `citation_count` | OpenAlex API per paper |
| `papers_with_citations` | fraction of library with ≥ 1 citation edge |
| `avg_depth` | mean shortest-path length between any two papers |
| `isolated_papers` | papers with zero citation edges |

**Source:** `crates/rairos-citations`

---

## Search Quality

### FTS5 BM25

Full-text search uses SQLite FTS5 with BM25 ranking:

```
BM25(k1=1.2, b=0.75)
Rank = BM25(terms) + recency_boost
```

### Semantic Similarity

Embedding-based deduplication via Ollama (`nomic-embed-text`, 768-dim):

| Threshold | Use case |
|-----------|----------|
| ≥ 0.85 | Duplicate detection (`find_similar`) |
| ≥ 0.70 | Related paper suggestion |
| < 0.70 | Distinct papers |

---

## System Health

### Test Coverage

```
3839+ tests, across all crates
100% clippy clean
100% rustfmt compliant
```

### CLI Commands

105 commands across the Rairos CLI. Run `./rairos.sh --help` to list all.

---

*To update this page after changing scoring logic, edit `docs/benchmark.md` and update the "Last updated" date.*
