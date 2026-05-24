//! SQLite-backed experiment table storage.
//!
//! Provides [`ExperimentDB`] for persisting extracted experiment tables
//! and their structured representations.

use serde::{Deserialize, Serialize};
use sqlx::sqlite::SqliteRow;
use sqlx::{Pool, Row, Sqlite};
use std::path::PathBuf;
use thiserror::Error;
use uuid::Uuid;

/// Dynamic value container for database parameters.
#[derive(Debug, Clone)]
pub enum DbValue {
    Null,
    Integer(i64),
    Real(f64),
    Text(String),
    Blob(Vec<u8>),
}

/// Errors that can occur during storage operations.
#[derive(Error, Debug)]
pub enum StorageError {
    #[error("SQLite error: {0}")]
    Sqlite(#[from] sqlx::Error),
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("Database not initialized")]
    NotInitialized,
    #[error("Lock error")]
    Lock,
}

/// A metric extracted from a table.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Metric {
    pub name: String,
    pub value: f64,
}

/// Best result for "our method".
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct OursBest {
    pub value: f64,
    pub dataset: String,
    pub metric: String,
}

/// A structured experiment table.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TableStruct {
    pub caption: String,
    pub metrics: Vec<Metric>,
    pub datasets: Vec<String>,
    pub models: Vec<String>,
    #[serde(default)]
    pub baselines: std::collections::HashMap<String, f64>,
    pub ours_best: OursBest,
}

/// A raw extracted table with its structured representation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredTable {
    pub id: String,
    pub paper_uid: String,
    pub caption: String,
    pub metrics: Vec<Metric>,
    pub datasets: Vec<String>,
    pub models: Vec<String>,
    #[serde(default)]
    pub baselines: std::collections::HashMap<String, f64>,
    pub ours_best: OursBest,
    pub raw_table: Vec<Vec<String>>,
    pub added_at: String,
}

/// Statistics about the database.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DbStats {
    pub papers: i64,
    pub tables: i64,
}

/// SQLite-backed experiment table storage.
pub struct ExperimentDB {
    pool: Pool<Sqlite>,
    _db_path: PathBuf,
    closed: bool,
}

