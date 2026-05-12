//! SubscriptionMixin — arXiv subscription management.

use rusqlite::{Connection, Result as SqliteResult, params};
use serde::{Deserialize, Serialize};

/// arXiv subscription record.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArxivSubscription {
    pub id: i64,
    pub topic: String,
    pub categories: String,
    pub keywords: String,
    pub enabled: bool,
    pub last_check_id: Option<String>,
    pub created_at: String,
}

/// Subscription paper record.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubscriptionPaper {
    pub paper_id: String,
    pub title: String,
    pub published: String,
    pub discovered_at: String,
}

/// Subscription mixin for arXiv subscription operations.
/// Expects the host struct to have a `_conn: Connection` field.
pub trait SubscriptionMixin {
    fn add_arxiv_subscription(&self, topic: &str, categories: &str, keywords: &str) -> SqliteResult<i64>;
    fn list_arxiv_subscriptions(&self) -> SqliteResult<Vec<ArxivSubscription>>;
    fn get_arxiv_subscription(&self, sub_id: i64) -> SqliteResult<Option<ArxivSubscription>>;
    fn delete_arxiv_subscription(&self, sub_id: i64) -> SqliteResult<bool>;
    fn record_subscription_paper(&self, sub_id: i64, paper_id: &str, title: &str, published: &str) -> SqliteResult<()>;
}

impl<T: AsRef<Connection>> SubscriptionMixin for T {
    fn add_arxiv_subscription(&self, topic: &str, categories: &str, keywords: &str) -> SqliteResult<i64> {
        let conn = self.as_ref();
        conn.execute(
            "INSERT INTO arxiv_subscriptions (topic, categories, keywords, enabled, created_at) \
            VALUES (?1, ?2, ?3, 1, datetime('now'))",
            params![topic, categories, keywords],
        )?;
        Ok(conn.last_insert_rowid())
    }

    fn list_arxiv_subscriptions(&self) -> SqliteResult<Vec<ArxivSubscription>> {
        let conn = self.as_ref();
        let mut stmt = conn.prepare(
            "SELECT id, topic, categories, keywords, enabled, last_check_id, created_at \
            FROM arxiv_subscriptions ORDER BY created_at DESC",
        )?;

        let rows = stmt.query_map([], |row| {
            Ok(ArxivSubscription {
                id: row.get(0)?,
                topic: row.get(1)?,
                categories: row.get(2)?,
                keywords: row.get(3)?,
                enabled: row.get::<_, i32>(4)? != 0,
                last_check_id: row.get(5)?,
                created_at: row.get(6)?,
            })
        })?;

        rows.collect()
    }

    fn get_arxiv_subscription(&self, sub_id: i64) -> SqliteResult<Option<ArxivSubscription>> {
        let conn = self.as_ref();
        let mut stmt = conn.prepare(
            "SELECT id, topic, categories, keywords, enabled, last_check_id, created_at \
            FROM arxiv_subscriptions WHERE id = ?",
        )?;

        let result = stmt.query_row([sub_id], |row| {
            Ok(ArxivSubscription {
                id: row.get(0)?,
                topic: row.get(1)?,
                categories: row.get(2)?,
                keywords: row.get(3)?,
                enabled: row.get::<_, i32>(4)? != 0,
                last_check_id: row.get(5)?,
                created_at: row.get(6)?,
            })
        });

        match result {
            Ok(sub) => Ok(Some(sub)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e),
        }
    }

    fn delete_arxiv_subscription(&self, sub_id: i64) -> SqliteResult<bool> {
        let conn = self.as_ref();
        conn.execute("DELETE FROM arxiv_subscriptions WHERE id = ?", [sub_id])?;
        Ok(true)
    }

    fn record_subscription_paper(&self, sub_id: i64, paper_id: &str, title: &str, published: &str) -> SqliteResult<()> {
        let conn = self.as_ref();
        conn.execute(
            "INSERT OR IGNORE INTO subscription_papers (sub_id, paper_id, title, published, discovered_at) \
            VALUES (?1, ?2, ?3, ?4, datetime('now'))",
            params![sub_id, paper_id, title, published],
        )?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_subscription_struct() {
        let sub = ArxivSubscription {
            id: 1,
            topic: "machine learning".to_string(),
            categories: "cs.AI".to_string(),
            keywords: "neural networks".to_string(),
            enabled: true,
            last_check_id: None,
            created_at: "2024-01-01T00:00:00".to_string(),
        };
        assert_eq!(sub.topic, "machine learning");
        assert!(sub.enabled);
    }

    #[test]
    fn test_subscription_paper_struct() {
        let paper = SubscriptionPaper {
            paper_id: "2401.12345".to_string(),
            title: "Test Paper".to_string(),
            published: "2024-01-01".to_string(),
            discovered_at: "2024-01-01T00:00:00".to_string(),
        };
        assert_eq!(paper.paper_id, "2401.12345");
    }
}
