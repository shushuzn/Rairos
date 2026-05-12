//! rairos-insight-types — Insight tracking types: actions, profiles, events, and trust.
//!
//! Ported from `llm/insight/preferences.py`, `llm/insight/profile.py`, `llm/insight/trust_tracker.py`.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

// ─── Enums ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum ExplorationAction {
    #[default]
    Viewed,
    Accepted,
    Rejected,
    Expanded,
    Hypothesized,
    Validated,
    Narrated,
    InsightRated,
    ImplementationPass,
    ImplementationFail,
}

impl ExplorationAction {
    pub fn as_str(&self) -> &'static str {
        match self {
            ExplorationAction::Viewed => "viewed",
            ExplorationAction::Accepted => "accepted",
            ExplorationAction::Rejected => "rejected",
            ExplorationAction::Expanded => "expanded",
            ExplorationAction::Hypothesized => "hypothesized",
            ExplorationAction::Validated => "validated",
            ExplorationAction::Narrated => "narrated",
            ExplorationAction::InsightRated => "insight_rated",
            ExplorationAction::ImplementationPass => "implementation_pass",
            ExplorationAction::ImplementationFail => "implementation_fail",
        }
    }

    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "viewed" => Some(ExplorationAction::Viewed),
            "accepted" => Some(ExplorationAction::Accepted),
            "rejected" => Some(ExplorationAction::Rejected),
            "expanded" => Some(ExplorationAction::Expanded),
            "hypothesized" => Some(ExplorationAction::Hypothesized),
            "validated" => Some(ExplorationAction::Validated),
            "narrated" => Some(ExplorationAction::Narrated),
            "insight_rated" => Some(ExplorationAction::InsightRated),
            "implementation_pass" => Some(ExplorationAction::ImplementationPass),
            "implementation_fail" => Some(ExplorationAction::ImplementationFail),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum PreferenceTag {
    #[default]
    MethodFocused,
    ApplicationFocused,
    TheoryFocused,
    HighRiskTolerant,
    LowRiskTolerant,
    Exploratory,
    Confirmatory,
    CrossDomain,
}

impl PreferenceTag {
    pub fn as_str(&self) -> &'static str {
        match self {
            PreferenceTag::MethodFocused => "method_focused",
            PreferenceTag::ApplicationFocused => "app_focused",
            PreferenceTag::TheoryFocused => "theory_focused",
            PreferenceTag::HighRiskTolerant => "high_risk",
            PreferenceTag::LowRiskTolerant => "low_risk",
            PreferenceTag::Exploratory => "exploratory",
            PreferenceTag::Confirmatory => "confirmatory",
            PreferenceTag::CrossDomain => "cross_domain",
        }
    }
}

// ─── Events ───────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvolutionEvent {
    pub timestamp: String,
    pub topic: String,
    pub action: ExplorationAction,
    #[serde(default)]
    pub gap_type: String,
    #[serde(default)]
    pub gap_title: String,
    #[serde(default)]
    pub gap_description: String,
    #[serde(default)]
    pub hypothesis_id: String,
    #[serde(default)]
    pub question_id: String,
    #[serde(default)]
    pub paper_ids: Vec<String>,
    #[serde(default)]
    pub duration_seconds: i32,
    #[serde(default)]
    pub notes: String,
    #[serde(default)]
    pub insight_card_id: String,
}

// ─── User Profile ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct UserPreferenceProfile {
    #[serde(default)]
    pub total_sessions: i32,
    #[serde(default)]
    pub total_events: i32,
    #[serde(default)]
    pub views: i32,
    #[serde(default)]
    pub accepts: i32,
    #[serde(default)]
    pub rejects: i32,
    #[serde(default)]
    pub expands: i32,
    #[serde(default)]
    pub hypothesizes: i32,
    #[serde(default)]
    pub gap_type_preferences: HashMap<String, f64>,
    #[serde(default)]
    pub keyword_preferences: HashMap<String, f64>,
    #[serde(default)]
    pub topics_explored: Vec<String>,
    #[serde(default)]
    pub topic_frequency: HashMap<String, i32>,
    #[serde(default)]
    pub preference_tags: HashMap<String, f64>,
    #[serde(default)]
    pub recent_topics: Vec<String>,
    #[serde(default)]
    pub last_updated: String,
}

impl UserPreferenceProfile {
    pub fn new() -> Self {
        Self::default()
    }
}

// ─── Gap Exploration State ───────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GapExplorationState {
    pub topic: String,
    pub session_id: String,
    pub started_at: String,
    #[serde(default)]
    pub events: Vec<EvolutionEvent>,
    #[serde(default)]
    pub gaps_explored: Vec<String>,
    #[serde(default)]
    pub gaps_accepted: Vec<String>,
    #[serde(default)]
    pub gaps_rejected: Vec<String>,
    #[serde(default)]
    pub hypotheses_generated: i32,
}

