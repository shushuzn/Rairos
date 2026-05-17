#!/bin/bash
# Paper Sync Cron Script
# Usage: Add to crontab for periodic sync
# */15 * * * * /path/to/Rairos/crates/rairos-api-gateway/tools/sync_cron.sh

set -e

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../../../.." && pwd)"

cd "$PROJECT_ROOT"

echo "$(date): Starting paper sync..."

# Run the Python sync script
python3 "$SCRIPT_DIR/sync_papers.py"

echo "$(date): Paper sync complete"
