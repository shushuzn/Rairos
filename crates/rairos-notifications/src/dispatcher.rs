//! Rich Webhook Notifications — Discord embeds and Feishu markdown cards.
//!
//! Supports
//! - Discord: webhook embeds with color, fields, and author
//! - Feishu: markdown card with sections, tags, and code blocks
//! - Generic: JSON POST to any webhook URL

use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::time::Duration;

/// Notification event type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NotificationType {
    GapAlert,
    ParadigmShift,
    PaperIngested,
    ResearchComplete,
    ContradictionDetected,
    TopicSuggestion,
}

impl NotificationType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::GapAlert => "gap_alert",
            Self::ParadigmShift => "paradigm_shift",
            Self::PaperIngested => "paper_ingested",
            Self::ResearchComplete => "research_complete",
            Self::ContradictionDetected => "contradiction_detected",
            Self::TopicSuggestion => "topic_suggestion",
        }
    }
}

/// Target platform for a webhook.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Platform {
    Discord,
    Feishu,
    Generic,
}

/// Configuration for a webhook destination.
#[derive(Debug, Clone)]
pub struct WebhookConfig {
    pub url: String,
    pub platform: Platform,
    pub enabled: bool,
    pub label: String,
}

impl WebhookConfig {
    pub fn new(url: &str) -> Self {
        Self {
            url: url.to_string(),
            platform: Platform::Generic,
            enabled: true,
            label: String::new(),
        }
    }

    pub fn with_platform(mut self, platform: Platform) -> Self {
        self.platform = platform;
        self
    }

    pub fn with_label(mut self, label: &str) -> Self {
        self.label = label.to_string();
        self
    }

    pub fn is_valid(&self) -> bool {
        !self.url.is_empty()
    }
}

/// Payload for a gap alert notification.
#[derive(Debug, Clone)]
pub struct GapAlertPayload {
    pub gap_type: String,
    pub title: String,
    pub novelty: f64,
    pub severity: String,
    pub supporting_papers: Vec<String>,
    pub source: String,
    pub confidence: f64,
    pub impact_score: f64,
}

impl GapAlertPayload {
    pub fn new(gap_type: &str, title: &str, novelty: f64, severity: &str) -> Self {
        Self {
            gap_type: gap_type.to_string(),
            title: title.to_string(),
            novelty,
            severity: severity.to_string(),
            supporting_papers: Vec::new(),
            source: "deep_research".to_string(),
            confidence: 0.0,
            impact_score: 0.0,
        }
    }

    pub fn with_supporting_papers(mut self, papers: Vec<String>) -> Self {
        self.supporting_papers = papers;
        self
    }

    pub fn with_source(mut self, source: &str) -> Self {
        self.source = source.to_string();
        self
    }

    pub fn with_confidence(mut self, confidence: f64) -> Self {
        self.confidence = confidence;
        self
    }

    pub fn with_impact_score(mut self, score: f64) -> Self {
        self.impact_score = score;
        self
    }
}

/// Payload for a paradigm shift alert.
#[derive(Debug, Clone)]
pub struct ParadigmShiftPayload {
    pub alert_type: String,
    pub gap_type: String,
    pub message: String,
    pub severity: String,
    pub contradictions: Vec<serde_json::Value>,
}

impl ParadigmShiftPayload {
    pub fn new(alert_type: &str, gap_type: &str, message: &str, severity: &str) -> Self {
        Self {
            alert_type: alert_type.to_string(),
            gap_type: gap_type.to_string(),
            message: message.to_string(),
            severity: severity.to_string(),
            contradictions: Vec::new(),
        }
    }

    pub fn with_contradictions(mut self, contradictions: Vec<serde_json::Value>) -> Self {
        self.contradictions = contradictions;
        self
    }
}

// ─── Discord Renderer ─────────────────────────────────────────────────────────

/// Discord embed field.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscordField {
    pub name: String,
    pub value: String,
    #[serde(default)]
    pub inline: bool,
}

