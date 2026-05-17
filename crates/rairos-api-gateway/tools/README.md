# Paper Sync Tool

Syncs papers from the main SQLite database (`rairos.db`) to the API Gateway PostgreSQL database.

## Why?

- **SQLite** (rairos-core): Primary storage for research data
- **PostgreSQL** (rairos-api-gateway): Scalable API storage

The sync tool keeps both databases in sync so the API can serve paper data.

## Features

- **Retry logic**: Exponential backoff for failed connections
- **Incremental sync**: Only syncs new/updated papers since last sync
- **Batch processing**: Configurable batch size for memory efficiency
- **State tracking**: JSON-based state file for monitoring
- **Status checking**: Separate script to check sync health

## Setup

### 1. Install dependencies

```bash
pip install -r requirements.txt
```

### 2. Set environment variables

```bash
export RAIROS_DB=rairos.db              # SQLite database path
export DATABASE_URL=postgres://...       # PostgreSQL connection URL
```

### 3. Run initial sync

```bash
python3 sync_papers.py --full
```

## Usage

### Manual Sync

```bash
# Incremental sync (only new papers since last sync)
python3 sync_papers.py

# Full sync
python3 sync_papers.py --full

# Custom batch size
python3 sync_papers.py --batch-size 200
```

### Cron Job Setup

Add to crontab for periodic syncs:

```bash
# Edit crontab
crontab -e

# Add line for every minute
* * * * * /path/to/tools/sync_cron.sh >> /var/log/rairos_sync.log 2>&1

# Or every 5 minutes
*/5 * * * * /path/to/tools/sync_cron.sh >> /var/log/rairos_sync.log 2>&1
```

### Check Sync Status

```bash
# View sync status
python3 check_sync_status.py

# Check from shell
./test_sync.sh
```

## How It Works

1. Reads `papers` table from SQLite (ordered by `published` DESC)
2. Tracks sync state in `.sync_state.json`:
   - `last_sync`: Timestamp of last successful sync
   - `papers_synced_total`: Total papers synced
   - `syncs_failed_total`: Total failed syncs
   - `last_error`: Last error message
3. Fetches papers in configurable batches (default: 100)
4. Inserts/updates papers in PostgreSQL with ON CONFLICT handling
5. Retries failed batches with exponential backoff (max 3 attempts)

## State File

`.sync_state.json` example:

```json
{
  "last_sync": "2025-05-17T14:30:00",
  "last_sync_time": "2025-05-17T14:30:05",
  "papers_synced_total": 5432,
  "syncs_failed_total": 2,
  "last_error": null
}
```

## Files

| File | Purpose |
|------|---------|
| `sync_papers.py` | Main sync script with retry logic |
| `sync_cron.sh` | Cron wrapper script |
| `check_sync_status.py` | Python status checker |
| `test_sync.sh` | Shell status checker |
| `requirements.txt` | Python dependencies |
