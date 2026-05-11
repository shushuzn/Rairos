//! Rairos Notify — Webhook notifications for Discord, Feishu, and generic platforms.
//!
//! Ported from `notifications/dispatcher.py`. Sends rich notifications to Discord
//! (embeds) and Feishu (interactive cards) via webhooks, with a generic JSON fallback.

use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::json;
use serde_json::Value;
use std::collections::HashMap;
use thiserror::Error;

// ============================================================================
// Error Types
// ============================================================================

#[derive(Error, Debug)]
pub enum NotifyError {
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("Webhook error: {0}")]
    Webhook(String),
    #[error("Invalid payload: {0}")]
    InvalidPayload(String),
}

pub type Result<T> = std::result::Result<T, NotifyError>;

// ============================================================================
// Enums
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
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
            NotificationType::GapAlert => "gap_alert",
            NotificationType::ParadigmShift => "paradigm_shift",
            NotificationType::PaperIngested => "paper_ingested",
            NotificationType::ResearchComplete => "research_complete",
            NotificationType::ContradictionDetected => "contradiction_detected",
            NotificationType::TopicSuggestion => "topic_suggestion",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Platform {
    Discord,
    Feishu,
    Generic,
}

impl Default for Platform {
    fn default() -> Self {
        Platform::Generic
    }
}

// ============================================================================
// Payloads
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GapAlertPayload {
    pub gap_type: String,
    pub title: String,
    pub novelty: f64,
    pub severity: String,
    #[serde(default)]
    pub supporting_papers: Vec<String>,
    #[serde(default = "default_source")]
    pub source: String,
    #[serde(default)]
    pub confidence: f64,
    #[serde(default)]
    pub impact_score: f64,
}