/// Discord embed.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscordEmbed {
    pub title: String,
    pub description: String,
    pub color: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fields: Option<Vec<DiscordField>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub footer: Option<DiscordFooter>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscordFooter {
    pub text: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct DiscordPayload {
    embeds: Vec<DiscordEmbed>,
}

struct DiscordRenderer;

impl DiscordRenderer {
    const SEVERITY_COLORS: &'static [( &'static str, u32)] = &[
        ("high", 0xFF4444),
        ("medium", 0xFFAA00),
        ("low", 0x44FF44),
    ];

    const GAP_TYPE_COLORS: &'static [( &'static str, u32)] = &[
        ("method_limitation", 0xCC88FF),
        ("scalability_issue", 0xFF8800),
        ("evaluation_gap", 0x88CCFF),
        ("contradiction", 0xFF4444),
        ("unexplored_application", 0x44FFAA),
        ("dataset_gap", 0xFFFF44),
    ];

    fn color_for(gap_type: &str, severity: &str) -> u32 {
        for (sev, color) in Self::SEVERITY_COLORS {
            if *sev == severity {
                return *color;
            }
        }
        for (gt, color) in Self::GAP_TYPE_COLORS {
            if *gt == gap_type.to_lowercase() {
                return *color;
            }
        }
        0x888888
    }

    fn timestamp() -> String {
        chrono::Local::now().format("%Y-%m-%d %H:%M").to_string()
    }

    pub fn render_gap_alert(payload: &GapAlertPayload) -> DiscordPayload {
        let color = Self::color_for(&payload.gap_type, &payload.severity);
        let novelty_pct = (payload.novelty * 100.0) as i32;

        let mut fields = vec![
            DiscordField {
                name: "Gap Type".to_string(),
                value: payload.gap_type.replace('_', " ").toUpperCase_initials(),
                inline: true,
            },
            DiscordField {
                name: "Novelty".to_string(),
                value: format!("{novelty_pct}%"),
                inline: true,
            },
        ];

        if payload.confidence > 0.0 {
            fields.push(DiscordField {
                name: "Confidence".to_string(),
                value: format!("{}%", (payload.confidence * 100.0) as i32),
                inline: true,
            });
        }

        if payload.impact_score > 0.0 {
            fields.push(DiscordField {
                name: "Impact Score".to_string(),
                value: format!("{:.2}", payload.impact_score),
                inline: true,
            });
        }

        if !payload.supporting_papers.is_empty() {
            let papers_str = if payload.supporting_papers.len() > 3 {
                let first3 = payload.supporting_papers[..3].join(", ");
                format!("{} +{} more", first3, payload.supporting_papers.len() - 3)
            } else {
                payload.supporting_papers.join(", ")
            };
            fields.push(DiscordField {
                name: "Supporting Papers".to_string(),
                value: papers_str,
                inline: false,
            });
        }

        let title = if payload.title.len() > 256 {
            payload.title[..256].to_string()
        } else {
            payload.title.clone()
        };

        let description = format!(
            "**{}** novelty gap discovered via **{}**",
            payload.severity.to_uppercase(),
            payload.source
        );

        let embed = DiscordEmbed {
            title: format!("🔬 {}", title),
            description,
            color,
            fields: Some(fields),
            footer: Some(DiscordFooter {
                text: format!("Rairos Research Agent • {}", Self::timestamp()),
            }),
        };

        DiscordPayload { embeds: vec![embed] }
    }

    pub fn render_paradigm_shift(payload: &ParadigmShiftPayload) -> DiscordPayload {
        let icon = if payload.alert_type == "contradiction_cluster" {
            "⚠️"
        } else {
            "🔄"
        };
        let color = if payload.severity == "high" {
            0xFF0000
        } else {
            0xFF8800
        };

        let fields = vec![
            DiscordField {
                name: "Alert Type".to_string(),
                value: payload.alert_type.replace('_', " ").toUppercase_initials(),
                inline: true,
            },
            DiscordField {
                name: "Severity".to_string(),
                value: payload.severity.to_uppercase(),
                inline: true,
            },
        ];

        let description = if payload.message.len() > 2048 {
            payload.message[..2048].to_string()
        } else {
            payload.message.clone()
        };

        let mut embed = DiscordEmbed {
            title: format!("{} Paradigm Shift Signal: {}", icon, payload.gap_type),
            description,
            color,
            fields: Some(fields),
            footer: Some(DiscordFooter {
                text: format!("Rairos Paradigm Watch • {}", Self::timestamp()),
            }),
        };

        if !payload.contradictions.is_empty() {
            if let Some(c) = payload.contradictions.first().as_ref() {
                let paper_a = c
                    .get("paper_a")
                    .and_then(|v| v.as_str())
                    .unwrap_or("?");
                let paper_b = c
                    .get("paper_b")
                    .and_then(|v| v.as_str())
                    .unwrap_or("?");
                let paper_a_short = if paper_a.len() > 32 {
                    &paper_a[..32]
                } else {
                    paper_a
                };
                let paper_b_short = if paper_b.len() > 32 {
                    &paper_b[..32]
                } else {
                    paper_b
                };

                embed.fields.as_mut().unwrap().push(DiscordField {
                    name: "Sample Contradiction".to_string(),
                    value: format!("Paper A: `{}`\nPaper B: `{}`", paper_a_short, paper_b_short),
                    inline: false,
                });
            }
        }

        DiscordPayload { embeds: vec![embed] }
    }

    pub fn render_paper_ingested(title: &str, arxiv_id: &str, tags: &[String]) -> DiscordPayload {
        let title_short = if title.len() > 256 {
            title[..256].to_string()
        } else {
            title.to_string()
        };

        let mut fields: Vec<DiscordField> = vec![DiscordField {
            name: "arXiv".to_string(),
            value: format!("`{}`", arxiv_id),
            inline: true,
        }];

        if !tags.is_empty() {
            let tags_str = tags
                .iter()
                .take(8)
                .map(|t| format!("`{}`", t))
                .collect::<Vec<_>>()
                .join(" ");
            fields.push(DiscordField {
                name: "Tags".to_string(),
                value: tags_str,
                inline: false,
            });
        }

        let embed = DiscordEmbed {
            title: format!("📄 {}", title_short),
            description: format!("**arXiv:** `{}`", arxiv_id),
            color: 0x88CCFF,
            fields: Some(fields),
            footer: Some(DiscordFooter {
                text: format!("Rairos • {}", Self::timestamp()),
            }),
        };

        DiscordPayload { embeds: vec![embed] }
    }
}

