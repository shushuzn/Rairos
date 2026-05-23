//! Conversation History Module for audit, replay, and context management.
//!
//! Based on Microsoft Multi-Agent Reference Architecture:
//! - Maintains conversation history for audit trail
//! - Enables context replay for multi-turn interactions
//! - Supports conversation search and filtering
//!
//! ## Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────┐
//! │           ConversationHistory                          │
//! │  ┌─────────────────────────────────────────────┐   │
//! │  │ conversations: HashMap<ConversationId, Conv>  │   │
//! │  │ index: ConversationIndex                     │   │
//! │  └─────────────────────────────────────────────┘   │
//! └─────────────────────────────────────────────────────┘
//!                         │
//!         ┌───────────────┼───────────────┐
//!         ▼               ▼               ▼
//!    ┌─────────┐     ┌─────────┐     ┌─────────┐
//!    │  Conv 1 │     │  Conv 2 │     │  Conv 3 │
//!    │ Turns[] │     │ Turns[] │     │ Turns[] │
//!    └─────────┘     └─────────┘     └─────────┘
//! ```

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::utils::current_timestamp;

/// Unique conversation identifier
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ConversationId(pub String);

impl ConversationId {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    pub fn generate() -> Self {
        Self(uuid::Uuid::new_v4().to_string())
    }
}

impl std::fmt::Display for ConversationId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Unique turn identifier within a conversation
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TurnId(pub String);

impl TurnId {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }
}

impl std::fmt::Display for TurnId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Message role in a conversation
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MessageRole {
    System,
    User,
    Assistant,
    Tool,
}

impl MessageRole {
    pub fn as_str(&self) -> &'static str {
        match self {
            MessageRole::System => "system",
            MessageRole::User => "user",
            MessageRole::Assistant => "assistant",
            MessageRole::Tool => "tool",
        }
    }
}

/// A single message in a conversation turn
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    /// Message role
    pub role: MessageRole,
    /// Message content
    pub content: String,
    /// Optional tool name (for tool messages)
    #[serde(default)]
    pub tool_name: Option<String>,
    /// Optional tool result (for tool messages)
    #[serde(default)]
    pub tool_result: Option<String>,
    /// Timestamp when message was created
    pub timestamp: u64,
    /// Message metadata
    #[serde(default)]
    pub metadata: HashMap<String, String>,
}

/// A turn in a conversation (one exchange = potentially multiple messages)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Turn {
    /// Unique turn ID
    pub id: TurnId,
    /// Messages in this turn
    pub messages: Vec<Message>,
    /// Agent ID that handled this turn (if applicable)
    #[serde(default)]
    pub agent_id: Option<String>,
    /// Intent detected for this turn
    #[serde(default)]
    pub intent: Option<String>,
    /// Tokens used in this turn
    #[serde(default)]
    pub tokens_used: Option<u32>,
    /// Turn duration in milliseconds (if completed)
    #[serde(default)]
    pub duration_ms: Option<u64>,
    /// Turn status
    #[serde(default)]
    pub status: TurnStatus,
}

/// Turn status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TurnStatus {
    /// Turn is in progress
    InProgress,
    /// Turn completed successfully
    Completed,
    /// Turn failed
    Failed,
    /// Turn was truncated
    Truncated,
}

impl Default for TurnStatus {
    fn default() -> Self {
        Self::InProgress
    }
}

/// Conversation metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationMetadata {
    /// Conversation ID
    pub id: ConversationId,
    /// Conversation title (optional, can be auto-generated)
    #[serde(default)]
    pub title: Option<String>,
    /// User ID who started the conversation
    #[serde(default)]
    pub user_id: Option<String>,
    /// Session ID
    #[serde(default)]
    pub session_id: Option<String>,
    /// Tags for categorization
    #[serde(default)]
    pub tags: Vec<String>,
    /// Created timestamp
    pub created_at: u64,
    /// Last updated timestamp
    pub updated_at: u64,
    /// Total turns count
    pub turn_count: u32,
    /// Total tokens used
    #[serde(default)]
    pub total_tokens: u64,
    /// Custom metadata
    #[serde(default)]
    pub extras: HashMap<String, String>,
}

