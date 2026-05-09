# Configuration

## Environment Variables

| Variable | Default | Description |
|----------|---------|-------------|
| `AI_RESEARCH_OS_DB` | `~/.ai_research_os/papers.db` | SQLite database path |
| `OLLAMA_BASE_URL` | `http://localhost:11434` | Ollama API endpoint (for dedup-semantic) |
| `OPENAI_API_KEY` | — | OpenAI API key for LLM calls |
| `ANTHROPIC_API_KEY` | — | Anthropic API key (claude-sonnet-4, etc.) |
| `MINIMAX_CN_API_KEY` | — | MiniMax API key (minimax-m2.7-highspeed, etc.) |
| `OPENAI_BASE_URL` | `https://api.openai.com/v1` | OpenAI-compatible base URL |
| `GITHUB_PERSONAL_ACCESS_TOKEN` | — | GitHub API token for PR workflow |
| `DISCORD_WEBHOOK_URL` | — | Discord webhook for gap notifications |
| `FEISHU_WEBHOOK_URL` | — | Feishu webhook for gap notifications |
| `LOG_LEVEL` | `INFO` | Logging level (DEBUG, INFO, WARNING, ERROR) |
| `AIROS_DEFAULT_MODEL` | `gpt-4o-mini` | Default LLM model for research |

## LLM Model Selection

Rairos supports multiple LLM providers. Set credentials via environment variables:

```bash
# MiniMax (recommended for speed/cost)
export MINIMAX_CN_API_KEY=your_key

# OpenAI-compatible
export OPENAI_API_KEY=your_key
export OPENAI_BASE_URL=https://api.openai.com/v1

# Anthropic
export ANTHROPIC_API_KEY=your_key
```

The `research --model` flag overrides the default. See `rairos research --help` for available models.

## Webhook Notifications

Gap alerts and paradigm shift notifications can be sent to Discord and Feishu:

```bash
export DISCORD_WEBHOOK_URL=https://discord.com/api/webhooks/...
export FEISHU_WEBHOOK_URL=https://open.feishu.cn/open-apis/bot/v2/hook/...
```

Rich payloads include: gap type, polarity, confidence score, new keywords, and timestamps.

## GenePool Saturation

GenePool automatically archives underperforming capsules. Configure via `config.toml`:

```toml
[gene_pool]
saturation_threshold = 0.85    # Archive when avg diversity pressure exceeds this
decay_lambda = 0.01            # Half-life ~69 days
min_impact_score = 0.1          # Archive if impact falls below this
consecutive_cycles = 3         # Cycles below threshold before archival
```

## Ollama (Optional)

Required only for `dedup-semantic` command.

```bash
ollama pull nomic-embed-text
```

## OpenAlex (Optional)

Required only for `cite-fetch` command. No API key needed — OpenAlex is free.