// ─── Feishu Renderer ──────────────────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize)]
struct FeishuMarkdownElement {
    tag: String,
    content: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct FeishuNoteElement {
    tag: String,
    elements: Vec<FeishuPlainText>,
}

#[derive(Debug, Serialize, Deserialize)]
struct FeishuPlainText {
    tag: String,
    content: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct FeishuCardHeader {
    title: FeishuPlainText,
    #[serde(rename = "template")]
    template: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct FeishuCard {
    header: FeishuCardHeader,
    elements: Vec<serde_json::Value>,
}

#[derive(Debug, Serialize, Deserialize)]
struct FeishuCardPayload {
    #[serde(rename = "msg_type")]
    msg_type: String,
    card: FeishuCard,
}

struct FeishuRenderer;

impl FeishuRenderer {
    fn severity_tag(severity: &str) -> String {
        let emoji = match severity {
            "high" => "🔴",
            "medium" => "🟡",
            "low" => "🟢",
            _ => "⚪",
        };
        format!("{} **{}**", emoji, severity.to_uppercase())
    }

    fn feishu_template(severity: &str) -> &'static str {
        match severity {
            "high" => "red",
            "medium" => "yellow",
            "low" => "green",
            _ => "grey",
        }
    }

    pub fn render_gap_alert(payload: &GapAlertPayload) -> FeishuCardPayload {
        let novelty_pct = (payload.novelty * 100.0) as i32;

        let mut elements: Vec<serde_json::Value> = vec![
            json!({
                "tag": "markdown",
                "content": format!("**Gap Type:** {}", payload.gap_type.replace('_', " ").toUppercase_initials())
            }),
            json!({
                "tag": "markdown",
                "content": format!("**Novelty:** {}% | **Severity:** {}", novelty_pct, Self::severity_tag(&payload.severity))
            }),
        ];

        if payload.confidence > 0.0 {
            elements.push(json!({
                "tag": "markdown",
                "content": format!("**Confidence:** {}%", (payload.confidence * 100.0) as i32)
            }));
        }

        if payload.impact_score > 0.0 {
            elements.push(json!({
                "tag": "markdown",
                "content": format!("**Impact Score:** {:.2}", payload.impact_score)
            }));
        }

        if !payload.supporting_papers.is_empty() {
            let papers_md = payload
                .supporting_papers
                .iter()
                .take(5)
                .map(|pid| format!("- `{}`", pid))
                .collect::<Vec<_>>()
                .join("\n");
            elements.push(json!({
                "tag": "markdown",
                "content": format!("**Supporting Papers:**\n{}", papers_md)
            }));
        }

        elements.push(json!({
            "tag": "note",
            "elements": [{"tag": "plain_text", "content": format!("Source: {}", payload.source)}]
        }));

        let title = if payload.title.len() > 100 {
            payload.title[..100].to_string()
        } else {
            payload.title.clone()
        };

        FeishuCardPayload {
            msg_type: "interactive".to_string(),
            card: FeishuCard {
                header: FeishuCardHeader {
                    title: FeishuPlainText {
                        tag: "plain_text".to_string(),
                        content: format!("🔬 {}", title),
                    },
                    template: Self::feishu_template(&payload.severity).to_string(),
                },
                elements,
            },
        }
    }

    pub fn render_paradigm_shift(payload: &ParadigmShiftPayload) -> FeishuCardPayload {
        let icon = if payload.alert_type == "contradiction_cluster" {
            "⚠️"
        } else {
            "🔄"
        };

        let content = if payload.message.len() > 2000 {
            payload.message[..2000].to_string()
        } else {
            payload.message.clone()
        };

        FeishuCardPayload {
            msg_type: "interactive".to_string(),
            card: FeishuCard {
                header: FeishuCardHeader {
                    title: FeishuPlainText {
                        tag: "plain_text".to_string(),
                        content: format!("{} Paradigm Shift: {}", icon, payload.gap_type),
                    },
                    template: if payload.severity == "high" {
                        "red".to_string()
                    } else {
                        "yellow".to_string()
                    },
                },
                elements: vec![
                    json!({
                        "tag": "markdown",
                        "content": content
                    }),
                    json!({
                        "tag": "markdown",
                        "content": format!("**Alert Type:** {}\n**Severity:** {}", payload.alert_type.replace('_', " ").toUppercase_initials(), Self::severity_tag(&payload.severity))
                    }),
                ],
            },
        }
    }
}

// ─── Webhook Dispatcher ───────────────────────────────────────────────────────

const DEFAULT_TIMEOUT_SECS: u64 = 10;

/// Send rich notifications to Discord, Feishu, or generic webhooks.
#[derive(Debug, Clone)]
pub struct WebhookDispatcher {
    webhook_url: String,
    platform: Platform,
    label: String,
    client: Client,
}

impl WebhookDispatcher {
    pub fn new(webhook_url: &str) -> Self {
        Self {
            webhook_url: webhook_url.to_string(),
            platform: Platform::Generic,
            label: webhook_url
                .split('/')
                .last()
                .unwrap_or("generic")
                .to_string(),
            client: Client::builder()
                .timeout(Duration::from_secs(DEFAULT_TIMEOUT_SECS))
                .build()
                .unwrap_or_default(),
        }
    }

