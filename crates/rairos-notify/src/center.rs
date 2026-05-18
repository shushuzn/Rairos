use std::collections::HashMap;

use crate::types::{NotifyError, Result};
use crate::webhook::WebhookDispatcher;

#[derive(Debug, Clone, Default)]
pub struct NotificationCenter {
    dispatchers: Vec<WebhookDispatcher>,
}

impl NotificationCenter {
    pub fn new() -> Self {
        Self {
            dispatchers: Vec::new(),
        }
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

    pub fn dispatchers(&self) -> &[WebhookDispatcher] {
        &self.dispatchers
    }

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
    ) -> HashMap<String, Result<()>> {
        let mut results = HashMap::new();
        for d in &self.dispatchers {
            if d.webhook_url.is_empty() {
                continue;
            }
            let label = d.label.clone();
            results.insert(
                label,
                d.send_gap_alert(
                    gap_type,
                    title,
                    novelty,
                    severity,
                    supporting_papers,
                    source,
                    confidence,
                    impact_score,
                )
                .await,
            );
        }
        results
    }

    pub async fn send_paradigm_shift(
        &self,
        alert_type: &str,
        gap_type: &str,
        message: &str,
        severity: &str,
        contradictions: Option<&[crate::payloads::ContradictionEntry]>,
    ) -> HashMap<String, Result<()>> {
        let mut results = HashMap::new();
        for d in &self.dispatchers {
            if d.webhook_url.is_empty() {
                continue;
            }
            let label = d.label.clone();
            results.insert(
                label,
                d.send_paradigm_shift(alert_type, gap_type, message, severity, contradictions)
                    .await,
            );
        }
        results
    }

    pub async fn send_paper_ingested(
        &self,
        paper_title: &str,
        arxiv_id: &str,
        tags: Option<&[String]>,
    ) -> HashMap<String, Result<()>> {
        let mut results = HashMap::new();
        for d in &self.dispatchers {
            if d.webhook_url.is_empty() {
                continue;
            }
            let label = d.label.clone();
            results.insert(
                label,
                d.send_paper_ingested(paper_title, arxiv_id, tags).await,
            );
        }
        results
    }

    pub async fn test_all(&self) -> HashMap<String, Result<()>> {
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
