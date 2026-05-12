//! AnomalyAlert struct.

use serde::{Deserialize, Serialize};

use super::stance::AnomalySeverity;

/// A new paper contradicts or challenges a prior research stance.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnomalyAlert {
    pub anomaly_id: String,
    pub stance_id: String,
    pub topic: String,
    pub stance_claim: String,
    pub paper_title: String,
    pub paper_arxiv_id: String,
    pub anomaly_type: String,
    pub severity: AnomalySeverity,
    pub description: String,
    pub created_at: f64,
}

impl AnomalyAlert {
    /// Create a new anomaly alert.
    pub fn new(
        anomaly_id: &str,
        stance_id: &str,
        topic: &str,
        stance_claim: &str,
        paper_title: &str,
        paper_arxiv_id: &str,
        anomaly_type: &str,
        severity: AnomalySeverity,
        description: &str,
        created_at: f64,
    ) -> Self {
        Self {
            anomaly_id: anomaly_id.to_string(),
            stance_id: stance_id.to_string(),
            topic: topic.to_string(),
            stance_claim: stance_claim.to_string(),
            paper_title: paper_title.to_string(),
            paper_arxiv_id: paper_arxiv_id.to_string(),
            anomaly_type: anomaly_type.to_string(),
            severity,
            description: description.to_string(),
            created_at,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_anomaly_creation() {
        let alert = AnomalyAlert::new(
            "abc12345",
            "def67890",
            "RAG vs fine-tuning",
            "Fine-tuning is better",
            "New paper title",
            "2301.00001",
            "contradiction",
            AnomalySeverity::High,
            "This paper shows the opposite",
            1700000000.0,
        );
        assert_eq!(alert.anomaly_id, "abc12345");
        assert_eq!(alert.severity, AnomalySeverity::High);
        assert_eq!(alert.anomaly_type, "contradiction");
    }
}