    pub fn with_platform(mut self, platform: Platform) -> Self {
        self.platform = platform;
        if self.label.is_empty() {
            self.label = match platform {
                Platform::Discord => "discord".to_string(),
                Platform::Feishu => "feishu".to_string(),
                Platform::Generic => "generic".to_string(),
            };
        }
        self
    }

    pub fn with_label(mut self, label: &str) -> Self {
        self.label = label.to_string();
        self
    }

    fn is_valid_url(&self) -> bool {
        if self.webhook_url.is_empty() {
            return false;
        }
        self.webhook_url.starts_with("http://") || self.webhook_url.starts_with("https://")
    }

    async fn send_json(&self, payload: &serde_json::Value) -> bool {
        if !self.is_valid_url() {
            tracing::warn!("Invalid webhook URL for {}: {}", self.label, self.webhook_url);
            return false;
        }
        match self
            .client
            .post(&self.webhook_url)
            .header("Content-Type", "application/json")
            .json(payload)
            .send()
            .await
        {
            Ok(resp) => {
                if resp.status().as_u16() == 200 || resp.status().as_u16() == 204 {
                    true
                } else {
                    tracing::warn!(
                        "Webhook POST to {} returned {}: {:?}",
                        self.label,
                        resp.status().as_u16(),
                        resp.text().await.unwrap_or_default()
                    );
                    false
                }
            }
            Err(e) => {
                tracing::warn!("Webhook error for {}: {}", self.label, e);
                false
            }
        }
    }