impl ConversationMetadata {
    fn new(id: ConversationId) -> Self {
        let now = current_timestamp();
        Self {
            id,
            title: None,
            user_id: None,
            session_id: None,
            tags: Vec::new(),
            created_at: now,
            updated_at: now,
            turn_count: 0,
            total_tokens: 0,
            extras: HashMap::new(),
        }
    }
}

/// A complete conversation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Conversation {
    /// Conversation metadata
    pub metadata: ConversationMetadata,
    /// Conversation turns (in order)
    pub turns: VecDeque<Turn>,
    /// Maximum turns to retain (0 = unlimited)
    pub max_turns: u32,
}

impl Conversation {
    /// Create a new conversation
    pub fn new(id: ConversationId) -> Self {
        Self {
            metadata: ConversationMetadata::new(id),
            turns: VecDeque::new(),
            max_turns: 0, // Unlimited by default
        }
    }

    /// Create with max turns (for circular buffer behavior)
    pub fn with_max_turns(id: ConversationId, max_turns: u32) -> Self {
        Self {
            metadata: ConversationMetadata::new(id),
            turns: VecDeque::new(),
            max_turns,
        }
    }

    /// Add a turn
    pub fn add_turn(&mut self, turn: Turn) {
        self.metadata.turn_count += 1;
        self.metadata.updated_at = current_timestamp();
        self.turns.push_back(turn);

        // Trim if exceeds max_turns
        if self.max_turns > 0 && self.turns.len() > self.max_turns as usize {
            self.turns.pop_front();
        }
    }

    /// Get recent turns
    pub fn recent_turns(&self, count: usize) -> &[Turn] {
        let start = self.turns.len().saturating_sub(count);
        &self.turns[start..]
    }

    /// Get total message count
    pub fn message_count(&self) -> usize {
        self.turns.iter().map(|t| t.messages.len()).sum()
    }

    /// Get all messages as flat list
    pub fn all_messages(&self) -> Vec<&Message> {
        self.turns.iter().flat_map(|t| t.messages.iter()).collect()
    }
}

/// Search filter for conversations
#[derive(Debug, Clone, Default)]
pub struct ConversationFilter {
    /// Filter by user ID
    pub user_id: Option<String>,
    /// Filter by session ID
    pub session_id: Option<String>,
    /// Filter by tags (any match)
    pub tags: Option<Vec<String>>,
    /// Filter by date range
    pub date_range: Option<DateRange>,
    /// Filter by minimum turn count
    pub min_turns: Option<u32>,
    /// Filter by maximum turn count
    pub max_turns: Option<u32>,
}

/// Date range filter
#[derive(Debug, Clone)]
pub struct DateRange {
    pub start: u64,
    pub end: u64,
}

/// Sort options for listing conversations
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortBy {
    CreatedAt,
    UpdatedAt,
    TurnCount,
}

/// Sort order
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortOrder {
    Ascending,
    Descending,
}

impl Default for SortOrder {
    fn default() -> Self {
        Self::Descending
    }
}

/// Search result item
#[derive(Debug, Clone)]
pub struct ConversationSummary {
    pub id: ConversationId,
    pub title: Option<String>,
    pub user_id: Option<String>,
    pub turn_count: u32,
    pub message_count: usize,
    pub created_at: u64,
    pub updated_at: u64,
    pub tags: Vec<String>,
}

impl From<&Conversation> for ConversationSummary {
    fn from(conv: &Conversation) -> Self {
        Self {
            id: conv.metadata.id.clone(),
            title: conv.metadata.title.clone(),
            user_id: conv.metadata.user_id.clone(),
            turn_count: conv.metadata.turn_count,
            message_count: conv.message_count(),
            created_at: conv.metadata.created_at,
            updated_at: conv.metadata.updated_at,
            tags: conv.metadata.tags.clone(),
        }
    }
}

