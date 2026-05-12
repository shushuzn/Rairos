//! rairos-insight-storage — Capsule storage mixin for EvolutionTracker — gene_pool.db (SQLite).
//!
//! Ported from `llm/insight/storage.py`.

use rairos_insight_credibility::CapsuleGene;
use rusqlite::{params, Connection, Result as SqliteResult};
use std::collections::HashMap;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use thiserror::Error;

const GENEPOOL_DB: &str = "gene_pool.db";

#[derive(Error, Debug)]
pub enum StorageError {
    #[error("SQLite error: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Serialization error: {0}")]
    Serde(#[from] serde_json::Error),
    #[error("Capsule not found: {0}")]
    NotFound(String),
}

pub struct CapsuleStorage {
    db_path: PathBuf,
    conn: Mutex<Connection>,
}

impl CapsuleStorage {
    pub fn new(data_dir: &Path) -> Result<Self, StorageError> {
        let db_path = data_dir.join(GENEPOOL_DB);
        let conn = Connection::open(&db_path)?;
        conn.execute_batch(
            "PRAGMA journal_mode=WAL;
             PRAGMA synchronous=NORMAL;
             CREATE TABLE IF NOT EXISTS capsules (
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
             );
             CREATE INDEX IF NOT EXISTS idx_capsules_status ON capsules(status);
             CREATE INDEX IF NOT EXISTS idx_capsules_gap_type ON capsules(trigger_gap_type);
             CREATE INDEX IF NOT EXISTS idx_capsules_topic ON capsules(trigger_topic);
             CREATE INDEX IF NOT EXISTS idx_capsules_created ON capsules(created_at);",
        )?;
        Ok(Self {
            db_path,
            conn: Mutex::new(conn),
        })
    }

    fn gene_pool_file(&self, data_dir: &Path) -> PathBuf {
        data_dir.join("gene_pool.jsonl")
    }