fn default_source() -> String {
    "deep_research".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ParadigmShiftPayload {
    pub alert_type: String,
    pub gap_type: String,
    pub message: String,
    pub severity: String,
    #[serde(default)]
    pub contradictions: Vec<ContradictionEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContradictionEntry {
    pub paper_a: String,
    pub paper_b: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PaperIngestedPayload {
    pub title: String,
    pub arxiv_id: String,
    #[serde(default)]
    pub tags: Vec<String>,
}

// ============================================================================
// Discord Renderer
// ============================================================================

pub struct DiscordRenderer;

impl DiscordRenderer {
    const SEVERITY_COLORS: &'static [( &'static str, u32 )] = &[
        ("high", 0xFF4444),
        ("medium", 0xFFAA00),
        ("low", 0x44FF44),
    ];

    const GAP_TYPE_COLORS: &'static [( &'static str, u32 )] = &[
        ("method_limitation", 0xCC88FF),
        ("scalability_issue", 0xFF8800),
        ("evaluation_gap", 0x88CCFF),
        ("contradiction", 0xFF4444),
        ("unexplored_application", 0x44FFAA),
        ("dataset_gap", 0xFFFF44),
    ];

    fn color_for(gap_type: &str, severity: &str) -> u32 {
        Self::SEVERITY_COLORS
            .iter()
            .find(|(s, _)| *s == severity)
            .map(|(_, c)| *c)
            .unwrap_or_else(|| {
                Self::GAP_TYPE_COLORS
                    .iter()
                    .find(|(t, _)| *t == gap_type.to_lowercase())
                    .map(|(_, c)| *c)
                    .unwrap_or(0x888888)
            })
    }

    pub fn render_gap_alert(payload: &GapAlertPayload) -> Value {
        let color = Self::color_for(&payload.gap_type, &payload.severity);
        let novelty_pct = (payload.novelty * 100.0) as i32;

        let mut fields: Vec<Value> = vec![
            json!({
                "name": "Gap Type",
                "value": Self::title_case(&payload.gap_type),
                "inline": true
            }),
            json!({
                "name": "Novelty",
                "value": format!("{}%", novelty_pct),
                "inline": true
            }),
        ];

        if payload.confidence > 0.0 {
            fields.push(json!({
                "name": "Confidence",
                "value": format!("{}%", (payload.confidence * 100.0) as i32),
                "inline": true
            }));
        }

        if payload.impact_score > 0.0 {
            fields.push(json!({
                "name": "Impact Score",
                "value": format!("{:.2}", payload.impact_score),
                "inline": true
            }));
        }

        if !payload.supporting_papers.is_empty() {
            let papers: Vec<&str> = payload.supporting_papers.iter().map(|s| s.as_str()).take(3).collect();
            let extra = payload.supporting_papers.len().saturating_sub(3);
            let mut papers_str = papers.join(", ");
            if extra > 0 {
                papers_str.push_str(&format!(" +{} more", extra));
            }
            fields.push(json!({
                "name": "Supporting Papers",
                "value": papers_str,
                "inline": false
            }));
        }

        let title = if payload.title.len() > 256 {
            payload.title[..256].to_string()
        } else {
            payload.title.clone()
        };

        let embed = json!({
            "title": format!("🔬 {}", title),
            "description": format!(
                "**{}** novelty gap discovered via **{}**",
                payload.severity.to_uppercase(),
                payload.source
            ),
            "color": color,
            "fields": fields,
            "footer": {
                "text": format!("Rairos Research Agent • {}", Utc::now().format("%Y-%m-%d %H:%M"))
            }
        });

        json!({ "embeds": [embed] })
    }

    pub fn render_paradigm_shift(payload: &ParadigmShiftPayload) -> Value {
        let icon = if payload.alert_type == "contradiction_cluster" { "⚠️" } else { "🔄" };
        let color = if payload.severity == "high" { 0xFF0000 } else { 0xFF8800 };

        let mut fields: Vec<Value> = vec![
            json!({
                "name": "Alert Type",
                "value": Self::title_case(&payload.alert_type),
                "inline": true
            }),
            json!({
                "name": "Severity",
                "value": payload.severity.to_uppercase(),
                "inline": true
            }),
        ];

        let description = if payload.message.len() > 2048 {
            payload.message[..2048].to_string()
        } else {
            payload.message.clone()
        };
        let footer_text = format!("Rairos Paradigm Watch • {}", Utc::now().format("%Y-%m-%d %H:%M"));

        let mut embed = json!({
            "title": format!("{} Paradigm Shift Signal: {}", icon, payload.gap_type),
            "description": description,
            "color": color,
            "fields": fields,
            "footer": {
                "text": footer_text
            }
        });

        if !payload.contradictions.is_empty() {
            let c = &payload.contradictions[0];
            let paper_a = if c.paper_a.len() > 32 {
                format!("{}...", &c.paper_a[..32])
            } else {
                c.paper_a.clone()
            };
            let paper_b = if c.paper_b.len() > 32 {
                format!("{}...", &c.paper_b[..32])
            } else {
                c.paper_b.clone()
            };
            fields.push(json!({
                "name": "Sample Contradiction",
                "value": format!("Paper A: `{}`\nPaper B: `{}`", paper_a, paper_b),
                "inline": false
            }));
            embed["fields"] = Value::Array(fields);
        }

        json!({ "embeds": [embed] })
    }

    pub fn render_paper_ingested(title: &str, arxiv_id: &str, tags: &[String]) -> Value {
        let title = if title.len() > 256 { &title[..256] } else { title };

        let mut embed = json!({
            "title": format!("📄 {}", title),
            "description": format!("**arXiv:** `{}`", arxiv_id),
            "color": 0x88CCFF,
            "fields": Value::Array(vec![]),
            "footer": {
                "text": format!("Rairos • {}", Utc::now().format("%Y-%m-%d %H:%M"))
            }
        });

        if !tags.is_empty() {
            let tags_str: Vec<String> = tags.iter().take(8).map(|t| format!("`{}`", t)).collect();
            embed["fields"] = json!([{
                "name": "Tags",
                "value": tags_str.join(" "),
                "inline": false
            }]);
        }

        json!({ "embeds": [embed] })
    }

    fn title_case(s: &str) -> String {
        s.split('_')
            .map(|word| {
                let mut chars = word.chars();
                match chars.next() {
                    None => String::new(),
                    Some(first) => first.to_uppercase().chain(chars).collect(),
                }
            })
            .collect::<Vec<_>>()
            .join(" ")
    }
}

// ============================================================================
// Feishu Renderer
// ============================================================================

pub struct FeishuRenderer;