impl GapExplorationState {
    pub fn new(topic: &str, session_id: &str, started_at: &str) -> Self {
        Self {
            topic: topic.to_string(),
            session_id: session_id.to_string(),
            started_at: started_at.to_string(),
            events: Vec::new(),
            gaps_explored: Vec::new(),
            gaps_accepted: Vec::new(),
            gaps_rejected: Vec::new(),
            hypotheses_generated: 0,
        }
    }
}

// ─── Source Trust ───────────────────────────────────────────────────────────

const DEFAULT_TRUST: f64 = 0.5;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceTrustEntry {
    pub category: String,
    pub trust_score: f64,
    pub capsule_count: i32,
    pub avg_success_score: f64,
    pub avg_feedback_count: f64,
    pub acceptance_rate: f64,
    pub last_updated: String,
}

impl SourceTrustEntry {
    pub fn new(category: &str) -> Self {
        Self {
            category: category.to_string(),
            trust_score: DEFAULT_TRUST,
            capsule_count: 0,
            avg_success_score: 0.0,
            avg_feedback_count: 0.0,
            acceptance_rate: 0.0,
            last_updated: chrono::Utc::now().to_rfc3339(),
        }
    }
}

pub struct SourceTrustTracker {
    data_dir: PathBuf,
    trust_data: HashMap<String, SourceTrustEntry>,
}

impl SourceTrustTracker {
    pub fn new(data_dir: Option<PathBuf>) -> Self {
        let data_dir = data_dir
            .unwrap_or_else(|| {
                dirs::home_dir()
                    .unwrap_or_else(|| PathBuf::from("."))
                    .join(".ai_research_os")
                    .join("evolution")
            })
            .join("source_trust.json");
        let mut tracker = Self {
            data_dir,
            trust_data: HashMap::new(),
        };
        tracker.load();
        tracker
    }

    fn load(&mut self) {
        if !self.data_dir.exists() {
            return;
        }
        if let Ok(text) = fs::read_to_string(&self.data_dir) {
            if let Ok(data) = serde_json::from_str::<HashMap<String, SourceTrustEntry>>(&text) {
                self.trust_data = data;
            }
        }
    }

    fn save(&self) {
        if let Some(parent) = self.data_dir.parent() {
            let _ = fs::create_dir_all(parent);
        }
        if let Ok(json) = serde_json::to_string_pretty(&self.trust_data) {
            let _ = fs::write(&self.data_dir, json);
        }
    }

    pub fn get_trust(&self, category: &str) -> f64 {
        self.trust_data
            .get(category)
            .map(|e| e.trust_score)
            .unwrap_or(DEFAULT_TRUST)
    }

    pub fn get_all_trusts(&self) -> HashMap<String, f64> {
        self.trust_data
            .iter()
            .map(|(k, v)| (k.clone(), v.trust_score))
            .collect()
    }

    pub fn get_all_entries(&self) -> HashMap<String, SourceTrustEntry> {
        self.trust_data.clone()
    }

    #[allow(clippy::too_many_arguments)]
    pub fn update_from_capsule(
        &mut self,
        _capsule_id: &str,
        outcome_success_score: f64,
        feedback_count: i32,
        source_arxiv_category: &str,
        accepts: i32,
        rejects: i32,
    ) {
        if source_arxiv_category.is_empty() {
            return;
        }

        let entry = self
            .trust_data
            .entry(source_arxiv_category.to_string())
            .or_insert_with(|| SourceTrustEntry::new(source_arxiv_category));

        let n = entry.capsule_count;
        entry.avg_success_score =
            (entry.avg_success_score * n as f64 + outcome_success_score) / (n as f64 + 1.0);
        entry.avg_feedback_count =
            (entry.avg_feedback_count * n as f64 + feedback_count as f64) / (n as f64 + 1.0);
        entry.capsule_count = n + 1;
        entry.last_updated = chrono::Utc::now().to_rfc3339();

        let total = accepts + rejects;
        entry.acceptance_rate = if total > 0 {
            accepts as f64 / total as f64
        } else {
            entry.acceptance_rate
        };

        let decay = (entry.capsule_count as f64 / 10.0).min(1.0);
        let base = entry.avg_success_score * 0.4
            + (entry.capsule_count as f64 / 20.0).min(1.0) * 0.3
            + entry.acceptance_rate * 0.3;
        entry.trust_score = (decay * base + (1.0 - decay) * DEFAULT_TRUST).clamp(0.0, 1.0);

        self.save();
    }

