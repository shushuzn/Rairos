# Advanced Commands Reference

Complete reference for all 23 CLI subcommands. See [README.md](README.md) for installation and quick-start.

---

## Paper Processing (main flow)

### `rairos <input> [flags]`

| Argument | Description | Default |
|----------|-------------|---------|
| `input` | arXiv ID/URL or DOI/doi.org URL | (required) |
| `--pdf <path>` | Use local PDF | - |
| `--ocr` | Enable OCR fallback | off |
| `--ocr-lang <lang>` | OCR language | `chi_sim+eng` |
| `--ocr-zoom <zoom>` | OCR render zoom | 2 |
| `--max-pages <n>` | Limit parsed pages | unlimited |
| `--ai` | Enable AI draft generation | off |
| `--ai-cnote` | AI-fill all C-Notes from existing P-Notes | off |
| `--ai-max-papers <n>` | Max P-notes to feed per C-note | 10 |
| `--model <name>` | LLM model name | `qwen3.5-plus` |
| `--base-url <url>` | API endpoint | DashScope compatible |
| `--api-key <key>` | API key | env `OPENAI_API_KEY` |
| `--ai-max-chars <n>` | Max chars of extracted text sent to AI | 8000 |
| `--tags <t1,t2>` | Comma-separated tags | auto-inferred |
| `--category <dir>` | Folder under root to place P-Note | auto |
| `--concept-dir <dir>` | Folder under root to place C-Notes | auto |
| `--comparison-dir <dir>` | Folder under root to place M-Notes | auto |

---

## CLI Subcommands

### `stats`
DB overview: total papers, status breakdown, queue size.

```bash
rairos stats
```

### `status`
Show current processing status and queue summary.

```bash
rairos status
```

### `cache`
Manage paper cache.

```bash
rairos cache --stats     # Show cache stats
rairos cache --clear     # Clear all cache
rairos cache --get UID  # Get cached path for UID
rairos cache --set UID PATH  # Set cached path for UID
```

### `import`
Batch add papers by arXiv ID / DOI / URL.

```bash
# One or more IDs
rairos import 2601.00155 10.48550/arXiv.2601.00155

# From file (one ID per line)
rairos import --file ids.txt

# With checkpoint (save/resume progress)
rairos import --file ids.txt --checkpoint ckpt.json
rairos import --resume --checkpoint ckpt.json
```

### `export`
Export DB to CSV or JSON.

```bash
rairos export
rairos export --format csv
rairos export --format json
```

### `search`
Full-text search with filters.

```bash
rairos search "scaling law"
rairos search "transformer" --tag LLM --limit 20
```

### `list`
List papers with sort/filter.

```bash
rairos list
rairos list --tag LLM --sort updated --limit 50
```

### `similar`
Find semantically similar papers via embeddings.

```bash
rairos similar PAPER_ID
rairos similar PAPER_ID --threshold 0.8 --limit 10
```

Requires Ollama running with `ollama serve` and `ollama pull nomic-embed-text`.

### `queue`
Manage pending paper queue.

```bash
rairos queue --list   # List pending papers
rairos queue --clear  # Reset all to idle
```

### `dedup`
Find exact duplicates by DOI/title.

```bash
rairos dedup
rairos dedup --dry-run
```

### `dedup-semantic`
Semantic deduplication via Ollama embeddings.

```bash
# Generate embeddings for all papers without them
rairos dedup-semantic --generate

# Show embedding coverage stats
rairos dedup-semantic --stats

# Run semantic dedup (requires embeddings)
rairos dedup-semantic
```

Requires Ollama running (`ollama serve`) and `ollama pull nomic-embed-text`.

### `merge`
Merge two duplicate papers.

```bash
rairos merge TARGET_ID DUPLICATE_ID
rairos merge --keep semantic --auto  # Auto-merge high-similarity pairs
```

### `citations --from`
Show papers cited by a paper (backward citations).

```bash
rairos citations --from PAPER_ID
```

### `citations --to`
Show papers citing a paper (forward citations).

```bash
rairos citations --to PAPER_ID
```

### `cite-fetch`
Fetch citations from OpenAlex API.

```bash
rairos cite-fetch PAPER_ID
rairos cite-fetch PAPER_ID1 PAPER_ID2  # Multiple
```

### `cite-import`
Bulk import citation edges from JSON.

```bash
rairos cite-import --file citations.json
```

### `cite-stats`
Citation graph statistics.

```bash
rairos cite-stats
rairos cite-stats --top 10  # Top cited papers
```

### `paper2code`
Generate code implementation from paper.

```bash
rairos paper2code PAPER_ID
rairos paper2code PAPER_ID --mode minimal
rairos paper2code PAPER_ID --mode standard
rairos paper2code --rebuild PAPER_ID  # Rebuild existing
```

### `evoskill`
EvoSkill benchmark evaluation.

```bash
# Initialize benchmark task
rairos evoskill --init --task TASK --dataset dataset.csv

# Run benchmark evaluation
rairos evoskill --benchmark
rairos evoskill --benchmark --continue  # Continue previous

# Generate evaluation report
rairos evoskill --report
```

### `rag`
Run RAG pipeline (paper2code + tests + benchmark).

```bash
rairos rag PAPER_ID
rairos rag PAPER_ID --mode minimal
```

### `visual`
Extract figures, formulas, tables from PDF.

```bash
rairos visual PAPER_ID
rairos visual PAPER_ID --output ./visuals/
```

### `kg`
Build/query knowledge graph.

```bash
rairos kg
rairos kg --export json
rairos kg --export graphml
```

### `research`
Run continuous research loop.

```bash
rairos research
rairos research --loop
rairos research --limit 10
```

---

## Ollama Setup (for semantic features)

```bash
# Start Ollama locally (required for dedup-semantic, similar)
ollama serve

# Pull embedding model (one-time)
ollama pull nomic-embed-text
```
