//! rairos-game-mode — Research Game Mode: badges and progression system.
//!
//! Ported from `llm/game_mode.py` (270 LOC, pure stdlib).

use chrono::Local;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

const CAPSULES_PATH: &str = ".ai_research_os/gene_pool/capsules.json";
const BADGES_PATH: &str = ".ai_research_os/badges.json";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Badge {
    pub id: String,
    pub name: String,
    pub description: String,
    pub icon: String,
    #[serde(default)]
    pub earned: bool,
    #[serde(default)]
    pub earned_at: Option<String>,
}

impl Badge {
    pub fn new(id: &str, name: &str, description: &str, icon: &str) -> Self {
        Self {
            id: id.to_string(),
            name: name.to_string(),
            description: description.to_string(),
            icon: icon.to_string(),
            earned: false,
            earned_at: None,
        }
    }
    pub fn award(&mut self) {
        if !self.earned {
            self.earned = true;
            self.earned_at = Some(Local::now().to_rfc3339());
        }
    }
}

pub struct BadgeManager {
    badges: HashMap<String, Badge>,
    capsules_path: PathBuf,
    badges_path: PathBuf,
}

impl Default for BadgeManager {
    fn default() -> Self {
        Self::new()
    }
}

impl BadgeManager {
    pub fn new() -> Self {
        let home = dirs::home_dir().unwrap_or_default();
        let capsules_path = home.join(CAPSULES_PATH);
        let badges_path = home.join(BADGES_PATH);
        Self {
            badges: HashMap::new(),
            capsules_path,
            badges_path,
        }
    }

    pub fn load_badges(&mut self) {
        if !self.badges_path.exists() {
            self.init_badges();
            return;
        }
        if let Ok(content) = fs::read_to_string(&self.badges_path) {
            if let Ok(loaded) = serde_json::from_str::<HashMap<String, Badge>>(&content) {
                self.badges = loaded;
                return;
            }
        }
        self.init_badges();
    }

    fn init_badges(&mut self) {
        self.badges.insert(
            "contradiction_hunter".to_string(),
            Badge::new(
                "contradiction_hunter",
                "Contradiction Hunter",
                "3+ contradiction pairs detected",
                "🔍",
            ),
        );
        self.badges.insert(
            "gap_extractor".to_string(),
            Badge::new(
                "gap_extractor",
                "Gap Extractor",
                "10+ capsules in Gene Pool",
                "💎",
            ),
        );
        self.badges.insert(
            "evolution_master".to_string(),
            Badge::new(
                "evolution_master",
                "Evolution Master",
                "1+ capsule that has been evolved",
                "🧬",
            ),
        );
        self.badges.insert(
            "bold_explorer".to_string(),
            Badge::new(
                "bold_explorer",
                "Bold Explorer",
                "5+ bold hypothesis capsules",
                "🚀",
            ),
        );
        self.badges.insert(
            "rigor_rater".to_string(),
            Badge::new(
                "rigor_rater",
                "Rigor Rater",
                "10+ papers with rigor scores",
                "📊",
            ),
        );
        self.badges.insert(
            "paradigm_sentinel".to_string(),
            Badge::new(
                "paradigm_sentinel",
                "Paradigm Sentinel",
                "Paradigm concentration alert triggered",
                "🛡️",
            ),
        );
    }

    pub fn check_and_award_badges(&mut self) {
        // Contradiction Hunter: check claim graph
        if let Ok(content) = fs::read_to_string(
            dirs::home_dir()
                .unwrap_or_default()
                .join(".ai_research_os/evolution/claim_graph.json"),
        ) {
            if let Ok(data) = serde_json::from_str::<serde_json::Value>(&content) {
                if let Some(edges) = data.get("edges").and_then(|e| e.as_array()) {
                    let contradictions: usize = edges
                        .iter()
                        .filter(|e| {
                            e.get("improvement_ratio")
                                .and_then(|r| r.as_f64())
                                .map(|r| r < 1.0)
                                .unwrap_or(false)
                        })
                        .count();
                    if contradictions >= 3 {
                        if let Some(b) = self.badges.get_mut("contradiction_hunter") {
                            b.award();
                        }
                    }
                }
            }
        }

        // Gap Extractor: 10+ capsules
        if self.capsules_path.exists() {
            if let Ok(content) = fs::read_to_string(&self.capsules_path) {
                if let Ok(capsules) = serde_json::from_str::<serde_json::Value>(&content) {
                    if let Some(arr) = capsules.as_array() {
                        if arr.len() >= 10 {
                            if let Some(b) = self.badges.get_mut("gap_extractor") {
                                b.award();
                            }
                        }
                    }
                }
            }
        }

        // Evolution Master: check evolved capsules
        if self.capsules_path.exists() {
            if let Ok(content) = fs::read_to_string(&self.capsules_path) {
                if let Ok(capsules) = serde_json::from_str::<serde_json::Value>(&content) {
                    if let Some(arr) = capsules.as_array() {
                        let evolved = arr.iter().any(|c| {
                            c.get("evolved")
                                .and_then(|e| e.as_bool())
                                .unwrap_or(false)
                        });
                        if evolved {
                            if let Some(b) = self.badges.get_mut("evolution_master") {
                                b.award();
                            }
                        }
                    }
                }
            }
        }

        // Bold Explorer: 5+ bold hypothesis capsules
        if self.capsules_path.exists() {
            if let Ok(content) = fs::read_to_string(&self.capsules_path) {
                if let Ok(capsules) = serde_json::from_str::<serde_json::Value>(&content) {
                    if let Some(arr) = capsules.as_array() {
                        let bold_count = arr.iter().filter(|c| {
                            c.get("is_bold")
                                .and_then(|b| b.as_bool())
                                .unwrap_or(false)
                        }).count();
                        if bold_count >= 5 {
                            if let Some(b) = self.badges.get_mut("bold_explorer") {
                                b.award();
                            }
                        }
                    }
                }
            }
        }

        // Rigor Rater: check rigor_scores directory
        let rigor_path = dirs::home_dir()
            .unwrap_or_default()
            .join(".ai_research_os/rigor_scores");
        if rigor_path.exists() {
            if let Ok(entries) = fs::read_dir(&rigor_path) {
                let count = entries.filter_map(std::result::Result::ok).count();
                if count >= 10 {
                    if let Some(b) = self.badges.get_mut("rigor_rater") {
                        b.award();
                    }
                }
            }
        }

        self.save_badges();
    }

