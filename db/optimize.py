"""Database optimization utilities."""
import logging
from typing import List, Tuple

logger = logging.getLogger(__name__)

OPTIMIZATION_INDEXES = [
    # Papers table indexes for common query patterns
    ("idx_papers_published", "CREATE INDEX IF NOT EXISTS idx_papers_published ON papers(published)"),
    ("idx_papers_primary_category", "CREATE INDEX IF NOT EXISTS idx_papers_primary_category ON papers(primary_category)"),
    ("idx_papers_doi", "CREATE INDEX IF NOT EXISTS idx_papers_doi ON papers(doi) WHERE doi != ''"),
    
    # Paper_tags indexes for tag-based queries
    ("idx_paper_tags_tag_id", "CREATE INDEX IF NOT EXISTS idx_paper_tags_tag_id ON paper_tags(tag_id)"),
    
    # Parse history indexes for analytics
    ("idx_parse_history_status", "CREATE INDEX IF NOT EXISTS idx_parse_history_status ON parse_history(status)"),
    ("idx_parse_history_attempted_at", "CREATE INDEX IF NOT EXISTS idx_parse_history_attempted_at ON parse_history(attempted_at)"),
    
    # Job queue indexes for priority queries
    ("idx_job_queue_priority", "CREATE INDEX IF NOT EXISTS idx_job_queue_priority ON job_queue(priority, status)"),
    
    # Experiment tables indexes
    ("idx_experiment_tables_page", "CREATE INDEX IF NOT EXISTS idx_experiment_tables_page ON experiment_tables(paper_id, page)"),
]

PRAGMA_SETTINGS = [
    ("cache_size", "-64000"),  # 64MB cache
    ("temp_store", "MEMORY"),
    ("mmap_size", "268435456"),  # 256MB mmap
    ("synchronous", "NORMAL"),
    ("journal_mode", "WAL"),
    ("read_uncommitted", "1"),
    ("writable_schema", "1"),
]


def apply_database_optimizations(db: "Database") -> List[str]:
    """
    Apply performance optimizations to the database.
    
    Args:
        db: Database instance
        
    Returns:
        List of applied optimizations
    """
    applied = []
    
    # Apply PRAGMA settings
    for pragma, value in PRAGMA_SETTINGS:
        try:
            db.conn.execute(f"PRAGMA {pragma} = {value}")
            applied.append(f"PRAGMA {pragma} = {value}")
            logger.info(f"Applied PRAGMA optimization: {pragma}")
        except Exception as e:
            logger.warning(f"Failed to apply PRAGMA {pragma}: {e}")
    
    # Create optimization indexes
    for idx_name, create_sql in OPTIMIZATION_INDEXES:
        try:
            db.conn.execute(create_sql)
            db.conn.commit()
            applied.append(f"Index: {idx_name}")
            logger.info(f"Created optimization index: {idx_name}")
        except Exception as e:
            logger.warning(f"Failed to create index {idx_name}: {e}")
    
    # Run ANALYZE to update statistics
    try:
        db.conn.execute("ANALYZE")
        db.conn.commit()
        applied.append("ANALYZE")
        logger.info("Database statistics updated")
    except Exception as e:
        logger.warning(f"Failed to run ANALYZE: {e}")
    
    return applied


def get_database_stats(db: "Database") -> dict:
    """Get database statistics for performance monitoring."""
    stats = {}
    
    try:
        cur = db.conn.cursor()
        
        # Table sizes
        tables = ["papers", "parse_history", "paper_tags", "tags", "citations", "experiment_tables"]
        for table in tables:
            try:
                cur.execute(f"SELECT COUNT(*) FROM {table}")
                stats[f"{table}_count"] = cur.fetchone()[0]
            except:
                stats[f"{table}_count"] = 0
        
        # Index count
        cur.execute("SELECT COUNT(*) FROM sqlite_master WHERE type = 'index'")
        stats["index_count"] = cur.fetchone()[0]
        
        # Database size
        cur.execute("PRAGMA page_count")
        page_count = cur.fetchone()[0]
        cur.execute("PRAGMA page_size")
        page_size = cur.fetchone()[0]
        stats["database_size_mb"] = (page_count * page_size) / (1024 * 1024)
        
    except Exception as e:
        logger.error(f"Failed to get database stats: {e}")
    
    return stats


def vacuum_database(db: "Database") -> bool:
    """
    Vacuum the database to reclaim space and optimize storage.
    
    Args:
        db: Database instance
        
    Returns:
        True if successful, False otherwise
    """
    try:
        logger.info("Starting database VACUUM...")
        db.conn.execute("VACUUM")
        logger.info("Database VACUUM completed successfully")
        return True
    except Exception as e:
        logger.error(f"Failed to VACUUM database: {e}")
        return False
