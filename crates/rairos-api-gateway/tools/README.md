# Paper Sync Tool

Syncs papers from the main SQLite database (`rairos.db`) to the API Gateway PostgreSQL database.

## Why?

- **SQLite** (rairos-core): Primary storage for research data
- **PostgreSQL** (rairos-api-gateway): Scalable API storage

The sync tool keeps both databases in sync so the API can serve paper data.

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
python3 sync_papers.py
```

## Usage

### Manual Sync

```bash
# Incremental sync (only new papers since last sync)
python3 sync_papers.py

# Full sync
python3 sync_papers.py --full
```

### Cron Job Setup

Add to crontab for periodic syncs:

```bash
# Edit crontab
crontab -e

# Add line for 15-minute sync
*/15 * * * * /path/to/tools/sync_cron.sh >> /var/log/rairos_sync.log 2>&1
```

## How it works

1. Reads `papers` table from SQLite
2. Compares with last sync time (stored in `.last_paper_sync`)
3. Inserts/updates papers in PostgreSQL
4. Saves sync timestamp for next incremental sync

## Testing

```bash
# Check sync status
./test_sync.sh
```

## Files

| File | Purpose |
|------|---------|
| `sync_papers.py` | Main sync script |
| `sync_cron.sh` | Cron wrapper script |
| `test_sync.sh` | Status check script |
| `requirements.txt` | Python dependencies |