    fn render_generic(&self, event_type: &str, data: serde_json::Value) -> serde_json::Value {
        json!({
            "event": event_type,
            "timestamp": chrono::Utc::now().to_rfc3339(),
            "source": "Rairos",
            "data": data,
        })
    }

    /// Send a gap alert notification.
    pub async fn send_gap_alert(&self, payload: &GapAlertPayload) -> bool {
        let rendered = match self.platform {
            Platform::Discord => {
                serde_json::to_value(DiscordRenderer::render_gap_alert(payload)).unwrap_or_default()
            }
            Platform::Feishu => {
                serde_json::to_value(FeishuRenderer::render_gap_alert(payload)).unwrap_or_default()
            }
            Platform::Generic => self.render_generic(
                "gap_alert",
                json!({
                    "gap_type": payload.gap_type,
                    "title": payload.title,
                    "novelty": payload.novelty,
                    "severity": payload.severity,
                    "supporting_papers": payload.supporting_papers,
                    "source": payload.source,
                    "confidence": payload.confidence,
                    "impact_score": payload.impact_score,
                }),
            ),
        };
        self.send_json(&rendered).await
    }

    /// Send a paradigm shift alert notification.
    pub async fn send_paradigm_shift(&self, payload: &ParadigmShiftPayload) -> bool {
        let rendered = match self.platform {
            Platform::Discord => serde_json::to_value(DiscordRenderer::render_paradigm_shift(payload))
                .unwrap_or_default(),
            Platform::Feishu => {
                serde_json::to_value(FeishuRenderer::render_paradigm_shift(payload))
                    .unwrap_or_default()
            }
            Platform::Generic => self.render_generic(
                "paradigm_shift",
                json!({
                    "alert_type": payload.alert_type,
                    "gap_type": payload.gap_type,
                    "message": payload.message,
                    "severity": payload.severity,
                    "contradictions": payload.contradictions,
                }),
            ),
        };
        self.send_json(&rendered).await
    }

    /// Send a paper ingested notification.
    pub async fn send_paper_ingested(&self, title: &str, arxiv_id: &str, tags: &[String]) -> bool {
        let rendered = match self.platform {
            Platform::Discord => {
                serde_json::to_value(DiscordRenderer::render_paper_ingested(title, arxiv_id, tags))
                    .unwrap_or_default()
            }
            Platform::Feishu | Platform::Generic => {
                self.render_generic(
                    "paper_ingested",
                    json!({
                        "title": title,
                        "arxiv_id": arxiv_id,
                        "tags": tags,
                    }),
                )
            }
        };
        self.send_json(&rendered).await
    }

