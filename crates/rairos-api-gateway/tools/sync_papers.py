#!/usr/bin/env python3
"""
SQLite to PostgreSQL Sync Tool for Rairos API Gateway

Syncs papers from the main SQLite database to the API Gateway PostgreSQL database.
Features:
    - Retry logic with exponential backoff
    - Incremental sync based on last update timestamp
    - Batch processing for efficiency
    - Sync status tracking

Usage:
    python sync_papers.py [--full] [--batch-size 100]

Options:
    --full        Perform full sync (default: incremental based on last_sync)
    --batch-size  Number of papers to sync per batch (default: 100)
"""

import sqlite3
import psycopg2
import os
import sys
import argparse
import logging
import time
import json
from datetime import datetime
from typing import Optional, List, Dict, Any
from contextlib import contextmanager

logging.basicConfig(
    level=logging.INFO,
    format='%(asctime)s - %(levelname)s - %(message)s',
    handlers=[
        logging.StreamHandler(),
        logging.FileHandler('sync.log')
    ]
)
logger = logging.getLogger(__name__)

SQLITE_DB_PATH = os.environ.get('RAIROS_DB', 'rairos.db')
POSTGRES_URL = os.environ.get('DATABASE_URL', 'postgres://postgres:postgres@localhost:5432/rairos_api')
SYNC_STATE_FILE = '.sync_state.json'
MAX_RETRIES = 3
INITIAL_RETRY_DELAY = 1


class SyncState:
    """Manages sync state for incremental syncs."""

    def __init__(self, state_file: str):
        self.state_file = state_file
        self.data = self._load()

    def _load(self) -> Dict[str, Any]:
        if os.path.exists(self.state_file):
            try:
                with open(self.state_file, 'r') as f:
                    return json.load(f)
            except (json.JSONDecodeError, IOError):
                pass
        return {
            'last_sync': None,
            'last_sync_time': None,
            'papers_synced_total': 0,
            'syncs_failed_total': 0,
            'last_error': None
        }

    def save(self):
        with open(self.state_file, 'w') as f:
            json.dump(self.data, f, indent=2)

    @property
    def last_sync(self) -> Optional[datetime]:
        if self.data.get('last_sync'):
            return datetime.fromisoformat(self.data['last_sync'])
        return None

    @last_sync.setter
    def last_sync(self, value: datetime):
        self.data['last_sync'] = value.isoformat()
        self.data['last_sync_time'] = datetime.now().isoformat()

    def record_success(self, count: int):
        self.data['papers_synced_total'] = self.data.get('papers_synced_total', 0) + count
        self.data['last_error'] = None
        self.save()

    def record_failure(self, error: str):
        self.data['syncs_failed_total'] = self.data.get('syncs_failed_total', 0) + 1
        self.data['last_error'] = error
        self.save()


@contextmanager
def get_postgres_connection(url: str, retries: int = MAX_RETRIES):
    """Get PostgreSQL connection with retry logic."""
    delay = INITIAL_RETRY_DELAY
    last_error = None

    for attempt in range(retries):
        try:
            conn = psycopg2.connect(url)
            yield conn
            conn.close()
            return
        except psycopg2.OperationalError as e:
            last_error = e
            logger.warning(f"PostgreSQL connection attempt {attempt + 1} failed: {e}")
            if attempt < retries - 1:
                time.sleep(delay)
                delay *= 2

    raise last_error


def connect_sqlite(path: str) -> sqlite3.Connection:
    """Connect to SQLite database."""
    conn = sqlite3.connect(path)
    conn.row_factory = sqlite3.Row
    return conn


def paper_from_row(row: sqlite3.Row) -> dict:
    """Convert SQLite row to paper dict."""
    return {
        'id': row['id'],
        'title': row['title'],
        'abstract': row['abstract_text'] or '',
        'authors': row['authors'] or '',
        'categories': row['categories'] or '',
        'published': row['published'],
    }


def fetch_papers_from_sqlite(
    conn: sqlite3.Connection,
    since: Optional[datetime] = None,
    limit: int = 100,
    offset: int = 0
) -> List[dict]:
    """Fetch papers from SQLite with pagination."""
    cursor = conn.cursor()

    if since:
        cursor.execute(
            """SELECT id, title, abstract_text, authors, categories, published
               FROM papers
               WHERE published >= ?
               ORDER BY published DESC
               LIMIT ? OFFSET ?""",
            (since.isoformat(), limit, offset)
        )
    else:
        cursor.execute(
            """SELECT id, title, abstract_text, authors, categories, published
               FROM papers
               ORDER BY published DESC
               LIMIT ? OFFSET ?""",
            (limit, offset)
        )

    return [paper_from_row(row) for row in cursor.fetchall()]