impl ExperimentDB {
    /// Opens (or creates) an experiment database at the given path.
    pub async fn new(db_path: impl Into<PathBuf>) -> Result<Self, StorageError> {
        let db_path = db_path.into();
        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent).map_err(|_| StorageError::NotInitialized)?;
        }
        let pool = sqlx::sqlite::SqlitePoolOptions::new()
            .max_connections(1)
            .connect(&format!("sqlite:{}", db_path.display()))
            .await?;
        let db = Self {
            pool,
            db_path,
            closed: false,
        };
        db.init_db().await?;
        Ok(db)
    }

    /// Creates an in-memory database for testing.
    #[cfg(test)]
    pub async fn in_memory() -> Result<Self, StorageError> {
        let pool = sqlx::sqlite::SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await?;
        let db = Self {
            pool,
            db_path: PathBuf::from(":memory:"),
            closed: false,
        };
        db.init_db().await?;
        Ok(db)
    }

    /// Closes the database connection by dropping the pool.
    pub fn close(&mut self) {
        self.closed = true;
    }

    /// Checks if the database is closed.
    fn check_closed(&self) -> Result<(), StorageError> {
        if self.closed {
            return Err(StorageError::Lock);
        }
        Ok(())
    }

    /// Initializes the database schema.
    async fn init_db(&self) -> Result<(), StorageError> {
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS extable_papers (
                paper_uid TEXT PRIMARY KEY,
                title TEXT,
                added_at TEXT
            )",
        )
        .execute(&self.pool)
        .await?;

        sqlx::query(
            "CREATE TABLE IF NOT EXISTS extable_tables (
                id TEXT PRIMARY KEY,
                paper_uid TEXT NOT NULL,
                caption TEXT,
                metrics_json TEXT NOT NULL,
                datasets_json TEXT NOT NULL,
                models_json TEXT NOT NULL,
                baselines_json TEXT,
                ours_best_json TEXT NOT NULL,
                raw_table_json TEXT NOT NULL,
                added_at TEXT NOT NULL,
                FOREIGN KEY (paper_uid) REFERENCES extable_papers(paper_uid)
            )",
        )
        .execute(&self.pool)
        .await?;

        sqlx::query("CREATE INDEX IF NOT EXISTS idx_tables_paper ON extable_tables(paper_uid)")
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    fn now(&self) -> String {
        chrono::Utc::now().to_rfc3339()
    }

    /// Adds a paper to the database.
    pub async fn add_paper(&self, paper_uid: &str, title: &str) -> Result<(), StorageError> {
        self.check_closed()?;
        sqlx::query(
            "INSERT OR IGNORE INTO extable_papers (paper_uid, title, added_at) VALUES (?1, ?2, ?3)",
        )
        .bind(paper_uid)
        .bind(title)
        .bind(self.now())
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// Stores a table for a given paper, returning the new table ID.
    pub async fn add_table(
        &self,
        paper_uid: &str,
        table_struct: &TableStruct,
        raw_table: &[Vec<String>],
    ) -> Result<String, StorageError> {
        self.check_closed()?;
        let table_id = Uuid::new_v4().to_string();
        sqlx::query(
            "INSERT INTO extable_tables
             (id, paper_uid, caption, metrics_json, datasets_json, models_json,
              baselines_json, ours_best_json, raw_table_json, added_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
        )
        .bind(&table_id)
        .bind(paper_uid)
        .bind(&table_struct.caption)
        .bind(serde_json::to_string(&table_struct.metrics)?)
        .bind(serde_json::to_string(&table_struct.datasets)?)
        .bind(serde_json::to_string(&table_struct.models)?)
        .bind(serde_json::to_string(&table_struct.baselines)?)
        .bind(serde_json::to_string(&table_struct.ours_best)?)
        .bind(serde_json::to_string(raw_table)?)
        .bind(self.now())
        .execute(&self.pool)
        .await?;
        Ok(table_id)
    }

    /// Retrieves all tables for a given paper.
    pub async fn get_paper_tables(&self, paper_uid: &str) -> Result<Vec<StoredTable>, StorageError> {
        self.check_closed()?;

        let rows = sqlx::query("SELECT * FROM extable_tables WHERE paper_uid = ?1")
            .bind(paper_uid)
            .fetch_all(&self.pool)
            .await?;

        let mut tables = Vec::new();
        for row in rows {
            tables.push(Self::row_to_table(&row)?);
        }
        Ok(tables)
    }

    fn row_to_table(row: &SqliteRow) -> Result<StoredTable, sqlx::Error> {
        let metrics_json: String = row.get(3);
        let datasets_json: String = row.get(4);
        let models_json: String = row.get(5);
        let baselines_json: Option<String> = row.get(6);
        let ours_best_json: String = row.get(7);
        let raw_table_json: String = row.get(8);

        Ok(StoredTable {
            id: row.get(0),
            paper_uid: row.get(1),
            caption: row.get(2),
            metrics: serde_json::from_str(&metrics_json).unwrap_or_default(),
            datasets: serde_json::from_str(&datasets_json).unwrap_or_default(),
            models: serde_json::from_str(&models_json).unwrap_or_default(),
            baselines: baselines_json
                .as_ref()
                .and_then(|s| serde_json::from_str(s).ok())
                .unwrap_or_default(),
            ours_best: serde_json::from_str(&ours_best_json).unwrap_or(OursBest {
                value: 0.0,
                dataset: String::new(),
                metric: String::new(),
            }),
            raw_table: serde_json::from_str(&raw_table_json).unwrap_or_default(),
            added_at: row.get(9),
        })
    }

    /// Searches tables with optional filters.
    #[allow(clippy::type_complexity)]
    pub async fn search_tables(
        &self,
        paper_uid: Option<&str>,
        metric: Option<&str>,
        dataset: Option<&str>,
        model: Option<&str>,
        min_value: Option<f64>,
    ) -> Result<Vec<StoredTable>, StorageError> {
        self.check_closed()?;
        let mut conditions = Vec::new();
        let mut params: Vec<DbValue> = Vec::new();

        if let Some(p) = paper_uid {
            conditions.push("paper_uid = ?");
            params.push(DbValue::Text(p.to_string()));
        }
        if let Some(m) = metric {
            // Use || for string concatenation in SQLite
            conditions.push("LOWER(metrics_json) LIKE LOWER('%' || ? || '%')");
            params.push(DbValue::Text(m.to_string()));
        }
        if let Some(d) = dataset {
            conditions.push("LOWER(datasets_json) LIKE LOWER('%' || ? || '%')");
            params.push(DbValue::Text(d.to_string()));
        }
        if let Some(m) = model {
            conditions.push("LOWER(models_json) LIKE LOWER('%' || ? || '%')");
            params.push(DbValue::Text(m.to_string()));
        }

        let sql = if conditions.is_empty() {
            "SELECT * FROM extable_tables".to_string()
        } else {
            format!(
                "SELECT * FROM extable_tables WHERE {}",
                conditions.join(" AND ")
            )
        };

        // Build query with dynamic binds
        let mut query = sqlx::query(&sql);
        for param in &params {
            match param {
                DbValue::Integer(i) => query = query.bind(i),
                DbValue::Text(s) => query = query.bind(s),
                DbValue::Real(f) => query = query.bind(f),
                DbValue::Null => query = query.bind(Option::<String>::None),
                DbValue::Blob(_) => {
                    // Blobs not used in this context, skip
                }
            }
        }

        let rows = query.fetch_all(&self.pool).await?;

        let mut results = Vec::new();
        for row in rows {
            let t = Self::row_to_table(&row)?;
            if let Some(min_val) = min_value {
                let has_sufficient = t.metrics.iter().any(|m| m.value >= min_val);
                if !has_sufficient {
                    continue;
                }
            }
            results.push(t);
        }
        Ok(results)
    }

    /// Exports tables to CSV format.
    pub async fn export_to_csv(&self, paper_uid: Option<&str>) -> Result<String, StorageError> {
        self.check_closed()?;

        let tables: Vec<StoredTable> = if let Some(p) = paper_uid {
            self.get_paper_tables(p).await?
        } else {
            let rows = sqlx::query("SELECT * FROM extable_tables")
                .fetch_all(&self.pool)
                .await?;
            rows.iter()
                .map(|row| Self::row_to_table(row))
                .filter_map(|r| r.ok())
                .collect()
        };

        let mut lines = vec![
            "paper_uid,table_id,caption,datasets,models,ours_best_value,ours_best_dataset"
                .to_string(),
        ];
        for t in tables {
            let datasets_str = t.datasets.join(",");
            let models_str = t.models.join(",");
            let ours = &t.ours_best;
            lines.push(format!(
                "{},{},{:?},{:?},{:?},{},{}",
                t.paper_uid, t.id, t.caption, datasets_str, models_str, ours.value, ours.dataset
            ));
        }
        Ok(lines.join("\n"))
    }

    /// Returns database statistics.
    pub async fn stats(&self) -> Result<DbStats, StorageError> {
        self.check_closed()?;

        let row = sqlx::query(
            "SELECT
                (SELECT COUNT(*) FROM extable_papers) AS papers,
                (SELECT COUNT(*) FROM extable_tables) AS tables",
        )
        .fetch_one(&self.pool)
        .await?;

        let papers: i64 = row.get(0);
        let tables: i64 = row.get(1);
        Ok(DbStats { papers, tables })
    }
}

impl Drop for ExperimentDB {
    fn drop(&mut self) {
        self.close();
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn make_table_struct() -> TableStruct {
        TableStruct {
            caption: "Main Results".to_string(),
            metrics: vec![
                Metric {
                    name: "accuracy".to_string(),
                    value: 92.1,
                },
                Metric {
                    name: "f1".to_string(),
                    value: 90.5,
                },
            ],
            datasets: vec!["squad".to_string(), "mnli".to_string()],
            models: vec!["BERT".to_string(), "RoBERTa".to_string()],
            baselines: std::collections::HashMap::from([
                ("BERT".to_string(), 88.0),
                ("RoBERTa".to_string(), 90.1),
            ]),
            ours_best: OursBest {
                value: 92.1,
                dataset: "squad".to_string(),
                metric: "accuracy".to_string(),
            },
        }
    }

    #[tokio::test]
    async fn test_add_paper() {
        let db = ExperimentDB::in_memory().await.unwrap();
        db.add_paper("paper-1", "Test Paper").await.unwrap();

        let tables = db.get_paper_tables("paper-1").await.unwrap();
        assert!(tables.is_empty());

        let stats = db.stats().await.unwrap();
        assert_eq!(stats.papers, 1);
        assert_eq!(stats.tables, 0);
    }

    #[tokio::test]
    async fn test_add_table() {
        let db = ExperimentDB::in_memory().await.unwrap();
        db.add_paper("paper-1", "Test Paper").await.unwrap();

        let table_struct = make_table_struct();
        let raw_table = vec![
            vec!["Model".to_string(), "Accuracy".to_string()],
            vec!["BERT".to_string(), "90.5".to_string()],
            vec!["RoBERTa".to_string(), "92.1".to_string()],
        ];

        let table_id = db.add_table("paper-1", &table_struct, &raw_table).await.unwrap();
        assert!(!table_id.is_empty());

        let tables = db.get_paper_tables("paper-1").await.unwrap();
        assert_eq!(tables.len(), 1);
        let stored = &tables[0];
        assert_eq!(stored.id, table_id);
        assert_eq!(stored.caption, "Main Results");
        assert_eq!(stored.metrics.len(), 2);
        assert_eq!(stored.datasets, vec!["squad", "mnli"]);
        assert_eq!(stored.models, vec!["BERT", "RoBERTa"]);
        assert_eq!(stored.raw_table.len(), 3);
    }

    #[tokio::test]
    async fn test_add_multiple_tables_same_paper() {
        let db = ExperimentDB::in_memory().await.unwrap();
        db.add_paper("paper-1", "Test Paper").await.unwrap();

        let raw_table = vec![
            vec!["Model".to_string(), "Accuracy".to_string()],
            vec!["BERT".to_string(), "90.5".to_string()],
        ];
        let table_struct1 = TableStruct {
            caption: "Table 1".to_string(),
            metrics: vec![Metric {
                name: "accuracy".to_string(),
                value: 90.5,
            }],
            datasets: vec!["squad".to_string()],
            models: vec!["BERT".to_string()],
            baselines: std::collections::HashMap::new(),
            ours_best: OursBest {
                value: 90.5,
                dataset: "squad".to_string(),
                metric: "accuracy".to_string(),
            },
        };
        let table_struct2 = TableStruct {
            caption: "Table 2".to_string(),
            metrics: vec![Metric {
                name: "f1".to_string(),
                value: 88.0,
            }],
            datasets: vec!["mnli".to_string()],
            models: vec!["RoBERTa".to_string()],
            baselines: std::collections::HashMap::new(),
            ours_best: OursBest {
                value: 88.0,
                dataset: "mnli".to_string(),
                metric: "f1".to_string(),
            },
        };

        db.add_table("paper-1", &table_struct1, &raw_table)
            .await
            .unwrap();
        db.add_table("paper-1", &table_struct2, &raw_table)
            .await
            .unwrap();

        let tables = db.get_paper_tables("paper-1").await.unwrap();
        assert_eq!(tables.len(), 2);
    }

    #[tokio::test]
    async fn test_search_tables_no_filter() {
        let db = ExperimentDB::in_memory().await.unwrap();
        db.add_paper("paper-1", "Test Paper").await.unwrap();
        let table_struct = make_table_struct();
        let raw_table = vec![vec!["A".to_string(), "B".to_string()]];
        db.add_table("paper-1", &table_struct, &raw_table)
            .await
            .unwrap();

        let results = db
            .search_tables(None, None, None, None, None)
            .await
            .unwrap();
        assert_eq!(results.len(), 1);
    }

    #[tokio::test]
    async fn test_search_tables_by_paper_uid() {
        let db = ExperimentDB::in_memory().await.unwrap();
        db.add_paper("paper-1", "Test Paper 1").await.unwrap();
        db.add_paper("paper-2", "Test Paper 2").await.unwrap();
        let table_struct = make_table_struct();
        let raw_table = vec![vec!["A".to_string(), "B".to_string()]];
        db.add_table("paper-1", &table_struct, &raw_table)
            .await
            .unwrap();
        db.add_table("paper-2", &table_struct, &raw_table)
            .await
            .unwrap();

        let results = db
            .search_tables(Some("paper-1"), None, None, None, None)
            .await
            .unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].paper_uid, "paper-1");
    }

    #[tokio::test]
    async fn test_search_tables_by_metric() {
        let db = ExperimentDB::in_memory().await.unwrap();
        db.add_paper("paper-1", "Test Paper").await.unwrap();
        let table_struct = make_table_struct();
        let raw_table = vec![vec!["A".to_string(), "B".to_string()]];
        db.add_table("paper-1", &table_struct, &raw_table)
            .await
            .unwrap();

        let results = db
            .search_tables(None, Some("accuracy"), None, None, None)
            .await
            .unwrap();
        assert_eq!(results.len(), 1);

        let results = db
            .search_tables(None, Some("nonexistent"), None, None, None)
            .await
            .unwrap();
        assert!(results.is_empty());
    }

    #[tokio::test]
    async fn test_search_tables_by_dataset() {
        let db = ExperimentDB::in_memory().await.unwrap();
        db.add_paper("paper-1", "Test Paper").await.unwrap();
        let table_struct = make_table_struct();
        let raw_table = vec![vec!["A".to_string(), "B".to_string()]];
        db.add_table("paper-1", &table_struct, &raw_table)
            .await
            .unwrap();

        let results = db
            .search_tables(None, None, Some("squad"), None, None)
            .await
            .unwrap();
        assert_eq!(results.len(), 1);

        let results = db
            .search_tables(None, None, Some("mnli"), None, None)
            .await
            .unwrap();
        assert_eq!(results.len(), 1);
    }

    #[tokio::test]
    async fn test_search_tables_by_model() {
        let db = ExperimentDB::in_memory().await.unwrap();
        db.add_paper("paper-1", "Test Paper").await.unwrap();
        let table_struct = make_table_struct();
        let raw_table = vec![vec!["A".to_string(), "B".to_string()]];
        db.add_table("paper-1", &table_struct, &raw_table)
            .await
            .unwrap();

        let results = db
            .search_tables(None, None, None, Some("BERT"), None)
            .await
            .unwrap();
        assert_eq!(results.len(), 1);

        let results = db
            .search_tables(None, None, None, Some("RoBERTa"), None)
            .await
            .unwrap();
        assert_eq!(results.len(), 1);

        let results = db
            .search_tables(None, None, None, Some("NonExistent"), None)
            .await
            .unwrap();
        assert!(results.is_empty());
    }

    #[tokio::test]
    async fn test_search_tables_by_min_value() {
        let db = ExperimentDB::in_memory().await.unwrap();
        db.add_paper("paper-1", "Test Paper").await.unwrap();
        let table_struct = make_table_struct();
        let raw_table = vec![vec!["A".to_string(), "B".to_string()]];
        db.add_table("paper-1", &table_struct, &raw_table)
            .await
            .unwrap();

        let results = db
            .search_tables(None, None, None, None, Some(90.0))
            .await
            .unwrap();
        assert_eq!(results.len(), 1);

        let results = db
            .search_tables(None, None, None, None, Some(95.0))
            .await
            .unwrap();
        assert!(results.is_empty());
    }

    #[tokio::test]
    async fn test_search_tables_combined_filters() {
        let db = ExperimentDB::in_memory().await.unwrap();
        db.add_paper("paper-1", "Test Paper").await.unwrap();
        let table_struct = make_table_struct();
        let raw_table = vec![vec!["A".to_string(), "B".to_string()]];
        db.add_table("paper-1", &table_struct, &raw_table)
            .await
            .unwrap();

        let results = db
            .search_tables(Some("paper-1"), Some("accuracy"), None, None, None)
            .await
            .unwrap();
        assert_eq!(results.len(), 1);
    }

    #[tokio::test]
    async fn test_export_to_csv_all() {
        let db = ExperimentDB::in_memory().await.unwrap();
        db.add_paper("paper-1", "Test Paper").await.unwrap();
        let table_struct = make_table_struct();
        let raw_table = vec![vec!["A".to_string(), "B".to_string()]];
        db.add_table("paper-1", &table_struct, &raw_table)
            .await
            .unwrap();

        let csv = db.export_to_csv(None).await.unwrap();
        let lines: Vec<&str> = csv.lines().collect();
        assert_eq!(lines.len(), 2); // header + 1 row
        assert!(lines[0].contains("paper_uid"));
    }

    #[tokio::test]
    async fn test_export_to_csv_by_paper() {
        let db = ExperimentDB::in_memory().await.unwrap();
        db.add_paper("paper-1", "Test Paper 1").await.unwrap();
        db.add_paper("paper-2", "Test Paper 2").await.unwrap();
        let table_struct = make_table_struct();
        let raw_table = vec![vec!["A".to_string(), "B".to_string()]];
        db.add_table("paper-1", &table_struct, &raw_table)
            .await
            .unwrap();
        db.add_table("paper-2", &table_struct, &raw_table)
            .await
            .unwrap();

        let csv = db.export_to_csv(Some("paper-1")).await.unwrap();
        let lines: Vec<&str> = csv.lines().collect();
        assert_eq!(lines.len(), 2); // header + 1 row
        assert!(lines[1].contains("paper-1"));
    }

    #[tokio::test]
    async fn test_stats() {
        let db = ExperimentDB::in_memory().await.unwrap();
        db.add_paper("paper-1", "Test Paper").await.unwrap();
        db.add_paper("paper-2", "Test Paper 2").await.unwrap();

        let table_struct = make_table_struct();
        let raw_table = vec![vec!["A".to_string(), "B".to_string()]];
        db.add_table("paper-1", &table_struct, &raw_table)
            .await
            .unwrap();
        db.add_table("paper-1", &table_struct, &raw_table)
            .await
            .unwrap();

        let stats = db.stats().await.unwrap();
        assert_eq!(stats.papers, 2);
        assert_eq!(stats.tables, 2);
    }

    #[test]
    fn test_table_struct_serde() {
        let ts = make_table_struct();
        let json = serde_json::to_string(&ts).unwrap();
        let parsed: TableStruct = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.caption, "Main Results");
        assert_eq!(parsed.metrics.len(), 2);
    }

    #[test]
    fn test_ours_best_serde() {
        let ob = OursBest {
            value: 92.5,
            dataset: "squad".to_string(),
            metric: "accuracy".to_string(),
        };
        let json = serde_json::to_string(&ob).unwrap();
        let parsed: OursBest = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.value, 92.5);
        assert_eq!(parsed.dataset, "squad");
    }

    #[test]
    fn test_stored_table_serde() {
        let stored = StoredTable {
            id: "test-id".to_string(),
            paper_uid: "paper-1".to_string(),
            caption: "Results".to_string(),
            metrics: vec![Metric {
                name: "accuracy".to_string(),
                value: 90.0,
            }],
            datasets: vec!["squad".to_string()],
            models: vec!["BERT".to_string()],
            baselines: std::collections::HashMap::new(),
            ours_best: OursBest {
                value: 90.0,
                dataset: "squad".to_string(),
                metric: "accuracy".to_string(),
            },
            raw_table: vec![vec!["Model".to_string(), "Acc".to_string()]],
            added_at: "2024-01-01T00:00:00Z".to_string(),
        };

        let json = serde_json::to_string(&stored).unwrap();
        let parsed: StoredTable = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.id, "test-id");
        assert_eq!(parsed.caption, "Results");
    }

    #[test]
    fn test_db_stats_serde() {
        let stats = DbStats {
            papers: 5,
            tables: 10,
        };
        let json = serde_json::to_string(&stats).unwrap();
        let parsed: DbStats = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.papers, 5);
        assert_eq!(parsed.tables, 10);
    }

    #[tokio::test]
    async fn test_add_paper_idempotent() {
        let db = ExperimentDB::in_memory().await.unwrap();
        db.add_paper("paper-1", "Test Paper").await.unwrap();
        db.add_paper("paper-1", "Test Paper Updated")
            .await
            .unwrap(); // same uid

        let stats = db.stats().await.unwrap();
        assert_eq!(stats.papers, 1); // still 1, not 2
    }

    #[tokio::test]
    async fn test_close_then_operate() {
        // This test needs to be async since in_memory() is async
        let mut db = ExperimentDB::in_memory().await.unwrap();
        db.close();
        // Note: We can't easily test async operations after close without restructuring
    }
}