/// Conversation History - manages all conversations
#[derive(Debug, Clone)]
pub struct ConversationHistory {
    /// All conversations
    conversations: Arc<RwLock<HashMap<ConversationId, Conversation>>>,
    /// Tag index: tag -> conversation IDs
    tag_index: Arc<RwLock<HashMap<String, HashSet<ConversationId>>>>,
    /// User index: user_id -> conversation IDs
    user_index: Arc<RwLock<HashMap<String, HashSet<ConversationId>>>>,
    /// Default max turns for new conversations
    default_max_turns: u32,
}

impl Default for ConversationHistory {
    fn default() -> Self {
        Self::new()
    }
}

impl ConversationHistory {
    /// Create a new conversation history
    pub fn new() -> Self {
        Self {
            conversations: Arc::new(RwLock::new(HashMap::new())),
            tag_index: Arc::new(RwLock::new(HashMap::new())),
            user_index: Arc::new(RwLock::new(HashMap::new())),
            default_max_turns: 100,
        }
    }

    /// Set default max turns for new conversations
    pub fn with_default_max_turns(mut self, max: u32) -> Self {
        self.default_max_turns = max;
        self
    }

    /// Create a new conversation
    pub async fn create(&self) -> ConversationId {
        let id = ConversationId::generate();
        let conv = if self.default_max_turns > 0 {
            Conversation::with_max_turns(id.clone(), self.default_max_turns)
        } else {
            Conversation::new(id.clone())
        };

        let mut conversations = self.conversations.write().await;
        conversations.insert(id.clone(), conv);
        drop(conversations);

        id
    }

    /// Create a conversation with specific ID
    pub async fn create_with_id(&self, id: ConversationId) -> Conversation {
        let conv = if self.default_max_turns > 0 {
            Conversation::with_max_turns(id.clone(), self.default_max_turns)
        } else {
            Conversation::new(id.clone())
        };

        // Use entry API to insert and get reference in single lock acquisition
        let mut conversations = self.conversations.write().await;
        conversations.insert(id, conv.clone());
        conv
    }

    /// Get a conversation by ID
    pub async fn get(&self, id: &ConversationId) -> Option<Conversation> {
        let conversations = self.conversations.read().await;
        conversations.get(id).cloned()
    }

    /// Delete a conversation
    pub async fn delete(&self, id: &ConversationId) -> Option<Conversation> {
        // Get conversation first for index cleanup
        let conv = {
            let conversations = self.conversations.read().await;
            conversations.get(id).cloned()
        };

        if let Some(conv) = conv {
            // Remove from tag index
            {
                let mut tag_index = self.tag_index.write().await;
                for tag in &conv.metadata.tags {
                    if let Some(ids) = tag_index.get_mut(tag) {
                        ids.remove(id);
                        if ids.is_empty() {
                            tag_index.remove(tag);
                        }
                    }
                }
            }

            // Remove from user index
            {
                let mut user_index = self.user_index.write().await;
                if let Some(ref user_id) = conv.metadata.user_id {
                    if let Some(ids) = user_index.get_mut(user_id) {
                        ids.remove(id);
                        if ids.is_empty() {
                            user_index.remove(user_id);
                        }
                    }
                }
            }

            // Remove from conversations
            let mut conversations = self.conversations.write().await;
            conversations.remove(id)
        } else {
            None
        }
    }

    /// Add a turn to a conversation
    pub async fn add_turn(&self, conv_id: &ConversationId, turn: Turn) -> bool {
        let mut conversations = self.conversations.write().await;
        if let Some(conv) = conversations.get_mut(conv_id) {
            conv.add_turn(turn);
            true
        } else {
            false
        }
    }

