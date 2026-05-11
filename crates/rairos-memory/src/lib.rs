//! Rairos Memory — Research stance tracking and anomaly detection
//!
//! Tracks research decisions over time and detects contradictions.
//! Replaces: llm/research_memory.py

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StanceType {
    #[default]
    Supported,
    Rejected,
    Deferred,
    Qualified,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AnomalySeverity {
    High,
    Medium,
    Low,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResearchStance {
    pub stance_id: String,
    pub topic: String,
    pub claim: String,
    pub stance: StanceType,
    pub evidence_refs: Vec<String>,
    pub reasoning: String,
    pub confidence: f64,
    pub created_at: String,
    pub updated_at: String,
    pub tags: Vec<String>,
    pub notes: String,
}

impl ResearchStance {
    pub fn new(topic: &str, claim: &str, stance: StanceType, reasoning: &str) -> Self {
        let now = chrono::Utc::now().to_rfc3339();
        Self {
            stance_id: uuid::Uuid::new_v4().to_string(),
            topic: topic.to_string(),
            claim: claim.to_string(),
            stance,
            evidence_refs: Vec::new(),
            reasoning: reasoning.to_string(),
            confidence: 0.5,
            created_at: now.clone(),
            updated_at: now,
            tags: Vec::new(),
            notes: String::new(),
        }
    }

    pub fn with_evidence(mut self, refs: Vec<String>) -> Self {
        self.evidence_refs = refs;
        self
    }

    pub fn with_confidence(mut self, confidence: f64) -> Self {
        self.confidence = confidence.clamp(0.0, 1.0);
        self
    }

    pub fn with_tags(mut self, tags: Vec<String>) -> Self {
        self.tags = tags;
        self
    }
}

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
    pub created_at: String,
}

impl AnomalyAlert {
    pub fn new(
        stance: &ResearchStance,
        paper_title: &str,
        paper_arxiv_id: &str,
        anomaly_type: &str,
        severity: AnomalySeverity,
        description: &str,
    ) -> Self {
        Self {
            anomaly_id: uuid::Uuid::new_v4().to_string(),
            stance_id: stance.stance_id.clone(),
            topic: stance.topic.clone(),
            stance_claim: stance.claim.clone(),
            paper_title: paper_title.to_string(),
            paper_arxiv_id: paper_arxiv_id.to_string(),
            anomaly_type: anomaly_type.to_string(),
            severity,
            description: description.to_string(),
            created_at: chrono::Utc::now().to_rfc3339(),
        }
    }
}

#[derive(Debug, Default)]
pub struct ResearchMemory {
    stances: Vec<ResearchStance>,
    anomalies: Vec<AnomalyAlert>,
}

impl ResearchMemory {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn default_path() -> std::path::PathBuf {
        dirs::home_dir()
            .unwrap_or_else(|| std::path::PathBuf::from("."))
            .join(".ai_research_os")
            .join("research_memory")
    }

    pub fn stances_path() -> std::path::PathBuf {
        Self::default_path().join("stances.json")
    }

    pub fn anomalies_path() -> std::path::PathBuf {
        Self::default_path().join("anomalies.json")
    }

    pub fn load() -> std::io::Result<Self> {
        let stances: Vec<ResearchStance> = if Self::stances_path().exists() {
            let text = std::fs::read_to_string(Self::stances_path())?;
            serde_json::from_str(&text).unwrap_or_default()
        } else {
            Vec::new()
        };

        let anomalies: Vec<AnomalyAlert> = if Self::anomalies_path().exists() {
            let text = std::fs::read_to_string(Self::anomalies_path())?;
            serde_json::from_str(&text).unwrap_or_default()
        } else {
            Vec::new()
        };

        Ok(Self { stances, anomalies })
    }

    pub fn save(&self) -> std::io::Result<()> {
        if let Some(parent) = Self::stances_path().parent() {
            std::fs::create_dir_all(parent)?;
        }

        let stances_json = serde_json::to_string_pretty(&self.stances)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        std::fs::write(Self::stances_path(), stances_json)?;

        let anomalies_json = serde_json::to_string_pretty(&self.anomalies)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        std::fs::write(Self::anomalies_path(), anomalies_json)?;

        Ok(())
    }

    pub fn add_stance(&mut self, stance: ResearchStance) {
        self.stances.push(stance);
    }

    pub fn get_stance(&self, id: &str) -> Option<&ResearchStance> {
        self.stances.iter().find(|s| s.stance_id == id)
    }

    pub fn stances(&self) -> &[ResearchStance] {
        &self.stances
    }

    pub fn stances_mut(&mut self) -> &mut Vec<ResearchStance> {
        &mut self.stances
    }

    pub fn find_by_topic(&self, topic: &str) -> Vec<&ResearchStance> {
        self.stances.iter().filter(|s| s.topic == topic).collect()
    }

    pub fn find_by_tag(&self, tag: &str) -> Vec<&ResearchStance> {
        self.stances.iter().filter(|s| s.tags.contains(&tag.to_string())).collect()
    }

    pub fn update_stance(&mut self, id: &str, stance: StanceType, reasoning: &str) -> Option<()> {
        if let Some(s) = self.stances.iter_mut().find(|st| st.stance_id == id) {
            s.stance = stance;
            s.reasoning = reasoning.to_string();
            s.updated_at = chrono::Utc::now().to_rfc3339();
            Some(())
        } else {
            None
        }
    }

    pub fn add_anomaly(&mut self, anomaly: AnomalyAlert) {
        self.anomalies.push(anomaly);
    }

    pub fn anomalies(&self) -> &[AnomalyAlert] {
        &self.anomalies
    }

    pub fn get_anomalies_by_stance(&self, stance_id: &str) -> Vec<&AnomalyAlert> {
        self.anomalies.iter().filter(|a| a.stance_id == stance_id).collect()
    }

    pub fn stats(&self) -> MemoryStats {
        let by_stance: HashMap<String, usize> = self.stances.iter()
            .fold(HashMap::new(), |mut acc, s| {
                *acc.entry(s.stance.to_string()).or_insert(0) += 1;
                acc
            });

        let by_severity: HashMap<String, usize> = self.anomalies.iter()
            .fold(HashMap::new(), |mut acc, a| {
                *acc.entry(format!("{:?}", a.severity)).or_insert(0) += 1;
                acc
            });

        MemoryStats {
            total_stances: self.stances.len(),
            total_anomalies: self.anomalies.len(),
            by_stance,
            by_severity,
        }
    }
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

impl std::fmt::Display for AnomalySeverity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AnomalySeverity::High => write!(f, "high"),
            AnomalySeverity::Medium => write!(f, "medium"),
            AnomalySeverity::Low => write!(f, "low"),
        }
    }
}

