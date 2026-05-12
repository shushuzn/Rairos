use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InsightCard {
    pub card_id: String,
    pub paper_id: String,
    pub paper_title: String,
    pub content: String,
    pub insight_type: String,
    pub tags: Vec<String>,
    pub evidence: String,
    pub page_ref: String,
    pub created_at: String,
    pub references: Vec<String>,
    pub quality_rating: i32,
    pub usefulness_score: f64,
    pub times_rated: i32,
}

impl InsightCard {
    pub fn new(
        card_id: &str,
        paper_id: &str,
        paper_title: &str,
        content: &str,
        insight_type: &str,
    ) -> Self {
        Self {
            card_id: card_id.to_string(),
            paper_id: paper_id.to_string(),
            paper_title: paper_title.to_string(),
            content: content.to_string(),
            insight_type: insight_type.to_string(),
            tags: Vec::new(),
            evidence: String::new(),
            page_ref: String::new(),
            created_at: Utc::now().format("%Y-%m-%d").to_string(),
            references: Vec::new(),
            quality_rating: 0,
            usefulness_score: 0.0,
            times_rated: 0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InsightCollection {
    pub collection_id: String,
    pub title: String,
    pub description: String,
    pub card_ids: Vec<String>,
    pub tags: Vec<String>,
}

pub struct InsightManager {
    data_dir: PathBuf,
    cards_file: PathBuf,
    collections_file: PathBuf,
}

impl InsightManager {
    pub fn new(data_dir: Option<PathBuf>) -> Self {
        let data_dir = data_dir.unwrap_or_else(|| {
            dirs::home_dir()
                .map(|p| p.join(".ai_research_os"))
                .unwrap_or_else(|| PathBuf::from("."))
        });
        let cards_file = data_dir.join("insight_cards.json");
        let collections_file = data_dir.join("insight_collections.json");
        Self {
            data_dir,
            cards_file,
            collections_file,
        }
    }

    fn load_cards(&self) -> Vec<HashMap<String, serde_json::Value>> {
        if self.cards_file.exists() {
            if let Ok(text) = std::fs::read_to_string(&self.cards_file) {
                if let Ok(data) = serde_json::from_str(&text) {
                    return data;
                }
            }
        }
        Vec::new()
    }

    fn save_cards(&self, data: &[HashMap<String, serde_json::Value>]) {
        if let Ok(text) = serde_json::to_string_pretty(data) {
            let _ = std::fs::write(&self.cards_file, text);
        }
    }

    fn load_collections(&self) -> Vec<HashMap<String, serde_json::Value>> {
        if self.collections_file.exists() {
            if let Ok(text) = std::fs::read_to_string(&self.collections_file) {
                if let Ok(data) = serde_json::from_str(&text) {
                    return data;
                }
            }
        }
        Vec::new()
    }

    fn save_collections(&self, data: &[HashMap<String, serde_json::Value>]) {
        if let Ok(text) = serde_json::to_string_pretty(data) {
            let _ = std::fs::write(&self.collections_file, text);
        }
    }

    pub fn add_card(
        &self,
        paper_id: &str,
        paper_title: &str,
        content: &str,
        insight_type: &str,
        tags: Option<Vec<String>>,
        evidence: &str,
        page_ref: &str,
    ) -> InsightCard {
        let mut data = self.load_cards();
        let card_id = format!("i{:04}", data.len() + 1);

        let card = InsightCard {
            card_id: card_id.clone(),
            paper_id: paper_id.to_string(),
            paper_title: paper_title.to_string(),
            content: content.to_string(),
            insight_type: insight_type.to_string(),
            tags: tags.unwrap_or_default(),
            evidence: evidence.to_string(),
            page_ref: page_ref.to_string(),
            created_at: Utc::now().format("%Y-%m-%d").to_string(),
            references: Vec::new(),
            quality_rating: 0,
            usefulness_score: 0.0,
            times_rated: 0,
        };

        let mut card_map = HashMap::new();
        card_map.insert("card_id".to_string(), serde_json::json!(card.card_id));
        card_map.insert("paper_id".to_string(), serde_json::json!(card.paper_id));
        card_map.insert(
            "paper_title".to_string(),
            serde_json::json!(card.paper_title),
        );
        card_map.insert("content".to_string(), serde_json::json!(card.content));
        card_map.insert(
            "insight_type".to_string(),
            serde_json::json!(card.insight_type),
        );
        card_map.insert("tags".to_string(), serde_json::json!(card.tags));
        card_map.insert("evidence".to_string(), serde_json::json!(card.evidence));
        card_map.insert("page_ref".to_string(), serde_json::json!(card.page_ref));
        card_map.insert("created_at".to_string(), serde_json::json!(card.created_at));
        card_map.insert("references".to_string(), serde_json::json!(card.references));
        card_map.insert(
            "quality_rating".to_string(),
            serde_json::json!(card.quality_rating),
        );
        card_map.insert(
            "usefulness_score".to_string(),
            serde_json::json!(card.usefulness_score),
        );
        card_map.insert(
            "times_rated".to_string(),
            serde_json::json!(card.times_rated),
        );

        data.push(card_map);
        self.save_cards(&data);
        card
    }

    pub fn get_card(&self, card_id: &str) -> Option<InsightCard> {
        let data = self.load_cards();
        for item in &data {
            if item.get("card_id").and_then(|v| v.as_str()) == Some(card_id) {
                return Some(InsightCard {
                    card_id: item
                        .get("card_id")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string(),
                    paper_id: item
                        .get("paper_id")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string(),
                    paper_title: item
                        .get("paper_title")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string(),
                    content: item
                        .get("content")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string(),
                    insight_type: item
                        .get("insight_type")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string(),
                    tags: item
                        .get("tags")
                        .and_then(|v| serde_json::from_value(v.clone()).ok())
                        .unwrap_or_default(),
                    evidence: item
                        .get("evidence")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string(),
                    page_ref: item
                        .get("page_ref")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string(),
                    created_at: item
                        .get("created_at")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string(),
                    references: item
                        .get("references")
                        .and_then(|v| serde_json::from_value(v.clone()).ok())
                        .unwrap_or_default(),
                    quality_rating: item
                        .get("quality_rating")
                        .and_then(|v| v.as_i64())
                        .unwrap_or(0) as i32,
                    usefulness_score: item
                        .get("usefulness_score")
                        .and_then(|v| v.as_f64())
                        .unwrap_or(0.0),
                    times_rated: item
                        .get("times_rated")
                        .and_then(|v| v.as_i64())
                        .unwrap_or(0) as i32,
                });
            }
        }
        None
    }

    pub fn rate_card(&self, card_id: &str, rating: i32) -> bool {
        if !(1..=5).contains(&rating) {
            return false;
        }

        let mut data = self.load_cards();
        for item in &mut data {
            if item.get("card_id").and_then(|v| v.as_str()) == Some(card_id) {
                let times_rated = item
                    .get("times_rated")
                    .and_then(|v| v.as_i64())
                    .unwrap_or(0);
                let old = item
                    .get("usefulness_score")
                    .and_then(|v| v.as_f64())
                    .unwrap_or(0.0);
                let new_score = old + (rating as f64 - old) * 0.3;

                item.insert(
                    "usefulness_score".to_string(),
                    serde_json::json!(round(new_score, 3)),
                );
                item.insert("quality_rating".to_string(), serde_json::json!(rating));
                item.insert(
                    "times_rated".to_string(),
                    serde_json::json!(times_rated + 1),
                );

                self.save_cards(&data);
                return true;
            }
        }
        false
    }

    pub fn like_card(&self, card_id: &str) -> bool {
        self.rate_card(card_id, 5)
    }

    pub fn dislike_card(&self, card_id: &str) -> bool {
        self.rate_card(card_id, 1)
    }

    pub fn update_card(
        &self,
        card_id: &str,
        content: Option<&str>,
        tags: Option<Vec<String>>,
        insight_type: Option<&str>,
    ) -> bool {
        let mut data = self.load_cards();
        for item in &mut data {
            if item.get("card_id").and_then(|v| v.as_str()) == Some(card_id) {
                if let Some(c) = content {
                    item.insert("content".to_string(), serde_json::json!(c));
                }
                if let Some(t) = tags {
                    item.insert("tags".to_string(), serde_json::json!(t));
                }
                if let Some(it) = insight_type {
                    item.insert("insight_type".to_string(), serde_json::json!(it));
                }
                self.save_cards(&data);
                return true;
            }
        }
        false
    }

    pub fn get_high_quality_cards(&self, min_rating: i32, min_scores: i32) -> Vec<InsightCard> {
        let data = self.load_cards();
        let mut results: Vec<InsightCard> = Vec::new();
        for item in &data {
            let rating = item
                .get("quality_rating")
                .and_then(|v| v.as_i64())
                .unwrap_or(0) as i32;
            let scores = item
                .get("times_rated")
                .and_then(|v| v.as_i64())
                .unwrap_or(0) as i32;
            if rating >= min_rating && scores >= min_scores {
                if let Some(card) = self._item_to_card(item) {
                    results.push(card);
                }
            }
        }
        results.sort_by(|a, b| b.usefulness_score.partial_cmp(&a.usefulness_score).unwrap());
        results
    }

    pub fn get_low_quality_cards(&self, max_rating: i32, min_scores: i32) -> Vec<InsightCard> {
        let data = self.load_cards();
        let mut results: Vec<InsightCard> = Vec::new();
        for item in &data {
            let rating = item
                .get("quality_rating")
                .and_then(|v| v.as_i64())
                .unwrap_or(0) as i32;
            let scores = item
                .get("times_rated")
                .and_then(|v| v.as_i64())
                .unwrap_or(0) as i32;
            if rating > 0 && rating <= max_rating && scores >= min_scores {
                if let Some(card) = self._item_to_card(item) {
                    results.push(card);
                }
            }
        }
        results.sort_by(|a, b| a.usefulness_score.partial_cmp(&b.usefulness_score).unwrap());
        results
    }

    pub fn add_reference(&self, from_card_id: &str, to_card_id: &str) -> bool {
        let mut data = self.load_cards();
        for item in &mut data {
            if item.get("card_id").and_then(|v| v.as_str()) == Some(from_card_id) {
                let refs: Vec<String> = item
                    .get("references")
                    .and_then(|v| serde_json::from_value(v.clone()).ok())
                    .unwrap_or_default();
                if !refs.contains(&to_card_id.to_string()) {
                    let mut new_refs = refs;
                    new_refs.push(to_card_id.to_string());
                    item.insert("references".to_string(), serde_json::json!(new_refs));
                    self.save_cards(&data);
                }
                return true;
            }
        }
        false
    }

    fn _item_to_card(
        &self,
        item: &std::collections::HashMap<String, serde_json::Value>,
    ) -> Option<InsightCard> {
        Some(InsightCard {
            card_id: item
                .get("card_id")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
            paper_id: item
                .get("paper_id")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
            paper_title: item
                .get("paper_title")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
            content: item
                .get("content")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
            insight_type: item
                .get("insight_type")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
            tags: item
                .get("tags")
                .and_then(|v| serde_json::from_value(v.clone()).ok())
                .unwrap_or_default(),
            evidence: item
                .get("evidence")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
            page_ref: item
                .get("page_ref")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
            created_at: item
                .get("created_at")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
            references: item
                .get("references")
                .and_then(|v| serde_json::from_value(v.clone()).ok())
                .unwrap_or_default(),
            quality_rating: item
                .get("quality_rating")
                .and_then(|v| v.as_i64())
                .unwrap_or(0) as i32,
            usefulness_score: item
                .get("usefulness_score")
                .and_then(|v| v.as_f64())
                .unwrap_or(0.0),
            times_rated: item
                .get("times_rated")
                .and_then(|v| v.as_i64())
                .unwrap_or(0) as i32,
        })
    }

    pub fn search_cards(
        &self,
        query: Option<&str>,
        tags: Option<Vec<&str>>,
        insight_type: Option<&str>,
        paper_id: Option<&str>,
    ) -> Vec<InsightCard> {
        let data = self.load_cards();
        let mut results = Vec::new();

        for item in &data {
            if let Some(q) = query {
                let q_lower = q.to_lowercase();
                let content = item
                    .get("content")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_lowercase();
                let title = item
                    .get("paper_title")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_lowercase();
                if !content.contains(&q_lower) && !title.contains(&q_lower) {
                    continue;
                }
            }

            if let Some(t) = &tags {
                let card_tags: Vec<String> = item
                    .get("tags")
                    .and_then(|v| serde_json::from_value(v.clone()).ok())
                    .unwrap_or_default();
                if !t.iter().any(|tag| card_tags.iter().any(|ct| ct == tag)) {
                    continue;
                }
            }

            if let Some(it) = insight_type {
                if item.get("insight_type").and_then(|v| v.as_str()) != Some(it) {
                    continue;
                }
            }

            if let Some(pid) = paper_id {
                if item.get("paper_id").and_then(|v| v.as_str()) != Some(pid) {
                    continue;
                }
            }

            results.push(InsightCard {
                card_id: item
                    .get("card_id")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string(),
                paper_id: item
                    .get("paper_id")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string(),
                paper_title: item
                    .get("paper_title")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string(),
                content: item
                    .get("content")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string(),
                insight_type: item
                    .get("insight_type")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string(),
                tags: item
                    .get("tags")
                    .and_then(|v| serde_json::from_value(v.clone()).ok())
                    .unwrap_or_default(),
                evidence: item
                    .get("evidence")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string(),
                page_ref: item
                    .get("page_ref")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string(),
                created_at: item
                    .get("created_at")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string(),
                references: item
                    .get("references")
                    .and_then(|v| serde_json::from_value(v.clone()).ok())
                    .unwrap_or_default(),
                quality_rating: item
                    .get("quality_rating")
                    .and_then(|v| v.as_i64())
                    .unwrap_or(0) as i32,
                usefulness_score: item
                    .get("usefulness_score")
                    .and_then(|v| v.as_f64())
                    .unwrap_or(0.0),
                times_rated: item
                    .get("times_rated")
                    .and_then(|v| v.as_i64())
                    .unwrap_or(0) as i32,
            });
        }

        results.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        results
    }

    pub fn get_tag_cloud(&self) -> HashMap<String, i32> {
        let data = self.load_cards();
        let mut tags: HashMap<String, i32> = HashMap::new();

        for item in &data {
            let card_tags: Vec<String> = item
                .get("tags")
                .and_then(|v| serde_json::from_value(v.clone()).ok())
                .unwrap_or_default();
            for tag in card_tags {
                *tags.entry(tag).or_insert(0) += 1;
            }
        }

        let mut sorted: Vec<_> = tags.into_iter().collect();
        sorted.sort_by(|a, b| b.1.cmp(&a.1));
        sorted.into_iter().collect()
    }

    pub fn create_collection(
        &self,
        title: &str,
        description: &str,
        tags: Option<Vec<String>>,
    ) -> InsightCollection {
        let mut collections = self.load_collections();
        let collection_id = format!("c{:04}", collections.len() + 1);

        let collection = InsightCollection {
            collection_id: collection_id.clone(),
            title: title.to_string(),
            description: description.to_string(),
            card_ids: Vec::new(),
            tags: tags.unwrap_or_default(),
        };

        let mut collection_map = HashMap::new();
        collection_map.insert(
            "collection_id".to_string(),
            serde_json::json!(collection.collection_id),
        );
        collection_map.insert("title".to_string(), serde_json::json!(collection.title));
        collection_map.insert(
            "description".to_string(),
            serde_json::json!(collection.description),
        );
        collection_map.insert(
            "card_ids".to_string(),
            serde_json::json!(collection.card_ids),
        );
        collection_map.insert("tags".to_string(), serde_json::json!(collection.tags));

        collections.push(collection_map);
        self.save_collections(&collections);
        collection
    }

    pub fn add_to_collection(&self, collection_id: &str, card_id: &str) -> bool {
        let mut collections = self.load_collections();
        for item in &mut collections {
            if item.get("collection_id").and_then(|v| v.as_str()) == Some(collection_id) {
                let mut card_ids: Vec<String> = item
                    .get("card_ids")
                    .and_then(|v| serde_json::from_value(v.clone()).ok())
                    .unwrap_or_default();
                if !card_ids.contains(&card_id.to_string()) {
                    card_ids.push(card_id.to_string());
                    item.insert("card_ids".to_string(), serde_json::json!(card_ids));
                    self.save_collections(&collections);
                }
                return true;
            }
        }
        false
    }

    pub fn extract_from_text(
        &self,
        paper_id: &str,
        paper_title: &str,
        text: &str,
    ) -> Vec<InsightCard> {
        let mut cards = Vec::new();
        let patterns = [
            r"improved by\s+(\d+\.?\d*)%",
            r"achieved\s+(\d+\.?\d*)%",
            r"outperforms?\s+[\w\s]+by\s+(\d+\.?\d*)%",
            r"reduced\s+(\w+)\s+by\s+(\d+\.?\d*)%",
        ];

        for pattern in &patterns {
            if let Ok(re) = regex::Regex::new(pattern) {
                for mat in re.find_iter(text) {
                    let context_start = mat.start().saturating_sub(100);
                    let context_end = (mat.end() + 100).min(text.len());
                    let mut context = text[context_start..context_end].to_string();
                    context = context.split_whitespace().collect::<Vec<_>>().join(" ");

                    if context.len() > 20 {
                        let card = self.add_card(
                            paper_id,
                            paper_title,
                            &format!(
                                "Key finding: {} - {}",
                                mat.as_str(),
                                &context[..context.len().min(200)]
                            ),
                            "finding",
                            None,
                            "",
                            "",
                        );
                        cards.push(card);
                    }
                }
            }
        }
        cards
    }

    pub fn render_text(&self, cards: &[InsightCard]) -> String {
        if cards.is_empty() {
            return "No insight cards found.".to_string();
        }

        let mut lines = vec![
            "=".repeat(70),
            "💡 Key Insight Cards".to_string(),
            "=".repeat(70),
            "".to_string(),
        ];

        let type_icons = HashMap::from([
            ("finding", "🎯"),
            ("method", "⚙️"),
            ("limitation", "⚠️"),
            ("future_work", "🔮"),
        ]);

        for card in cards.iter().take(20) {
            let icon = type_icons.get(card.insight_type.as_str()).unwrap_or(&"💡");
            lines.push(format!(
                "{} [{}] {}",
                icon,
                card.card_id,
                card.insight_type.to_uppercase()
            ));
            lines.push(format!(
                "   Paper: {}",
                &card.paper_title[..card.paper_title.len().min(50)]
            ));
            lines.push(format!(
                "   {}",
                &card.content[..card.content.len().min(100)]
            ));
            if !card.tags.is_empty() {
                lines.push(format!("   Tags: {}", card.tags.join(", ")));
            }
            if card.quality_rating > 0 {
                let stars = "★".repeat(card.quality_rating as usize)
                    + &"☆".repeat((5 - card.quality_rating) as usize);
                let votes = if card.times_rated != 1 { "s" } else { "" };
                lines.push(format!(
                    "   Rating: {} ({:.2}, {} vote{})",
                    stars, card.usefulness_score, card.times_rated, votes
                ));
            }
            lines.push("".to_string());
        }

        lines.push(format!("Total: {} cards", cards.len()));
        lines.push("=".repeat(70));
        lines.join("\n")
    }

    pub fn render_markdown(&self, cards: &[InsightCard]) -> String {
        let mut lines = vec!["# Key Insight Cards\n".to_string()];

        if cards.is_empty() {
            return format!("{}\nNo cards found.", lines.join(""));
        }

        let mut by_paper: HashMap<String, Vec<&InsightCard>> = HashMap::new();
        for card in cards {
            by_paper
                .entry(card.paper_id.clone())
                .or_default()
                .push(card);
        }

        for (paper_id, paper_cards) in &by_paper {
            lines.push(format!(
                "## {}\n",
                &paper_cards[0].paper_title[..paper_cards[0].paper_title.len().min(60)]
            ));
            lines.push(format!("*From: {}*\n", paper_id));

            for card in paper_cards {
                let type_icon = type_icons_markdown(card.insight_type.as_str());
                lines.push(format!(
                    "### {} {}",
                    type_icon,
                    capitalize(&card.insight_type)
                ));
                lines.push(format!("{}\n", card.content));

                if card.quality_rating > 0 {
                    let stars = "★".repeat(card.quality_rating as usize)
                        + &"☆".repeat((5 - card.quality_rating) as usize);
                    lines.push(format!(
                        "**Rating:** {} ({:.2}/5)\n",
                        stars, card.usefulness_score
                    ));
                }

                if !card.evidence.is_empty() {
                    lines.push(format!("> Evidence: {}\n", card.evidence));
                }

                if !card.tags.is_empty() {
                    let tag_str = card
                        .tags
                        .iter()
                        .map(|t| format!("#{}", t))
                        .collect::<Vec<_>>()
                        .join(", ");
                    lines.push(format!("*Tags: {}*\n", tag_str));
                }
            }
        }

        lines.join("")
    }
}

fn type_icons_markdown(insight_type: &str) -> &str {
    match insight_type {
        "finding" => "🎯",
        "method" => "⚙️",
        "limitation" => "⚠️",
        "future_work" => "🔮",
        _ => "💡",
    }
}

fn capitalize(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        None => String::new(),
        Some(c) => c.to_uppercase().collect::<String>() + chars.as_str(),
    }
}

fn round(v: f64, decimals: usize) -> f64 {
    let mul = 10_f64.powi(decimals as i32);
    (v * mul).round() / mul
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Once;
    static INIT: Once = Once::new();

    fn setup() {
        INIT.call_once(|| {
            let temp_dir = tempfile::tempdir().unwrap();
            std::env::set_var("TEST_DATA_DIR", temp_dir.path().to_str().unwrap());
        });
    }

    #[test]
    fn test_insight_card_creation() {
        let card = InsightCard::new("i0001", "p1", "Test Paper", "Test content", "finding");
        assert_eq!(card.card_id, "i0001");
        assert_eq!(card.insight_type, "finding");
        assert_eq!(card.quality_rating, 0);
    }

    #[test]
    fn test_round() {
        assert_eq!(round(1.23456, 2), 1.23);
        assert_eq!(round(1.235, 2), 1.24);
        assert_eq!(round(1.999, 2), 2.0);
    }

    #[test]
    fn test_capitalize() {
        assert_eq!(capitalize("finding"), "Finding");
        assert_eq!(capitalize("method"), "Method");
        assert_eq!(capitalize(""), "");
    }

    #[test]
    fn test_type_icons_markdown() {
        assert_eq!(type_icons_markdown("finding"), "🎯");
        assert_eq!(type_icons_markdown("method"), "⚙️");
        assert_eq!(type_icons_markdown("limitation"), "⚠️");
        assert_eq!(type_icons_markdown("future_work"), "🔮");
        assert_eq!(type_icons_markdown("unknown"), "💡");
    }
}
