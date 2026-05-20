# Advanced Commands Reference

Rairos CLI ships with **105 commands**. This page covers the most useful ones by category.

> Full help: `./rairos.sh --help` or `./rairos.sh <command> --help`

---

## Paper Management

### `./rairos.sh add <arxiv_id>`
Add a paper by arXiv ID (auto-fetches metadata).

```bash
./rairos.sh add 2601.00155
```

### `./rairos.sh list`
List papers with filters and sorting.

```bash
./rairos.sh list
./rairos.sh list --tag LLM --limit 20 --sort published
./rairos.sh list --status done --year 2025 --format json
```

### `./rairos.sh search <query>`
Full-text search with BM25 ranking.

```bash
./rairos.sh search "scaling law"
./rairos.sh search "transformer" --limit 20 --field title
```

### `./rairos.sh show <id>`
Show paper details.

```bash
./rairos.sh show 2601.00155
./rairos.sh show 2601.00155 --format json
```

### `./rairos.sh delete <ids...>`
Delete papers.

```bash
./rairos.sh delete 2601.00155
./rairos.sh delete 2601.00155 2302.00763 --force
```

### `./rairos.sh import`
Batch import from arXiv ID, DOI, or JSON file.

```bash
./rairos.sh import --ids 2601.00155 2302.00763
./rairos.sh import --path papers.json
./rairos.sh import --ids DOI:10.48550/arXiv.2601.00155 --skip-existing
```

### `./rairos.sh export`
Export database to JSON or CSV.

```bash
rairos export data.json
rairos export data.csv --format csv
rairos export done_papers.json --status done
```

---

## Research Analysis

### `rairos analyze <id>`
Extract insights from a paper.

```bash
rairos analyze 2601.00155 --kind summary
rairos analyze 2601.00155 --kind keywords
rairos analyze 2601.00155 --kind topics
rairos analyze 2601.00155 --kind quality
```

### `rairos compare <ids...>`
Compare multiple papers.

```bash
rairos compare 2601.00155,2302.00763 --aspect abstract
rairos compare 2601.00155,2302.00763 --aspect method,results
```

### `rairos ask <question>`
Ask a question about papers in your library.

```bash
rairos ask "What architectures are used for long-context LLMs?"
rairos ask "Compare scaling patterns" --max-papers 20
```

### `rairos trend <topic>`
Analyze research trends over time.

```bash
rairos trend "reinforcement learning" --range 2y
rairos trend "transformer" --range 6m --format json
```

---

## Gap Detection & Gene Pool

### `rairos gap --topic <topic>`
Detect research gaps for a topic.

```bash
rairos gap --topic "long context LLM"
rairos gap --topic "VLA" --limit 10 --category LLM
```

### `rairos gap-list`
List detected gaps.

```bash
rairos gap-list
rairos gap-list --limit 50
```

### `rairos gap-show <id>`
Show gap details.

```bash
rairos gap-show gap_abc123
```

### `rairos gene-add`
Add a capsule to the Gene Pool.

```bash
rairos gene-add --approach "Use adaptive context length" --gap-type scalability_issue --keywords "context,window,length"
```

### `rairos gene-list`
List genes/capsules in the pool.

```bash
rairos gene-list
rairos gene-list --status active --gap-type scalability_issue
```

### `rairos gene-evolve`
Run evolution cycle on Gene Pool.

```bash
rairos gene-evolve --max-crossovers 5
```

### `rairos gene-feedback`
Record feedback for a gene.

```bash
rairos gene-feedback gene_xyz --positive
rairos gene-feedback gene_xyz --negative
```

---

## Knowledge Graph

### `rairos kg-stats`
Knowledge graph statistics.

```bash
rairos kg-stats
```

### `rairos kg-rank`
PageRank-based paper importance ranking.

```bash
rairos kg-rank --limit 20
```

### `rairos kg-path <source> <target>`
Find shortest path between two papers.

```bash
rairos kg-path 2601.00155 2302.00763
```

### `rairos kg-graph <id>`
Show a paper's ego graph (neighbors).

```bash
rairos kg-graph 2601.00155 --depth 2
```

### `rairos kg-search <keyword>`
Search nodes in the knowledge graph.

```bash
rairos kg-search "transformer" --type Paper
rairos kg-search "attention" --type Tag
```

---

## Citations

### `rairos citations`
Show citation relationships.

```bash
rairos citations --from 2601.00155   # Papers this paper cites
rairos citations --to 2601.00155     # Papers that cite this paper
```

### `rairos cite-stats`
Citation statistics.

```bash
rairos cite-stats
rairos cite-stats --paper 2601.00155
rairos cite-stats --top 10
```

### `rairos cite-import`
Import citation links from JSON.

