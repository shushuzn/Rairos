//! rairos-insight — Unified facade for all insight-related crates.
//!
//! Re-exports from sub-crates:
//! - rairos-insight-types: action types, profiles, events, trust tracking
//! - rairos-insight-credibility: credibility scoring, trendslop detection
//! - rairos-insight-storage: CapsuleStorage (SQLite gene_pool.db)
//! - rairos-insight-tracker: EvolutionTracker (event recording, profiles)
//! - rairos-insight-evolution: EvolutionEngine (audit, propose, evaluate, apply)
//!
//! Also provides InsightCard / InsightCollection (ported from llm/insight_cards.py).

// Re-export all sub-crates so consumers can use `rairos_insight::types::*` etc.
pub use rairos_insight_types as types;
pub use rairos_insight_credibility as credibility;
pub use rairos_insight_storage as storage;
pub use rairos_insight_tracker as tracker;
pub use rairos_insight_evolution as evolution;

// ─── InsightCard (from llm/insight_cards.py) ─────────────────────────────────

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

fn insight_dir() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".ai_research_os")
        .join("insights")
}

fn insight_cards_file() -> PathBuf {
    insight_dir().join("insight_cards.json")
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InsightCard {
    pub id: String,
    pub title: String,
    pub content: String,
    pub topic: String,
    pub gap_type: String,
    pub source_paper_id: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub quality_score: f64,
    pub novelty_score: f64,
    pub impact_tags: Vec<String>,
    #[serde(default)]
    pub related_capsule_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct InsightCollection {
    #[serde(default)]
    pub cards: Vec<InsightCard>,
}

impl InsightCollection {
    pub fn new() -> Self {
        Self { cards: Vec::new() }
    }

    pub fn load() -> Self {
        let path = insight_cards_file();
        if !path.exists() {
            return Self::new();
        }
        match fs::read_to_string(&path) {
            Ok(text) => serde_json::from_str(&text).unwrap_or_default(),
            Err(_) => Self::new(),
        }
    }

    pub fn save(&self) -> bool {
        let dir = insight_dir();
        if fs::create_dir_all(&dir).is_err() {
            return false;
        }
        let path = insight_cards_file();
        match serde_json::to_string_pretty(self) {
            Ok(json) => fs::write(&path, json).is_ok(),
            Err(_) => false,
        }
    }

    pub fn add_card(&mut self, card: InsightCard) {
        self.cards.push(card);
    }

    pub fn get_card(&self, id: &str) -> Option<&InsightCard> {
        self.cards.iter().find(|c| c.id == id)
    }

    pub fn get_card_mut(&mut self, id: &str) -> Option<&mut InsightCard> {
        self.cards.iter_mut().find(|c| c.id == id)
    }

    pub fn remove_card(&mut self, id: &str) -> bool {
        let initial_len = self.cards.len();
        self.cards.retain(|c| c.id != id);
        self.cards.len() < initial_len
    }

    pub fn list_cards(&self, topic: Option<&str>, gap_type: Option<&str>) -> Vec<&InsightCard> {
        self.cards
            .iter()
            .filter(|c| {
                let topic_match = topic.map(|t| c.topic == t).unwrap_or(true);
                let gap_match = gap_type.map(|g| c.gap_type == g).unwrap_or(true);
                topic_match && gap_match
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_insight_collection_new() {
        let col = InsightCollection::new();
        assert!(col.cards.is_empty());
    }

    #[test]
    fn test_add_card() {
        let mut col = InsightCollection::new();
        let card = InsightCard {
            id: "ins_001".to_string(),
            title: "Test Insight".to_string(),
            content: "This is a test insight.".to_string(),
            topic: "NLP".to_string(),
            gap_type: "method_gap".to_string(),
            source_paper_id: None,
            created_at: "2024-01-01T00:00:00Z".to_string(),
            updated_at: "2024-01-01T00:00:00Z".to_string(),
            quality_score: 0.8,
            novelty_score: 0.7,
            impact_tags: vec!["novel".to_string()],
            related_capsule_ids: Vec::new(),
        };
        col.add_card(card);
        assert_eq!(col.cards.len(), 1);
    }

    #[test]
    fn test_sub_crate_re_exports() {
        // Verify that sub-crate types are accessible through the facade
        let _event = types::EvolutionEvent {
            timestamp: "2024-01-01T00:00:00Z".to_string(),
            topic: "NLP".to_string(),
            action: types::ExplorationAction::Viewed,
            gap_type: String::new(),
            gap_title: String::new(),
            gap_description: String::new(),
            hypothesis_id: String::new(),
            question_id: String::new(),
            paper_ids: Vec::new(),
            duration_seconds: 0,
            notes: String::new(),
            insight_card_id: String::new(),
        };
        let _score = credibility::CredibilityScore {
            capsule_id: "test".to_string(),
            overall: 0.5,
            novelty_v2: 0.5,
            evidence_strength: 0.5,
            source_trust: 0.5,
            consistency: 0.5,
            trendslop: false,
            trendslop_reason: String::new(),
            badge: "medium".to_string(),
        };
        let _quality = evolution::CapsuleQuality {
            capsule_id: "test".to_string(),
            quality_score: 0.5,
            novelty: 0.5,
            utility: 0.5,
            freshness: 0.5,
            overall: 0.5,
        };
    }
}
