//! ChatMixin — Chat session persistence.

use rusqlite::{Connection, Result as SqliteResult, params};
use serde::{Deserialize, Serialize};

/// Chat session record.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatSession {
    pub session_id: String,
    pub title: String,
    pub created_at: String,
    pub last_message: Option<String>,
}

/// Chat message record.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub id: i64,
    pub role: String,
    pub content: String,
    pub sources: Option<String>,
    pub created_at: String,
}

/// Chat mixin for session and message operations.
/// Expects the host struct to have a `_conn: Connection` field.
pub trait ChatMixin {
    fn create_chat_session(&self, session_id: &str, title: &str) -> SqliteResult<()>;
    fn add_chat_message(&self, session_id: &str, role: &str, content: &str, sources: Option<&str>) -> SqliteResult<()>;
    fn get_chat_sessions(&self, limit: usize) -> SqliteResult<Vec<ChatSession>>;
    fn get_chat_messages(&self, session_id: &str) -> SqliteResult<Vec<ChatMessage>>;
}

impl<T: AsRef<Connection>> ChatMixin for T {
    fn create_chat_session(&self, session_id: &str, title: &str) -> SqliteResult<()> {
        let conn = self.as_ref();
        conn.execute(
            "INSERT OR IGNORE INTO chat_sessions (id, title, created_at, updated_at) VALUES (?1, ?2, datetime('now'), datetime('now'))",
            [session_id, title],
        )?;
        Ok(())
    }

    fn add_chat_message(&self, session_id: &str, role: &str, content: &str, sources: Option<&str>) -> SqliteResult<()> {
        let conn = self.as_ref();
        conn.execute(
            "INSERT INTO chat_messages (session_id, role, content, sources, created_at) VALUES (?1, ?2, ?3, ?4, datetime('now'))",
            params![session_id, role, content, sources],
        )?;
        // Update session updated_at
        conn.execute(
            "UPDATE chat_sessions SET updated_at = datetime('now') WHERE id = ?",
            [session_id],
        )?;
        Ok(())
    }

    fn get_chat_sessions(&self, limit: usize) -> SqliteResult<Vec<ChatSession>> {
        let conn = self.as_ref();
        let sql = "SELECT cs.session_id, cs.title, cs.created_at, \
            (SELECT content FROM chat_messages WHERE session_id = cs.session_id ORDER BY created_at DESC LIMIT 1) AS last_message \
            FROM chat_sessions cs ORDER BY cs.created_at DESC LIMIT ?";
        let mut stmt = conn.prepare(sql)?;

        let rows = stmt.query_map([limit as i64], |row| {
            Ok(ChatSession {
                session_id: row.get(0)?,
                title: row.get(1)?,
                created_at: row.get(2)?,
                last_message: row.get(3)?,
            })
        })?;

        rows.collect()
    }

    fn get_chat_messages(&self, session_id: &str) -> SqliteResult<Vec<ChatMessage>> {
        let conn = self.as_ref();
        let sql = "SELECT id, role, content, sources, created_at FROM chat_messages \
            WHERE session_id = ? ORDER BY created_at ASC";
        let mut stmt = conn.prepare(sql)?;

        let rows = stmt.query_map([session_id], |row| {
            Ok(ChatMessage {
                id: row.get(0)?,
                role: row.get(1)?,
                content: row.get(2)?,
                sources: row.get(3)?,
                created_at: row.get(4)?,
            })
        })?;

        rows.collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chat_session_struct() {
        let session = ChatSession {
            session_id: "test-123".to_string(),
            title: "Test Session".to_string(),
            created_at: "2024-01-01T00:00:00".to_string(),
            last_message: Some("Hello".to_string()),
        };
        assert_eq!(session.session_id, "test-123");
    }

    #[test]
    fn test_chat_message_struct() {
        let msg = ChatMessage {
            id: 1,
            role: "user".to_string(),
            content: "Hello world".to_string(),
            sources: None,
            created_at: "2024-01-01T00:00:00".to_string(),
        };
        assert_eq!(msg.role, "user");
    }
}