impl FeishuRenderer {
    fn severity_emoji(severity: &str) -> &'static str {
        match severity {
            "high" => "🔴",
            "medium" => "🟡",
            "low" => "🟢",
            _ => "⚪",
        }
    }

    fn feishu_template(severity: &str) -> &'static str {
        match severity {
            "high" => "red",
            "medium" => "yellow",
            "low" => "green",
            _ => "grey",
        }
    }

    pub fn render_gap_alert(payload: &GapAlertPayload) -> Value {
        let novelty_pct = (payload.novelty * 100.0) as i32;
        let severity_emoji = Self::severity_emoji(&payload.severity);

        let mut elements: Vec<Value> = vec![
            json!({
                "tag": "markdown",
                "content": format!("**Gap Type:** {}", Self::title_case(&payload.gap_type))
            }),
            json!({
                "tag": "markdown",
                "content": format!(
                    "**Novelty:** {}% | **Severity:** {} **{}**",
                    novelty_pct,
                    severity_emoji,
                    payload.severity.to_uppercase()
                )
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
            let papers: Vec<String> = payload
                .supporting_papers
                .iter()
                .take(5)
                .map(|pid| format!("- `{}`", pid))
                .collect();
            elements.push(json!({
                "tag": "markdown",
                "content": format!("**Supporting Papers:**\n{}", papers.join("\n"))
            }));
        }

        elements.push(json!({
            "tag": "note",
            "elements": [{
                "tag": "plain_text",
                "content": format!("Source: {}", payload.source)
            }]
        }));

        let title = if payload.title.len() > 100 {
            payload.title[..100].to_string()
        } else {
            payload.title.clone()
        };

        json!({
            "msg_type": "interactive",
            "card": {
                "header": {
                    "title": {
                        "tag": "plain_text",
                        "content": format!("🔬 {}", title)
                    },
                    "template": Self::feishu_template(&payload.severity)
                },
                "elements": elements
            }
        })
    }

    pub fn render_paradigm_shift(payload: &ParadigmShiftPayload) -> Value {
        let icon = if payload.alert_type == "contradiction_cluster" { "⚠️" } else { "🔄" };
        let severity_emoji = Self::severity_emoji(&payload.severity);
        let template = if payload.severity == "high" { "red" } else { "yellow" };

        let message = if payload.message.len() > 2000 {
            payload.message[..2000].to_string()
        } else {
            payload.message.clone()
        };

        json!({
            "msg_type": "interactive",
            "card": {
                "header": {
                    "title": {
                        "tag": "plain_text",
                        "content": format!("{} Paradigm Shift: {}", icon, payload.gap_type)
                    },
                    "template": template
                },
                "elements": [
                    { "tag": "markdown", "content": message },
                    {
                        "tag": "markdown",
                        "content": format!(
                            "**Alert Type:** {}\n**Severity:** {} **{}**",
                            Self::title_case(&payload.alert_type),
                            severity_emoji,
                            payload.severity.to_uppercase()
                        )
                    }
                ]
            }
        })
    }

    fn title_case(s: &str) -> String {
        s.split('_')
            .map(|word| {
                let mut chars = word.chars();
                match chars.next() {
                    None => String::new(),
                    Some(first) => first.to_uppercase().chain(chars).collect(),
                }
            })
            .collect::<Vec<_>>()
            .join(" ")
    }
}

// ============================================================================
// Webhook Dispatcher
// ============================================================================

#[derive(Debug, Clone)]
pub struct WebhookDispatcher {
    pub webhook_url: String,
    pub platform: Platform,
    pub label: String,
    http_client: reqwest::Client,
}

impl WebhookDispatcher {
    const DEFAULT_TIMEOUT_SECS: u64 = 10;

    pub fn new(webhook_url: &str, platform: Platform, label: &str) -> Self {
        Self {
            webhook_url: webhook_url.to_string(),
            platform,
            label: label.to_string(),
            http_client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(Self::DEFAULT_TIMEOUT_SECS))
                .build()
                .expect("valid reqwest client"),
        }
    }

    pub fn discord(webhook_url: &str, label: &str) -> Self {
        Self::new(webhook_url, Platform::Discord, label)
    }

    pub fn feishu(webhook_url: &str, label: &str) -> Self {
        Self::new(webhook_url, Platform::Feishu, label)
    }

    pub fn generic(webhook_url: &str, label: &str) -> Self {
        Self::new(webhook_url, Platform::Generic, label)
    }

    async fn send_payload(&self, payload: Value) -> Result<()> {
        let resp = self
            .http_client
            .post(&self.webhook_url)
            .header("Content-Type", "application/json")
            .json(&payload)
            .send()
            .await?;

        let status = resp.status();
        if status.as_u16() == 200 || status.as_u16() == 204 {
            Ok(())
        } else {
            let body = resp.text().await.unwrap_or_default();
            Err(NotifyError::Webhook(format!(
                "POST to {} returned {}: {}",
                self.label,
                status.as_u16(),
                &body[..body.len().min(200)]
            )))
        }
    }

    /// Send a gap alert notification.
    pub async fn send_gap_alert(
        &self,
        gap_type: &str,
        title: &str,
        novelty: f64,
        severity: &str,
        supporting_papers: Option<&[String]>,
        source: Option<&str>,
        confidence: Option<f64>,
        impact_score: Option<f64>,
    ) -> Result<()> {
        let payload = GapAlertPayload {
            gap_type: gap_type.to_string(),
            title: title.to_string(),
            novelty,
            severity: severity.to_string(),
            supporting_papers: supporting_papers.map(|v| v.to_vec()).unwrap_or_default(),
            source: source.unwrap_or("deep_research").to_string(),
            confidence: confidence.unwrap_or(0.0),
            impact_score: impact_score.unwrap_or(0.0),
        };

        let rendered = match self.platform {
            Platform::Discord => DiscordRenderer::render_gap_alert(&payload),
            Platform::Feishu => FeishuRenderer::render_gap_alert(&payload),
            Platform::Generic => self.render_generic("gap_alert", &payload),
        };

        self.send_payload(rendered).await
    }

    /// Send a paradigm shift notification.
    pub async fn send_paradigm_shift(
        &self,
        alert_type: &str,
        gap_type: &str,
        message: &str,
        severity: &str,
        contradictions: Option<&[ContradictionEntry]>,
    ) -> Result<()> {
        let payload = ParadigmShiftPayload {
            alert_type: alert_type.to_string(),
            gap_type: gap_type.to_string(),
            message: message.to_string(),
            severity: severity.to_string(),
            contradictions: contradictions.map(|v| v.to_vec()).unwrap_or_default(),
        };

        let rendered = match self.platform {
            Platform::Discord => DiscordRenderer::render_paradigm_shift(&payload),
            Platform::Feishu => FeishuRenderer::render_paradigm_shift(&payload),
            Platform::Generic => self.render_generic("paradigm_shift", &payload),
        };

        self.send_payload(rendered).await
    }

    /// Send a paper ingested notification.
    pub async fn send_paper_ingested(
        &self,
        paper_title: &str,
        arxiv_id: &str,
        tags: Option<&[String]>,
    ) -> Result<()> {
        let rendered = match self.platform {
            Platform::Discord => DiscordRenderer::render_paper_ingested(paper_title, arxiv_id, tags.unwrap_or(&[])),
            Platform::Feishu | Platform::Generic => {
                self.render_generic("paper_ingested", &PaperIngestedPayload {
                    title: paper_title.to_string(),
                    arxiv_id: arxiv_id.to_string(),
                    tags: tags.map(|v| v.to_vec()).unwrap_or_default(),
                })
            }
        };

        self.send_payload(rendered).await
    }

    fn render_generic(&self, event_type: &str, data: &impl Serialize) -> Value {
        json!({
            "event": event_type,
            "timestamp": Utc::now().to_rfc3339(),
            "source": "Rairos",
            "data": data
        })
    }

    /// Send a test notification.
    pub async fn test(&self) -> Result<()> {
        self.send_gap_alert(
            "test",
            "Test notification from Rairos",
            0.5,
            "low",
            None,
            Some("webhook_test"),
            None,
            None,
        )
        .await
    }
}

