//! rairos-insight — Insight Cards: track and manage research insights.
//!
//! Ported from `llm/insight_cards.py` (deferred to Phase 2 for LLM-dependent methods).

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

pub fn render_insight_card(card: &InsightCard) -> String {
    let mut lines = Vec::new();
    lines.push(format!("# {}", card.title));
    lines.push(format!(
        "Topic: {} | Gap Type: {}",
        card.topic, card.gap_type
    ));
    lines.push(format!(
        "Quality: {:.2} | Novelty: {:.2}",
        card.quality_score, card.novelty_score
    ));
    if !card.impact_tags.is_empty() {
        lines.push(format!("Tags: {}", card.impact_tags.join(", ")));
    }
    lines.push(String::new());
    lines.push(card.content.clone());
    lines.join("\n")
}

pub fn render_insight_collection(collection: &InsightCollection) -> String {
    if collection.cards.is_empty() {
        return "No insight cards yet.".to_string();
    }

    let mut lines = Vec::new();
    lines.push(format!(
        "# Insight Collection ({} cards)",
        collection.cards.len()
    ));
    lines.push(String::new());

    for card in &collection.cards {
        lines.push(render_insight_card(card));
        lines.push(String::new());
        lines.push(String::from("---"));
        lines.push(String::new());
    }

    lines.join("\n")
}

pub fn export_insights_json(collection: &InsightCollection) -> String {
    serde_json::to_string_pretty(collection).unwrap_or_default()
}

pub fn import_insights_json(json: &str) -> Option<InsightCollection> {
    serde_json::from_str(json).ok()
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
    fn test_get_card() {
        let mut col = InsightCollection::new();
        let card = InsightCard {
            id: "ins_001".to_string(),
            title: "Test".to_string(),
            content: "Content".to_string(),
            topic: "NLP".to_string(),
            gap_type: "method_gap".to_string(),
            source_paper_id: None,
            created_at: "2024-01-01T00:00:00Z".to_string(),
            updated_at: "2024-01-01T00:00:00Z".to_string(),
            quality_score: 0.8,
            novelty_score: 0.7,
            impact_tags: Vec::new(),
            related_capsule_ids: Vec::new(),
        };
        col.add_card(card.clone());
        assert!(col.get_card("ins_001").is_some());
        assert!(col.get_card("nonexistent").is_none());
    }

    #[test]
    fn test_remove_card() {
        let mut col = InsightCollection::new();
        let card = InsightCard {
            id: "ins_001".to_string(),
            title: "Test".to_string(),
            content: "Content".to_string(),
            topic: "NLP".to_string(),
            gap_type: "method_gap".to_string(),
            source_paper_id: None,
            created_at: "2024-01-01T00:00:00Z".to_string(),
            updated_at: "2024-01-01T00:00:00Z".to_string(),
            quality_score: 0.8,
            novelty_score: 0.7,
            impact_tags: Vec::new(),
            related_capsule_ids: Vec::new(),
        };
        col.add_card(card);
        assert!(col.remove_card("ins_001"));
        assert!(col.cards.is_empty());
    }

    #[test]
    fn test_list_cards_filter() {
        let mut col = InsightCollection::new();
        for i in 0..5 {
            col.add_card(InsightCard {
                id: format!("ins_{:03}", i),
                title: format!("Insight {}", i),
                content: "Content".to_string(),
                topic: if i < 3 {
                    "NLP".to_string()
                } else {
                    "Vision".to_string()
                },
                gap_type: "method_gap".to_string(),
                source_paper_id: None,
                created_at: "2024-01-01T00:00:00Z".to_string(),
                updated_at: "2024-01-01T00:00:00Z".to_string(),
                quality_score: 0.8,
                novelty_score: 0.7,
                impact_tags: Vec::new(),
                related_capsule_ids: Vec::new(),
            });
        }

        let nlp_cards = col.list_cards(Some("NLP"), None);
        assert_eq!(nlp_cards.len(), 3);
    }

    #[test]
    fn test_render_insight_card() {
        let card = InsightCard {
            id: "ins_001".to_string(),
            title: "Test Insight".to_string(),
            content: "This is the content.".to_string(),
            topic: "NLP".to_string(),
            gap_type: "method_gap".to_string(),
            source_paper_id: None,
            created_at: "2024-01-01T00:00:00Z".to_string(),
            updated_at: "2024-01-01T00:00:00Z".to_string(),
            quality_score: 0.85,
            novelty_score: 0.72,
            impact_tags: vec!["novel".to_string(), "impactful".to_string()],
            related_capsule_ids: Vec::new(),
        };

        let rendered = render_insight_card(&card);
        assert!(rendered.contains("Test Insight"));
        assert!(rendered.contains("NLP"));
        assert!(rendered.contains("method_gap"));
    }

    #[test]
    fn test_export_import_roundtrip() {
        let mut col = InsightCollection::new();
        col.add_card(InsightCard {
            id: "ins_001".to_string(),
            title: "Test".to_string(),
            content: "Content".to_string(),
            topic: "NLP".to_string(),
            gap_type: "method_gap".to_string(),
            source_paper_id: None,
            created_at: "2024-01-01T00:00:00Z".to_string(),
            updated_at: "2024-01-01T00:00:00Z".to_string(),
            quality_score: 0.8,
            novelty_score: 0.7,
            impact_tags: Vec::new(),
            related_capsule_ids: Vec::new(),
        });

        let json = export_insights_json(&col);
        let imported = import_insights_json(&json).unwrap();
        assert_eq!(imported.cards.len(), 1);
        assert_eq!(imported.cards[0].id, "ins_001");
    }
}
