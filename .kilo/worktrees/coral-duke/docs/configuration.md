# Configuration

## Environment Variables

| Variable | Default | Description |
|----------|---------|-------------|
| `AI_RESEARCH_OS_DB` | `~/.ai_research_os/papers.db` | SQLite database path |
| `OLLAMA_BASE_URL` | `http://localhost:11434` | Ollama API endpoint (for dedup-semantic) |

## Ollama (Optional)

Required only for `dedup-semantic` command.

```bash
ollama pull nomic-embed-text
```

## OpenAlex (Optional)

Required only for `cite-fetch` command. No API key needed — OpenAlex is free.

## Unused Legacy File

`API_CONFIG.md` (removed) previously documented DashScope API configuration for the deprecated paper-writing pipeline (`--ai` flag). The current CLI does not use DashScope.
