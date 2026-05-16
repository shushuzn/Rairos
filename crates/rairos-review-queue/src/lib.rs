//! rairos-review-queue — Capsule Review Queue for AI Research OS.
//!
//! Ported from `llm/review_queue.py`.
//!
//! Capsules enter the queue when:
//!   - status is empty/active AND
//!   - has never received a verdict (no feedback entry in feedback store)

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

/// Default path to the capsules JSON file.
pub fn capsules_path() -> PathBuf {
    PathBuf::from(std::env::var("HOME").unwrap_or_else(|_| "~".to_string()))
        .join(".ai_research_os")
        .join("gene_pool")
        .join("capsules.json")
}

/// Default path to the feedback JSON file.
pub fn feedback_path() -> PathBuf {
    PathBuf::from(std::env::var("HOME").unwrap_or_else(|_| "~".to_string()))
        .join(".ai_research_os")
        .join("insights")
        .join("feedback.json")
}

// ============================================================================
// Data Structures
// ============================================================================

/// A capsule awaiting first feedback in the review queue.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueuedCapsule {
    pub capsule_id: String,
    pub gap_title: String,
    pub gap_type: String,
    pub polarity: String,
    pub trigger_keywords: Vec<String>,
    pub outcome_score: f64,
    pub source_paper_id: Option<String>,
    pub created_days_ago: i64,
}

// ============================================================================
// Internal JSON Structures
// ============================================================================

#[derive(Debug, Deserialize)]
struct CapsulesRoot {
    capsules: Vec<CapsuleEntry>,
}

#[derive(Debug, Deserialize)]
struct CapsuleEntry {
    capsule_id: Option<String>,
    status: Option<String>,
    #[serde(rename = "action_gap_title")]
    action_gap_title: Option<String>,
    #[serde(rename = "action_gap_type")]
    action_gap_type: Option<String>,
    polarity: Option<String>,
    #[serde(rename = "trigger_keywords")]
    trigger_keywords: Option<Vec<String>>,
    #[serde(rename = "outcome_success_score")]
    outcome_success_score: Option<f64>,
    #[serde(rename = "source_paper_id")]
    source_paper_id: Option<String>,
    #[serde(rename = "created_at")]
    created_at: Option<String>,
}

#[derive(Debug, Deserialize)]
struct FeedbackRoot {
    #[serde(flatten)]
    entries: std::collections::HashMap<String, FeedbackEntry>,
}

#[derive(Debug, Deserialize)]
struct FeedbackEntry {
    verdict: Option<String>,
    #[serde(rename = "capsule_id")]
    capsule_id: Option<String>,
}

// ============================================================================
// Private Helpers
// ============================================================================

fn _load_capsules() -> Vec<CapsuleEntry> {
    let path = capsules_path();
    if !path.exists() {
        return Vec::new();
    }
    let data = match fs::read_to_string(&path) {
        Ok(s) => s,
        Err(_) => return Vec::new(),
    };
    let root: CapsulesRoot = match serde_json::from_str(&data) {
        Ok(r) => r,
        Err(_) => return Vec::new(),
    };
    root.capsules
}

fn _load_feedback() -> std::collections::HashMap<String, FeedbackEntry> {
    let path = feedback_path();
    if !path.exists() {
        return std::collections::HashMap::new();
    }
    let data = match fs::read_to_string(&path) {
        Ok(s) => s,
        Err(_) => return std::collections::HashMap::new(),
    };
    let root: FeedbackRoot = match serde_json::from_str(&data) {
        Ok(r) => r,
        Err(_) => return std::collections::HashMap::new(),
    };
    root.entries
}

fn _days_ago(ts: &str) -> i64 {
    use chrono::{DateTime, Utc};

    let cleaned = ts.replace('Z', "+00:00");
    match DateTime::parse_from_rfc3339(&cleaned) {
        Ok(dt) => {
            let now = Utc::now();
            let duration = now.signed_duration_since(dt.with_timezone(&Utc));
            duration.num_days()
        }
        Err(_) => 0,
    }
}

fn _short_id(id: &str) -> String {
    if id.len() >= 12 {
        id[..12].to_string()
    } else {
        id.to_string()
    }
}

// ============================================================================
// Public API
// ============================================================================