#[derive(Debug, Serialize)]
pub struct MemoryStats {
    pub total_stances: usize,
    pub total_anomalies: usize,
    pub by_stance: HashMap<String, usize>,
    pub by_severity: HashMap<String, usize>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stance_creation() {
        let stance = ResearchStance::new(
            "RAG vs Fine-tuning",
            "Fine-tuning is better for domain knowledge",
            StanceType::Supported,
            "Based on experiments"
        );
        assert_eq!(stance.topic, "RAG vs Fine-tuning");
        assert_eq!(stance.stance, StanceType::Supported);
    }

    #[test]
    fn test_stance_with_options() {
        let stance = ResearchStance::new("topic", "claim", StanceType::Rejected, "reason")
            .with_evidence(vec!["paper1".to_string()])
            .with_confidence(0.8)
            .with_tags(vec!["nlp".to_string()]);

        assert_eq!(stance.evidence_refs.len(), 1);
        assert_eq!(stance.confidence, 0.8);
        assert_eq!(stance.tags, vec!["nlp"]);
    }

    #[test]
    fn test_find_by_topic() {
        let mut memory = ResearchMemory::new();
        memory.add_stance(ResearchStance::new("AI", "claim1", StanceType::Supported, "r1"));
        memory.add_stance(ResearchStance::new("AI", "claim2", StanceType::Rejected, "r2"));
        memory.add_stance(ResearchStance::new("ML", "claim3", StanceType::Supported, "r3"));

        let ai_stances = memory.find_by_topic("AI");
        assert_eq!(ai_stances.len(), 2);
    }

