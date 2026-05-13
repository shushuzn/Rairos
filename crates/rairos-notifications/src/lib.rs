//! rairos-notifications — Notification system with webhook delivery.
//!
//! Ported from `core/notifications.py`.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::LazyLock;
use std::sync::Mutex as StdMutex;

/// Notification levels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum NotificationLevel {
    Info,
    Success,
    Warning,
    Error,
}

impl NotificationLevel {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Info => "info",
            Self::Success => "success",
            Self::Warning => "warning",
            Self::Error => "error",
        }
    }
}

/// A notification message.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Notification {
    pub level: NotificationLevel,
    pub title: String,
    pub message: String,
    pub timestamp: f64,
}

impl Notification {
    pub fn new(level: NotificationLevel, title: &str, message: &str) -> Self {
        Self {
            level,
            title: title.to_string(),
            message: message.to_string(),
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs_f64())
                .unwrap_or(0.0),
        }
    }
}

/// Manage notifications.
#[derive(Debug, Default)]
pub struct NotificationManager {
    notifications: Vec<Notification>,
}

impl NotificationManager {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add(&mut self, level: NotificationLevel, title: &str, message: &str) {
        self.notifications.push(Notification::new(level, title, message));
    }

    pub fn info(&mut self, title: &str, message: &str) {
        self.add(NotificationLevel::Info, title, message);
    }

    pub fn success(&mut self, title: &str, message: &str) {
        self.add(NotificationLevel::Success, title, message);
    }

    pub fn warning(&mut self, title: &str, message: &str) {
        self.add(NotificationLevel::Warning, title, message);
    }

    pub fn error(&mut self, title: &str, message: &str) {
        self.add(NotificationLevel::Error, title, message);
    }

    pub fn get_all(&self) -> &[Notification] {
        &self.notifications
    }

    pub fn get_by_level(&self, level: NotificationLevel) -> Vec<&Notification> {
        self.notifications.iter().filter(|n| n.level == level).collect()
    }

    pub fn clear(&mut self) {
        self.notifications.clear();
    }
}

/// Global notification manager.
static MANAGER: LazyLock<StdMutex<NotificationManager>> =
    LazyLock::new(|| StdMutex::new(NotificationManager::new()));

fn get_global_manager() -> &'static StdMutex<NotificationManager> {
    &MANAGER
}

/// Get the global notification manager.
pub fn get_notification_manager() -> std::sync::MutexGuard<'static, NotificationManager> {
    get_global_manager().lock().unwrap()
}

// ─── Webhook Notifier ────────────────────────────────────────────────────────

/// A Discord embed field.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscordField {
    pub name: String,
    pub value: String,
    #[serde(default)]
    pub inline: bool,
}

/// A Discord embed payload.
#[derive(Debug, Serialize, Deserialize)]
pub struct DiscordEmbed {
    pub title: String,
    pub description: String,
    pub color: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fields: Option<Vec<DiscordField>>,
}

/// A Discord payload.
#[derive(Debug, Serialize, Deserialize)]
pub struct DiscordPayload {
    pub embeds: Vec<DiscordEmbed>,
}

