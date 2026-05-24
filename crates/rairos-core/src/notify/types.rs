//! Rairos Notify — Webhook notifications for Discord, Feishu, and generic platforms.
//!
//! Ported from `notifications/dispatcher.py`. Sends rich notifications to Discord
//! (embeds) and Feishu (interactive cards) via webhooks, with a generic JSON fallback.

#![allow(clippy::too_many_arguments)]

use serde::{Deserialize, Serialize};
use thiserror::Error;

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
#[derive(Default)]
pub enum Platform {
    Discord,
    Feishu,
    #[default]
    Generic,
}