    #[test]
    fn test_anomaly_creation() {
        let stance = ResearchStance::new("topic", "claim", StanceType::Supported, "reason");
        let anomaly = AnomalyAlert::new(
            &stance,
            "Contradicting Paper",
            "2301.00001",
            "contradiction",
            AnomalySeverity::High,
            "This paper shows the opposite"
        );

        assert_eq!(anomaly.topic, "topic");
        assert_eq!(anomaly.severity, AnomalySeverity::High);
    }

    #[test]
    fn test_stats() {
        let mut memory = ResearchMemory::new();
        memory.add_stance(ResearchStance::new("T1", "c1", StanceType::Supported, "r"));
        memory.add_stance(ResearchStance::new("T2", "c2", StanceType::Rejected, "r"));
        memory.add_stance(ResearchStance::new("T3", "c3", StanceType::Supported, "r"));

        let stats = memory.stats();
        assert_eq!(stats.total_stances, 3);
        assert_eq!(stats.by_stance.get("supported"), Some(&2));
        assert_eq!(stats.by_stance.get("rejected"), Some(&1));
    }

    #[test]
    fn test_find_by_tag() {
        let mut memory = ResearchMemory::new();
        memory.add_stance(ResearchStance::new("T1", "c1", StanceType::Supported, "r").with_tags(vec!["nlp".to_string()]));
        memory.add_stance(ResearchStance::new("T2", "c2", StanceType::Supported, "r").with_tags(vec!["cv".to_string()]));
        memory.add_stance(ResearchStance::new("T3", "c3", StanceType::Supported, "r").with_tags(vec!["nlp".to_string()]));

        let nlp_stances = memory.find_by_tag("nlp");
        assert_eq!(nlp_stances.len(), 2);
    }

    #[test]
    fn test_update_stance() {
        let mut memory = ResearchMemory::new();
        let stance = ResearchStance::new("T1", "c1", StanceType::Supported, "r");
        let id = stance.stance_id.clone();
        memory.add_stance(stance);

        let result = memory.update_stance(&id, StanceType::Rejected, "Changed my mind");
        assert!(result.is_some());

        let updated = memory.get_stance(&id).unwrap();
        assert_eq!(updated.stance, StanceType::Rejected);
        assert_eq!(updated.reasoning, "Changed my mind");
    }

    #[test]
    fn test_get_stance() {
        let mut memory = ResearchMemory::new();
        let stance = ResearchStance::new("T1", "c1", StanceType::Supported, "r");
        let id = stance.stance_id.clone();
        memory.add_stance(stance);

        let found = memory.get_stance(&id);
        assert!(found.is_some());
        assert_eq!(found.unwrap().topic, "T1");

        let not_found = memory.get_stance("nonexistent");
        assert!(not_found.is_none());
    }

    #[test]
    fn test_confidence_clamping() {
        let stance = ResearchStance::new("T", "C", StanceType::Supported, "R").with_confidence(1.5);
        assert_eq!(stance.confidence, 1.0);

        let stance2 = ResearchStance::new("T", "C", StanceType::Supported, "R").with_confidence(-0.5);
        assert_eq!(stance2.confidence, 0.0);
    }

    #[test]
    fn test_anomalies_by_stance() {
        let mut memory = ResearchMemory::new();
        let stance = ResearchStance::new("T1", "c1", StanceType::Supported, "r");
        let stance_id = stance.stance_id.clone();
        memory.add_stance(stance.clone());

        let anomaly = AnomalyAlert::new(
            &stance,
            "Paper X",
            "2301.00001",
            "contradiction",
            AnomalySeverity::High,
            "Shows opposite"
        );
        memory.add_anomaly(anomaly);

        let anomalies = memory.get_anomalies_by_stance(&stance_id);
        assert_eq!(anomalies.len(), 1);
    }

    #[test]
    fn test_stance_display() {
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