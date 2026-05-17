#!/usr/bin/env python3
"""
Sync Status Checker

Check the status of the SQLite to PostgreSQL sync.
Usage: python check_sync_status.py
"""

import os
import sys
import json
from datetime import datetime

SYNC_STATE_FILE = '.sync_state.json'

def parse_datetime(dt_str):
    if not dt_str:
        return None
    return datetime.fromisoformat(dt_str)

def main():
    if not os.path.exists(SYNC_STATE_FILE):
        print("No sync state found. Run sync_papers.py first.")
        sys.exit(1)

    with open(SYNC_STATE_FILE, 'r') as f:
        state = json.load(f)

    print("=== Rairos Paper Sync Status ===\n")

    last_sync = parse_datetime(state.get('last_sync'))
    last_sync_time = parse_datetime(state.get('last_sync_time'))

    print(f"Last sync:        {last_sync or 'Never'}")
    print(f"Sync time:        {last_sync_time or 'N/A'}")

    if last_sync_time:
        ago = datetime.now() - last_sync_time
        minutes = int(ago.total_seconds() / 60)
        if minutes < 1:
            print(f"Time since sync:  Just now")
        else:
            print(f"Time since sync:  {minutes} minutes ago")

    print(f"\nPapers synced (total):  {state.get('papers_synced_total', 0)}")
    print(f"Syncs failed (total):   {state.get('syncs_failed_total', 0)}")

    last_error = state.get('last_error')
    if last_error:
        print(f"\nLast error:")
        print(f"  {last_error}")
    else:
        print(f"\nLast error:           None")

    if last_sync_time:
        if (datetime.now() - last_sync_time).total_seconds() > 300:
            print("\n⚠️  WARNING: Last sync was more than 5 minutes ago!")
            sys.exit(1)
        else:
            print("\n✅ Sync is healthy")
            sys.exit(0)
    else:
        print("\n❌ No successful sync completed yet")
        sys.exit(1)

if __name__ == '__main__':
    main()
