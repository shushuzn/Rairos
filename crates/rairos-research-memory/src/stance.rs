//! Stance types and ResearchStance struct.

use chrono::Utc;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Research stance type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[derive(Default)]
pub enum StanceType {
    #[default]
    Supported,
    Rejected,
    Deferred,
    Qualified,
}


impl std::fmt::Display for StanceType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StanceType::Supported => write!(f, "supported"),
            StanceType::Rejected => write!(f, "rejected"),
            StanceType::Deferred => write!(f, "deferred"),
            StanceType::Qualified => write!(f, "qualified"),
        }
    }
}

/// Anomaly severity level.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[derive(Default)]
pub enum AnomalySeverity {
    High,
    #[default]
    Medium,
    Low,
}


impl std::fmt::Display for AnomalySeverity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AnomalySeverity::High => write!(f, "high"),
            AnomalySeverity::Medium => write!(f, "medium"),
            AnomalySeverity::Low => write!(f, "low"),
        }
    }
}

/// A research decision — stance on a claim, method, or hypothesis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResearchStance {
    pub stance_id: String,
    /// Linked belief ID in BeliefNetwork (if any)
    pub belief_id: Option<String>,
    pub topic: String,
    pub claim: String,
    pub stance: StanceType,
    pub evidence_refs: Vec<String>,
    pub reasoning: String,
    pub confidence: f64,
    pub created_at: f64,
    pub updated_at: f64,
    pub tags: Vec<String>,
    pub notes: String,
}

impl ResearchStance {
    /// Create a new stance with a generated ID and current timestamp.
    pub fn new(
        topic: &str,
        claim: &str,
        stance: StanceType,
        evidence_refs: Vec<String>,
        reasoning: &str,
        confidence: f64,
        tags: Vec<String>,
        notes: &str,
    ) -> Self {
        let now = Utc::now().timestamp() as f64 + Utc::now().timestamp_subsec_nanos() as f64 * 1e-9;
        Self {
            stance_id: Uuid::new_v4().to_string()[..8].to_string(),
            belief_id: None,
            topic: topic.to_string(),
            claim: claim.to_string(),
            stance,
            evidence_refs,
            reasoning: reasoning.to_string(),
            confidence: confidence.clamp(0.0, 1.0),
            created_at: now,
            updated_at: now,
            tags,
            notes: notes.to_string(),
        }
    }

    /// Create a new stance linked to a belief.
    pub fn new_with_belief(
        topic: &str,
        claim: &str,
        stance: StanceType,
        evidence_refs: Vec<String>,
        reasoning: &str,
        confidence: f64,
        tags: Vec<String>,
        notes: &str,
        belief_id: &str,
    ) -> Self {
        let now = Utc::now().timestamp() as f64 + Utc::now().timestamp_subsec_nanos() as f64 * 1e-9;
        Self {
            stance_id: Uuid::new_v4().to_string()[..8].to_string(),
            belief_id: Some(belief_id.to_string()),
            topic: topic.to_string(),
            claim: claim.to_string(),
            stance,
            evidence_refs,
            reasoning: reasoning.to_string(),
            confidence: confidence.clamp(0.0, 1.0),
            created_at: now,
            updated_at: now,
            tags,
            notes: notes.to_string(),
        }
    }

    /// Update the stance in-place and refresh updated_at.
    pub fn update(
        &mut self,
        claim: Option<&str>,
        stance: Option<StanceType>,
        reasoning: Option<&str>,
        confidence: Option<f64>,
        notes: Option<&str>,
        tags: Option<Vec<String>>,
    ) {
        if let Some(v) = claim {
            self.claim = v.to_string();
        }
        if let Some(v) = stance {
            self.stance = v;
        }
        if let Some(v) = reasoning {
            self.reasoning = v.to_string();
        }
        if let Some(v) = confidence {
            self.confidence = v.clamp(0.0, 1.0);
        }
        if let Some(v) = notes {
            self.notes = v.to_string();
        }
        if let Some(v) = tags {
            self.tags = v;
        }
        self.updated_at =
            Utc::now().timestamp() as f64 + Utc::now().timestamp_subsec_nanos() as f64 * 1e-9;
    }

    /// Link this stance to a belief.
    pub fn link_belief(&mut self, belief_id: &str) {
        self.belief_id = Some(belief_id.to_string());
        self.updated_at =
            Utc::now().timestamp() as f64 + Utc::now().timestamp_subsec_nanos() as f64 * 1e-9;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stance_creation() {
        let s = ResearchStance::new(
            "RAG vs fine-tuning",
            "Fine-tuning is better for domain knowledge",
            StanceType::Supported,
            vec!["2301.00001".to_string()],
            "Based on experiments",
            0.8,
            vec!["nlp".to_string()],
            "Notes here",
        );
        assert_eq!(s.topic, "RAG vs fine-tuning");
        assert_eq!(s.stance, StanceType::Supported);
        assert_eq!(s.confidence, 0.8);
        assert!(!s.stance_id.is_empty());
    }

    #[test]
    fn test_confidence_clamping() {
        let s = ResearchStance::new(
            "t",
            "c",
            StanceType::Supported,
            vec![],
            "r",
            1.5,
            vec![],
            "",
        );
        assert_eq!(s.confidence, 1.0);

        let s2 = ResearchStance::new(
            "t",
            "c",
            StanceType::Supported,
            vec![],
            "r",
            -0.5,
            vec![],
            "",
        );
        assert_eq!(s2.confidence, 0.0);
    }

    #[test]
    fn test_update() {
        let mut s = ResearchStance::new(
            "t",
            "c",
            StanceType::Supported,
            vec![],
            "r",
            0.5,
            vec![],
            "",
        );
        let original_updated = s.updated_at;
        std::thread::sleep(std::time::Duration::from_millis(10));
        s.update(
            Some("new claim"),
            Some(StanceType::Rejected),
            Some("new reasoning"),
            Some(0.9),
            Some("new notes"),
            Some(vec!["tag1".to_string()]),
        );
        assert_eq!(s.claim, "new claim");
        assert_eq!(s.stance, StanceType::Rejected);
        assert_eq!(s.confidence, 0.9);
        assert!(s.updated_at > original_updated);
    }

    #[test]
    fn test_stance_type_display() {
        assert_eq!(StanceType::Supported.to_string(), "supported");
        assert_eq!(StanceType::Rejected.to_string(), "rejected");
        assert_eq!(StanceType::Deferred.to_string(), "deferred");
        assert_eq!(StanceType::Qualified.to_string(), "qualified");
    }

    #[test]
    fn test_severity_display() {
        assert_eq!(AnomalySeverity::High.to_string(), "high");
        assert_eq!(AnomalySeverity::Medium.to_string(), "medium");
        assert_eq!(AnomalySeverity::Low.to_string(), "low");
    }
}
