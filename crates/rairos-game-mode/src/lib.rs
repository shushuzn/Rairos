//! rairos-game-mode — Research Game Mode: badges and progression system.
//!
//! Ported from `llm/game_mode.py`.

use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

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
    pub fn new(
        id: &str,
        name: &str,
        description: &str,
        icon: &str,
        earned: bool,
        earned_at: Option<&str>,
    ) -> Self {
        Self {
            id: id.to_string(),
            name: name.to_string(),
            description: description.to_string(),
            icon: icon.to_string(),
            earned,
            earned_at: earned_at.map(|s| s.to_string()),
        }
    }
}

fn get_capsules_path() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".ai_research_os")
        .join("gene_pool")
        .join("capsules.json")
}

fn get_badges_path() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".ai_research_os")
        .join("badges.json")
}

fn load_capsules() -> Vec<serde_json::Map<String, serde_json::Value>> {
    let path = get_capsules_path();
    if !path.exists() {
        return Vec::new();
    }
    match fs::read_to_string(&path) {
        Ok(text) => {
            if let Ok(data) = serde_json::from_str::<serde_json::Value>(&text) {
                data.get("capsules")
                    .and_then(|v| v.as_array())
                    .map(|arr| arr.iter().filter_map(|v| v.as_object().cloned()).collect())
                    .unwrap_or_default()
            } else {
                Vec::new()
            }
        }
        Err(_) => Vec::new(),
    }
}

fn load_badges() -> HashMap<String, serde_json::Value> {
    let path = get_badges_path();
    if !path.exists() {
        return HashMap::new();
    }
    match fs::read_to_string(&path) {
        Ok(text) => serde_json::from_str(&text).unwrap_or_else(|_| HashMap::new()),
        Err(_) => HashMap::new(),
    }
}

fn save_badges(badges: &HashMap<String, serde_json::Value>) -> std::io::Result<()> {
    let path = get_badges_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let json = serde_json::to_string_pretty(badges)?;
    fs::write(path, json)
}

fn check_gap_extractor() -> bool {
    let capsules = load_capsules();
    let active: Vec<_> = capsules
        .iter()
        .filter(|c| {
            c.get("status")
                .and_then(|v| v.as_str())
                .map(|s| s == "active" || s.is_empty())
                .unwrap_or(false)
        })
        .collect();
    active.len() >= 10
}

fn check_evolution_master() -> bool {
    let capsules = load_capsules();
    capsules
        .iter()
        .any(|c| c.get("evolved_from").is_some() || c.get("source_cap_id").is_some())
}

fn check_bold_explorer() -> bool {
    let capsules = load_capsules();
    let bold_types = ["theoretical_gap"];
    let bold_polarity = ["negative"];
    let mut count = 0i32;

    for c in &capsules {
        let gap_type = c
            .get("action_gap_type")
            .or_else(|| c.get("trigger_gap_type"))
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let polarity = c
            .get("polarity")
            .and_then(|v| v.as_str())
            .unwrap_or("positive");

        if bold_types.contains(&gap_type) || bold_polarity.contains(&polarity) {
            count += 1;
        }
        if count >= 5 {
            return true;
        }
    }
    false
}

fn check_rigor_rater() -> bool {
    let flag = dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".ai_research_os")
        .join(".rigor_rated");
    if !flag.exists() {
        return false;
    }
    match fs::read_to_string(&flag) {
        Ok(text) => text.trim().parse::<i32>().map(|n| n >= 10).unwrap_or(false),
        Err(_) => false,
    }
}

pub fn compute_badges() -> Vec<Badge> {
    let checks: Vec<(&str, &str, &str, fn() -> bool)> = vec![
        (
            "contradiction_hunter",
            "Contradiction Hunter",
            "Detect 3+ contradiction pairs",
            || false,
        ),
        (
            "gap_extractor",
            "Gap Extractor",
            "Build Gene Pool to 10+ capsules",
            check_gap_extractor,
        ),
        (
            "evolution_master",
            "Evolution Master",
            "Have 1 capsule evolved",
            check_evolution_master,
        ),
        (
            "bold_explorer",
            "Bold Explorer",
            "Collect 5 bold hypothesis capsules",
            check_bold_explorer,
        ),
        (
            "rigor_rater",
            "Rigor Rater",
            "Score 10+ papers for research rigor",
            check_rigor_rater,
        ),
        (
            "paradigm_sentinel",
            "Paradigm Sentinel",
            "Trigger a paradigm concentration alert",
            || false,
        ),
    ];

    let icons: HashMap<&str, &str> = [
        ("contradiction_hunter", "🎯"),
        ("gap_extractor", "🧬"),
        ("evolution_master", "🔄"),
        ("bold_explorer", "🔴"),
        ("rigor_rater", "🏆"),
        ("paradigm_sentinel", "⚠️"),
    ]
    .into_iter()
    .collect();

    let mut saved = load_badges();
    let mut badges = Vec::new();

    for (bid, name, desc, check_fn) in checks {
        let earned = check_fn();
        let saved_entry = saved.get(bid).and_then(|v| v.as_object());
        let earned_at = if earned {
            let existing_at = saved_entry
                .and_then(|e| e.get("earned_at"))
                .and_then(|v| v.as_str());
            if existing_at.is_some() && !existing_at.unwrap().is_empty() {
                existing_at.map(|s| s.to_string())
            } else {
                let ts = Utc::now().to_rfc3339();
                let mut entry = saved_entry.cloned().unwrap_or_default();
                entry.insert("earned_at".to_string(), serde_json::json!(ts));
                saved.insert(bid.to_string(), serde_json::json!(entry));
                Some(ts)
            }
        } else {
            saved_entry
                .and_then(|e| e.get("earned_at"))
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
        };

        badges.push(Badge::new(
            bid,
            name,
            desc,
            icons.get(bid).unwrap_or(&"?"),
            earned,
            earned_at.as_deref(),
        ));
    }

    let _ = save_badges(&saved);
    badges
}