/// Return all capsules pending first feedback.
///
/// A capsule is queued when:
///   - its status is empty or "active", AND
///   - it has no matching verdict in the feedback store.
///
/// Results are sorted by creation age (oldest first).
pub fn get_review_queue() -> Vec<QueuedCapsule> {
    let capsules = _load_capsules();
    let feedback = _load_feedback();

    // Build set of capsules that have received a verdict
    let mut verdicted: std::collections::HashSet<String> = std::collections::HashSet::new();
    for entry in feedback.values() {
        if entry.verdict.as_ref().is_some_and(|v| !v.is_empty()) {
            if let Some(ref cid) = entry.capsule_id {
                if !cid.is_empty() {
                    verdicted.insert(_short_id(cid));
                }
            }
        }
    }

    let mut results: Vec<QueuedCapsule> = Vec::new();

    for cap in capsules {
        let status = cap.status.as_deref().unwrap_or("");
        if !status.is_empty() && status != "active" {
            continue;
        }

        let cid = cap.capsule_id.as_deref().unwrap_or("");
        if cid.is_empty() {
            continue;
        }

        // Skip if already verdicted
        if verdicted.contains(&_short_id(cid)) {
            continue;
        }

        let created = cap.created_at.as_deref().unwrap_or("");

        results.push(QueuedCapsule {
            capsule_id: cid.to_string(),
            gap_title: cap.action_gap_title.as_deref().unwrap_or("").to_string(),
            gap_type: cap.action_gap_type.as_deref().unwrap_or("").to_string(),
            polarity: cap.polarity.as_deref().unwrap_or("positive").to_string(),
            trigger_keywords: cap
                .trigger_keywords
                .as_ref()
                .map(|v| v.iter().take(5).cloned().collect())
                .unwrap_or_default(),
            outcome_score: cap.outcome_success_score.unwrap_or(0.0),
            source_paper_id: cap.source_paper_id.clone(),
            created_days_ago: _days_ago(created),
        });
    }

    results.sort_by_key(|c| std::cmp::Reverse(c.created_days_ago));
    results
}

