# Advanced Commands Reference

Complete reference for all 23 CLI subcommands. See [README.md](README.md) for installation and quick-start.

---

## Paper Processing (main flow)

### `python -m cli <input> [flags]`

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
python -m cli stats
```

### `status`
Show current processing status and queue summary.

```bash
python -m cli status
```

### `cache`
Manage paper cache.

```bash
python -m cli cache --stats     # Show cache stats
python -m cli cache --clear     # Clear all cache
python -m cli cache --get UID  # Get cached path for UID
python -m cli cache --set UID PATH  # Set cached path for UID
```

### `import`
Batch add papers by arXiv ID / DOI / URL.

```bash
# One or more IDs
python -m cli import 2601.00155 10.48550/arXiv.2601.00155

# From file (one ID per line)
python -m cli import --file ids.txt

# With checkpoint (save/resume progress)
python -m cli import --file ids.txt --checkpoint ckpt.json
python -m cli import --resume --checkpoint ckpt.json
```

### `export`
Export DB to CSV or JSON.

```bash
python -m cli export
python -m cli export --format csv
python -m cli export --format json
```

### `search`
Full-text search with filters.

```bash
python -m cli search "scaling law"
python -m cli search "transformer" --tag LLM --limit 20
```

### `list`
List papers with sort/filter.

```bash
python -m cli list
python -m cli list --tag LLM --sort updated --limit 50
```

### `similar`
Find semantically similar papers via embeddings.

```bash
python -m cli similar PAPER_ID
python -m cli similar PAPER_ID --threshold 0.8 --limit 10
```

Requires Ollama running with `ollama serve` and `ollama pull nomic-embed-text`.

### `queue`
Manage pending paper queue.

```bash
python -m cli queue --list   # List pending papers
python -m cli queue --clear  # Reset all to idle
```

### `dedup`
Find exact duplicates by DOI/title.

```bash
python -m cli dedup
python -m cli dedup --dry-run
```

### `dedup-semantic`
Semantic deduplication via Ollama embeddings.

```bash
# Generate embeddings for all papers without them
python -m cli dedup-semantic --generate

# Show embedding coverage stats
python -m cli dedup-semantic --stats

# Run semantic dedup (requires embeddings)
python -m cli dedup-semantic
```

Requires Ollama running (`ollama serve`) and `ollama pull nomic-embed-text`.

### `merge`
Merge two duplicate papers.

```bash
python -m cli merge TARGET_ID DUPLICATE_ID
python -m cli merge --keep semantic --auto  # Auto-merge high-similarity pairs
```

### `citations --from`
Show papers cited by a paper (backward citations).

```bash
python -m cli citations --from PAPER_ID
```

### `citations --to`
Show papers citing a paper (forward citations).

```bash
python -m cli citations --to PAPER_ID
```

### `cite-fetch`
Fetch citations from OpenAlex API.

```bash
python -m cli cite-fetch PAPER_ID
python -m cli cite-fetch PAPER_ID1 PAPER_ID2  # Multiple
```

### `cite-import`
Bulk import citation edges from JSON.

```bash
python -m cli cite-import --file citations.json
```

### `cite-stats`
Citation graph statistics.

```bash
python -m cli cite-stats
python -m cli cite-stats --top 10  # Top cited papers
```

### `paper2code`
Generate code implementation from paper.

```bash
python -m cli paper2code PAPER_ID
python -m cli paper2code PAPER_ID --mode minimal
python -m cli paper2code PAPER_ID --mode standard
python -m cli paper2code --rebuild PAPER_ID  # Rebuild existing
```

### `evoskill`
EvoSkill benchmark evaluation.

```bash
# Initialize benchmark task
python -m cli evoskill --init --task TASK --dataset dataset.csv

# Run benchmark evaluation
python -m cli evoskill --benchmark
python -m cli evoskill --benchmark --continue  # Continue previous

# Generate evaluation report
python -m cli evoskill --report
```

### `rag`
Run RAG pipeline (paper2code + tests + benchmark).

```bash
python -m cli rag PAPER_ID
python -m cli rag PAPER_ID --mode minimal
```

### `visual`
Extract figures, formulas, tables from PDF.

```bash
python -m cli visual PAPER_ID
python -m cli visual PAPER_ID --output ./visuals/
```

### `kg`
Build/query knowledge graph.

```bash
python -m cli kg
python -m cli kg --export json
python -m cli kg --export graphml
```

### `research`
Run continuous research loop.

```bash
python -m cli research
python -m cli research --loop
python -m cli research --limit 10
```

---

## Ollama Setup (for semantic features)

```bash
# Start Ollama locally (required for dedup-semantic, similar)
ollama serve

# Pull embedding model (one-time)
ollama pull nomic-embed-text
```
