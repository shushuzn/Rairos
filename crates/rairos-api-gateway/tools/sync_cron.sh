#!/bin/bash
# Paper Sync Cron Script
# Usage: Add to crontab for periodic sync
#
# For every minute:
# * * * * * /path/to/Rairos/crates/rairos-api-gateway/tools/sync_cron.sh >> /var/log/rairos_sync.log 2>&1
#
# For every 5 minutes:
# */5 * * * * /path/to/Rairos/crates/rairos-api-gateway/tools/sync_cron.sh >> /var/log/rairos_sync.log 2>&1

set -e

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../../../.." && pwd)"
LOG_FILE="${LOG_FILE:-/var/log/rairos_sync.log}"

cd "$PROJECT_ROOT"

log() {
    echo "$(date '+%Y-%m-%d %H:%M:%S'): $1" | tee -a "$LOG_FILE"
}

log "Starting paper sync..."

# Run the Python sync script
if python3 "$SCRIPT_DIR/sync_papers.py"; then
    log "Paper sync completed successfully"
else
    log "Paper sync failed with exit code $?"
    exit 1
fi
