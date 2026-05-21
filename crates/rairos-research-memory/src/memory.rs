//! ResearchMemory — personal research stance log with anomaly detection.
//!
//! Translates: llm/research_memory.py (ResearchMemory class)

use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

use chrono::Utc;

use crate::alert::AnomalyAlert;
use crate::memory_stats::MemoryStats;
use crate::stance::{AnomalySeverity, ResearchStance, StanceType};

// ─── Paths ───────────────────────────────────────────────────────────────────

fn get_memory_path() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".ai_research_os")
        .join("research_memory")
}

fn get_stance_path() -> PathBuf {
    get_memory_path().join("stances.json")
}

fn get_anomaly_path() -> PathBuf {
    get_memory_path().join("anomalies.json")
}

// ─── Raw JSON load/save ───────────────────────────────────────────────────────

fn load_stances_raw() -> Vec<serde_json::Map<String, serde_json::Value>> {
    let path = get_stance_path();
    if path.exists() {
        if let Ok(text) = fs::read_to_string(&path) {
            if let Ok(parsed) = serde_json::from_str(&text) {
                return parsed;
            }
        }
    }
    Vec::new()
}

fn save_stances_raw(stances: &[serde_json::Map<String, serde_json::Value>]) -> std::io::Result<()> {
    let path = get_stance_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let text = serde_json::to_string_pretty(stances)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
    fs::write(path, text)
}

fn load_anomalies_raw() -> Vec<serde_json::Map<String, serde_json::Value>> {
    let path = get_anomaly_path();
    if path.exists() {
        if let Ok(text) = fs::read_to_string(&path) {
            if let Ok(parsed) = serde_json::from_str(&text) {
                return parsed;
            }
        }
    }
    Vec::new()
}

fn save_anomalies_raw(
    anomalies: &[serde_json::Map<String, serde_json::Value>],
) -> std::io::Result<()> {
    let path = get_anomaly_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let text = serde_json::to_string_pretty(anomalies)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
    fs::write(path, text)
}

// ─── ResearchMemory ───────────────────────────────────────────────────────────

/// Personal research stance log with anomaly detection.
#[derive(Debug, Clone)]
pub struct ResearchMemory {
    stances: Vec<ResearchStance>,
    anomalies: Vec<AnomalyAlert>,
}

impl Default for ResearchMemory {
    fn default() -> Self {
        Self::new()
    }
}

impl ResearchMemory {
    /// Create a new ResearchMemory, loading persisted data from disk.
    pub fn new() -> Self {
        let stances = load_stances_raw()
            .into_iter()
            .filter_map(stance_from_dict)
            .collect();
        let anomalies = load_anomalies_raw()
            .into_iter()
            .filter_map(anomaly_from_dict)
            .collect();
        Self { stances, anomalies }
    }

    /// Create a fresh in-memory ResearchMemory without loading any persisted data.
    /// Used for testing to avoid polluting ~/.ai_research_os
    #[cfg(test)]
    pub fn for_testing() -> Self {
        Self {
            stances: Vec::new(),
            anomalies: Vec::new(),
        }
    }

    /// Persist stances and anomalies to disk.
    fn persist(&self) {
        let stances: Vec<_> = self.stances.iter().map(stance_to_dict).collect();
        let _ = save_stances_raw(&stances);
        let anomalies: Vec<_> = self.anomalies.iter().map(anomaly_to_dict).collect();
        let _ = save_anomalies_raw(&anomalies);
    }

    // ── Stance CRUD ────────────────────────────────────────────────────────

    /// Record a new research stance.
    pub fn add_stance(
        &mut self,
        topic: &str,
        claim: &str,
        stance: StanceType,
        evidence_refs: Vec<String>,
        reasoning: &str,
        confidence: f64,
        tags: Vec<String>,
        notes: &str,
    ) -> &ResearchStance {
        let s = ResearchStance::new(
            topic,
            claim,
            stance,
            evidence_refs,
            reasoning,
            confidence,
            tags,
            notes,
        );
        self.stances.push(s);
        self.persist();
        self.stances.last().unwrap()
    }