    pub fn render_html(&self) -> String {
        let mut entries: Vec<&SourceTrustEntry> = self
            .trust_data
            .values()
            .filter(|e| e.capsule_count >= 1)
            .collect();
        if entries.is_empty() {
            return "<p>No trust data yet. Import papers and create capsules first.</p>"
                .to_string();
        }

        entries.sort_by(|a, b| {
            b.trust_score
                .partial_cmp(&a.trust_score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        let mut lines = vec![
            "<div class=\"trust-panel\">".to_string(),
            "<h3>Source Trust Scores</h3>".to_string(),
            "<p style='color:#666;font-size:13px;'>Per-arXiv-category trust ratings based on capsule quality history.</p>".to_string(),
            "<table class=\"trust-table\">".to_string(),
            "<thead><tr><th>Category</th><th>Trust Score</th><th>Capsules</th><th>Avg Score</th><th>Avg Feedback</th></tr></thead>".to_string(),
            "<tbody>".to_string(),
        ];

        for e in entries {
            let bar_width = (e.trust_score * 100.0) as i32;
            let color = if e.trust_score >= 0.5 {
                "#7A9E7A"
            } else {
                "#C4706A"
            };
            lines.push(format!("<tr><td><code>{}</code></td>", e.category));
            lines.push(format!(
                "<td><div class=\"trust-bar\" style=\"width:{}%;background:{};padding:2px 6px;min-width:40px\">{:.2}</div></td>",
                bar_width, color, e.trust_score
            ));
            lines.push(format!(
                "<td>{}</td><td>{:.2}</td><td>{:.1}</td></tr>",
                e.capsule_count, e.avg_success_score, e.avg_feedback_count
            ));
        }

        lines.extend([
            "</tbody></table>".to_string(),
            "<style>".to_string(),
            ".trust-panel { font-family: Georgia, serif; }".to_string(),
            ".trust-table { width: 100%; border-collapse: collapse; margin-top: 1rem; }".to_string(),
            ".trust-table th, .trust-table td { padding: 0.4rem 0.8rem; border-bottom: 1px solid #e8e4de; text-align: left; font-size: 13px; }".to_string(),
            ".trust-table th { font-size: 0.75rem; text-transform: uppercase; letter-spacing: 0.05em; color: #7a7570; }".to_string(),
            ".trust-bar { height: 1.4em; border-radius: 4px; font-size: 0.8rem; color: white; display: inline-block; text-align: center; }".to_string(),
            "</style>".to_string(),
            "</div>".to_string(),
        ]);

        lines.join("\n")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_exploration_action_roundtrip() {
        for action in [
            ExplorationAction::Viewed,
            ExplorationAction::Accepted,
            ExplorationAction::Rejected,
            ExplorationAction::Expanded,
            ExplorationAction::Hypothesized,
            ExplorationAction::Validated,
            ExplorationAction::Narrated,
            ExplorationAction::InsightRated,
            ExplorationAction::ImplementationPass,
            ExplorationAction::ImplementationFail,
        ] {
            let s = action.as_str();
            let restored = ExplorationAction::from_str(s);
            assert_eq!(Some(action), restored);
        }
    }

    #[test]
    fn test_evolution_event_serialize() {
        let event = EvolutionEvent {
            timestamp: "2024-01-01T00:00:00Z".to_string(),
            topic: "NLP".to_string(),
            action: ExplorationAction::Accepted,
            gap_type: "method_gap".to_string(),
            gap_title: "Improve attention".to_string(),
            gap_description: "".to_string(),
            hypothesis_id: "".to_string(),
            question_id: "".to_string(),
            paper_ids: vec![],
            duration_seconds: 0,
            notes: "".to_string(),
            insight_card_id: "".to_string(),
        };
        let json = serde_json::to_string(&event).unwrap();
        let restored: EvolutionEvent = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.topic, "NLP");
    }

    #[test]
    fn test_user_preference_profile_new() {
        let profile = UserPreferenceProfile::new();
        assert_eq!(profile.total_events, 0);
        assert!(profile.gap_type_preferences.is_empty());
    }

    #[test]
    fn test_source_trust_entry_new() {
        let entry = SourceTrustEntry::new("cs.CL");
        assert_eq!(entry.category, "cs.CL");
        assert_eq!(entry.trust_score, DEFAULT_TRUST);
        assert_eq!(entry.capsule_count, 0);
    }

    #[test]
    fn test_source_trust_tracker_default_trust() {
        let tracker = SourceTrustTracker::new(None);
        assert_eq!(tracker.get_trust("unknown_category"), DEFAULT_TRUST);
    }

    #[test]
    fn test_source_trust_tracker_update() {
        let mut tracker = SourceTrustTracker::new(Some(PathBuf::from("/tmp/test_trust_rust.json")));
        tracker.update_from_capsule("cap1", 0.8, 5, "cs.CL", 3, 1);
        assert!(tracker.get_trust("cs.CL") > DEFAULT_TRUST);
    }

    #[test]
    fn test_gap_exploration_state_new() {
        let state = GapExplorationState::new("NLP", "sess123", "2024-01-01T00:00:00Z");
        assert_eq!(state.topic, "NLP");
        assert_eq!(state.session_id, "sess123");
        assert!(state.gaps_explored.is_empty());
    }
}