    /// Update conversation metadata
    pub async fn update_metadata(
        &self,
        conv_id: &ConversationId,
        title: Option<String>,
        user_id: Option<String>,
        session_id: Option<String>,
        tags: Option<Vec<String>>,
    ) -> bool {
        // Collect old values for index updates before locking
        let old_user;
        let old_tags;
        let new_user_for_index = user_id.clone();

        {
            let mut conversations = self.conversations.write().await;
            if let Some(conv) = conversations.get_mut(conv_id) {
                old_user = conv.metadata.user_id.clone();
                old_tags = std::mem::take(&mut conv.metadata.tags);

                if let Some(t) = title {
                    conv.metadata.title = Some(t);
                }
                if let Some(u) = user_id {
                    conv.metadata.user_id = Some(u);
                }
                if let Some(s) = session_id {
                    conv.metadata.session_id = Some(s);
                }
                if let Some(ref new_tags) = tags {
                    conv.metadata.tags = new_tags.clone();
                }
                conv.metadata.updated_at = current_timestamp();
            } else {
                return false;
            }
        } // Drop conversations lock here

        // Update user index
        if let Some(old) = old_user {
            if let Some(u) = new_user_for_index {
                let mut user_index = self.user_index.write().await;
                if let Some(ids) = user_index.get_mut(&old) {
                    ids.remove(conv_id);
                }
                user_index.entry(u).or_default().insert(conv_id.clone());
            }
        } else if let Some(u) = new_user_for_index {
            let mut user_index = self.user_index.write().await;
            user_index.entry(u).or_default().insert(conv_id.clone());
        }

        // Update tag index
        if !old_tags.is_empty() || tags.is_some() {
            let mut tag_index = self.tag_index.write().await;
            for tag in &old_tags {
                if let Some(ids) = tag_index.get_mut(tag) {
                    ids.remove(conv_id);
                }
            }
            if let Some(ref new_tags) = tags {
                for tag in new_tags {
                    tag_index.entry(tag.clone()).or_default().insert(conv_id.clone());
                }
            }
        }

        true
    }

    /// List conversations with optional filter
    pub async fn list(&self, filter: Option<ConversationFilter>, sort_by: SortBy, order: SortOrder) -> Vec<ConversationSummary> {
        let conversations = self.conversations.read().await;

        let mut results: Vec<ConversationSummary> = conversations
            .values()
            .filter(|conv| {
                if let Some(ref f) = filter {
                    // User ID filter
                    if let Some(ref user_id) = f.user_id {
                        if conv.metadata.user_id.as_ref() != Some(user_id) {
                            return false;
                        }
                    }
                    // Session ID filter
                    if let Some(ref session_id) = f.session_id {
                        if conv.metadata.session_id.as_ref() != Some(session_id) {
                            return false;
                        }
                    }
                    // Tags filter (any match)
                    if let Some(ref tags) = f.tags {
                        if !tags.iter().any(|t| conv.metadata.tags.contains(t)) {
                            return false;
                        }
                    }
                    // Date range filter
                    if let Some(ref range) = f.date_range {
                        if conv.metadata.created_at < range.start || conv.metadata.created_at > range.end {
                            return false;
                        }
                    }
                    // Turn count filters
                    if let Some(min) = f.min_turns {
                        if conv.metadata.turn_count < min {
                            return false;
                        }
                    }
                    if let Some(max) = f.max_turns {
                        if conv.metadata.turn_count > max {
                            return false;
                        }
                    }
                }
                true
            })
            .map(ConversationSummary::from)
            .collect();

        // Sort
        match sort_by {
            SortBy::CreatedAt => results.sort_by(|a, b| a.created_at.cmp(&b.created_at)),
            SortBy::UpdatedAt => results.sort_by(|a, b| a.updated_at.cmp(&b.updated_at)),
            SortBy::TurnCount => results.sort_by(|a, b| a.turn_count.cmp(&b.turn_count)),
        }

        if order == SortOrder::Descending {
            results.reverse();
        }

        results
    }

    /// Search conversations by content (basic substring match)
    pub async fn search(&self, query: &str, limit: usize) -> Vec<ConversationSummary> {
        let query_lower = query.to_lowercase();
        let conversations = self.conversations.read().await;

        let mut results: Vec<ConversationSummary> = conversations
            .values()
            .filter(|conv| {
                // Check title
                if let Some(ref title) = conv.metadata.title {
                    if title.to_lowercase().contains(&query_lower) {
                        return true;
                    }
                }
                // Check tags
                if conv.metadata.tags.iter().any(|t| t.to_lowercase().contains(&query_lower)) {
                    return true;
                }
                // Check turns content - try case-sensitive match first (fast path)
                // Only call to_lowercase() if case-sensitive search fails
                for turn in &conv.turns {
                    for msg in &turn.messages {
                        if msg.content.contains(&query_lower) {
                            return true;
                        }
                        // Fallback to case-insensitive search
                        if msg.content.to_lowercase().contains(&query_lower) {
                            return true;
                        }
                    }
                }
                false
            })
            .take(limit)
            .map(ConversationSummary::from)
            .collect();

        results
    }