/// Render the review queue as an HTML string.
///
/// If `queue` is `None`, it is populated by calling `get_review_queue()`.
pub fn render_review_queue_html(queue: Option<Vec<QueuedCapsule>>) -> String {
    let queue = queue.unwrap_or_else(get_review_queue);

    let mut lines: Vec<String> = Vec::new();
    lines.push("<div class=\"review-queue\">".to_string());
    lines.push("<h3>📋 Capsule Review Queue</h3>".to_string());

    if queue.is_empty() {
        lines.push(
            "<p style='font-size:14px;color:#A89E8C'>All capsules reviewed! 🎉 Check back \
             after extracting gaps from new papers.</p>"
                .to_string(),
        );
    } else {
        lines.push(format!(
            "<p style='font-size:13px;color:#A89E8C;margin-bottom:16px'>{} capsules pending \
             review</p>",
            queue.len()
        ));

        for c in &queue {
            let age_str = if c.created_days_ago > 0 {
                format!("{}d ago", c.created_days_ago)
            } else {
                "today".to_string()
            };
            let kw_str = c
                .trigger_keywords
                .iter()
                .take(4)
                .map(|kw| format!("<code>{}</code>", kw))
                .collect::<Vec<_>>()
                .join(", ");

            let html = format!(
                r#"<div style="border: 1px solid #e0dbd4; border-radius: 6px; padding: 14px; margin-bottom: 12px; background: rgba(107,143,181,0.04);">
  <div style="display: flex; justify-content: space-between; align-items: flex-start; margin-bottom: 6px;">
    <div>
      <span style="font-size: 10px; background: var(--pen-blue); color: white; padding: 1px 6px; border-radius: 2px; margin-right: 6px;">{}</span>
      <span style="font-size: 10px; color: #A89E8C; margin-left: 4px;">{}</span>
    </div>
    <span style="font-size: 11px; color: #A89E8C;">{}</span>
  </div>
  <div style="font-size: 14px; font-weight: 600; color: #2a2a2a; margin-bottom: 6px; line-height: 1.4;">{}</div>
  <div style="font-size: 11px; color: #7a7570; margin-bottom: 8px;">{}</div>
  <div style="display: flex; gap: 8px; align-items: center;">
    <button onclick="submitVerdict('{}', 'match')"
      style="background: #6B8FB5; color: white; border: none; border-radius: 4px; padding: 5px 14px; cursor: pointer; font-size: 12px;">
      ✅ Match
    </button>
    <button onclick="submitVerdict('{}', 'partial')"
      style="background: transparent; color: #D4A055; border: 1px solid #D4A055; border-radius: 4px; padding: 5px 14px; cursor: pointer; font-size: 12px;">
      ⚠️ Partial
    </button>
    <button onclick="submitVerdict('{}', 'not_relevant')"
      style="background: transparent; color: #A89E8C; border: 1px solid #ccc; border-radius: 4px; padding: 5px 14px; cursor: pointer; font-size: 12px;">
      ❌ Not Relevant
    </button>
  </div>
</div>"#,
                c.gap_type,
                c.polarity,
                age_str,
                html_escape(&c.gap_title),
                kw_str,
                c.capsule_id,
                c.capsule_id,
                c.capsule_id
            );
            lines.push(html);
        }
    }

    lines.push(
        r#"<script>
function submitVerdict(capsuleId, verdict) {
    fetch('/insights/queue/verdict', {
        method: 'POST',
        headers: {'Content-Type': 'application/json'},
        body: JSON.stringify({capsule_id: capsuleId, verdict: verdict})
    }).then(r => r.json()).then(d => {
        if (d.success) location.reload();
        else alert('Error: ' + (d.error || 'unknown'));
    });
}
</script>"#
        .to_string(),
    );

    lines.push("<style>".to_string());
    lines.push(".review-queue { font-family: Georgia, serif; }".to_string());
    lines.push("</style>".to_string());
    lines.push("</div>".to_string());

    lines.join("\n")
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    

    #[test]
    fn test_short_id() {
        assert_eq!(_short_id("abc"), "abc");
        assert_eq!(_short_id("abcdefghijklm"), "abcdefghijkl");
        assert_eq!(_short_id(""), "");
    }

    #[test]
    fn test_days_ago_invalid() {
        assert_eq!(_days_ago("not-a-date"), 0);
        assert_eq!(_days_ago(""), 0);
    }

    #[test]
    fn test_days_ago_valid() {
        use chrono::{Duration, Utc};

        let past = Utc::now() - Duration::days(5);
        let ts = past.to_rfc3339();
        let days = _days_ago(&ts);
        assert!((4..=5).contains(&days));
    }

    #[test]
    fn test_render_html_empty() {
        let html = render_review_queue_html(Some(vec![]));
        assert!(html.contains("All capsules reviewed"));
    }

    #[test]
    fn test_render_html_with_capsules() {
        let capsules = vec![QueuedCapsule {
            capsule_id: "test-capsule-001234".to_string(),
            gap_title: "Missing interpretability analysis".to_string(),
            gap_type: "reasoning".to_string(),
            polarity: "positive".to_string(),
            trigger_keywords: vec!["attention".to_string(), "transformer".to_string()],
            outcome_score: 0.85,
            source_paper_id: Some("arxiv:1234.5678".to_string()),
            created_days_ago: 3,
        }];
        let html = render_review_queue_html(Some(capsules));
        assert!(html.contains("Capsule Review Queue"));
        assert!(html.contains("Missing interpretability analysis"));
        assert!(html.contains("reasoning"));
        assert!(html.contains("3d ago"));
        assert!(html.contains("test-capsule-001234"));
    }

    #[test]
    fn test_get_review_queue_empty() {
        // With no files present, should return empty
        let queue = get_review_queue();
        assert!(queue.is_empty());
    }

    #[test]
    fn test_get_review_queue_skips_verdicted() {
        let temp_dir = tempfile::tempdir().unwrap();

        // Set HOME BEFORE building paths so capsules_path() resolves correctly
        std::env::set_var("HOME", temp_dir.path().to_str().unwrap());

        // Build paths AFTER setting HOME (so they resolve to temp dir)
        let capsules_file = capsules_path();
        let feedback_file = feedback_path();

        // Create parent directories if they don't exist
        if let Some(parent) = capsules_file.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        if let Some(parent) = feedback_file.parent() {
            fs::create_dir_all(parent).unwrap();
        }

        // Write capsules
        let capsules_data = serde_json::json!({
            "capsules": [
                {
                    "capsule_id": "verdicted-capsule-01",
                    "status": "active",
                    "action_gap_title": "Should be skipped",
                    "action_gap_type": "reasoning",
                    "polarity": "positive",
                    "trigger_keywords": [],
                    "outcome_success_score": 0.5,
                    "created_at": "2024-01-01T00:00:00Z"
                },
                {
                    "capsule_id": "pending-capsule-02",
                    "status": "active",
                    "action_gap_title": "Should appear",
                    "action_gap_type": "factual",
                    "polarity": "negative",
                    "trigger_keywords": ["test"],
                    "outcome_success_score": 0.8,
                    "created_at": "2024-01-02T00:00:00Z"
                }
            ]
        });
        fs::write(&capsules_file, capsules_data.to_string()).unwrap();

        // Write feedback for the first capsule
        let feedback_data = serde_json::json!({
            "entry1": {
                "verdict": "match",
                "capsule_id": "verdicted-capsule-01"
            }
        });
        fs::write(&feedback_file, feedback_data.to_string()).unwrap();

        let queue = get_review_queue();

        // Only the pending capsule should appear
        assert_eq!(queue.len(), 1);
        assert_eq!(queue[0].capsule_id, "pending-capsule-02");
        assert_eq!(queue[0].gap_title, "Should appear");

        // Cleanup
        std::env::remove_var("HOME");
    }

    #[test]
    fn test_render_html_escape() {
        // HTML special chars in title should be escaped
        let capsules = vec![QueuedCapsule {
            capsule_id: "test-id-001234".to_string(),
            gap_title: "A <b>test</b> & title".to_string(),
            gap_type: "type".to_string(),
            polarity: "pos".to_string(),
            trigger_keywords: vec![],
            outcome_score: 0.0,
            source_paper_id: None,
            created_days_ago: 0,
        }];
        let html = render_review_queue_html(Some(capsules));
        // Should contain escaped version, not raw HTML
        assert!(html.contains("&lt;b&gt;test&lt;/b&gt;"));
        assert!(!html.contains("<b>test</b>"));
    }
}
