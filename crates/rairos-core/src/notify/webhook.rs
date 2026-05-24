use chrono::Utc;
use serde::Serialize;
use serde_json::{json, Value};

use crate::notify::types::{NotifyError, Platform, Result};
use crate::notify::payloads::{GapAlertPayload, PaperIngestedPayload, ParadigmShiftPayload};
use crate::notify::discord::DiscordRenderer;
use crate::notify::feishu::FeishuRenderer;

#[derive(Debug, Clone)]
pub struct GapAlertParams<'a> {
    pub gap_type: &'a str,
    pub title: &'a str,
    pub novelty: f64,
    pub severity: &'a str,
    pub supporting_papers: Option<&'a [String]>,
    pub source: Option<&'a str>,
    pub confidence: Option<f64>,
    pub impact_score: Option<f64>,
}

impl<'a> GapAlertParams<'a> {
    pub fn to_payload(&self) -> GapAlertPayload {
        GapAlertPayload {
            gap_type: self.gap_type.to_string(),
            title: self.title.to_string(),
            novelty: self.novelty,
            severity: self.severity.to_string(),
            supporting_papers: self.supporting_papers.map(|v| v.to_vec()).unwrap_or_default(),
            source: self.source.unwrap_or("deep_research").to_string(),
            confidence: self.confidence.unwrap_or(0.0),
            impact_score: self.impact_score.unwrap_or(0.0),
        }
    }
}

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

    pub async fn send_gap_alert(&self, params: GapAlertParams<'_>) -> Result<()> {
        let payload = params.to_payload();

        let rendered = match self.platform {
            Platform::Discord => DiscordRenderer::render_gap_alert(&payload),
            Platform::Feishu => FeishuRenderer::render_gap_alert(&payload),
            Platform::Generic => self.render_generic("gap_alert", &payload),
        };

        self.send_payload(rendered).await
    }

    pub async fn send_paradigm_shift(
        &self,
        alert_type: &str,
        gap_type: &str,
        message: &str,
        severity: &str,
        contradictions: Option<&[crate::notify::payloads::ContradictionEntry]>,
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

    pub async fn send_paper_ingested(
        &self,
        paper_title: &str,
        arxiv_id: &str,
        tags: Option<&[String]>,
    ) -> Result<()> {
        let rendered = match self.platform {
            Platform::Discord => {
                DiscordRenderer::render_paper_ingested(paper_title, arxiv_id, tags.unwrap_or(&[]))
            }
            Platform::Feishu | Platform::Generic => self.render_generic(
                "paper_ingested",
                &PaperIngestedPayload {
                    title: paper_title.to_string(),
                    arxiv_id: arxiv_id.to_string(),
                    tags: tags.map(|v| v.to_vec()).unwrap_or_default(),
                },
            ),
        };

        self.send_payload(rendered).await
    }

    pub(crate) fn render_generic(&self, event_type: &str, data: &impl Serialize) -> Value {
        json!({
            "event": event_type,
            "timestamp": Utc::now().to_rfc3339(),
            "source": "Rairos",
            "data": data
        })
    }

    pub async fn test(&self) -> Result<()> {
        self.send_gap_alert(GapAlertParams {
            gap_type: "test",
            title: "Test notification from Rairos",
            novelty: 0.5,
            severity: "low",
            supporting_papers: None,
            source: Some("webhook_test"),
            confidence: None,
            impact_score: None,
        })
        .await
    }
}
