use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

use rairos_core::ResearchGap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrchestratorState {
    pub running: bool,
    pub interval_minutes: i32,
    pub last_check: String,
    #[serde(default)]
    pub sessions: Vec<String>,
    #[serde(default)]
    pub alerts: Vec<ResearchAlert>,
}

impl Default for OrchestratorState {
    fn default() -> Self {
        Self {
            running: false,
            interval_minutes: 30,
            last_check: String::new(),
            sessions: Vec::new(),
            alerts: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrchestratorConfig {
    pub interval_minutes: i32,
    pub min_gap_severity_for_alert: String,
    pub min_gene_pool_score_for_alert: f64,
    pub min_papers_for_deep_analysis: i32,
    pub max_alerts_stored: i32,
}

impl Default for OrchestratorConfig {
    fn default() -> Self {
        Self {
            interval_minutes: 30,
            min_gap_severity_for_alert: "MEDIUM".to_string(),
            min_gene_pool_score_for_alert: 0.3,
            min_papers_for_deep_analysis: 3,
            max_alerts_stored: 50,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResearchAlert {
    pub alert_id: String,
    pub session_id: String,
    pub topic: String,
    pub triggered_by: String,
    pub trigger_title: String,
    pub gaps_found: i32,
    pub top_gap_title: String,
    pub top_gap_type: String,
    pub severity: String,
    pub gene_pool_score: f64,
    pub preference_boost: bool,
    pub created_at: f64,
}

impl ResearchAlert {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        alert_id: String,
        session_id: String,
        topic: String,
        triggered_by: String,
        trigger_title: String,
        gaps_found: i32,
        top_gap_title: String,
        top_gap_type: String,
        severity: String,
        gene_pool_score: f64,
        preference_boost: bool,
    ) -> Self {
        Self {
            alert_id,
            session_id,
            topic,
            triggered_by,
            trigger_title,
            gaps_found,
            top_gap_title,
            top_gap_type,
            severity,
            gene_pool_score,
            preference_boost,
            created_at: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|d| d.as_secs_f64())
                .unwrap_or(0.0),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaperInfo {
    pub arxiv_id: String,
    pub title: String,
    pub abstract_text: String,
    #[serde(default)]
    pub pdf_url: String,
    #[serde(default)]
    pub categories: String,
    #[serde(default)]
    pub authors: Vec<String>,
    #[serde(default)]
    pub published: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScoredGap {
    pub gap: ResearchGap,
    pub gap_type: String,
    pub title: String,
    pub description: String,
    pub severity: String,
    pub gene_pool_score: f64,
    pub preference_boost: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeepResearchResult {
    pub gaps: Vec<ResearchGap>,
    pub papers_analyzed: i32,
    pub session_id: String,
    #[serde(default)]
    pub iterations: i32,
    #[serde(default)]
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FilterStats {
    pub seen: i32,
    pub suppressed: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GenePoolStats {
    #[serde(default)]
    pub total: i32,
    #[serde(default)]
    pub avg_score: f64,
    #[serde(default)]
    pub by_gap_type: HashMap<String, i32>,
}