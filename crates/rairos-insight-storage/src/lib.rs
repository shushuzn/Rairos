//! rairos-insight-storage — Capsule storage mixin for EvolutionTracker — gene_pool.db (SQLite).
#![allow(dead_code)]
#![allow(clippy::too_many_arguments)]
//!
//! Ported from `llm/insight/storage.py`.

use rairos_insight_credibility::CapsuleGene;
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions, SqliteRow};
use sqlx::{Pool, Row, Sqlite};
use std::collections::HashMap;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::Duration;
use thiserror::Error;

const GENEPOOL_DB: &str = "gene_pool.db";

#[derive(Error, Debug)]
pub enum StorageError {
    #[error("SQLite error: {0}")]
    Sqlite(#[from] sqlx::Error),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Serialization error: {0}")]
    Serde(#[from] serde_json::Error),
    #[error("Capsule not found: {0}")]
    NotFound(String),
}

pub struct CapsuleStorage {
    db_path: PathBuf,
    pool: Pool<Sqlite>,
}

impl CapsuleStorage {
    pub async fn new(data_dir: &Path) -> Result<Self, StorageError> {
        let db_path = data_dir.join(GENEPOOL_DB);
        let options = SqliteConnectOptions::new()
            .filename(&db_path)
            .pragma("journal_mode", "WAL")
            .pragma("synchronous", "NORMAL")
            .create_if_missing(true);
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .acquire_timeout(Duration::from_secs(30))
            .connect_with(options)
            .await?;

        sqlx::query(
            "CREATE TABLE IF NOT EXISTS capsules (
                 capsule_id TEXT PRIMARY KEY,
                 created_at TEXT NOT NULL DEFAULT '',
                 trigger_topic TEXT NOT NULL DEFAULT '',
                 trigger_gap_type TEXT NOT NULL DEFAULT '',
                 trigger_keywords TEXT NOT NULL DEFAULT '[]',
                 action_gap_type TEXT NOT NULL DEFAULT '',
                 action_gap_title TEXT NOT NULL DEFAULT '',
                 outcome_success_score REAL NOT NULL DEFAULT 0.5,
                 feedback_count INTEGER NOT NULL DEFAULT 0,
                 evolved_generation INTEGER NOT NULL DEFAULT 0,
                 archetype TEXT NOT NULL DEFAULT '{}',
                 status TEXT NOT NULL DEFAULT 'active',
                 low_score_streak INTEGER NOT NULL DEFAULT 0,
                 credibility_score REAL NOT NULL DEFAULT 0.5,
                 trendslop INTEGER NOT NULL DEFAULT 0,
                 trendslop_reason TEXT NOT NULL DEFAULT '',
                 credibility_badge TEXT NOT NULL DEFAULT 'medium',
                 source_arxiv_category TEXT NOT NULL DEFAULT '',
                 title_embedding BLOB
             )",
        )
        .execute(&pool)
        .await?;

        sqlx::query("CREATE INDEX IF NOT EXISTS idx_capsules_status ON capsules(status)")
            .execute(&pool)
            .await?;
        sqlx::query(
            "CREATE INDEX IF NOT EXISTS idx_capsules_gap_type ON capsules(trigger_gap_type)",
        )
        .execute(&pool)
        .await?;
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_capsules_topic ON capsules(trigger_topic)")
            .execute(&pool)
            .await?;
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_capsules_created ON capsules(created_at)")
            .execute(&pool)
            .await?;

        Ok(Self { db_path, pool })
    }

    fn gene_pool_file(&self, data_dir: &Path) -> PathBuf {
        data_dir.join("gene_pool.jsonl")
    }

    pub async fn encode_capsule(
        &self,
        topic: &str,
        gap_type: &str,
        gap_title: &str,
        _gap_description: &str,
        success_score: f64,
        status: &str,
        source_paper_id: &str,
        source_arxiv_category: &str,
        archetype: Option<HashMap<String, serde_json::Value>>,
        capsule_id: Option<&str>,
        data_dir: &Path,
    ) -> Result<CapsuleGene, StorageError> {
        let mut arch = archetype.unwrap_or_default();
        if !source_paper_id.is_empty() {
            arch.insert(
                "source_paper_id".to_string(),
                serde_json::json!(source_paper_id),
            );
        }
        if !source_arxiv_category.is_empty() {
            arch.insert(
                "source_arxiv_category".to_string(),
                serde_json::json!(source_arxiv_category),
            );
        }
        arch.insert("gap_type".to_string(), serde_json::json!(gap_type));

        let normalized_gap_type = normalize_gap_type(gap_type);
        arch.insert(
            "gap_type".to_string(),
            serde_json::json!(normalized_gap_type),
        );

        let cid = capsule_id
            .map(|s| s.to_string())
            .unwrap_or_else(|| uuid::Uuid::new_v4().to_string()[..12].to_string());
        let created_at = chrono::Utc::now().to_rfc3339();
        let trigger_keywords = extract_keywords_simple(gap_title);

        let capsule = CapsuleGene {
            capsule_id: cid.clone(),
            created_at,
            trigger_topic: topic.to_string(),
            trigger_gap_type: normalized_gap_type.clone(),
            trigger_keywords: trigger_keywords.clone(),
            action_gap_type: normalized_gap_type.clone(),
            action_gap_title: gap_title.to_string(),
            outcome_success_score: success_score,
            feedback_count: 1,
            evolved_generation: 0,
            archetype: arch,
            status: status.to_string(),
            low_score_streak: 0,
            credibility_score: 0.5,
            trendslop: false,
            trendslop_reason: String::new(),
            credibility_badge: "medium".to_string(),
            source_arxiv_category: source_arxiv_category.to_string(),
        };

        if capsule.action_gap_title.to_lowercase().starts_with("test ") {
            return Ok(capsule);
        }

        self.insert_capsule(&capsule).await?;

        let jsonl_path = self.gene_pool_file(data_dir);
        if jsonl_path.exists() {
            let capsule_dict = capsule_to_dict(&capsule);
            let json_line = serde_json::to_string(&capsule_dict)?;
            std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(&jsonl_path)?
                .write_all(json_line.as_bytes())?;
        }

        Ok(capsule)
    }

    pub async fn find_capsule(
        &self,
        topic: &str,
        gap_type: &str,
        keywords: &[String],
        min_score: f64,
    ) -> Result<Vec<CapsuleGene>, StorageError> {
        let capsules = self.load_capsules().await?;
        let mut scored: Vec<(CapsuleGene, f64)> = Vec::new();

        for capsule in capsules {
            if capsule.status == "archived" {
                continue;
            }
            let match_score = capsule.trigger_match(topic, gap_type, keywords);
            if match_score >= min_score {
                scored.push((capsule, match_score));
            }
        }

        scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
        Ok(scored.into_iter().map(|(c, _)| c).collect())
    }

    pub async fn find_capsule_hybrid(
        &self,
        topic: &str,
        gap_type: &str,
        keywords: &[String],
        _min_lex_score: f64,
        min_total_score: f64,
        semantic_weight: f64,
        top_k: usize,
    ) -> Result<Vec<CapsuleGene>, StorageError> {
        let capsules = self.load_capsules().await?;
        let mut scored: Vec<(CapsuleGene, f64)> = Vec::new();

        for capsule in capsules {
            if capsule.status == "archived" {
                continue;
            }

            let lex_score = capsule.trigger_match(topic, gap_type, keywords);

            let sem_score = 0.0;

            let total = (1.0 - semantic_weight) * lex_score + semantic_weight * sem_score;
            if total >= min_total_score {
                scored.push((capsule, total));
            }
        }

        scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
        Ok(scored.into_iter().take(top_k).map(|(c, _)| c).collect())
    }

    pub async fn archive_capsule(&self, capsule_id: &str) -> Result<bool, StorageError> {
        let row = sqlx::query(
            "SELECT action_gap_title, action_gap_type FROM capsules WHERE capsule_id = ?",
        )
        .bind(capsule_id)
        .fetch_optional(&self.pool)
        .await?;

        if row.is_none() {
            return Ok(false);
        }

        sqlx::query("UPDATE capsules SET status = 'archived' WHERE capsule_id = ?")
            .bind(capsule_id)
            .execute(&self.pool)
            .await?;
        Ok(true)
    }

    pub async fn get_capsule_by_id(
        &self,
        capsule_id: &str,
    ) -> Result<Option<CapsuleGene>, StorageError> {
        let row = sqlx::query("SELECT * FROM capsules WHERE capsule_id = ?")
            .bind(capsule_id)
            .fetch_optional(&self.pool)
            .await?;

        match row {
            Some(r) => Ok(Some(capsule_from_row(&r)?)),
            None => Ok(None),
        }
    }

    pub async fn get_capsule_by_title(
        &self,
        gap_title: &str,
        topic: &str,
    ) -> Result<Option<CapsuleGene>, StorageError> {
        let row = if topic.is_empty() {
            sqlx::query(
                "SELECT * FROM capsules WHERE LOWER(action_gap_title) = LOWER(?) AND status = 'active'",
            )
            .bind(gap_title)
            .fetch_optional(&self.pool)
            .await?
        } else {
            sqlx::query(
                "SELECT * FROM capsules WHERE LOWER(action_gap_title) = LOWER(?) AND LOWER(trigger_topic) = LOWER(?) AND status = 'active'",
            )
            .bind(gap_title)
            .bind(topic)
            .fetch_optional(&self.pool)
            .await?
        };

        match row {
            Some(r) => Ok(Some(capsule_from_row(&r)?)),
            None => Ok(None),
        }
    }

    pub async fn update_capsule(&self, capsule: &CapsuleGene) -> Result<(), StorageError> {
        self.insert_capsule(capsule).await
    }

    pub async fn recompute_credibility_all(
        &self,
    ) -> Result<HashMap<String, serde_json::Value>, StorageError> {
        use rairos_insight_credibility::CredibilityScorer;

        let capsules = self.load_capsules().await?;
        let scorer = CredibilityScorer::new(None);
        let scores = scorer.compute_novelty_scores(&capsules);

        let mut updated = 0;
        let mut errors = 0;

        for capsule in &capsules {
            if let Some(score) = scores.get(&capsule.capsule_id) {
                let mut updated_capsule = capsule.clone();
                updated_capsule.credibility_score = score.overall;
                updated_capsule.trendslop = score.trendslop;
                updated_capsule.trendslop_reason = score.trendslop_reason.clone();
                updated_capsule.credibility_badge = score.badge.clone();
                if self.update_capsule(&updated_capsule).await.is_ok() {
                    updated += 1;
                } else {
                    errors += 1;
                }
            } else {
                errors += 1;
            }
        }

        let mut result = HashMap::new();
        result.insert("updated".to_string(), serde_json::json!(updated));
        result.insert("errors".to_string(), serde_json::json!(errors));
        Ok(result)
    }

    pub async fn get_gene_pool_stats(
        &self,
    ) -> Result<HashMap<String, serde_json::Value>, StorageError> {
        let total: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM capsules")
            .fetch_one(&self.pool)
            .await?;

        if total == 0 {
            let mut stats = HashMap::new();
            stats.insert("total".to_string(), serde_json::json!(0));
            stats.insert("avg_score".to_string(), serde_json::json!(0.0));
            stats.insert("by_gap_type".to_string(), serde_json::json!({}));
            return Ok(stats);
        }

        let avg: f64 = sqlx::query_scalar("SELECT AVG(outcome_success_score) FROM capsules")
            .fetch_one(&self.pool)
            .await?;

        let rows = sqlx::query(
            "SELECT action_gap_type, COUNT(*) as cnt FROM capsules GROUP BY action_gap_type",
        )
        .fetch_all(&self.pool)
        .await?;

        let mut by_type: HashMap<String, i64> = HashMap::new();
        for row in rows {
            let gt: String = row.get("action_gap_type");
            let cnt: i64 = row.get("cnt");
            by_type.insert(gt, cnt);
        }

        let gen_rows = sqlx::query(
            "SELECT DISTINCT evolved_generation FROM capsules ORDER BY evolved_generation",
        )
        .fetch_all(&self.pool)
        .await?;

        let generations: Vec<i32> = gen_rows
            .iter()
            .map(|row| row.get("evolved_generation"))
            .collect();

        let mut stats = HashMap::new();
        stats.insert("total".to_string(), serde_json::json!(total));
        stats.insert(
            "avg_score".to_string(),
            serde_json::json!((avg * 1000.0).round() / 1000.0),
        );
        stats.insert("by_gap_type".to_string(), serde_json::json!(by_type));
        stats.insert("generations".to_string(), serde_json::json!(generations));
        Ok(stats)
    }

    async fn insert_capsule(&self, capsule: &CapsuleGene) -> Result<(), StorageError> {
        sqlx::query(
            "INSERT OR REPLACE INTO capsules (
                capsule_id, created_at, trigger_topic, trigger_gap_type,
                trigger_keywords, action_gap_type, action_gap_title,
                outcome_success_score, feedback_count, evolved_generation,
                archetype, status, low_score_streak,
                credibility_score, trendslop, trendslop_reason,
                credibility_badge, source_arxiv_category, title_embedding
            ) VALUES (
                ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19
            )",
        )
        .bind(&capsule.capsule_id)
        .bind(&capsule.created_at)
        .bind(&capsule.trigger_topic)
        .bind(&capsule.trigger_gap_type)
        .bind(serde_json::to_string(&capsule.trigger_keywords)?)
        .bind(&capsule.action_gap_type)
        .bind(&capsule.action_gap_title)
        .bind(capsule.outcome_success_score)
        .bind(capsule.feedback_count)
        .bind(capsule.evolved_generation)
        .bind(serde_json::to_string(&capsule.archetype)?)
        .bind(&capsule.status)
        .bind(capsule.low_score_streak)
        .bind(capsule.credibility_score)
        .bind(capsule.trendslop as i32)
        .bind(&capsule.trendslop_reason)
        .bind(&capsule.credibility_badge)
        .bind(&capsule.source_arxiv_category)
        .bind(None::<Vec<u8>>)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn load_capsules(&self) -> Result<Vec<CapsuleGene>, StorageError> {
        let rows = sqlx::query("SELECT * FROM capsules")
            .fetch_all(&self.pool)
            .await?;
        let capsules: Vec<CapsuleGene> = rows
            .iter()
            .filter_map(|r| capsule_from_row(r).ok())
            .collect();
        Ok(capsules)
    }

    /// Load all capsules (public wrapper).
    pub async fn load_all_capsules(&self) -> Result<Vec<CapsuleGene>, StorageError> {
        self.load_capsules().await
    }

    /// Save (insert or replace) a batch of capsules.
    /// Used after evolution to persist the evolved gene pool.
    pub async fn save_capsules(&self, capsules: &[CapsuleGene]) -> Result<(), StorageError> {
        for c in capsules {
            self.insert_capsule(c).await?;
        }
        Ok(())
    }

    pub fn close(&self) {}
}

fn capsule_from_row(row: &SqliteRow) -> Result<CapsuleGene, sqlx::Error> {
    let archetype_str: String = row.try_get("archetype")?;
    let archetype: HashMap<String, serde_json::Value> =
        serde_json::from_str(&archetype_str).unwrap_or_default();

    let trigger_kw_str: String = row.try_get("trigger_keywords")?;
    let trigger_keywords: Vec<String> = serde_json::from_str(&trigger_kw_str).unwrap_or_default();

    Ok(CapsuleGene {
        capsule_id: row.try_get("capsule_id")?,
        created_at: row.try_get("created_at")?,
        trigger_topic: row.try_get("trigger_topic")?,
        trigger_gap_type: row.try_get("trigger_gap_type")?,
        trigger_keywords,
        action_gap_type: row.try_get("action_gap_type")?,
        action_gap_title: row.try_get("action_gap_title")?,
        outcome_success_score: row.try_get("outcome_success_score")?,
        feedback_count: row.try_get("feedback_count")?,
        evolved_generation: row.try_get("evolved_generation")?,
        archetype,
        status: row.try_get("status")?,
        low_score_streak: row.try_get("low_score_streak")?,
        credibility_score: row.try_get("credibility_score")?,
        trendslop: row.try_get::<i32, _>("trendslop")? != 0,
        trendslop_reason: row.try_get("trendslop_reason")?,
        credibility_badge: row.try_get("credibility_badge")?,
        source_arxiv_category: row.try_get("source_arxiv_category")?,
    })
}

fn capsule_to_dict(capsule: &CapsuleGene) -> HashMap<String, serde_json::Value> {
    let mut map = HashMap::new();
    map.insert(
        "capsule_id".to_string(),
        serde_json::json!(capsule.capsule_id),
    );
    map.insert(
        "created_at".to_string(),
        serde_json::json!(capsule.created_at),
    );
    map.insert(
        "trigger_topic".to_string(),
        serde_json::json!(capsule.trigger_topic),
    );
    map.insert(
        "trigger_gap_type".to_string(),
        serde_json::json!(capsule.trigger_gap_type),
    );
    map.insert(
        "trigger_keywords".to_string(),
        serde_json::json!(capsule.trigger_keywords),
    );
    map.insert(
        "action_gap_type".to_string(),
        serde_json::json!(capsule.action_gap_type),
    );
    map.insert(
        "action_gap_title".to_string(),
        serde_json::json!(capsule.action_gap_title),
    );
    map.insert(
        "outcome_success_score".to_string(),
        serde_json::json!(capsule.outcome_success_score),
    );
    map.insert(
        "feedback_count".to_string(),
        serde_json::json!(capsule.feedback_count),
    );
    map.insert(
        "evolved_generation".to_string(),
        serde_json::json!(capsule.evolved_generation),
    );
    map.insert(
        "archetype".to_string(),
        serde_json::json!(capsule.archetype),
    );
    map.insert("status".to_string(), serde_json::json!(capsule.status));
    map.insert(
        "low_score_streak".to_string(),
        serde_json::json!(capsule.low_score_streak),
    );
    map.insert(
        "credibility_score".to_string(),
        serde_json::json!(capsule.credibility_score),
    );
    map.insert(
        "trendslop".to_string(),
        serde_json::json!(capsule.trendslop),
    );
    map.insert(
        "trendslop_reason".to_string(),
        serde_json::json!(capsule.trendslop_reason),
    );
    map.insert(
        "credibility_badge".to_string(),
        serde_json::json!(capsule.credibility_badge),
    );
    map.insert(
        "source_arxiv_category".to_string(),
        serde_json::json!(capsule.source_arxiv_category),
    );
    map
}

const VALID_GAP_TYPES: &[&str] = &[
    "unexplored_application",
    "method_limitation",
    "contradiction",
    "evaluation_gap",
    "scalability_issue",
    "theoretical_gap",
    "dataset_gap",
    "generalization_gap",
    "method_gap",
    "exploration_gap",
    "implementation",
    "theory_gap",
];

const GAP_TYPE_FALLBACK: &[(&str, &str)] = &[
    ("capability", "method_limitation"),
    ("application_gap", "unexplored_application"),
    ("theory_gap", "theoretical_gap"),
    ("method_gap", "method_limitation"),
    ("exploration_gap", "unexplored_application"),
    ("general_gap", "method_limitation"),
];

fn normalize_gap_type(gap_type: &str) -> String {
    if gap_type.is_empty() {
        return "method_limitation".to_string();
    }
    let normalized = gap_type.trim().to_lowercase();
    if VALID_GAP_TYPES.contains(&normalized.as_str()) {
        return normalized;
    }
    for (legacy, standard) in GAP_TYPE_FALLBACK {
        if normalized == *legacy {
            return standard.to_string();
        }
    }
    "method_limitation".to_string()
}

fn extract_keywords_simple(text: &str) -> Vec<String> {
    let text_lower = text.to_lowercase();
    let words: Vec<&str> = text_lower.split_whitespace().collect();
    let stopwords = [
        "a", "an", "the", "and", "or", "but", "in", "on", "at", "to", "for", "of", "with", "by",
        "from", "is", "are", "was", "were", "be", "been", "being", "have", "has", "had", "do",
        "does", "did", "will", "would", "could", "should", "may", "might", "must", "shall", "can",
        "need", "that", "which", "who", "whom", "this", "these", "those", "it", "its", "over",
    ];
    words
        .into_iter()
        .filter(|w| w.len() > 2 && !stopwords.contains(w))
        .map(|w| w.to_string())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn create_test_storage() -> (CapsuleStorage, tempfile::TempDir) {
        let temp_dir = tempfile::tempdir().unwrap();
        let storage = CapsuleStorage::new(temp_dir.path()).await.unwrap();
        (storage, temp_dir)
    }

    #[tokio::test]
    async fn test_new_storage() {
        let (storage, _temp_dir) = create_test_storage().await;
        assert!(storage.get_gene_pool_stats().await.is_ok());
    }

    #[tokio::test]
    async fn test_encode_capsule() {
        let (storage, temp_dir) = create_test_storage().await;
        let capsule = storage
            .encode_capsule(
                "NLP",
                "method_limitation",
                "Attention mechanism improvements",
                "Better attention for transformers",
                0.8,
                "active",
                "",
                "cs.CL",
                None,
                None,
                temp_dir.path(),
            )
            .await
            .unwrap();
        assert_eq!(capsule.trigger_topic, "NLP");
        assert_eq!(capsule.action_gap_type, "method_limitation");
        assert!(!capsule.capsule_id.is_empty());
    }

    #[tokio::test]
    async fn test_find_capsule() {
        let (storage, temp_dir) = create_test_storage().await;
        storage
            .encode_capsule(
                "NLP",
                "method_limitation",
                "Attention improvements",
                "Better attention",
                0.8,
                "active",
                "",
                "cs.CL",
                None,
                None,
                temp_dir.path(),
            )
            .await
            .unwrap();

        let found = storage
            .find_capsule("NLP", "method_limitation", &[], 0.0)
            .await
            .unwrap();
        assert!(!found.is_empty());
    }

    #[tokio::test]
    async fn test_archive_capsule() {
        let (storage, temp_dir) = create_test_storage().await;
        let capsule = storage
            .encode_capsule(
                "NLP",
                "method_limitation",
                "Capsule for archival testing",
                "Test",
                0.5,
                "active",
                "",
                "cs.CL",
                None,
                None,
                temp_dir.path(),
            )
            .await
            .unwrap();

        let result = storage.archive_capsule(&capsule.capsule_id).await.unwrap();
        assert!(result);
    }

    #[tokio::test]
    async fn test_get_capsule_by_id() {
        let (storage, temp_dir) = create_test_storage().await;
        let capsule = storage
            .encode_capsule(
                "NLP",
                "method_limitation",
                "Capsule for ID lookup test",
                "Test",
                0.5,
                "active",
                "",
                "cs.CL",
                None,
                None,
                temp_dir.path(),
            )
            .await
            .unwrap();

        let found = storage
            .get_capsule_by_id(&capsule.capsule_id)
            .await
            .unwrap();
        assert!(found.is_some());
    }

    #[tokio::test]
    async fn test_get_capsule_by_title() {
        let (storage, temp_dir) = create_test_storage().await;
        storage
            .encode_capsule(
                "NLP",
                "method_limitation",
                "Unique test title 123",
                "Test",
                0.5,
                "active",
                "",
                "cs.CL",
                None,
                None,
                temp_dir.path(),
            )
            .await
            .unwrap();

        let found = storage
            .get_capsule_by_title("Unique test title 123", "NLP")
            .await
            .unwrap();
        assert!(found.is_some());
    }

    #[test]
    fn test_normalize_gap_type() {
        assert_eq!(normalize_gap_type("method_limitation"), "method_limitation");
        assert_eq!(normalize_gap_type("capability"), "method_limitation");
        assert_eq!(
            normalize_gap_type("application_gap"),
            "unexplored_application"
        );
        assert_eq!(normalize_gap_type(""), "method_limitation");
    }

    #[test]
    fn test_extract_keywords_simple() {
        let keywords = extract_keywords_simple("The quick brown fox jumps over the lazy dog");
        assert!(keywords.contains(&"quick".to_string()));
        assert!(keywords.contains(&"brown".to_string()));
        assert!(keywords.contains(&"fox".to_string()));
        assert!(!keywords.contains(&"the".to_string()));
        assert!(!keywords.contains(&"over".to_string()));
    }

    #[tokio::test]
    async fn test_gene_pool_stats() {
        let (storage, temp_dir) = create_test_storage().await;
        storage
            .encode_capsule(
                "NLP",
                "method_limitation",
                "Capsule for stats testing 1",
                "Test",
                0.8,
                "active",
                "",
                "cs.CL",
                None,
                None,
                temp_dir.path(),
            )
            .await
            .unwrap();
        storage
            .encode_capsule(
                "Vision",
                "unexplored_application",
                "Capsule for stats testing 2",
                "Test",
                0.6,
                "active",
                "",
                "cs.CV",
                None,
                None,
                temp_dir.path(),
            )
            .await
            .unwrap();

        let stats = storage.get_gene_pool_stats().await.unwrap();
        assert_eq!(stats["total"], serde_json::json!(2));
    }
}
