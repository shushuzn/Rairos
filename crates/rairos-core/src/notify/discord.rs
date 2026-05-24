use chrono::Utc;
use serde_json::{json, Value};

use crate::notify::limits;
use crate::notify::payloads::{GapAlertPayload, ParadigmShiftPayload};

pub struct DiscordRenderer;

impl DiscordRenderer {
    const SEVERITY_COLORS: &'static [(&'static str, u32)] =
        &[("high", 0xFF4444), ("medium", 0xFFAA00), ("low", 0x44FF44)];

    const GAP_TYPE_COLORS: &'static [(&'static str, u32)] = &[
        ("method_limitation", 0xCC88FF),
        ("scalability_issue", 0xFF8800),
        ("evaluation_gap", 0x88CCFF),
        ("contradiction", 0xFF4444),
        ("unexplored_application", 0x44FFAA),
        ("dataset_gap", 0xFFFF44),
    ];

    pub(crate) fn color_for(gap_type: &str, severity: &str) -> u32 {
        Self::SEVERITY_COLORS
            .iter()
            .find(|(s, _)| *s == severity)
            .map(|(_, c)| *c)
            .unwrap_or_else(|| {
                Self::GAP_TYPE_COLORS
                    .iter()
                    .find(|(t, _)| *t == gap_type.to_lowercase())
                    .map(|(_, c)| *c)
                    .unwrap_or(0x888888)
            })
    }

    pub fn render_gap_alert(payload: &GapAlertPayload) -> Value {
        let color = Self::color_for(&payload.gap_type, &payload.severity);
        let novelty_pct = (payload.novelty * 100.0) as i32;

        let mut fields: Vec<Value> = vec![
            json!({
                "name": "Gap Type",
                "value": Self::title_case(&payload.gap_type),
                "inline": true
            }),
            json!({
                "name": "Novelty",
                "value": format!("{}%", novelty_pct),
                "inline": true
            }),
        ];

        if payload.confidence > 0.0 {
            fields.push(json!({
                "name": "Confidence",
                "value": format!("{}%", (payload.confidence * 100.0) as i32),
                "inline": true
            }));
        }

        if payload.impact_score > 0.0 {
            fields.push(json!({
                "name": "Impact Score",
                "value": format!("{:.2}", payload.impact_score),
                "inline": true
            }));
        }

        if !payload.supporting_papers.is_empty() {
            let papers: Vec<&str> = payload
                .supporting_papers
                .iter()
                .map(|s| s.as_str())
                .take(3)
                .collect();
            let extra = payload.supporting_papers.len().saturating_sub(3);
            let mut papers_str = papers.join(", ");
            if extra > 0 {
                papers_str.push_str(&format!(" +{} more", extra));
            }
            fields.push(json!({
                "name": "Supporting Papers",
                "value": papers_str,
                "inline": false
            }));
        }

        let title = if payload.title.len() > limits::discord::TITLE_MAX_LEN {
            payload.title[..limits::discord::TITLE_MAX_LEN].to_string()
        } else {
            payload.title.clone()
        };

        let embed = json!({
            "title": format!("🔬 {}", title),
            "description": format!(
                "**{}** novelty gap discovered via **{}**",
                payload.severity.to_uppercase(),
                payload.source
            ),
            "color": color,
            "fields": fields,
            "footer": {
                "text": format!("Rairos Research Agent • {}", Utc::now().format("%Y-%m-%d %H:%M"))
            }
        });

        json!({ "embeds": [embed] })
    }

    pub fn render_paradigm_shift(payload: &ParadigmShiftPayload) -> Value {
        let icon = if payload.alert_type == "contradiction_cluster" {
            "⚠️"
        } else {
            "🔄"
        };
        let color = if payload.severity == "high" {
            0xFF0000
        } else {
            0xFF8800
        };

        let mut fields: Vec<Value> = vec![
            json!({
                "name": "Alert Type",
                "value": Self::title_case(&payload.alert_type),
                "inline": true
            }),
            json!({
                "name": "Severity",
                "value": payload.severity.to_uppercase(),
                "inline": true
            }),
        ];

        let description = if payload.message.len() > limits::discord::MESSAGE_MAX_LEN {
            payload.message[..limits::discord::MESSAGE_MAX_LEN].to_string()
        } else {
            payload.message.clone()
        };
        let footer_text = format!(
            "Rairos Paradigm Watch • {}",
            Utc::now().format("%Y-%m-%d %H:%M")
        );

        let mut embed = json!({
            "title": format!("{} Paradigm Shift Signal: {}", icon, payload.gap_type),
            "description": description,
            "color": color,
            "fields": fields,
            "footer": {
                "text": footer_text
            }
        });

        if !payload.contradictions.is_empty() {
            let c = &payload.contradictions[0];
            let paper_a = if c.paper_a.len() > limits::discord::PAPER_NAME_MAX_LEN {
                format!("{}...", &c.paper_a[..limits::discord::PAPER_NAME_MAX_LEN])
            } else {
                c.paper_a.clone()
            };
            let paper_b = if c.paper_b.len() > limits::discord::PAPER_NAME_MAX_LEN {
                format!("{}...", &c.paper_b[..limits::discord::PAPER_NAME_MAX_LEN])
            } else {
                c.paper_b.clone()
            };
            fields.push(json!({
                "name": "Sample Contradiction",
                "value": format!("Paper A: `{}`\nPaper B: `{}`", paper_a, paper_b),
                "inline": false
            }));
            embed["fields"] = Value::Array(fields);
        }

        json!({ "embeds": [embed] })
    }

    pub fn render_paper_ingested(title: &str, arxiv_id: &str, tags: &[String]) -> Value {
        let title = if title.len() > limits::discord::TITLE_MAX_LEN {
            &title[..limits::discord::TITLE_MAX_LEN]
        } else {
            title
        };

        let mut embed = json!({
            "title": format!("📄 {}", title),
            "description": format!("**arXiv:** `{}`", arxiv_id),
            "color": 0x88CCFF,
            "fields": Value::Array(vec![]),
            "footer": {
                "text": format!("Rairos • {}", Utc::now().format("%Y-%m-%d %H:%M"))
            }
        });

        if !tags.is_empty() {
            let tags_str: Vec<String> = tags.iter().take(8).map(|t| format!("`{}`", t)).collect();
            embed["fields"] = json!([{
                "name": "Tags",
                "value": tags_str.join(" "),
                "inline": false
            }]);
        }

        json!({ "embeds": [embed] })
    }

    pub(crate) fn title_case(s: &str) -> String {
        s.split('_')
            .map(|word| {
                let mut chars = word.chars();
                match chars.next() {
                    None => String::new(),
                    Some(first) => first.to_uppercase().chain(chars).collect(),
                }
            })
            .collect::<Vec<_>>()
            .join(" ")
    }
}