    /// Update an existing stance by ID.
    pub fn update_stance(
        &mut self,
        stance_id: &str,
        claim: Option<&str>,
        stance: Option<StanceType>,
        reasoning: Option<&str>,
        confidence: Option<f64>,
        notes: Option<&str>,
        tags: Option<Vec<String>>,
    ) -> Option<&ResearchStance> {
        // Find the index first to avoid holding a mutable borrow across persist()
        let idx = self.stances.iter().position(|s| s.stance_id == stance_id);
        if let Some(i) = idx {
            let s = &mut self.stances[i];
            s.update(claim, stance, reasoning, confidence, notes, tags);
            self.persist();
            return self.stances.get(i);
        }
        None
    }

    /// Get all stances, optionally filtered by topic or stance type.
    pub fn get_stances(
        &self,
        topic: Option<&str>,
        stance_type: Option<StanceType>,
    ) -> Vec<ResearchStance> {
        let mut results: Vec<ResearchStance> = self.stances.to_vec();
        if let Some(t) = topic {
            let t_lower = t.to_lowercase();
            results.retain(|s| {
                let topic_lower = s.topic.to_lowercase();
                topic_lower.contains(&t_lower)
            });
        }
        if let Some(st) = stance_type {
            results.retain(|s| s.stance == st);
        }
        results.sort_by(|a, b| b.created_at.partial_cmp(&a.created_at).unwrap());
        results
    }

    /// Get a specific stance by ID.
    pub fn get_stance(&self, stance_id: &str) -> Option<&ResearchStance> {
        self.stances.iter().find(|s| s.stance_id == stance_id)
    }

    // ── Anomaly detection ───────────────────────────────────────────────────

    /// Check a paper against all prior stances. Returns any detected anomalies.
    ///
    /// If `use_llm` is true, performs a deeper LLM-powered check using the
    /// `rairos-constants` LLM_BASE_URL / LLM_MODEL defaults. Caller provides
    /// `api_key` and optionally overrides `base_url` / `model`.
    pub fn check_paper_against_stances(
        &mut self,
        paper: &HashMap<String, String>,
        use_llm: bool,
        api_key: Option<&str>,
        base_url: Option<&str>,
        model: Option<&str>,
    ) -> Vec<AnomalyAlert> {
        let mut detected: Vec<AnomalyAlert> = Vec::new();

        for stance in &self.stances {
            let anomaly = if use_llm {
                self.llm_check_against_stance(paper, stance, api_key, base_url, model)
            } else {
                self.keyword_check_against_stance(paper, stance)
            };

            if let Some(a) = anomaly {
                // Deduplicate: only add if not already present for this paper+stance pair
                let exists = self
                    .anomalies
                    .iter()
                    .any(|x| x.paper_arxiv_id == a.paper_arxiv_id && x.stance_id == a.stance_id);
                if !exists {
                    self.anomalies.push(a.clone());
                }
                detected.push(a);
            }
        }

        if !detected.is_empty() {
            self.persist();
        }

        detected
    }

