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
./rairos.sh export data.json
./rairos.sh export data.csv --format csv
./rairos.sh export done_papers.json --status done
```

---

## Research Analysis

### `./rairos.sh analyze <id>`
Extract insights from a paper.

```bash
./rairos.sh analyze 2601.00155 --kind summary
./rairos.sh analyze 2601.00155 --kind keywords
./rairos.sh analyze 2601.00155 --kind topics
./rairos.sh analyze 2601.00155 --kind quality
```

### `./rairos.sh compare <ids...>`
Compare multiple papers.

```bash
./rairos.sh compare 2601.00155,2302.00763 --aspect abstract
./rairos.sh compare 2601.00155,2302.00763 --aspect method,results
```

### `./rairos.sh ask <question>`
Ask a question about papers in your library.

```bash
./rairos.sh ask "What architectures are used for long-context LLMs?"
./rairos.sh ask "Compare scaling patterns" --max-papers 20
```

### `./rairos.sh trend <topic>`
Analyze research trends over time.

```bash
./rairos.sh trend "reinforcement learning" --range 2y
./rairos.sh trend "transformer" --range 6m --format json
```

---

## Gap Detection & Gene Pool

### `./rairos.sh gap --topic <topic>`
Detect research gaps for a topic.

```bash
./rairos.sh gap --topic "long context LLM"
./rairos.sh gap --topic "VLA" --limit 10 --category LLM
```

### `./rairos.sh gap-list`
List detected gaps.

```bash
./rairos.sh gap-list
./rairos.sh gap-list --limit 50
```

### `./rairos.sh gap-show <id>`
Show gap details.

```bash
./rairos.sh gap-show gap_abc123
```

### `./rairos.sh gene-add`
Add a capsule to the Gene Pool.

```bash
./rairos.sh gene-add --approach "Use adaptive context length" --gap-type scalability_issue --keywords "context,window,length"
```

### `./rairos.sh gene-list`
List genes/capsules in the pool.

```bash
./rairos.sh gene-list
./rairos.sh gene-list --status active --gap-type scalability_issue
```

### `./rairos.sh gene-evolve`
Run evolution cycle on Gene Pool.

```bash
./rairos.sh gene-evolve --max-crossovers 5
```

### `./rairos.sh gene-feedback`
Record feedback for a gene.

```bash
./rairos.sh gene-feedback gene_xyz --positive
./rairos.sh gene-feedback gene_xyz --negative
```

---

## Knowledge Graph

### `./rairos.sh kg-stats`
Knowledge graph statistics.

```bash
./rairos.sh kg-stats
```

### `./rairos.sh kg-rank`
PageRank-based paper importance ranking.

```bash
./rairos.sh kg-rank --limit 20
```

### `./rairos.sh kg-path <source> <target>`
Find shortest path between two papers.

```bash
./rairos.sh kg-path 2601.00155 2302.00763
```

### `./rairos.sh kg-graph <id>`
Show a paper's ego graph (neighbors).

```bash
./rairos.sh kg-graph 2601.00155 --depth 2
```

### `./rairos.sh kg-search <keyword>`
Search nodes in the knowledge graph.

```bash
./rairos.sh kg-search "transformer" --type Paper
./rairos.sh kg-search "attention" --type Tag
```

---

## Citations

### `./rairos.sh citations`
Show citation relationships.

```bash
./rairos.sh citations --from 2601.00155   # Papers this paper cites
./rairos.sh citations --to 2601.00155     # Papers that cite this paper
```

### `./rairos.sh cite-stats`
Citation statistics.

```bash
./rairos.sh cite-stats
./rairos.sh cite-stats --paper 2601.00155
./rairos.sh cite-stats --top 10
```

### `./rairos.sh cite-import`
Import citation links from JSON.

```bash
./rairos.sh cite-import --file citations.json
./rairos.sh cite-import --extract --paper 2601.00155  # Extract from plain text
```

### `./rairos.sh cite-fetch`
Fetch citations from OpenAlex API.

```bash
./rairos.sh cite-fetch 2601.00155
```

---

## Research Automation

### `./rairos.sh agent <topic>`
Autonomous research agent — searches, reads, analyzes.

```bash
./rairos.sh agent "scaling laws for RL"
./rairos.sh agent "VLA generalization" --max-papers 20 --max-time 30
```

### `./rairos.sh hypothesize <topic>`
Generate research hypotheses from gaps and trends.

```bash
./rairos.sh hypothesize "adaptive inference" --gap-context "..."
./rairos.sh hypothesize "multi-agent reasoning" --num-hypotheses 5 --json
```

### `./rairos.sh pipeline <topic>`
Full research pipeline: gap analysis → hypothesis → experiment.

```bash
./rairos.sh pipeline "long context LLM"
./rairos.sh pipeline "multi-modal RL" --skip-experiments
```

### `./rairos.sh research`
Manage research logs.

```bash
./rairos.sh research list
./rairos.sh research add "Noticed interesting scaling behavior in sparse models"
```

### `./rairos.sh digest`
Generate weekly research digest.

```bash
./rairos.sh digest --weeks 2
```

### `./rairos.sh demo`
Run end-to-end pipeline demo.

```bash
./rairos.sh demo              # Full demo
./rairos.sh demo --quick      # 30-second demo
```

---

## Subscriptions & Monitoring

### `./rairos.sh daemon`
Start Rairos as a background service.

```bash
./rairos.sh daemon
./rairos.sh daemon --port 8080 --foreground
```

### `./rairos.sh subscribe`
Subscribe to arXiv searches with continuous monitoring.

```bash
./rairos.sh subscribe "transformer attention" --interval 60 --auto-add
```

### `./rairos.sh signal <keyword>`
Match event keywords against Gene Pool patterns.

```bash
./rairos.sh signal "transformer"
./rairos.sh signal "reinforcement learning"
```

---

## Environment & Diagnostics

### `./rairos.sh doctor`
Diagnose environment and report issues.

```bash
./rairos.sh doctor
```

### `./rairos.sh stats`
Database statistics.

```bash
./rairos.sh stats
./rairos.sh stats --json
```

### `./rairos.sh status`
Real-time database status.

```bash
./rairos.sh status
```

### `./rairos.sh setup`
Run the setup wizard.

```bash
./rairos.sh setup
./rairos.sh setup --guide     # Quick start guide only
```

---

## Deduplication

### `./rairos.sh dedup`
Find duplicate papers by DOI/title.

```bash
./rairos.sh dedup find                    # Find duplicates
./rairos.sh dedup semantic 2601.00155    # Semantic similarity
./rairos.sh dedup remove ID1,ID2         # Remove duplicates
./rairos.sh dedup stats                   # Show embedding coverage
```

### `./rairos.sh similar <id>`
Find semantically similar papers.

```bash
./rairos.sh similar 2601.00155
./rairos.sh similar 2601.00155 --limit 10 --threshold 0.8
```

### `./rairos.sh merge`
Merge duplicate papers.

```bash
./rairos.sh merge TARGET_ID DUPLICATE_ID
./rairos.sh merge --auto                  # Auto-merge high-similarity pairs
```

---

## Queue & Data Management

### `./rairos.sh queue`
Manage the processing queue.

```bash
./rairos.sh queue list
./rairos.sh queue add 2601.00155
./rairos.sh queue clear
```

### `./rairos.sh cache`
Manage cached data.

```bash
./rairos.sh cache stats
./rairos.sh cache clear
./rairos.sh cache list
```

### `./rairos.sh parse <id>`
Parse a paper's full text.

```bash
./rairos.sh parse 2601.00155
```

---

## Web UI & Visualization

### `./rairos.sh dashboard`
Start the Web UI dashboard.

```bash
./rairos.sh dashboard
./rairos.sh dashboard --port 3000
```

### `./rairos.sh visual`
Generate visualizations.

```bash
./rairos.sh visual 2601.00155
./rairos.sh visual 2601.00155 --output ./viz.html
```

### `./rairos.sh slides`
Generate slides from papers.

```bash
./rairos.sh slides 2601.00155 --format md --style academic
./rairos.sh slides 2601.00155 2302.00763 --slides 15 --notes
```

---

## Research Memory

### `./rairos.sh stance-add`
Add a research stance.

```bash
./rairos.sh stance-add --topic "scaling laws" --claim "Inference cost grows O(n²)" --stance supported
```

### `./rairos.sh stance-list`
List stances by topic or tag.

```bash
./rairos.sh stance-list --topic "scaling laws"
```

### `./rairos.sh narrative`
Manage research narratives.

```bash
./rairos.sh narrative list
./rairos.sh narrative track "multi-modal reasoning"
```

### `./rairos.sh story <topic>`
Weave research into narrative stories.

```bash
./rairos.sh story "evolution of transformer architecture"
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