    pub fn get_unlocked_badges(&self) -> Vec<&Badge> {
        self.badges
            .values()
            .filter(|b| b.earned)
            .collect()
    }

    pub fn render_badges_html(&self) -> String {
        let earned: Vec<_> = self.badges.values().filter(|b| b.earned).collect();
        let locked: Vec<_> = self.badges.values().filter(|b| !b.earned).collect();

        let mut html = r#"<div class="badges-container" style="font-family:Georgia,serif;padding:16px">"#.to_string();
        html.push_str("<h3>🏆 Research Badges</h3>");

        if earned.is_empty() {
            html.push_str("<p style='color:#888'>No badges earned yet. Keep researching!</p>");
        } else {
            html.push_str("<div style='display:flex;flex-wrap:wrap;gap:12px;margin-bottom:20px'>");
            for b in &earned {
                let date = b.earned_at.as_deref().unwrap_or("");
                html.push_str(&format!(
                    r#"<div style='text-align:center;padding:12px;border:1px solid #ddd;border-radius:8px;background:#f9f9f9;min-width:100px'>
<div style='font-size:28px'>{}</div>
<div style='font-weight:bold'>{}</div>
<div style='font-size:11px;color:#666'>{}</div>
<div style='font-size:10px;color:#999;margin-top:4px'>{}</div>
</div>"#,
                    b.icon, b.name, b.description, &date[..10]
                ));
            }
            html.push_str("</div>");
        }

        if !locked.is_empty() {
            html.push_str("<details style='margin-top:12px'><summary style='cursor:pointer;color:#888;font-size:13px'>Locked Badges</summary>");
            html.push_str("<div style='display:flex;flex-wrap:wrap;gap:12px;margin-top:12px;opacity:0.5'>");
            for b in &locked {
                html.push_str(&format!(
                    r#"<div style='text-align:center;padding:12px;border:1px solid #ddd;border-radius:8px;background:#f0f0f0;min-width:100px'>
<div style='font-size:28px'>🔒</div>
<div style='font-weight:bold'>{}</div>
<div style='font-size:11px;color:#666'>{}</div>
</div>"#,
                    b.name, b.description
                ));
            }
            html.push_str("</div></details>");
        }

        html.push_str("</div>");
        html
    }

    pub fn save_badges(&self) {
        if let Ok(json) = serde_json::to_string_pretty(&self.badges) {
            let _ = fs::write(&self.badges_path, json);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_badge_award() {
        let mut badge = Badge::new("test", "Test", "desc", "⭐");
        assert!(!badge.earned);
        badge.award();
        assert!(badge.earned);
        assert!(badge.earned_at.is_some());
        // Awarding again doesn't change earned_at
        let first_earned = badge.earned_at.clone();
        badge.award();
        assert_eq!(badge.earned_at, first_earned);
    }

    #[test]
    fn test_init_badges() {
        let mut m = BadgeManager::new();
        m.init_badges();
        assert_eq!(m.badges.len(), 6);
        assert!(m.badges.contains_key("contradiction_hunter"));
        assert!(m.badges.contains_key("gap_extractor"));
        assert!(m.badges.contains_key("evolution_master"));
        assert!(m.badges.contains_key("bold_explorer"));
        assert!(m.badges.contains_key("rigor_rater"));
        assert!(m.badges.contains_key("paradigm_sentinel"));
    }

    #[test]
    fn test_render_badges_html_empty() {
        let mut m = BadgeManager::new();
        m.init_badges();
        let html = m.render_badges_html();
        assert!(html.contains("No badges earned yet"));
        assert!(html.contains("Locked Badges"));
    }

    #[test]
    fn test_render_badges_html_with_earned() {
        let mut m = BadgeManager::new();
        m.init_badges();
        m.badges.get_mut("gap_extractor").unwrap().award();
        let html = m.render_badges_html();
        assert!(html.contains("Gap Extractor"));
        assert!(!html.contains("No badges earned yet"));
    }

    #[test]
    fn test_get_unlocked_badges() {
        let mut m = BadgeManager::new();
        m.init_badges();
        assert!(m.get_unlocked_badges().is_empty());
        m.badges.get_mut("contradiction_hunter").unwrap().award();
        m.badges.get_mut("gap_extractor").unwrap().award();
        let unlocked = m.get_unlocked_badges();
        assert_eq!(unlocked.len(), 2);
    }
}
