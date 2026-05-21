//! Database optimization utilities.
//!
//! Provides PRAGMA settings, index management, and database statistics
//! for performance monitoring and optimization.

use serde::{Deserialize, Serialize};
use sqlx::{Pool, Sqlite, Row};

/// Optimization index definition.
#[derive(Debug, Clone)]
pub struct OptimizationIndex {
    pub name: &'static str,
    pub sql: &'static str,
}

/// List of optimization indexes for common query patterns.
pub const OPTIMIZATION_INDEXES: &[OptimizationIndex] = &[
    OptimizationIndex {
        name: "idx_papers_published",
        sql: "CREATE INDEX IF NOT EXISTS idx_papers_published ON papers(published)",
    },
    OptimizationIndex {
        name: "idx_papers_primary_category",
        sql: "CREATE INDEX IF NOT EXISTS idx_papers_primary_category ON papers(primary_category)",
    },
    OptimizationIndex {
        name: "idx_papers_doi",
        sql: "CREATE INDEX IF NOT EXISTS idx_papers_doi ON papers(doi) WHERE doi != ''",
    },
    OptimizationIndex {
        name: "idx_paper_tags_tag_id",
        sql: "CREATE INDEX IF NOT EXISTS idx_paper_tags_tag_id ON paper_tags(tag_id)",
    },
    OptimizationIndex {
        name: "idx_parse_history_status",
        sql: "CREATE INDEX IF NOT EXISTS idx_parse_history_status ON parse_history(status)",
    },
    OptimizationIndex {
        name: "idx_parse_history_attempted_at",
        sql: "CREATE INDEX IF NOT EXISTS idx_parse_history_attempted_at ON parse_history(attempted_at)",
    },
    OptimizationIndex {
        name: "idx_job_queue_priority",
        sql: "CREATE INDEX IF NOT EXISTS idx_job_queue_priority ON job_queue(priority, status)",
    },
    OptimizationIndex {
        name: "idx_experiment_tables_page",
        sql: "CREATE INDEX IF NOT EXISTS idx_experiment_tables_page ON experiment_tables(paper_id, page)",
    },
];

/// PRAGMA setting for database optimization.
#[derive(Debug, Clone)]
pub struct PragmaSetting {
    pub name: &'static str,
    pub value: &'static str,
}

/// List of PRAGMA settings for performance optimization.
pub const PRAGMA_SETTINGS: &[PragmaSetting] = &[
    PragmaSetting {
        name: "cache_size",
        value: "-64000",
    }, // 64MB cache
    PragmaSetting {
        name: "temp_store",
        value: "MEMORY",
    },
    PragmaSetting {
        name: "mmap_size",
        value: "268435456",
    }, // 256MB mmap
    PragmaSetting {
        name: "synchronous",
        value: "NORMAL",
    },
    PragmaSetting {
        name: "journal_mode",
        value: "WAL",
    },
    PragmaSetting {
        name: "read_uncommitted",
        value: "1",
    },
];

/// Database statistics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseStats {
    pub papers_count: i64,
    pub parse_history_count: i64,
    pub paper_tags_count: i64,
    pub tags_count: i64,
    pub citations_count: i64,
    pub experiment_tables_count: i64,
    pub index_count: i64,
    pub database_size_mb: f64,
}

/// Apply PRAGMA settings to the database.
pub async fn apply_pragma_settings(pool: &Pool<Sqlite>) -> Result<Vec<String>, sqlx::Error> {
    let mut applied = Vec::new();
    for setting in PRAGMA_SETTINGS {
        let sql = format!("PRAGMA {} = {}", setting.name, setting.value);
        if sqlx::query(&sql).execute(pool).await.is_ok() {
            applied.push(sql);
        }
    }
    Ok(applied)
}

/// Create optimization indexes on the database.
pub async fn create_optimization_indexes(pool: &Pool<Sqlite>) -> Result<Vec<String>, sqlx::Error> {
    let mut applied = Vec::new();
    for idx in OPTIMIZATION_INDEXES {
        if sqlx::query(idx.sql).execute(pool).await.is_ok() {
            applied.push(idx.name.to_string());
        }
    }
    sqlx::query("PRAGMA optimize").execute(pool).await?;
    Ok(applied)
}