// ============================================================================
// Notification Center (multi-destination)
// ============================================================================

#[derive(Debug, Clone, Default)]
pub struct NotificationCenter {
    dispatchers: Vec<WebhookDispatcher>,
}

impl NotificationCenter {
    pub fn new() -> Self {
        Self { dispatchers: Vec::new() }
    }

    pub fn add(&mut self, dispatcher: WebhookDispatcher) {
        self.dispatchers.push(dispatcher);
    }

    pub fn remove(&mut self, label: &str) -> bool {
        if let Some(pos) = self.dispatchers.iter().position(|d| d.label == label) {
            self.dispatchers.remove(pos);
            true
        } else {
            false
        }
    }

    /// Send a gap alert to all registered dispatchers.
    pub async fn send_gap_alert(
        &self,
        gap_type: &str,
        title: &str,
        novelty: f64,
        severity: &str,
        supporting_papers: Option<&[String]>,
        source: Option<&str>,
        confidence: Option<f64>,
        impact_score: Option<f64>,
    ) -> HashMap<String, std::result::Result<(), NotifyError>> {
        let mut results = HashMap::new();
        for d in &self.dispatchers {
            if d.webhook_url.is_empty() {
                continue;
            }
            let label = d.label.clone();
            results.insert(label, d.send_gap_alert(
                gap_type,
                title,
                novelty,
                severity,
                supporting_papers,
                source,
                confidence,
                impact_score,
            ).await);
        }
        results
    }