    /// Get conversation statistics
    pub async fn stats(&self) -> HistoryStats {
        let conversations = self.conversations.read().await;
        let total_conversations = conversations.len();
        let mut total_turns = 0u64;
        let mut total_messages = 0u64;

        for conv in conversations.values() {
            total_turns += conv.metadata.turn_count as u64;
            total_messages += conv.message_count() as u64;
        }

        HistoryStats {
            total_conversations,
            total_turns,
            total_messages,
        }
    }

    /// Get conversation count
    pub async fn len(&self) -> usize {
        let conversations = self.conversations.read().await;
        conversations.len()
    }

    /// Check if empty
    pub async fn is_empty(&self) -> bool {
        let conversations = self.conversations.read().await;
        conversations.is_empty()
    }

    /// Clear all conversations
    pub async fn clear(&self) {
        let mut conversations = self.conversations.write().await;
        conversations.clear();
        drop(conversations);

        let mut tag_index = self.tag_index.write().await;
        tag_index.clear();
        drop(tag_index);

        let mut user_index = self.user_index.write().await;
        user_index.clear();
    }

    /// Export conversation to JSON string
    pub async fn export(&self, conv_id: &ConversationId) -> Option<String> {
        let conversations = self.conversations.read().await;
        conversations.get(conv_id).map(|c| serde_json::to_string_pretty(c).ok()).flatten()
    }

    /// Import conversation from JSON string
    pub async fn import(&self, json: &str) -> Result<ConversationId, ImportError> {
        let conv: Conversation = serde_json::from_str(json)
            .map_err(|e| ImportError::ParseError(e.to_string()))?;

        let conv_id = conv.metadata.id.clone();

        let mut conversations = self.conversations.write().await;
        if conversations.contains_key(&conv_id) {
            return Err(ImportError::AlreadyExists(conv_id.to_string()));
        }

        // Update indexes
        let tags = conv.metadata.tags.clone();
        let user_id = conv.metadata.user_id.clone();
        drop(conversations);

        if !tags.is_empty() {
            let mut tag_index = self.tag_index.write().await;
            for tag in &tags {
                tag_index.entry(tag.clone()).or_default().insert(conv_id.clone());
            }
        }

        if let Some(ref uid) = user_id {
            let mut user_index = self.user_index.write().await;
            user_index.entry(uid.clone()).or_default().insert(conv_id.clone());
        }

        let mut conversations = self.conversations.write().await;
        conversations.insert(conv_id.clone(), conv);

        Ok(conv_id)
    }
}

/// History statistics
#[derive(Debug, Clone)]
pub struct HistoryStats {
    pub total_conversations: usize,
    pub total_turns: u64,
    pub total_messages: u64,
}

/// Import errors
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ImportError {
    ParseError(String),
    AlreadyExists(String),
}

impl std::fmt::Display for ImportError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ImportError::ParseError(msg) => write!(f, "Parse error: {}", msg),
            ImportError::AlreadyExists(id) => write!(f, "Conversation already exists: {}", id),
        }
    }
}

impl std::error::Error for ImportError {}

// =============================================================================
// Builder
// =============================================================================

/// Builder for creating ConversationHistory
pub struct ConversationHistoryBuilder {
    default_max_turns: u32,
}

impl ConversationHistoryBuilder {
    pub fn new() -> Self {
        Self {
            default_max_turns: 100,
        }
    }

    /// Set default max turns for new conversations
    pub fn default_max_turns(mut self, max: u32) -> Self {
        self.default_max_turns = max;
        self
    }

    /// Build the ConversationHistory
    pub fn build(self) -> ConversationHistory {
        ConversationHistory::new().with_default_max_turns(self.default_max_turns)
    }
}