/// Apply all database optimizations.
pub async fn apply_database_optimizations(pool: &Pool<Sqlite>) -> Result<Vec<String>, sqlx::Error> {
    let mut applied = Vec::new();

    // Apply PRAGMA settings
    applied.extend(apply_pragma_settings(pool).await?);

    // Create indexes
    applied.extend(create_optimization_indexes(pool).await?);

    // Run ANALYZE
    if sqlx::query("ANALYZE").execute(pool).await.is_ok() {
        applied.push("ANALYZE".to_string());
    }

    Ok(applied)
}

/// Get database statistics for performance monitoring.
pub async fn get_database_stats(pool: &Pool<Sqlite>) -> Result<DatabaseStats, sqlx::Error> {
    let mut stats = DatabaseStats {
        papers_count: 0,
        parse_history_count: 0,
        paper_tags_count: 0,
        tags_count: 0,
        citations_count: 0,
        experiment_tables_count: 0,
        index_count: 0,
        database_size_mb: 0.0,
    };

    // Table counts
    let tables = [
        ("papers", "papers_count" as &str),
        ("parse_history", "parse_history_count"),
        ("paper_tags", "paper_tags_count"),
        ("tags", "tags_count"),
        ("citations", "citations_count"),
        ("experiment_tables", "experiment_tables_count"),
    ];

    for (table, field) in tables {
        let query = format!("SELECT COUNT(*) FROM {table}");
        if let Ok(count) = sqlx::query(&query)
            .fetch_one(pool)
            .await
            .map(|row| row.get::<i64, _>(0))
        {
            match field {
                "papers_count" => stats.papers_count = count,
                "parse_history_count" => stats.parse_history_count = count,
                "paper_tags_count" => stats.paper_tags_count = count,
                "tags_count" => stats.tags_count = count,
                "citations_count" => stats.citations_count = count,
                "experiment_tables_count" => stats.experiment_tables_count = count,
                _ => {}
            }
        }
    }

    // Index count
    if let Ok(count) = sqlx::query("SELECT COUNT(*) FROM sqlite_master WHERE type = 'index'")
        .fetch_one(pool)
        .await
        .map(|row| row.get::<i64, _>(0))
    {
        stats.index_count = count;
    }

    // Database size
    let page_count: i64 = sqlx::query("PRAGMA page_count")
        .fetch_one(pool)
        .await
        .map(|row| row.get::<i64, _>(0))
        .unwrap_or(0);
    let page_size: i64 = sqlx::query("PRAGMA page_size")
        .fetch_one(pool)
        .await
        .map(|row| row.get::<i64, _>(0))
        .unwrap_or(0);
    stats.database_size_mb = (page_count as f64 * page_size as f64) / (1024.0 * 1024.0);

    Ok(stats)
}

/// Vacuum the database to reclaim space.
pub async fn vacuum_database(pool: &Pool<Sqlite>) -> Result<(), sqlx::Error> {
    sqlx::query("VACUUM").execute(pool).await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::sqlite::{SqlitePoolOptions, SqliteConnectOptions};

    #[test]
    fn test_optimization_indexes_defined() {
        assert!(!OPTIMIZATION_INDEXES.is_empty());
        assert_eq!(OPTIMIZATION_INDEXES[0].name, "idx_papers_published");
    }

    #[test]
    fn test_pragma_settings_defined() {
        assert!(!PRAGMA_SETTINGS.is_empty());
        assert_eq!(PRAGMA_SETTINGS[0].name, "cache_size");
    }

    #[tokio::test]
    async fn test_get_database_stats() {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect(":memory:")
            .await
            .unwrap();
        let stats = get_database_stats(&pool).await.unwrap();
        assert_eq!(stats.papers_count, 0);
        assert_eq!(stats.database_size_mb, 0.0);
    }
}
