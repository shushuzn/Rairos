#!/bin/bash
# Manual Paper Sync Test
# Tests the sync between SQLite and PostgreSQL

set -e

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../../../.." && pwd)"

cd "$PROJECT_ROOT"

echo "=== Rairos Paper Sync Test ==="
echo ""

# Check if SQLite DB exists
if [ ! -f "rairos.db" ]; then
    echo "ERROR: rairos.db not found. Run 'cargo run -p rairos-cli' first."
    exit 1
fi

# Count papers in SQLite
SQLITE_COUNT=$(sqlite3 rairos.db "SELECT COUNT(*) FROM papers" 2>/dev/null || echo "0")
echo "SQLite papers: $SQLITE_COUNT"

# Check PostgreSQL connection
if [ -z "$DATABASE_URL" ]; then
    echo "WARNING: DATABASE_URL not set. Skipping PostgreSQL check."
    echo "Set it with: export DATABASE_URL=postgres://postgres:postgres@localhost:5432/rairos_api"
else
    PGPASSWORD=$(echo "$DATABASE_URL" | grep -oP '(?<=:)(.*?)(?=@)' | tail -1)
    PGHOST=$(echo "$DATABASE_URL" | grep -oP '(?<=@)(.*?)(?=:)' | head -1)
    PGPORT=$(echo "$DATABASE_URL" | grep -oP '(?<=:)\d+(?=/)' | head -1)
    PGDATABASE=$(echo "$DATABASE_URL" | grep -oP '(?<=:5432/)(.*)' | head -1)
    PGUSER=$(echo "$DATABASE_URL" | grep -oP '(?<=://)(.*?)(?=:)' | head -1)
    
    export PGPASSWORD PGHOST PGPORT PGDATABASE PGUSER
    
    PG_COUNT=$(psql -h "$PGHOST" -p "$PGPORT" -U "$PGUSER" -d "$PGDATABASE" -t -c "SELECT COUNT(*) FROM papers" 2>/dev/null || echo "0")
    echo "PostgreSQL papers: $PG_COUNT"
    
    if [ "$PG_COUNT" -gt 0 ]; then
        echo "Sync is working! Papers are in PostgreSQL."
    else
        echo "PostgreSQL is empty. Run sync script to populate."
    fi
fi

echo ""
echo "=== To run sync manually ==="
echo "  python3 $SCRIPT_DIR/sync_papers.py"
echo ""
echo "=== To set up cron (every 15 min) ==="
echo "  */15 * * * * $SCRIPT_DIR/sync_cron.sh"