```bash
rairos cite-import --file citations.json
rairos cite-import --extract --paper 2601.00155  # Extract from plain text
```

### `rairos cite-fetch`
Fetch citations from OpenAlex API.

```bash
rairos cite-fetch 2601.00155
```

---

## Research Automation

### `rairos agent <topic>`
Autonomous research agent — searches, reads, analyzes.

```bash
rairos agent "scaling laws for RL"
rairos agent "VLA generalization" --max-papers 20 --max-time 30
```

### `rairos hypothesize <topic>`
Generate research hypotheses from gaps and trends.

```bash
rairos hypothesize "adaptive inference" --gap-context "..."
rairos hypothesize "multi-agent reasoning" --num-hypotheses 5 --json
```

### `rairos pipeline <topic>`
Full research pipeline: gap analysis → hypothesis → experiment.

```bash
rairos pipeline "long context LLM"
rairos pipeline "multi-modal RL" --skip-experiments
```

### `rairos research`
Manage research logs.

```bash
rairos research list
rairos research add "Noticed interesting scaling behavior in sparse models"
```

### `rairos digest`
Generate weekly research digest.

```bash
rairos digest --weeks 2
```

### `rairos demo`
Run end-to-end pipeline demo.

```bash
rairos demo              # Full demo
rairos demo --quick      # 30-second demo
```

---

## Subscriptions & Monitoring

### `rairos daemon`
Start Rairos as a background service.

```bash
rairos daemon
rairos daemon --port 8080 --foreground
```

### `rairos subscribe`
Subscribe to arXiv searches with continuous monitoring.

```bash
rairos subscribe "transformer attention" --interval 60 --auto-add
```

### `rairos signal <keyword>`
Match event keywords against Gene Pool patterns.

```bash
rairos signal "transformer"
rairos signal "reinforcement learning"
```

---

## Environment & Diagnostics

### `rairos doctor`
Diagnose environment and report issues.

```bash
rairos doctor
```

### `rairos stats`
Database statistics.

```bash
rairos stats
rairos stats --json
```

### `rairos status`
Real-time database status.

```bash
rairos status
```

### `rairos setup`
Run the setup wizard.

```bash
rairos setup
rairos setup --guide     # Quick start guide only
```

---

## Deduplication

### `rairos dedup`
Find duplicate papers by DOI/title.

```bash
rairos dedup find                    # Find duplicates
rairos dedup semantic 2601.00155    # Semantic similarity
rairos dedup remove ID1,ID2         # Remove duplicates
rairos dedup stats                   # Show embedding coverage
```

### `rairos similar <id>`
Find semantically similar papers.

```bash
rairos similar 2601.00155
rairos similar 2601.00155 --limit 10 --threshold 0.8
```

### `rairos merge`
Merge duplicate papers.

```bash
rairos merge TARGET_ID DUPLICATE_ID
rairos merge --auto                  # Auto-merge high-similarity pairs
```

---

## Queue & Data Management

### `rairos queue`
Manage the processing queue.

```bash
rairos queue list
rairos queue add 2601.00155
rairos queue clear
```

### `rairos cache`
Manage cached data.

```bash
rairos cache stats
rairos cache clear
rairos cache list
```

### `rairos parse <id>`
Parse a paper's full text.

```bash
rairos parse 2601.00155
```

---

## Web UI & Visualization

### `rairos dashboard`
Start the Web UI dashboard.

```bash
rairos dashboard
rairos dashboard --port 3000
```

### `rairos visual`
Generate visualizations.

```bash
rairos visual 2601.00155
rairos visual 2601.00155 --output ./viz.html
```

### `rairos slides`
Generate slides from papers.

```bash
rairos slides 2601.00155 --format md --style academic
rairos slides 2601.00155 2302.00763 --slides 15 --notes
```

---

## Research Memory

### `rairos stance-add`
Add a research stance.

```bash
rairos stance-add --topic "scaling laws" --claim "Inference cost grows O(n²)" --stance supported
```

### `rairos stance-list`
List stances by topic or tag.

```bash
rairos stance-list --topic "scaling laws"
```

### `rairos narrative`
Manage research narratives.

```bash
rairos narrative list
rairos narrative track "multi-modal reasoning"
```

### `rairos story <topic>`
Weave research into narrative stories.

```bash
rairos story "evolution of transformer architecture"
```

---

## Ollama Setup (for semantic features)

```bash
# Start Ollama locally (required for dedup semantic, similar)
ollama serve

# Pull embedding model (one-time)
ollama pull nomic-embed-text
```

## Build & Test

```bash
# Build (memory-intensive — single job required)
make build-dev

# Run all tests
make test

# Test a specific crate
make test -p rairos-core
```