    pub fn encode_capsule(
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

        self.insert_capsule(&capsule)?;

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

    pub fn find_capsule(
        &self,
        topic: &str,
        gap_type: &str,
        keywords: &[String],
        min_score: f64,
    ) -> Result<Vec<CapsuleGene>, StorageError> {
        let capsules = self.load_capsules()?;
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

    pub fn find_capsule_hybrid(
        &self,
        topic: &str,
        gap_type: &str,
        keywords: &[String],
        _min_lex_score: f64,
        min_total_score: f64,
        semantic_weight: f64,
        top_k: usize,
    ) -> Result<Vec<CapsuleGene>, StorageError> {
        let capsules = self.load_capsules()?;
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

    pub fn archive_capsule(&self, capsule_id: &str) -> Result<bool, StorageError> {
        let conn = self.conn.lock().unwrap();
        let rows = conn.query_row(
            "SELECT action_gap_title, action_gap_type FROM capsules WHERE capsule_id = ?",
            params![capsule_id],
            |_row| Ok(()),
        );
        match rows {
            Ok(_) => {}
            Err(rusqlite::Error::QueryReturnedNoRows) => return Ok(false),
            Err(e) => return Err(e.into()),
        }
        conn.execute(
            "UPDATE capsules SET status = 'archived' WHERE capsule_id = ?",
            params![capsule_id],
        )?;
        Ok(true)
    }

    pub fn get_capsule_by_id(&self, capsule_id: &str) -> Result<Option<CapsuleGene>, StorageError> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare("SELECT * FROM capsules WHERE capsule_id = ?")?;
        let result = stmt.query_row(params![capsule_id], capsule_from_row);
        match result {
            Ok(c) => Ok(Some(c)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    pub fn get_capsule_by_title(
        &self,
        gap_title: &str,
        topic: &str,
    ) -> Result<Option<CapsuleGene>, StorageError> {
        let conn = self.conn.lock().unwrap();
        let query = if topic.is_empty() {
            "SELECT * FROM capsules WHERE LOWER(action_gap_title) = LOWER(?) AND status = 'active'"
        } else {
            "SELECT * FROM capsules WHERE LOWER(action_gap_title) = LOWER(?) AND LOWER(trigger_topic) = LOWER(?) AND status = 'active'"
        };

        let mut stmt = conn.prepare(query)?;
        let capsule = if topic.is_empty() {
            stmt.query_row(params![gap_title], capsule_from_row)
        } else {
            stmt.query_row(params![gap_title, topic], capsule_from_row)
        };

        match capsule {
            Ok(c) => Ok(Some(c)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    pub fn update_capsule(&self, capsule: &CapsuleGene) -> Result<(), StorageError> {
        self.insert_capsule(capsule)
    }

    pub fn get_gene_pool_stats(&self) -> Result<HashMap<String, serde_json::Value>, StorageError> {
        let conn = self.conn.lock().unwrap();
        let total: i32 = conn.query_row("SELECT COUNT(*) FROM capsules", [], |row| row.get(0))?;

        if total == 0 {
            let mut stats = HashMap::new();
            stats.insert("total".to_string(), serde_json::json!(0));
            stats.insert("avg_score".to_string(), serde_json::json!(0.0));
            stats.insert("by_gap_type".to_string(), serde_json::json!({}));
            return Ok(stats);
        }

        let avg: f64 = conn.query_row(
            "SELECT AVG(outcome_success_score) FROM capsules",
            [],
            |row| row.get(0),
        )?;

        let mut by_type: HashMap<String, i32> = HashMap::new();
        let mut stmt = conn.prepare(
            "SELECT action_gap_type, COUNT(*) as cnt FROM capsules GROUP BY action_gap_type",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, i32>(1)?))
        })?;
        for row in rows {
            let (gt, cnt) = row?;
            by_type.insert(gt, cnt);
        }

        let mut stmt = conn.prepare(
            "SELECT DISTINCT evolved_generation FROM capsules ORDER BY evolved_generation",
        )?;
        let generations: Vec<i32> = stmt
            .query_map([], |row| row.get(0))?
            .filter_map(|r| r.ok())
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

    pub fn recompute_credibility_all(
        &self,
    ) -> Result<HashMap<String, serde_json::Value>, StorageError> {
        use rairos_insight_credibility::CredibilityScorer;

        let capsules = self.load_capsules()?;
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
                if self.update_capsule(&updated_capsule).is_ok() {
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

    fn insert_capsule(&self, capsule: &CapsuleGene) -> Result<(), StorageError> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
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
            params![
                capsule.capsule_id,
                capsule.created_at,
                capsule.trigger_topic,
                capsule.trigger_gap_type,
                serde_json::to_string(&capsule.trigger_keywords)?,
                capsule.action_gap_type,
                capsule.action_gap_title,
                capsule.outcome_success_score,
                capsule.feedback_count,
                capsule.evolved_generation,
                serde_json::to_string(&capsule.archetype)?,
                capsule.status,
                capsule.low_score_streak,
                capsule.credibility_score,
                capsule.trendslop as i32,
                capsule.trendslop_reason,
                capsule.credibility_badge,
                capsule.source_arxiv_category,
                None::<Vec<u8>>,
            ],
        )?;
        Ok(())
    }

    fn load_capsules(&self) -> Result<Vec<CapsuleGene>, StorageError> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare("SELECT * FROM capsules")?;
        let rows = stmt.query_map([], capsule_from_row)?;
        let capsules: Vec<CapsuleGene> = rows.filter_map(|r| r.ok()).collect();
        Ok(capsules)
    }

    pub fn close(&self) {}
}

fn capsule_from_row(row: &rusqlite::Row) -> SqliteResult<CapsuleGene> {
    let archetype_str: String = row.get("archetype")?;
    let archetype: HashMap<String, serde_json::Value> =
        serde_json::from_str(&archetype_str).unwrap_or_default();

    let trigger_kw_str: String = row.get("trigger_keywords")?;
    let trigger_keywords: Vec<String> = serde_json::from_str(&trigger_kw_str).unwrap_or_default();

    Ok(CapsuleGene {
        capsule_id: row.get("capsule_id")?,
        created_at: row.get("created_at")?,
        trigger_topic: row.get("trigger_topic")?,
        trigger_gap_type: row.get("trigger_gap_type")?,
        trigger_keywords,
        action_gap_type: row.get("action_gap_type")?,
        action_gap_title: row.get("action_gap_title")?,
        outcome_success_score: row.get("outcome_success_score")?,
        feedback_count: row.get("feedback_count")?,
        evolved_generation: row.get("evolved_generation")?,
        archetype,
        status: row.get("status")?,
        low_score_streak: row.get("low_score_streak")?,
        credibility_score: row.get("credibility_score")?,
        trendslop: row.get::<_, i32>("trendslop")? != 0,
        trendslop_reason: row.get("trendslop_reason")?,
        credibility_badge: row.get("credibility_badge")?,
        source_arxiv_category: row.get("source_arxiv_category")?,
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
    use std::collections::HashMap;

    fn create_test_storage() -> (CapsuleStorage, tempfile::TempDir) {
        let temp_dir = tempfile::tempdir().unwrap();
        let storage = CapsuleStorage::new(temp_dir.path()).unwrap();
        (storage, temp_dir)
    }

    #[test]
    fn test_new_storage() {
        let (storage, _temp_dir) = create_test_storage();
        assert!(storage.get_gene_pool_stats().is_ok());
    }

    #[test]
    fn test_encode_capsule() {
        let (storage, temp_dir) = create_test_storage();
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
            .unwrap();
        assert_eq!(capsule.trigger_topic, "NLP");
        assert_eq!(capsule.action_gap_type, "method_limitation");
        assert!(!capsule.capsule_id.is_empty());
    }

    #[test]
    fn test_find_capsule() {
        let (storage, temp_dir) = create_test_storage();
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
            .unwrap();

        let found = storage
            .find_capsule("NLP", "method_limitation", &[], 0.0)
            .unwrap();
        assert!(!found.is_empty());
    }

    #[test]
    fn test_archive_capsule() {
        let (storage, temp_dir) = create_test_storage();
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
            .unwrap();

        let result = storage.archive_capsule(&capsule.capsule_id).unwrap();
        assert!(result);
    }

    #[test]
    fn test_get_capsule_by_id() {
        let (storage, temp_dir) = create_test_storage();
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
            .unwrap();

        let found = storage.get_capsule_by_id(&capsule.capsule_id).unwrap();
        assert!(found.is_some());
    }

    #[test]
    fn test_get_capsule_by_title() {
        let (storage, temp_dir) = create_test_storage();
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
            .unwrap();

        let found = storage
            .get_capsule_by_title("Unique test title 123", "NLP")
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

    #[test]
    fn test_gene_pool_stats() {
        let (storage, temp_dir) = create_test_storage();
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
            .unwrap();

        let stats = storage.get_gene_pool_stats().unwrap();
        assert_eq!(stats["total"], serde_json::json!(2));
    }
}