    /// Fast keyword-based contradiction check.
    fn keyword_check_against_stance(
        &self,
        paper: &HashMap<String, String>,
        stance: &ResearchStance,
    ) -> Option<AnomalyAlert> {
        let paper_text = format!(
            "{} {}",
            paper.get("title").unwrap_or(&String::new()),
            paper.get("abstract").unwrap_or(&String::new())
        )
        .to_lowercase();
        let claim_lower = stance.claim.to_lowercase();
        let claim_words: Vec<_> = claim_lower.split_whitespace().take(5).collect();

        let contradiction_signals = [
            "fail to",
            "does not",
            "cannot",
            "ineffective",
            "worse than",
            "no evidence",
            "contrary to",
        ];

        for signal in contradiction_signals {
            if paper_text.contains(signal) && claim_words.iter().any(|w| paper_text.contains(w)) {
                return Some(AnomalyAlert::new(
                    &uuid::Uuid::new_v4().to_string()[..8],
                    &stance.stance_id,
                    &stance.topic,
                    &stance.claim,
                    &paper.get("title").unwrap_or(&String::new())
                        [..120.min(paper.get("title").map(|s| s.len()).unwrap_or(0))],
                    paper.get("arxiv_id").unwrap_or(&String::new()),
                    "challenge",
                    AnomalySeverity::Medium,
                    "Paper discusses limitations that challenge the claimed stance",
                    Utc::now().timestamp() as f64,
                ));
            }
        }
        None
    }

    /// LLM-powered deep contradiction check.
    #[allow(clippy::too_many_arguments)]
    fn llm_check_against_stance(
        &self,
        _paper: &HashMap<String, String>,
        _stance: &ResearchStance,
        _api_key: Option<&str>,
        _base_url: Option<&str>,
        _model: Option<&str>,
    ) -> Option<AnomalyAlert> {
        // NOTE: Full LLM integration requires the `call_llm_chat_completions` function
        // from `llm.chat` / `llm.client` (which lives in the Python codebase).
        // This Rust port provides the keyword-based check; the LLM path is a placeholder
        // that returns None until the LLM client is also ported.
        None
    }

    // ── Batch check ────────────────────────────────────────────────────────

    /// Check multiple papers for anomalies against all stances.
    pub fn check_papers_batch(
        &mut self,
        papers: &[HashMap<String, String>],
        use_llm: bool,
        api_key: Option<&str>,
    ) -> Vec<AnomalyAlert> {
        let mut all: Vec<AnomalyAlert> = Vec::new();
        for paper in papers {
            all.extend(self.check_paper_against_stances(paper, use_llm, api_key, None, None));
        }
        all
    }

    // ── Anomaly access ─────────────────────────────────────────────────────

    /// Get most recent anomaly alerts, up to `limit`.
    pub fn get_recent_anomalies(&self, limit: usize) -> Vec<AnomalyAlert> {
        let mut sorted: Vec<_> = self.anomalies.to_vec();
        sorted.sort_by(|a, b| b.created_at.partial_cmp(&a.created_at).unwrap());
        sorted.into_iter().take(limit).collect()
    }

    /// Get all anomalies for a given stance ID.
    pub fn get_anomalies_by_stance(&self, stance_id: &str) -> Vec<AnomalyAlert> {
        self.anomalies
            .iter()
            .filter(|a| a.stance_id == stance_id)
            .cloned()
            .collect()
    }

    /// Dismiss (remove) an anomaly by ID.
    pub fn dismiss_anomaly(&mut self, anomaly_id: &str) {
        self.anomalies.retain(|a| a.anomaly_id != anomaly_id);
        self.persist();
    }

    // ── Summary ────────────────────────────────────────────────────────────

    /// Get memory summary statistics.
    pub fn get_summary(&self) -> MemoryStats {
        let mut stance_counts: HashMap<String, usize> = HashMap::new();
        for s in &self.stances {
            *stance_counts.entry(s.stance.to_string()).or_insert(0) += 1;
        }
        let now = Utc::now().timestamp() as f64;
        let recent_anomalies = self
            .anomalies
            .iter()
            .filter(|a| now - a.created_at < 86400.0)
            .count();
        MemoryStats {
            total_stances: self.stances.len(),
            stance_breakdown: stance_counts,
            total_anomalies: self.anomalies.len(),
            recent_anomalies,
        }
    }
}

// ─── Dict conversion ──────────────────────────────────────────────────────────

