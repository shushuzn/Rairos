//! LiteratureMixin — Literature review storage.

use rusqlite::{Connection, Result as SqliteResult, params};
use serde::{Deserialize, Serialize};

/// Literature review record.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LiteratureReview {
    pub review_id: String,
    pub topic: String,
    pub content: Option<String>,
    pub paper_ids: Option<String>,
    pub created_at: String,
}

/// Literature review summary (without full content).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LiteratureReviewSummary {
    pub review_id: String,
    pub topic: String,
    pub created_at: String,
    pub word_count: i64,
}

/// Literature mixin for review operations.
/// Expects the host struct to have a `_conn: Connection` field.
pub trait LiteratureMixin {
    fn add_literature_review(&self, review_id: &str, topic: &str, content: &str, paper_ids: Option<&str>) -> SqliteResult<bool>;
    fn list_literature_reviews(&self) -> SqliteResult<Vec<LiteratureReviewSummary>>;
    fn get_literature_review(&self, review_id: &str) -> SqliteResult<Option<LiteratureReview>>;
    fn update_literature_review(&self, review_id: &str, content: Option<&str>, paper_ids: Option<&str>) -> SqliteResult<bool>;
    fn delete_literature_review(&self, review_id: &str) -> SqliteResult<bool>;
}

impl<T: AsRef<Connection>> LiteratureMixin for T {
    fn add_literature_review(&self, review_id: &str, topic: &str, content: &str, paper_ids: Option<&str>) -> SqliteResult<bool> {
        let conn = self.as_ref();
        let result = conn.execute(
            "INSERT INTO literature_reviews (review_id, topic, content, paper_ids, created_at) \
            VALUES (?1, ?2, ?3, ?4, datetime('now'))",
            params![review_id, topic, content, paper_ids],
        );
        match result {
            Ok(_) => Ok(true),
            Err(_) => Ok(false),
        }
    }

    fn list_literature_reviews(&self) -> SqliteResult<Vec<LiteratureReviewSummary>> {
        let conn = self.as_ref();
        let mut stmt = conn.prepare(
            "SELECT review_id, topic, created_at, \
            LENGTH(content) - LENGTH(REPLACE(content, ' ', '')) + 1 AS word_count \
            FROM literature_reviews ORDER BY created_at DESC",
        )?;

        let rows = stmt.query_map([], |row| {
            Ok(LiteratureReviewSummary {
                review_id: row.get(0)?,
                topic: row.get(1)?,
                created_at: row.get(2)?,
                word_count: row.get(3)?,
            })
        })?;

        rows.collect()
    }

    fn get_literature_review(&self, review_id: &str) -> SqliteResult<Option<LiteratureReview>> {
        let conn = self.as_ref();
        let mut stmt = conn.prepare(
            "SELECT review_id, topic, content, paper_ids, created_at FROM literature_reviews WHERE review_id = ?",
        )?;

        let result = stmt.query_row([review_id], |row| {
            Ok(LiteratureReview {
                review_id: row.get(0)?,
                topic: row.get(1)?,
                content: row.get(2)?,
                paper_ids: row.get(3)?,
                created_at: row.get(4)?,
            })
        });

        match result {
            Ok(review) => Ok(Some(review)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e),
        }
    }

    fn update_literature_review(&self, review_id: &str, content: Option<&str>, paper_ids: Option<&str>) -> SqliteResult<bool> {
        let conn = self.as_ref();

        let mut updates: Vec<&str> = Vec::new();
        let mut params_vec: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();

        if let Some(c) = content {
            updates.push("content = ?");
            params_vec.push(Box::new(c.to_string()));
        }
        if let Some(p) = paper_ids {
            updates.push("paper_ids = ?");
            params_vec.push(Box::new(p.to_string()));
        }

        if updates.is_empty() {
            return Ok(false);
        }

        params_vec.push(Box::new(review_id.to_string()));
        let sql = format!("UPDATE literature_reviews SET {} WHERE review_id = ?", updates.join(", "));

        let result = conn.execute(&sql, rusqlite::params_from_iter(params_vec.iter().map(|p| p.as_ref())));
        match result {
            Ok(_) => Ok(true),
            Err(_) => Ok(false),
        }
    }

    fn delete_literature_review(&self, review_id: &str) -> SqliteResult<bool> {
        let conn = self.as_ref();
        conn.execute("DELETE FROM literature_reviews WHERE review_id = ?", [review_id])?;
        Ok(true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_literature_review_struct() {
        let review = LiteratureReview {
            review_id: "review-123".to_string(),
            topic: "Neural Architecture Search".to_string(),
            content: Some("This is a review of NAS methods.".to_string()),
            paper_ids: Some("paper1,paper2".to_string()),
            created_at: "2024-01-01T00:00:00".to_string(),
        };
        assert_eq!(review.review_id, "review-123");
        assert!(review.content.is_some());
    }

    #[test]
    fn test_literature_review_summary_struct() {
        let summary = LiteratureReviewSummary {
            review_id: "review-456".to_string(),
            topic: "Test Topic".to_string(),
            created_at: "2024-01-01T00:00:00".to_string(),
            word_count: 100,
        };
        assert_eq!(summary.word_count, 100);
    }
}