    /// Test if the webhook is reachable (returns false for invalid URLs).
    pub async fn test(&self) -> bool {
        if !self.is_valid_url() {
            return false;
        }
        self.send_json(&serde_json::json!({"content": "Rairos webhook test"})).await
    }
}

// ─── Helper for title case ────────────────────────────────────────────────────

trait UpperCaseInitials {
    fn toUppercase_initials(&self) -> String;
}

impl UpperCaseInitials for str {
    fn toUppercase_initials(&self) -> String {
        let mut result = String::new();
        let mut capitalize_next = true;
        for c in self.chars() {
            if c == ' ' || c == '_' || c == '-' {
                result.push(c);
                capitalize_next = true;
            } else if capitalize_next {
                result.extend(c.to_uppercase());
                capitalize_next = false;
            } else {
                result.push(c);
            }
        }
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_webhook_config_new() {
        let cfg = WebhookConfig::new("https://discord.com/webhook/123");
        assert!(cfg.is_valid());
        assert_eq!(cfg.platform, Platform::Generic);
        assert!(cfg.enabled);
    }

    #[test]
    fn test_webhook_config_with_platform() {
        let cfg = WebhookConfig::new("https://discord.com/webhook/123")
            .with_platform(Platform::Discord);
        assert_eq!(cfg.platform, Platform::Discord);
    }

    #[test]
    fn test_webhook_config_invalid() {
        let cfg = WebhookConfig::new("");
        assert!(!cfg.is_valid());
    }

    #[test]
    fn test_gap_alert_payload_builder() {
        let payload = GapAlertPayload::new("method_limitation", "Test Gap", 0.85, "high")
            .with_source("deep_research")
            .with_confidence(0.9)
            .with_impact_score(7.5)
            .with_supporting_papers(vec!["paper1".to_string(), "paper2".to_string()]);

        assert_eq!(payload.gap_type, "method_limitation");
        assert_eq!(payload.novelty, 0.85);
        assert_eq!(payload.severity, "high");
        assert_eq!(payload.supporting_papers.len(), 2);
        assert_eq!(payload.source, "deep_research");
        assert_eq!(payload.confidence, 0.9);
        assert_eq!(payload.impact_score, 7.5);
    }

    #[test]
    fn test_gap_alert_payload_default() {
        let payload = GapAlertPayload::new("scalability_issue", "Scaling Problem", 0.5, "medium");
        assert_eq!(payload.source, "deep_research");
        assert!(payload.supporting_papers.is_empty());
    }

    #[test]
    fn test_paradigm_shift_payload_builder() {
        let contradictions = vec![
            json!({"paper_a": "paper1", "paper_b": "paper2"})
        ];
        let payload = ParadigmShiftPayload::new(
            "contradiction_cluster",
            "RLScaling",
            "Contradiction detected",
            "high",
        )
        .with_contradictions(contradictions.clone());

        assert_eq!(payload.alert_type, "contradiction_cluster");
        assert_eq!(payload.gap_type, "RLScaling");
        assert_eq!(payload.contradictions.len(), 1);
    }

    #[test]
    fn test_notification_type_as_str() {
        assert_eq!(NotificationType::GapAlert.as_str(), "gap_alert");
        assert_eq!(NotificationType::ParadigmShift.as_str(), "paradigm_shift");
        assert_eq!(NotificationType::PaperIngested.as_str(), "paper_ingested");
    }

    #[test]
    fn test_platform_default() {
        let d = WebhookDispatcher::new("https://example.com/webhook");
        assert_eq!(d.platform, Platform::Generic);
        assert!(d.is_valid_url());
    }

    #[test]
    fn test_dispatcher_invalid_url() {
        let d = WebhookDispatcher::new("");
        assert!(!d.is_valid_url());
    }

    #[test]
    fn test_dispatcher_with_platform() {
        let d = WebhookDispatcher::new("https://discord.com/webhook/123")
            .with_platform(Platform::Discord);
        assert_eq!(d.platform, Platform::Discord);
    }

    #[test]
    fn test_dispatcher_with_label() {
        let d = WebhookDispatcher::new("https://discord.com/webhook/123")
            .with_label("my-discord");
        assert_eq!(d.label, "my-discord");
    }

    #[test]
    fn test_dispatcher_discord_platform_label() {
        let d = WebhookDispatcher::new("https://discord.com/webhook/123")
            .with_platform(Platform::Discord);
        assert_eq!(d.label, "discord");
    }

    #[test]
    fn test_uppercase_initials() {
        assert_eq!("method_limitation".toUppercase_initials(), "Method Limitation");
        assert_eq!("scalability_issue".toUppercase_initials(), "Scalability Issue");
        assert_eq!("rl_scaling".toUppercase_initials(), "Rl Scaling");
    }

    #[test]
    fn test_discord_renderer_gap_alert() {
        let payload = GapAlertPayload::new("method_limitation", "Test Gap", 0.85, "high")
            .with_source("deep_research");
        let discord = DiscordRenderer::render_gap_alert(&payload);
        assert_eq!(discord.embeds.len(), 1);
        let embed = &discord.embeds[0];
        assert!(embed.title.contains("Test Gap"));
        assert_eq!(embed.color, 0xCC88FF); // method_limitation color
    }

    #[test]
    fn test_discord_renderer_gap_alert_high_severity() {
        let payload = GapAlertPayload::new("unexplored", "Test", 0.9, "high");
        let discord = DiscordRenderer::render_gap_alert(&payload);
        assert_eq!(discord.embeds[0].color, 0xFF4444); // high severity red
    }

    #[test]
    fn test_discord_renderer_gap_alert_medium_severity() {
        let payload = GapAlertPayload::new("dataset_gap", "Test", 0.5, "medium");
        let discord = DiscordRenderer::render_gap_alert(&payload);
        assert_eq!(discord.embeds[0].color, 0xFFAA00); // medium severity orange
    }

    #[test]
    fn test_discord_renderer_gap_alert_low_severity() {
        let payload = GapAlertPayload::new("dataset_gap", "Test", 0.5, "low");
        let discord = DiscordRenderer::render_gap_alert(&payload);
        assert_eq!(discord.embeds[0].color, 0x44FF44); // low severity green
    }

    #[test]
    fn test_discord_renderer_gap_alert_with_confidence() {
        let payload = GapAlertPayload::new("evaluation_gap", "Test", 0.7, "medium")
            .with_confidence(0.8);
        let discord = DiscordRenderer::render_gap_alert(&payload);
        let fields = discord.embeds[0].fields.as_ref().unwrap();
        assert!(fields.iter().any(|f| f.name == "Confidence"));
    }

    #[test]
    fn test_discord_renderer_gap_alert_with_impact_score() {
        let payload = GapAlertPayload::new("evaluation_gap", "Test", 0.7, "medium")
            .with_impact_score(5.5);
        let discord = DiscordRenderer::render_gap_alert(&payload);
        let fields = discord.embeds[0].fields.as_ref().unwrap();
        assert!(fields.iter().any(|f| f.name == "Impact Score"));
    }

    #[test]
    fn test_discord_renderer_gap_alert_with_supporting_papers() {
        let payload = GapAlertPayload::new("evaluation_gap", "Test", 0.7, "medium")
            .with_supporting_papers(vec!["paper1".to_string(), "paper2".to_string(), "paper3".to_string(), "paper4".to_string()]);
        let discord = DiscordRenderer::render_gap_alert(&payload);
        let fields = discord.embeds[0].fields.as_ref().unwrap();
        let papers_field = fields.iter().find(|f| f.name == "Supporting Papers").unwrap();
        assert!(papers_field.value.contains("paper1"));
        assert!(papers_field.value.contains("+1 more"));
    }

    #[test]
    fn test_discord_renderer_paradigm_shift_contradiction_cluster() {
        let payload = ParadigmShiftPayload::new(
            "contradiction_cluster",
            "RLScaling",
            "Multiple papers contradict each other",
            "high",
        );
        let discord = DiscordRenderer::render_paradigm_shift(&payload);
        assert!(discord.embeds[0].title.contains("⚠️"));
        assert_eq!(discord.embeds[0].color, 0xFF0000);
    }

    #[test]
    fn test_discord_renderer_paradigm_shift_polarity_reversal() {
        let payload = ParadigmShiftPayload::new(
            "polarity_reversal",
            "ScalingLaws",
            "Evidence shows opposite scaling behavior",
            "medium",
        );
        let discord = DiscordRenderer::render_paradigm_shift(&payload);
        assert!(discord.embeds[0].title.contains("🔄"));
        assert_eq!(discord.embeds[0].color, 0xFF8800);
    }

    #[test]
    fn test_discord_renderer_paradigm_shift_with_contradictions() {
        let contradictions = vec![
            json!({"paper_a": "paper_one_long_name_123", "paper_b": "paper_two_long_name_456"})
        ];
        let payload = ParadigmShiftPayload::new(
            "contradiction_cluster",
            "Scaling",
            "Contradiction found",
            "high",
        )
        .with_contradictions(contradictions);
        let discord = DiscordRenderer::render_paradigm_shift(&payload);
        let fields = discord.embeds[0].fields.as_ref().unwrap();
        let sample = fields.iter().find(|f| f.name == "Sample Contradiction").unwrap();
        assert!(sample.value.contains("paper_one_long")); // truncated to 32
    }

    #[test]
    fn test_discord_renderer_paper_ingested() {
        let discord = DiscordRenderer::render_paper_ingested(
            "Attention Is All You Need",
            "1706.03762",
            &["LLM".to_string(), "Transformer".to_string()],
        );
        assert_eq!(discord.embeds.len(), 1);
        let embed = &discord.embeds[0];
        assert!(embed.title.contains("Attention Is All You Need"));
        assert!(embed.description.contains("1706.03762"));
        assert!(embed.fields.as_ref().unwrap().iter().any(|f| f.name == "Tags"));
    }

    #[test]
    fn test_feishu_renderer_gap_alert() {
        let payload = GapAlertPayload::new("method_limitation", "Test Gap", 0.85, "high");
        let feishu = FeishuRenderer::render_gap_alert(&payload);
        assert_eq!(feishu.msg_type, "interactive");
        assert_eq!(feishu.card.header.template, "red");
        assert!(!feishu.card.elements.is_empty());
    }

    #[test]
    fn test_feishu_renderer_gap_alert_severity_templates() {
        let high = FeishuRenderer::render_gap_alert(
            &GapAlertPayload::new("x", "y", 0.5, "high")
        );
        assert_eq!(high.card.header.template, "red");

        let medium = FeishuRenderer::render_gap_alert(
            &GapAlertPayload::new("x", "y", 0.5, "medium")
        );
        assert_eq!(medium.card.header.template, "yellow");

        let low = FeishuRenderer::render_gap_alert(
            &GapAlertPayload::new("x", "y", 0.5, "low")
        );
        assert_eq!(low.card.header.template, "green");
    }

    #[test]
    fn test_feishu_renderer_paradigm_shift() {
        let payload = ParadigmShiftPayload::new(
            "contradiction_cluster",
            "Scaling",
            "A major contradiction was found",
            "high",
        );
        let feishu = FeishuRenderer::render_paradigm_shift(&payload);
        assert_eq!(feishu.msg_type, "interactive");
        assert_eq!(feishu.card.header.template, "red");
    }

    #[test]
    fn test_feishu_renderer_paradigm_shift_medium() {
        let payload = ParadigmShiftPayload::new(
            "polarity_reversal",
            "Scaling",
            "Polarity reversed",
            "medium",
        );
        let feishu = FeishuRenderer::render_paradigm_shift(&payload);
        assert_eq!(feishu.card.header.template, "yellow");
    }

    #[tokio::test]
    async fn test_dispatcher_test_invalid_url() {
        let d = WebhookDispatcher::new("");
        assert!(!d.test().await);
    }

    #[tokio::test]
    async fn test_dispatcher_send_gap_alert_empty_url() {
        let d = WebhookDispatcher::new("");
        let payload = GapAlertPayload::new("test", "test", 0.5, "low");
        assert!(!d.send_gap_alert(&payload).await);
    }

    #[tokio::test]
    async fn test_dispatcher_send_paradigm_shift_empty_url() {
        let d = WebhookDispatcher::new("");
        let payload = ParadigmShiftPayload::new("x", "y", "msg", "low");
        assert!(!d.send_paradigm_shift(&payload).await);
    }

    #[tokio::test]
    async fn test_dispatcher_send_paper_ingested_empty_url() {
        let d = WebhookDispatcher::new("");
        assert!(!d.send_paper_ingested("Title", "1234.5678", &[]).await);
    }

    #[test]
    fn test_feishu_severity_tag() {
        assert!(FeishuRenderer::severity_tag("high").contains("🔴"));
        assert!(FeishuRenderer::severity_tag("medium").contains("🟡"));
        assert!(FeishuRenderer::severity_tag("low").contains("🟢"));
        assert!(FeishuRenderer::severity_tag("unknown").contains("⚪"));
    }

    #[test]
    fn test_discord_renderer_gap_alert_truncates_title() {
        let long_title = "A".repeat(300);
        let payload = GapAlertPayload::new("x", &long_title, 0.5, "low");
        let discord = DiscordRenderer::render_gap_alert(&payload);
        // Discord title should be truncated to 256 chars
        assert!(discord.embeds[0].title.len() <= 256 + 2); // +2 for emoji prefix "🔬 "
    }

    #[test]
    fn test_discord_renderer_paper_ingested_no_tags() {
        let discord = DiscordRenderer::render_paper_ingested("Test Paper", "1234.5678", &[]);
        // Should have arXiv field but no tags field
        let fields = discord.embeds[0].fields.as_ref().unwrap();
        assert!(!fields.iter().any(|f| f.name == "Tags"));
    }
}