fn stance_to_dict(s: &ResearchStance) -> serde_json::Map<String, serde_json::Value> {
    let mut m = serde_json::Map::new();
    m.insert(
        "stance_id".to_string(),
        serde_json::Value::String(s.stance_id.clone()),
    );
    m.insert(
        "topic".to_string(),
        serde_json::Value::String(s.topic.clone()),
    );
    m.insert(
        "claim".to_string(),
        serde_json::Value::String(s.claim.clone()),
    );
    m.insert(
        "stance".to_string(),
        serde_json::Value::String(s.stance.to_string()),
    );
    m.insert(
        "evidence_refs".to_string(),
        serde_json::Value::Array(
            s.evidence_refs
                .iter()
                .map(|r| serde_json::Value::String(r.clone()))
                .collect(),
        ),
    );
    m.insert(
        "reasoning".to_string(),
        serde_json::Value::String(s.reasoning.clone()),
    );
    m.insert(
        "confidence".to_string(),
        serde_json::Value::Number(
            serde_json::Number::from_f64(s.confidence).unwrap_or(serde_json::Number::from(0)),
        ),
    );
    m.insert(
        "created_at".to_string(),
        serde_json::Value::Number(
            serde_json::Number::from_f64(s.created_at).unwrap_or(serde_json::Number::from(0)),
        ),
    );
    m.insert(
        "updated_at".to_string(),
        serde_json::Value::Number(
            serde_json::Number::from_f64(s.updated_at).unwrap_or(serde_json::Number::from(0)),
        ),
    );
    m.insert(
        "tags".to_string(),
        serde_json::Value::Array(
            s.tags
                .iter()
                .map(|t| serde_json::Value::String(t.clone()))
                .collect(),
        ),
    );
    m.insert(
        "notes".to_string(),
        serde_json::Value::String(s.notes.clone()),
    );
    m
}

fn stance_from_dict(d: serde_json::Map<String, serde_json::Value>) -> Option<ResearchStance> {
    let stance_str = d.get("stance")?.as_str()?;
    let stance = match stance_str {
        "supported" => StanceType::Supported,
        "rejected" => StanceType::Rejected,
        "deferred" => StanceType::Deferred,
        "qualified" => StanceType::Qualified,
        _ => StanceType::Supported,
    };
    Some(ResearchStance {
        stance_id: d.get("stance_id")?.as_str()?.to_string(),
        topic: d.get("topic")?.as_str()?.to_string(),
        claim: d.get("claim")?.as_str()?.to_string(),
        stance,
        evidence_refs: d
            .get("evidence_refs")?
            .as_array()?
            .iter()
            .filter_map(|v| v.as_str().map(String::from))
            .collect(),
        reasoning: d.get("reasoning")?.as_str()?.to_string(),
        confidence: d.get("confidence")?.as_f64().unwrap_or(0.5),
        created_at: d.get("created_at")?.as_f64().unwrap_or(0.0),
        updated_at: d.get("updated_at")?.as_f64().unwrap_or(0.0),
        tags: d
            .get("tags")?
            .as_array()?
            .iter()
            .filter_map(|v| v.as_str().map(String::from))
            .collect(),
        notes: d.get("notes")?.as_str()?.to_string(),
    })
}

fn anomaly_to_dict(a: &AnomalyAlert) -> serde_json::Map<String, serde_json::Value> {
    let mut m = serde_json::Map::new();
    m.insert(
        "anomaly_id".to_string(),
        serde_json::Value::String(a.anomaly_id.clone()),
    );
    m.insert(
        "stance_id".to_string(),
        serde_json::Value::String(a.stance_id.clone()),
    );
    m.insert(
        "topic".to_string(),
        serde_json::Value::String(a.topic.clone()),
    );
    m.insert(
        "stance_claim".to_string(),
        serde_json::Value::String(a.stance_claim.clone()),
    );
    m.insert(
        "paper_title".to_string(),
        serde_json::Value::String(a.paper_title.clone()),
    );
    m.insert(
        "paper_arxiv_id".to_string(),
        serde_json::Value::String(a.paper_arxiv_id.clone()),
    );
    m.insert(
        "anomaly_type".to_string(),
        serde_json::Value::String(a.anomaly_type.clone()),
    );
    m.insert(
        "severity".to_string(),
        serde_json::Value::String(a.severity.to_string()),
    );
    m.insert(
        "description".to_string(),
        serde_json::Value::String(a.description.clone()),
    );
    m.insert(
        "created_at".to_string(),
        serde_json::Value::Number(
            serde_json::Number::from_f64(a.created_at).unwrap_or(serde_json::Number::from(0)),
        ),
    );
    m
}