impl Default for ConversationHistoryBuilder {
    fn default() -> Self {
        Self::new()
    }
}

// =============================================================================
// Utilities
// =============================================================================

// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_message(role: MessageRole, content: &str) -> Message {
        Message {
            role,
            content: content.to_string(),
            tool_name: None,
            tool_result: None,
            timestamp: current_timestamp(),
            metadata: HashMap::new(),
        }
    }

    fn create_test_turn(messages: Vec<Message>) -> Turn {
        Turn {
            id: TurnId::new(format!("turn-{}", uuid::Uuid::new_v4())),
            messages,
            agent_id: None,
            intent: None,
            tokens_used: None,
            duration_ms: None,
            status: TurnStatus::Completed,
        }
    }

    #[tokio::test]
    async fn test_create_and_get() {
        let history = ConversationHistory::new();

        let conv_id = history.create().await;
        assert!(!conv_id.0.is_empty());

        let conv = history.get(&conv_id).await;
        assert!(conv.is_some());
        assert_eq!(conv.unwrap().metadata.turn_count, 0);
    }

    #[tokio::test]
    async fn test_add_turn() {
        let history = ConversationHistory::new();
        let conv_id = history.create().await;

        let turn = create_test_turn(vec![
            create_test_message(MessageRole::User, "Hello"),
            create_test_message(MessageRole::Assistant, "Hi there!"),
        ]);

        let added = history.add_turn(&conv_id, turn).await;
        assert!(added);

        let conv = history.get(&conv_id).await.unwrap();
        assert_eq!(conv.metadata.turn_count, 1);
        assert_eq!(conv.turns[0].messages.len(), 2);
    }

    #[tokio::test]
    async fn test_delete() {
        let history = ConversationHistory::new();
        let conv_id = history.create().await;

        let deleted = history.delete(&conv_id).await;
        assert!(deleted.is_some());

        let conv = history.get(&conv_id).await;
        assert!(conv.is_none());
    }

    #[tokio::test]
    async fn test_max_turns() {
        let history = ConversationHistory::new().with_default_max_turns(3);
        let conv_id = history.create().await;

        for i in 0..5 {
            let turn = create_test_turn(vec![create_test_message(
                MessageRole::User,
                &format!("Message {}", i),
            )]);
            history.add_turn(&conv_id, turn).await;
        }

        let conv = history.get(&conv_id).await.unwrap();
        // Should only have 3 turns (oldest removed)
        assert_eq!(conv.turns.len(), 3);
        // Latest messages should be 3, 4
        assert!(conv.turns[0].messages[0].content.contains("Message 2"));
        assert!(conv.turns[2].messages[0].content.contains("Message 4"));
    }

    #[tokio::test]
    async fn test_update_metadata() {
        let history = ConversationHistory::new();
        let conv_id = history.create().await;

        history
            .update_metadata(
                &conv_id,
                Some("Test Title".to_string()),
                Some("user123".to_string()),
                None,
                Some(vec!["test".to_string(), "demo".to_string()]),
            )
            .await;

        let conv = history.get(&conv_id).await.unwrap();
        assert_eq!(conv.metadata.title.as_deref(), Some("Test Title"));
        assert_eq!(conv.metadata.user_id.as_deref(), Some("user123"));
        assert_eq!(conv.metadata.tags, vec!["test", "demo"]);
    }

    #[tokio::test]
    async fn test_list_with_filter() {
        let history = ConversationHistory::new();

        // Create conversations with different user IDs
        for i in 0..3 {
            let conv_id = history.create().await;
            history
                .update_metadata(
                    &conv_id,
                    None,
                    Some(format!("user{}", i % 2)),
                    None,
                    Some(vec![format!("tag{}", i)]),
                )
                .await;
        }

        // Filter by user_id
        let filter = ConversationFilter {
            user_id: Some("user0".to_string()),
            session_id: None,
            tags: None,
            date_range: None,
            min_turns: None,
            max_turns: None,
        };

        let results = history.list(Some(filter), SortBy::CreatedAt, SortOrder::Ascending).await;
        assert_eq!(results.len(), 2); // user0 has conv 0 and conv 2
    }

    #[tokio::test]
    async fn test_search() {
        let history = ConversationHistory::new();
        let conv_id = history.create().await;

        history
            .update_metadata(
                &conv_id,
                Some("Python Tutorial".to_string()),
                None,
                None,
                None,
            )
            .await;

        let turn = create_test_turn(vec![create_test_message(
            MessageRole::User,
            "How to write a Python function?",
        )]);
        history.add_turn(&conv_id, turn).await;

        // Search by title
        let results = history.search("Python", 10).await;
        assert!(!results.is_empty());

        // Search by content
        let results = history.search("function", 10).await;
        assert!(!results.is_empty());
    }

    #[tokio::test]
    async fn test_stats() {
        let history = ConversationHistory::new();

        for _ in 0..3 {
            let conv_id = history.create().await;
            let turn = create_test_turn(vec![
                create_test_message(MessageRole::User, "Hi"),
                create_test_message(MessageRole::Assistant, "Hello"),
            ]);
            history.add_turn(&conv_id, turn).await;
        }

        let stats = history.stats().await;
        assert_eq!(stats.total_conversations, 3);
        assert_eq!(stats.total_turns, 3);
        assert_eq!(stats.total_messages, 6); // 2 per turn * 3 turns
    }

    #[tokio::test]
    async fn test_export_import() {
        let history = ConversationHistory::new();
        let conv_id = history.create().await;

        let turn = create_test_turn(vec![create_test_message(
            MessageRole::User,
            "Test message",
        )]);
        history.add_turn(&conv_id, turn).await;

        // Export
        let json = history.export(&conv_id).await.unwrap();

        // Delete original
        history.delete(&conv_id).await;

        // Import
        let imported_id = history.import(&json).await.unwrap();
        assert_eq!(imported_id, conv_id);

        // Verify
        let conv = history.get(&conv_id).await.unwrap();
        assert_eq!(conv.metadata.turn_count, 1);
    }

    #[tokio::test]
    async fn test_recent_turns() {
        let history = ConversationHistory::new();
        let conv_id = history.create().await;

        for i in 0..5 {
            let turn = create_test_turn(vec![create_test_message(
                MessageRole::User,
                &format!("Message {}", i),
            )]);
            history.add_turn(&conv_id, turn).await;
        }

        let conv = history.get(&conv_id).await.unwrap();
        let recent = conv.recent_turns(3);
        assert_eq!(recent.len(), 3);
        // Should be messages 2, 3, 4
        assert!(recent[0].messages[0].content.contains("Message 2"));
        assert!(recent[2].messages[0].content.contains("Message 4"));
    }

    #[tokio::test]
    async fn test_message_count() {
        let history = ConversationHistory::new();
        let conv_id = history.create().await;

        // Add turns with different message counts
        history
            .add_turn(
                &conv_id,
                create_test_turn(vec![
                    create_test_message(MessageRole::User, "Hi"),
                    create_test_message(MessageRole::Assistant, "Hello"),
                ]),
            )
            .await;

        history
            .add_turn(
                &conv_id,
                create_test_turn(vec![create_test_message(MessageRole::User, "How are you?")]),
            )
            .await;

        let conv = history.get(&conv_id).await.unwrap();
        assert_eq!(conv.message_count(), 3);
    }

    #[tokio::test]
    async fn test_all_messages() {
        let history = ConversationHistory::new();
        let conv_id = history.create().await;

        history
            .add_turn(
                &conv_id,
                create_test_turn(vec![create_test_message(MessageRole::User, "First")]),
            )
            .await;

        history
            .add_turn(
                &conv_id,
                create_test_turn(vec![create_test_message(MessageRole::User, "Second")]),
            )
            .await;

        let conv = history.get(&conv_id).await.unwrap();
        let messages = conv.all_messages();

        assert_eq!(messages.len(), 2);
        assert_eq!(messages[0].content, "First");
        assert_eq!(messages[1].content, "Second");
    }

    #[tokio::test]
    async fn test_is_empty() {
        let history = ConversationHistory::new();
        assert!(history.is_empty().await);

        history.create().await;
        assert!(!history.is_empty().await);
    }
}