    /// Send a paradigm shift alert to all registered dispatchers.
    pub async fn send_paradigm_shift(
        &self,
        alert_type: &str,
        gap_type: &str,
        message: &str,
        severity: &str,
        contradictions: Option<&[ContradictionEntry]>,
    ) -> HashMap<String, std::result::Result<(), NotifyError>> {
        let mut results = HashMap::new();
        for d in &self.dispatchers {
            if d.webhook_url.is_empty() {
                continue;
            }
            let label = d.label.clone();
            results.insert(label, d.send_paradigm_shift(
                alert_type,
                gap_type,
                message,
                severity,
                contradictions,
            ).await);
        }
        results
    }

    /// Send a paper ingested notification to all registered dispatchers.
    pub async fn send_paper_ingested(
        &self,
        paper_title: &str,
        arxiv_id: &str,
        tags: Option<&[String]>,
    ) -> HashMap<String, std::result::Result<(), NotifyError>> {
        let mut results = HashMap::new();
        for d in &self.dispatchers {
            if d.webhook_url.is_empty() {
                continue;
            }
            let label = d.label.clone();
            results.insert(label, d.send_paper_ingested(paper_title, arxiv_id, tags).await);
        }
        results
    }

    /// Test all registered dispatchers.
    pub async fn test_all(&self) -> HashMap<String, std::result::Result<(), NotifyError>> {
        let mut results = HashMap::new();
        for d in &self.dispatchers {
            if d.webhook_url.is_empty() {
                continue;
            }
            let label = d.label.clone();
            results.insert(label, d.test().await);
        }
        results
    }
}

// ============================================================================
// Re-exports for convenience
// ============================================================================

pub use gap_alert::{GapAlertBuilder, GapAlertSender};
pub use paradigm_shift::{ParadigmShiftBuilder, ParadigmShiftSender};
pub use paper_ingested::{PaperIngestedBuilder, PaperIngestedSender};

pub mod gap_alert {
    use super::*;

    pub struct GapAlertSender<'a> {
        dispatcher: &'a WebhookDispatcher,
        gap_type: String,
        title: String,
        novelty: f64,
        severity: String,
        supporting_papers: Vec<String>,
        source: String,
        confidence: f64,
        impact_score: f64,
    }

    impl<'a> GapAlertSender<'a> {
        pub fn new(dispatcher: &'a WebhookDispatcher, gap_type: &str, title: &str, novelty: f64, severity: &str) -> Self {
            Self {
                dispatcher,
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

        pub fn supporting_papers(mut self, papers: Vec<String>) -> Self {
            self.supporting_papers = papers;
            self
        }

        pub fn source(mut self, source: &str) -> Self {
            self.source = source.to_string();
            self
        }

        pub fn confidence(mut self, confidence: f64) -> Self {
            self.confidence = confidence;
            self
        }

        pub fn impact_score(mut self, impact_score: f64) -> Self {
            self.impact_score = impact_score;
            self
        }

        pub async fn send(self) -> Result<()> {
            self.dispatcher.send_gap_alert(
                &self.gap_type,
                &self.title,
                self.novelty,
                &self.severity,
                Some(&self.supporting_papers),
                Some(&self.source),
                Some(self.confidence),
                Some(self.impact_score),
            ).await
        }
    }

    pub type GapAlertBuilder = GapAlertSender<'static>;

    pub fn gap_alert<'a>(dispatcher: &'a WebhookDispatcher, gap_type: &'a str, title: &'a str, novelty: f64, severity: &'a str) -> GapAlertSender<'a> {
        GapAlertSender::new(dispatcher, gap_type, title, novelty, severity)
    }
}

pub mod paradigm_shift {
    use super::*;

    pub struct ParadigmShiftSender<'a> {
        dispatcher: &'a WebhookDispatcher,
        alert_type: String,
        gap_type: String,
        message: String,
        severity: String,
        contradictions: Vec<ContradictionEntry>,
    }

    impl<'a> ParadigmShiftSender<'a> {
        pub fn new(dispatcher: &'a WebhookDispatcher, alert_type: &str, gap_type: &str, message: &str, severity: &str) -> Self {
            Self {
                dispatcher,
                alert_type: alert_type.to_string(),
                gap_type: gap_type.to_string(),
                message: message.to_string(),
                severity: severity.to_string(),
                contradictions: Vec::new(),
            }
        }

        pub fn contradictions(mut self, contradictions: Vec<ContradictionEntry>) -> Self {
            self.contradictions = contradictions;
            self
        }

        pub async fn send(self) -> Result<()> {
            self.dispatcher.send_paradigm_shift(
                &self.alert_type,
                &self.gap_type,
                &self.message,
                &self.severity,
                Some(&self.contradictions),
            ).await
        }
    }

    pub type ParadigmShiftBuilder = ParadigmShiftSender<'static>;

    pub fn paradigm_shift<'a>(dispatcher: &'a WebhookDispatcher, alert_type: &'a str, gap_type: &'a str, message: &'a str, severity: &'a str) -> ParadigmShiftSender<'a> {
        ParadigmShiftSender::new(dispatcher, alert_type, gap_type, message, severity)
    }
}

