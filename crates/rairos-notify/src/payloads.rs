use chrono::Utc;
use serde::{Deserialize, Serialize};

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
