use serde_json::json;
use serde_json::Value;

use crate::payloads::{GapAlertPayload, ParadigmShiftPayload};

pub struct FeishuRenderer;

impl FeishuRenderer {
    pub(crate) fn severity_emoji(severity: &str) -> &'static str {
        match severity {
            "high" => "🔴",
            "medium" => "🟡",
            "low" => "🟢",
            _ => "⚪",
        }
    }

    pub(crate) fn feishu_template(severity: &str) -> &'static str {
        match severity {
            "high" => "red",
            "medium" => "yellow",
            "low" => "green",
            _ => "grey",
        }
    }

    pub fn render_gap_alert(payload: &GapAlertPayload) -> Value {
        let novelty_pct = (payload.novelty * 100.0) as i32;
        let severity_emoji = Self::severity_emoji(&payload.severity);

        let mut elements: Vec<Value> = vec![
            json!({
                "tag": "markdown",
                "content": format!("**Gap Type:** {}", Self::title_case(&payload.gap_type))
            }),
            json!({
                "tag": "markdown",
                "content": format!(
                    "**Novelty:** {}% | **Severity:** {} **{}**",
                    novelty_pct,
                    severity_emoji,
                    payload.severity.to_uppercase()
                )
            }),
        ];

        if payload.confidence > 0.0 {
            elements.push(json!({
                "tag": "markdown",
                "content": format!("**Confidence:** {}%", (payload.confidence * 100.0) as i32)
            }));
        }

        if payload.impact_score > 0.0 {
            elements.push(json!({
                "tag": "markdown",
                "content": format!("**Impact Score:** {:.2}", payload.impact_score)
            }));
        }

        if !payload.supporting_papers.is_empty() {
            let papers: Vec<String> = payload
                .supporting_papers
                .iter()
                .take(5)
                .map(|pid| format!("- `{}`", pid))
                .collect();
            elements.push(json!({
                "tag": "markdown",
                "content": format!("**Supporting Papers:**\n{}", papers.join("\n"))
            }));
        }

        elements.push(json!({
            "tag": "note",
            "elements": [{
                "tag": "plain_text",
                "content": format!("Source: {}", payload.source)
            }]
        }));

        let title = if payload.title.len() > 100 {
            payload.title[..100].to_string()
        } else {
            payload.title.clone()
        };

        json!({
            "msg_type": "interactive",
            "card": {
                "header": {
                    "title": {
                        "tag": "plain_text",
                        "content": format!("🔬 {}", title)
                    },
                    "template": Self::feishu_template(&payload.severity)
                },
                "elements": elements
            }
        })
    }

    pub fn render_paradigm_shift(payload: &ParadigmShiftPayload) -> Value {
        let icon = if payload.alert_type == "contradiction_cluster" {
            "⚠️"
        } else {
            "🔄"
        };
        let severity_emoji = Self::severity_emoji(&payload.severity);
        let template = if payload.severity == "high" {
            "red"
        } else {
            "yellow"
        };

        let message = if payload.message.len() > 2000 {
            payload.message[..2000].to_string()
        } else {
            payload.message.clone()
        };

        json!({
            "msg_type": "interactive",
            "card": {
                "header": {
                    "title": {
                        "tag": "plain_text",
                        "content": format!("{} Paradigm Shift: {}", icon, payload.gap_type)
                    },
                    "template": template
                },
                "elements": [
                    { "tag": "markdown", "content": message },
                    {
                        "tag": "markdown",
                        "content": format!(
                            "**Alert Type:** {}\n**Severity:** {} **{}**",
                            Self::title_case(&payload.alert_type),
                            severity_emoji,
                            payload.severity.to_uppercase()
                        )
                    }
                ]
            }
        })
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