fn anomaly_from_dict(d: serde_json::Map<String, serde_json::Value>) -> Option<AnomalyAlert> {
    let severity_str = d
        .get("severity")
        .and_then(|v| v.as_str())
        .unwrap_or("medium");
    let severity = match severity_str {
        "high" => AnomalySeverity::High,
        "medium" => AnomalySeverity::Medium,
        "low" => AnomalySeverity::Low,
        _ => AnomalySeverity::Medium,
    };
    Some(AnomalyAlert {
        anomaly_id: d.get("anomaly_id")?.as_str()?.to_string(),
        stance_id: d.get("stance_id")?.as_str()?.to_string(),
        topic: d.get("topic")?.as_str()?.to_string(),
        stance_claim: d.get("stance_claim")?.as_str()?.to_string(),
        paper_title: d.get("paper_title")?.as_str()?.to_string(),
        paper_arxiv_id: d.get("paper_arxiv_id")?.as_str()?.to_string(),
        anomaly_type: d.get("anomaly_type")?.as_str()?.to_string(),
        severity,
        description: d.get("description")?.as_str()?.to_string(),
        created_at: d.get("created_at")?.as_f64().unwrap_or(0.0),
    })
}

// ─── MemoryStats re-export ────────────────────────────────────────────────────

pub mod memory_stats {
    use serde::{Deserialize, Serialize};
    use std::collections::HashMap;

