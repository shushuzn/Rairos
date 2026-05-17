#!/usr/bin/env python3
"""
SQLite to PostgreSQL Sync Tool for Rairos API Gateway

Syncs papers from the main SQLite database to the API Gateway PostgreSQL database.
Usage: python sync_papers.py [--full]

Options:
    --full    Perform full sync (default: incremental based on last_sync)
"""

import sqlite3
import psycopg2
import os
import sys
import argparse
import logging
from datetime import datetime
from typing import Optional, List, Tuple

logging.basicConfig(level=logging.INFO, format='%(asctime)s - %(levelname)s - %(message)s')
logger = logging.getLogger(__name__)

SQLITE_DB_PATH = os.environ.get('RAIROS_DB', 'rairos.db')
POSTGRES_URL = os.environ.get('DATABASE_URL', 'postgres://postgres:postgres@localhost:5432/rairos_api')
LAST_SYNC_FILE = '.last_paper_sync'

def get_last_sync_time() -> Optional[datetime]:
    """Get the last sync timestamp from file."""
    if os.path.exists(LAST_SYNC_FILE):
        with open(LAST_SYNC_FILE, 'r') as f:
            return datetime.fromisoformat(f.read().strip())
    return None

def set_last_sync_time(dt: datetime):
    """Save the last sync timestamp."""
    with open(LAST_SYNC_FILE, 'w') as f:
        f.write(dt.isoformat())

def connect_sqlite(path: str) -> sqlite3.Connection:
    """Connect to SQLite database."""
    conn = sqlite3.connect(path)
    conn.row_factory = sqlite3.Row
    return conn

def connect_postgres(url: str) -> psycopg2.extensions.connection:
    """Connect to PostgreSQL database."""
    return psycopg2.connect(url)

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

def fetch_papers_from_sqlite(conn: sqlite3.Connection, since: Optional[datetime] = None) -> List[dict]:
    """Fetch papers from SQLite, optionally since a given timestamp."""
    cursor = conn.cursor()
    
    if since:
        cursor.execute(
            """SELECT id, title, abstract_text, authors, categories, published 
               FROM papers 
               WHERE published >= ? 
               ORDER BY published DESC""",
            (since.isoformat(),)
        )
    else:
        cursor.execute(
            """SELECT id, title, abstract_text, authors, categories, published 
               FROM papers 
               ORDER BY published DESC
               LIMIT 10000"""
        )
    
    return [paper_from_row(row) for row in cursor.fetchall()]

def sync_paper_to_postgres(conn: psycopg2.extensions.connection, paper: dict) -> bool:
    """Insert or update a paper in PostgreSQL."""
    cursor = conn.cursor()
    try:
        cursor.execute("""
            INSERT INTO papers (id, title, abstract, authors, categories, published, created_at)
            VALUES (%(id)s, %(title)s, %(abstract)s, %(authors)s, %(categories)s, %(published)s, NOW())
            ON CONFLICT (id) DO UPDATE SET
                title = EXCLUDED.title,
                abstract = EXCLUDED.abstract,
                authors = EXCLUDED.authors,
                categories = EXCLUDED.categories
        """, paper)
        return True
    except Exception as e:
        logger.error(f"Failed to sync paper {paper['id']}: {e}")
        return False

def get_paper_count(conn: psycopg2.extensions.connection) -> int:
    """Get count of papers in PostgreSQL."""
    cursor = conn.cursor()
    cursor.execute("SELECT COUNT(*) FROM papers")
    return cursor.fetchone()[0]

def main():
    parser = argparse.ArgumentParser(description='Sync papers from SQLite to PostgreSQL')
    parser.add_argument('--full', action='store_true', help='Perform full sync')
    args = parser.parse_args()
    
    # Check if SQLite DB exists
    if not os.path.exists(SQLITE_DB_PATH):
        logger.warning(f"SQLite DB not found at {SQLITE_DB_PATH}. Run 'cargo run' first to create it.")
        sys.exit(1)
    
    logger.info(f"Starting paper sync...")
    logger.info(f"SQLite DB: {SQLITE_DB_PATH}")
    logger.info(f"PostgreSQL: {POSTGRES_URL[:50]}...")
    
    # Get last sync time
    last_sync = None if args.full else get_last_sync_time()
    if last_sync:
        logger.info(f"Incremental sync since: {last_sync}")
    else:
        logger.info("Full sync mode")
    
    # Connect to databases
    sqlite_conn = connect_sqlite(SQLITE_DB_PATH)
    pg_conn = connect_postgres(POSTGRES_URL)
    
    try:
        # Fetch papers from SQLite
        papers = fetch_papers_from_sqlite(sqlite_conn, since=last_sync)
        logger.info(f"Found {len(papers)} papers in SQLite")
        
        if not papers:
            logger.info("No new papers to sync")
            return
        
        # Sync each paper
        synced = 0
        failed = 0
        for paper in papers:
            if sync_paper_to_postgres(pg_conn, paper):
                synced += 1
            else:
                failed += 1
        
        # Commit and update last sync time
        pg_conn.commit()
        set_last_sync_time(datetime.now())
        
        # Report
        before_count = get_paper_count(pg_conn) - synced
        after_count = get_paper_count(pg_conn)
        
        logger.info(f"Sync complete: {synced} synced, {failed} failed")
        logger.info(f"PostgreSQL papers: {before_count} -> {after_count}")
        
    finally:
        sqlite_conn.close()
        pg_conn.close()

if __name__ == '__main__':
    main()