/// A Feishu text payload.
#[derive(Debug, Serialize, Deserialize)]
pub struct FeishuTextPayload {
    pub msg_type: String,
    pub content: FeishuTextContent,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FeishuTextContent {
    pub text: String,
}

/// A Feishu interactive card payload.
#[derive(Debug, Serialize, Deserialize)]
pub struct FeishuCardPayload {
    pub msg_type: String,
    pub card: FeishuCard,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FeishuCard {
    pub header: FeishuCardHeader,
    pub elements: Vec<serde_json::Value>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FeishuCardHeader {
    pub title: FeishuCardHeaderTitle,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FeishuCardHeaderTitle {
    pub tag: String,
    pub content: String,
}

/// A paper summary dict (arbitrary JSON value).
pub type PaperDict = serde_json::Map<String, serde_json::Value>;

/// Send notifications to external webhooks (Discord, Feishu, etc.).
#[derive(Debug, Clone)]
pub struct WebhookNotifier {
    webhook_url: String,
    client: reqwest::Client,
}

impl WebhookNotifier {
    pub fn new(webhook_url: &str) -> Self {
        Self {
            webhook_url: webhook_url.to_string(),
            client: reqwest::Client::new(),
        }
    }

    pub fn with_url(webhook_url: &str) -> Self {
        Self::new(webhook_url)
    }

    fn is_valid_url(&self) -> bool {
        if self.webhook_url.is_empty() {
            return false;
        }
        if let Ok(parsed) = url::Url::parse(&self.webhook_url) {
            parsed.scheme() == "http" || parsed.scheme() == "https"
        } else {
            false
        }
    }

    async fn send_json(&self, payload: &serde_json::Value) -> bool {
        if !self.is_valid_url() {
            return false;
        }
        match self
            .client
            .post(&self.webhook_url)
            .header("Content-Type", "application/json")
            .json(payload)
            .timeout(std::time::Duration::from_secs(10))
            .send()
            .await
        {
            Ok(resp) => resp.status().as_u16() == 200 || resp.status().as_u16() == 204,
            Err(_) => false,
        }
    }

    /// Send a Discord embed notification.
    pub async fn send_discord(
        &self,
        title: &str,
        description: &str,
        color: u32,
        fields: Option<Vec<DiscordField>>,
    ) -> bool {
        if self.webhook_url.is_empty() {
            return false;
        }
        let embed = DiscordEmbed {
            title: title.to_string(),
            description: description.to_string(),
            color,
            fields,
        };
        let payload = DiscordPayload { embeds: vec![embed] };
        self.send_json(&serde_json::to_value(&payload).unwrap_or_default()).await
    }

    /// Send a Feishu text or interactive notification.
    pub async fn send_feishu(&self, title: &str, content: &str, msg_type: &str) -> bool {
        if self.webhook_url.is_empty() {
            return false;
        }
        let payload = if msg_type == "interactive" {
            serde_json::to_value(FeishuCardPayload {
                msg_type: "interactive".to_string(),
                card: FeishuCard {
                    header: FeishuCardHeader {
                        title: FeishuCardHeaderTitle {
                            tag: "plain_text".to_string(),
                            content: title.to_string(),
                        },
                    },
                    elements: vec![serde_json::json!({
                        "tag": "div",
                        "text": {
                            "tag": "lark_md",
                            "content": content
                        }
                    })],
                },
            })
            .unwrap_or_default()
        } else {
            serde_json::to_value(FeishuTextPayload {
                msg_type: "text".to_string(),
                content: FeishuTextContent {
                    text: format!("{}\n{}", title, content),
                },
            })
            .unwrap_or_default()
        };
        self.send_json(&payload).await
    }

    /// Send notification for newly found papers. Returns number notified (max 5).
    pub async fn notify_papers_found(
        &self,
        subscription_topic: &str,
        papers: &[PaperDict],
        min_score: f64,
    ) -> usize {
        if papers.is_empty() {
            return 0;
        }
        let mut count = 0;
        for paper in papers.iter().take(5) {
            let score = paper
                .get("score")
                .and_then(|v| v.as_f64())
                .unwrap_or(0.0);
            if score < min_score {
                continue;
            }
            let title = paper
                .get("title")
                .and_then(|v| v.as_str())
                .unwrap_or("Untitled");
            let title = if title.len() > 200 { &title[..200] } else { title };
            let arxiv_id = paper
                .get("arxiv_id")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let url = if !arxiv_id.is_empty() {
                format!("https://arxiv.org/abs/{}", arxiv_id)
            } else {
                String::new()
            };

            // Try Discord
            let fields = if !arxiv_id.is_empty() {
                Some(vec![DiscordField {
                    name: "arXiv".to_string(),
                    value: format!("[{}]({})", arxiv_id, url),
                    inline: true,
                }])
            } else {
                None
            };
            let _ = self
                .send_discord(
                    &format!("📄 New Paper — {}", subscription_topic),
                    &format!("**{}**\nScore: {:.2}", title, score),
                    0x00FF00,
                    fields,
                )
                .await;

            // Try Feishu
            let _ = self
                .send_feishu(
                    &format!("📄 New Paper — {}", subscription_topic),
                    &format!("**{}**\nScore: {:.2}\n{}", title, score, url),
                    "text",
                )
                .await;

            count += 1;
        }
        count
    }
}

// ─── Global webhook notifier ─────────────────────────────────────────────────

static WEBHOOK_NOTIFIER: StdMutex<Option<WebhookNotifier>> = StdMutex::new(None);

/// Get the global webhook notifier.
pub fn get_webhook_notifier() -> Option<std::sync::MutexGuard<'static, Option<WebhookNotifier>>> {
    Some(WEBHOOK_NOTIFIER.lock().unwrap())
}

/// Configure the global webhook URL and return the notifier.
pub fn configure_webhook(url: &str) -> WebhookNotifier {
    let notifier = WebhookNotifier::new(url);
    let mut guard = WEBHOOK_NOTIFIER.lock().unwrap();
    *guard = Some(notifier.clone());
    notifier
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_notification_level_as_str() {
        assert_eq!(NotificationLevel::Info.as_str(), "info");
        assert_eq!(NotificationLevel::Success.as_str(), "success");
        assert_eq!(NotificationLevel::Warning.as_str(), "warning");
        assert_eq!(NotificationLevel::Error.as_str(), "error");
    }

    #[test]
    fn test_notification_new() {
        let n = Notification::new(NotificationLevel::Info, "title", "msg");
        assert_eq!(n.title, "title");
        assert_eq!(n.message, "msg");
        assert_eq!(n.level, NotificationLevel::Info);
        assert!(n.timestamp > 0.0);
    }

    #[test]
    fn test_notification_manager_add() {
        let mut mgr = NotificationManager::new();
        mgr.add(NotificationLevel::Info, "t", "m");
        assert_eq!(mgr.get_all().len(), 1);
        mgr.info("info title", "info msg");
        assert_eq!(mgr.get_all().len(), 2);
    }

    #[test]
    fn test_notification_manager_get_by_level() {
        let mut mgr = NotificationManager::new();
        mgr.info("a", "b");
        mgr.error("c", "d");
        mgr.info("e", "f");
        assert_eq!(mgr.get_by_level(NotificationLevel::Info).len(), 2);
        assert_eq!(mgr.get_by_level(NotificationLevel::Error).len(), 1);
    }

    #[test]
    fn test_notification_manager_clear() {
        let mut mgr = NotificationManager::new();
        mgr.info("a", "b");
        mgr.clear();
        assert!(mgr.get_all().is_empty());
    }

    #[test]
    fn test_get_notification_manager() {
        let mut mgr = get_notification_manager();
        mgr.info("test", "from global manager");
        assert_eq!(mgr.get_all().len(), 1);
    }

    #[test]
    fn test_webhook_notifier_valid_url() {
        let n = WebhookNotifier::new("https://discord.com/api/webhooks/123");
        assert!(n.is_valid_url());
    }

    #[test]
    fn test_webhook_notifier_invalid_url() {
        let n = WebhookNotifier::new("");
        assert!(!n.is_valid_url());
        let n2 = WebhookNotifier::new("ftp://example.com");
        assert!(!n2.is_valid_url());
    }

    #[test]
    fn test_configure_webhook() {
        let n = configure_webhook("https://example.com/webhook");
        assert!(n.is_valid_url());
    }

    #[tokio::test]
    async fn test_notify_papers_found_empty() {
        let n = WebhookNotifier::new("");
        let papers: Vec<PaperDict> = vec![];
        let count = n.notify_papers_found("topic", &papers, 0.5).await;
        assert_eq!(count, 0);
    }
}