pub fn render_game_mode_html(badges: Option<Vec<Badge>>) -> String {
    let badges = badges.unwrap_or_else(compute_badges);
    let earned: Vec<_> = badges.iter().filter(|b| b.earned).collect();
    let locked: Vec<_> = badges.iter().filter(|b| !b.earned).collect();

    let mut lines = vec!["<div class=\"game-mode\">".to_string()];
    lines.push("<h3>🎮 Research Game Mode</h3>".to_string());
    lines.push(format!(
        "<p style='font-size:13px;color:#A89E8C;margin-bottom:20px'>{} / {} badges earned</p>",
        earned.len(),
        badges.len()
    ));

    if !earned.is_empty() {
        lines.push("<div class='badge-grid'>".to_string());
        for b in &earned {
            lines.push(format!(
                "<div class='badge-card earned'>\
                 <div class='badge-icon'>{}</div>\
                 <div class='badge-name'>{}</div>\
                 <div class='badge-desc'>{}</div>\
                 </div>",
                b.icon, b.name, b.description
            ));
        }
        lines.push("</div>".to_string());
    }

    if !locked.is_empty() {
        lines.push("<div style='margin-top:16px;font-size:12px;color:#A89E8C;text-transform:uppercase;letter-spacing:0.5px;margin-bottom:8px'>Locked</div>".to_string());
        lines.push("<div class='badge-grid'>".to_string());
        for b in &locked {
            lines.push(format!(
                "<div class='badge-card locked'>\
                 <div class='badge-icon' style='opacity:0.3'>{}</div>\
                 <div class='badge-name' style='color:#A89E8C'>{}</div>\
                 <div class='badge-desc' style='color:#C0B8AE'>{}</div>\
                 </div>",
                b.icon, b.name, b.description
            ));
        }
        lines.push("</div>".to_string());
    }

    lines.extend(vec![
        "<style>".to_string(),
        ".badge-grid { display: grid; grid-template-columns: repeat(auto-fill, minmax(160px, 1fr)); gap: 10px; }".to_string(),
        ".badge-card { border-radius: 8px; padding: 14px; text-align: center; }".to_string(),
        ".badge-card.earned { border: 2px solid #6B8FB5; background: rgba(107,143,181,0.08); }".to_string(),
        ".badge-card.locked { border: 1px dashed #A89E8C; background: rgba(168,158,140,0.04); }".to_string(),
        ".badge-icon { font-size: 28px; margin-bottom: 6px; }".to_string(),
        ".badge-name { font-weight: 700; font-size: 13px; margin-bottom: 4px; color: #2a2a2a; }".to_string(),
        ".badge-desc { font-size: 11px; color: #7a7570; line-height: 1.4; }".to_string(),
        "</style>".to_string(),
        "</div>".to_string(),
    ]);

    lines.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_badge_new() {
        let badge = Badge::new(
            "test",
            "Test Badge",
            "A test badge",
            "🏆",
            true,
            Some("2024-01-01"),
        );
        assert_eq!(badge.id, "test");
        assert_eq!(badge.name, "Test Badge");
        assert!(badge.earned);
        assert!(badge.earned_at.is_some());
    }

    #[test]
    fn test_badge_not_earned() {
        let badge = Badge::new("test", "Test Badge", "A test badge", "🏆", false, None);
        assert!(!badge.earned);
        assert!(badge.earned_at.is_none());
    }

    #[test]
    fn test_compute_badges_returns_vec() {
        let badges = compute_badges();
        assert_eq!(badges.len(), 6);
    }

    #[test]
    fn test_compute_badges_has_required_fields() {
        let badges = compute_badges();
        let ids: Vec<_> = badges.iter().map(|b| b.id.as_str()).collect();
        assert!(ids.contains(&"gap_extractor"));
        assert!(ids.contains(&"evolution_master"));
        assert!(ids.contains(&"bold_explorer"));
    }

    #[test]
    fn test_render_game_mode_html() {
        let badges = vec![
            Badge::new(
                "test1",
                "Test 1",
                "Description 1",
                "🏆",
                true,
                Some("2024-01-01"),
            ),
            Badge::new("test2", "Test 2", "Description 2", "🎯", false, None),
        ];
        let html = render_game_mode_html(Some(badges));
        assert!(html.contains("game-mode"));
        assert!(html.contains("Test 1"));
        assert!(html.contains("Test 2"));
        assert!(html.contains("badge-grid"));
    }

    #[test]
    fn test_render_game_mode_html_earned_count() {
        let badges = vec![
            Badge::new(
                "test1",
                "Test 1",
                "Description 1",
                "🏆",
                true,
                Some("2024-01-01"),
            ),
            Badge::new(
                "test2",
                "Test 2",
                "Description 2",
                "🎯",
                true,
                Some("2024-01-02"),
            ),
            Badge::new("test3", "Test 3", "Description 3", "🎯", false, None),
        ];
        let html = render_game_mode_html(Some(badges));
        assert!(html.contains("2 / 3 badges earned"));
    }
}