def get_total_paper_count(conn: sqlite3.Connection, since: Optional[datetime] = None) -> int:
    """Get total count of papers to sync."""
    cursor = conn.cursor()
    if since:
        cursor.execute(
            "SELECT COUNT(*) FROM papers WHERE published >= ?",
            (since.isoformat(),)
        )
    else:
        cursor.execute("SELECT COUNT(*) FROM papers")
    return cursor.fetchone()[0]


def sync_papers_batch(
    pg_conn: psycopg2.extensions.connection,
    papers: List[dict]
) -> tuple[int, int]:
    """Sync a batch of papers to PostgreSQL. Returns (success_count, failure_count)."""
    if not papers:
        return 0, 0

    cursor = pg_conn.cursor()
    success = 0
    failed = 0

    for paper in papers:
        try:
            cursor.execute("""
                INSERT INTO papers (id, title, abstract, authors, categories, published, created_at)
                VALUES (%(id)s, %(title)s, %(abstract)s, %(authors)s, %(categories)s, %(published)s, NOW())
                ON CONFLICT (id) DO UPDATE SET
                    title = EXCLUDED.title,
                    abstract = EXCLUDED.abstract,
                    authors = EXCLUDED.authors,
                    categories = EXCLUDED.categories,
                    published = EXCLUDED.published
            """, paper)
            success += 1
        except Exception as e:
            logger.error(f"Failed to sync paper {paper.get('id', 'unknown')}: {e}")
            failed += 1

    return success, failed


def get_paper_count(pg_conn: psycopg2.extensions.connection) -> int:
    """Get count of papers in PostgreSQL."""
    cursor = pg_conn.cursor()
    cursor.execute("SELECT COUNT(*) FROM papers")
    return cursor.fetchone()[0]


def run_sync(full: bool, batch_size: int) -> bool:
    """Run the sync process. Returns True on success, False on failure."""
    state = SyncState(SYNC_STATE_FILE)

    if not os.path.exists(SQLITE_DB_PATH):
        logger.error(f"SQLite DB not found at {SQLITE_DB_PATH}")
        state.record_failure(f"SQLite DB not found: {SQLITE_DB_PATH}")
        return False

    logger.info(f"Starting paper sync...")
    logger.info(f"SQLite DB: {SQLITE_DB_PATH}")
    logger.info(f"Batch size: {batch_size}")

    if full:
        logger.info("FULL SYNC MODE")
        last_sync = None
    else:
        last_sync = state.last_sync
        if last_sync:
            logger.info(f"Incremental sync since: {last_sync}")
        else:
            logger.info("No previous sync found, performing full sync")
            last_sync = None

    try:
        sqlite_conn = connect_sqlite(SQLITE_DB_PATH)

        try:
            total_count = get_total_paper_count(sqlite_conn, since=last_sync)
            logger.info(f"Total papers to sync: {total_count}")

            if total_count == 0:
                logger.info("No papers to sync")
                return True

            offset = 0
            total_synced = 0
            total_failed = 0

            with get_postgres_connection(POSTGRES_URL) as pg_conn:
                while offset < total_count:
                    papers = fetch_papers_from_sqlite(
                        sqlite_conn,
                        since=last_sync,
                        limit=batch_size,
                        offset=offset
                    )

                    if not papers:
                        break

                    synced, failed = sync_papers_batch(pg_conn, papers)
                    total_synced += synced
                    total_failed += failed

                    logger.info(f"Batch: {len(papers)} papers, {synced} synced, {failed} failed")

                    pg_conn.commit()
                    offset += len(papers)

                    if failed > 0 and failed < len(papers):
                        logger.warning(f"Some papers failed to sync, continuing...")

            state.last_sync = datetime.now()
            state.record_success(total_synced)

            before_count = get_paper_count(pg_conn) - total_synced
            after_count = get_paper_count(pg_conn)

            logger.info(f"Sync complete: {total_synced} synced, {total_failed} failed")
            logger.info(f"PostgreSQL papers: {before_count} -> {after_count}")

            return True

        finally:
            sqlite_conn.close()

    except Exception as e:
        logger.error(f"Sync failed: {e}")
        state.record_failure(str(e))
        return False


def main():
    parser = argparse.ArgumentParser(description='Sync papers from SQLite to PostgreSQL')
    parser.add_argument('--full', action='store_true', help='Perform full sync')
    parser.add_argument('--batch-size', type=int, default=100,
                        help='Number of papers to sync per batch (default: 100)')
    args = parser.parse_args()

    success = run_sync(full=args.full, batch_size=args.batch_size)
    sys.exit(0 if success else 1)


if __name__ == '__main__':
    main()