    /// Overall memory statistics.
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct MemoryStats {
        pub total_stances: usize,
        pub stance_breakdown: HashMap<String, usize>,
        pub total_anomalies: usize,
        pub recent_anomalies: usize,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add_and_get_stance() {
        let mut mem = ResearchMemory::for_testing();
        mem.add_stance(
            "AI",
            "LLMs are the future",
            StanceType::Supported,
            vec!["2301.00001".to_string()],
            "Evidence from research",
            0.8,
            vec!["nlp".to_string()],
            "",
        );
        assert_eq!(mem.stances.len(), 1);
        assert_eq!(
            mem.get_stance(&mem.stances[0].stance_id).unwrap().topic,
            "AI"
        );
    }

    #[test]
    fn test_get_stances_filtered() {
        let mut mem = ResearchMemory::for_testing();
        mem.add_stance(
            "AI",
            "c1",
            StanceType::Supported,
            vec![],
            "r",
            0.5,
            vec![],
            "",
        );
        mem.add_stance(
            "AI",
            "c2",
            StanceType::Rejected,
            vec![],
            "r",
            0.5,
            vec![],
            "",
        );
        mem.add_stance(
            "ML",
            "c3",
            StanceType::Supported,
            vec![],
            "r",
            0.5,
            vec![],
            "",
        );

        let ai_stances = mem.get_stances(Some("AI"), None);
        assert_eq!(ai_stances.len(), 2);

        let supported = mem.get_stances(None, Some(StanceType::Supported));
        assert_eq!(supported.len(), 2);
    }

    #[test]
    fn test_update_stance() {
        let mut mem = ResearchMemory::for_testing();
        let id = mem
            .add_stance(
                "T",
                "c",
                StanceType::Supported,
                vec![],
                "r",
                0.5,
                vec![],
                "",
            )
            .stance_id
            .clone();
        let result = mem.update_stance(
            &id,
            Some("new claim"),
            Some(StanceType::Rejected),
            Some("new r"),
            Some(0.9),
            Some("n"),
            Some(vec!["tag".to_string()]),
        );
        assert!(result.is_some());
        let updated = mem.get_stance(&id).unwrap();
        assert_eq!(updated.claim, "new claim");
        assert_eq!(updated.stance, StanceType::Rejected);
    }

    #[test]
    fn test_anomalies_by_stance() {
        let mut mem = ResearchMemory::for_testing();
        let id = mem
            .add_stance(
                "T",
                "c",
                StanceType::Supported,
                vec![],
                "r",
                0.5,
                vec![],
                "",
            )
            .stance_id
            .clone();
        let mut paper = HashMap::new();
        paper.insert(
            "title".to_string(),
            "This fails to support the claim".to_string(),
        );
        paper.insert("abstract".to_string(), "does not work".to_string());
        paper.insert("arxiv_id".to_string(), "2301.00001".to_string());

        let anomalies = mem.check_paper_against_stances(&paper, false, None, None, None);
        assert!(!anomalies.is_empty());

        let by_stance = mem.get_anomalies_by_stance(&id);
        assert_eq!(by_stance.len(), 1);
    }

    #[test]
    fn test_dismiss_anomaly() {
        let mut mem = ResearchMemory::for_testing();
        // Use a claim whose words appear in the paper AND the paper has a contradiction signal
        let id = mem
            .add_stance(
                "T",
                "is supported",
                StanceType::Supported,
                vec![],
                "r",
                0.5,
                vec![],
                "",
            )
            .stance_id
            .clone();
        let mut paper = HashMap::new();
        // Paper contains claim word "supported" AND contradiction signal "does not"
        paper.insert("title".to_string(), "does not support this".to_string());
        paper.insert(
            "abstract".to_string(),
            "method is not supported".to_string(),
        );
        paper.insert("arxiv_id".to_string(), "2301.00001".to_string());

        let anomalies = mem.check_paper_against_stances(&paper, false, None, None, None);
        assert!(!anomalies.is_empty(), "expected anomaly but got none");
        let anomaly_id = anomalies[0].anomaly_id.clone();
        mem.dismiss_anomaly(&anomaly_id);
        assert!(mem.get_anomalies_by_stance(&id).is_empty());
    }

    #[test]
    fn test_summary() {
        let mut mem = ResearchMemory::for_testing();
        mem.add_stance(
            "T1",
            "c1",
            StanceType::Supported,
            vec![],
            "r",
            0.5,
            vec![],
            "",
        );
        mem.add_stance(
            "T2",
            "c2",
            StanceType::Rejected,
            vec![],
            "r",
            0.5,
            vec![],
            "",
        );
        let summary = mem.get_summary();
        assert_eq!(summary.total_stances, 2);
        assert_eq!(summary.stance_breakdown.get("supported"), Some(&1));
        assert_eq!(summary.stance_breakdown.get("rejected"), Some(&1));
    }

    #[test]
    fn test_recent_anomalies_limit() {
        let mut mem = ResearchMemory::for_testing();
        let _id = mem
            .add_stance(
                "T",
                "is supported",
                StanceType::Supported,
                vec![],
                "r",
                0.5,
                vec![],
                "",
            )
            .stance_id
            .clone();
        for i in 0..5 {
            let mut paper = HashMap::new();
            paper.insert("title".to_string(), format!("does not is {}", i));
            paper.insert(
                "abstract".to_string(),
                "method is not supported".to_string(),
            );
            paper.insert("arxiv_id".to_string(), format!("2301.{}", i));
            mem.check_paper_against_stances(&paper, false, None, None, None);
        }
        let recent = mem.get_recent_anomalies(3);
        assert_eq!(recent.len(), 3);
    }
}