pub mod paper_ingested {
    use super::*;

    pub struct PaperIngestedSender<'a> {
        dispatcher: &'a WebhookDispatcher,
        title: String,
        arxiv_id: String,
        tags: Vec<String>,
    }

    impl<'a> PaperIngestedSender<'a> {
        pub fn new(dispatcher: &'a WebhookDispatcher, title: &str, arxiv_id: &str) -> Self {
            Self {
                dispatcher,
                title: title.to_string(),
                arxiv_id: arxiv_id.to_string(),
                tags: Vec::new(),
            }
        }

        pub fn tags(mut self, tags: Vec<String>) -> Self {
            self.tags = tags;
            self
        }

        pub async fn send(self) -> Result<()> {
            self.dispatcher.send_paper_ingested(
                &self.title,
                &self.arxiv_id,
                Some(&self.tags),
            ).await
        }
    }

    pub type PaperIngestedBuilder = PaperIngestedSender<'static>;

    pub fn paper_ingested<'a>(dispatcher: &'a WebhookDispatcher, title: &'a str, arxiv_id: &'a str) -> PaperIngestedSender<'a> {
        PaperIngestedSender::new(dispatcher, title, arxiv_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_discord_render_gap_alert() {
        let payload = GapAlertPayload {
            gap_type: "method_limitation".to_string(),
            title: "Attention scales poorly".to_string(),
            novelty: 0.85,
            severity: "high".to_string(),
            supporting_papers: vec!["paper1".to_string(), "paper2".to_string()],
            source: "deep_research".to_string(),
            confidence: 0.9,
            impact_score: 7.5,
        };

        let rendered = DiscordRenderer::render_gap_alert(&payload);
        assert!(rendered.get("embeds").is_some());
        let embeds = rendered["embeds"].as_array().unwrap();
        assert_eq!(embeds.len(), 1);
        let embed = &embeds[0];
        // Severity ("high") takes priority over gap_type per Python _color_for logic
        assert_eq!(embed["color"], 0xFF4444); // high severity = red
        assert!(embed["title"].as_str().unwrap().contains("Attention"));
    }

    #[test]
    fn test_feishu_render_gap_alert() {
        let payload = GapAlertPayload {
            gap_type: "scalability_issue".to_string(),
            title: "Scaling test".to_string(),
            novelty: 0.7,
            severity: "medium".to_string(),
            supporting_papers: vec![],
            source: "deep_research".to_string(),
            confidence: 0.0,
            impact_score: 0.0,
        };

        let rendered = FeishuRenderer::render_gap_alert(&payload);
        assert_eq!(rendered["msg_type"], "interactive");
        let header = &rendered["card"]["header"];
        assert_eq!(header["template"], "yellow");
    }

    #[test]
    fn test_generic_render() {
        let dispatcher = WebhookDispatcher::generic("http://example.com", "test");
        let payload = PaperIngestedPayload {
            title: "Test Paper".to_string(),
            arxiv_id: "1234.5678".to_string(),
            tags: vec!["ml".to_string()],
        };
        let rendered = dispatcher.render_generic("paper_ingested", &payload);
        assert_eq!(rendered["event"], "paper_ingested");
        assert_eq!(rendered["source"], "Rairos");
    }
}
