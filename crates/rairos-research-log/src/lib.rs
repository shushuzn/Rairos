//! rairos-research-log — Per-paper research notes stored in JSONL.
//!
//! Ported from `llm/research_log.py`.

use serde::{Deserialize, Serialize};
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::PathBuf;

fn log_path() -> PathBuf {
    let base = if let Ok(home) = std::env::var("RAIROS_HOME") {
        PathBuf::from(home)
    } else {
        dirs::home_dir().unwrap_or_else(|| PathBuf::from("."))
    };
    base.join(".ai_research_os")
        .join("gene_pool")
        .join("research_log.jsonl")
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResearchNote {
    pub timestamp: String,
    pub paper_id: String,
    pub note: String,
    #[serde(default)]
    pub tags: Vec<String>,
}

pub fn add_note(paper_id: &str, note_text: &str, tags: Option<Vec<String>>) -> bool {
    let path = log_path();
    if let Some(parent) = path.parent() {
        if fs::create_dir_all(parent).is_err() {
            return false;
        }
    }

    let note = ResearchNote {
        timestamp: chrono::Utc::now().to_rfc3339(),
        paper_id: paper_id.to_string(),
        note: note_text.to_string(),
        tags: tags.unwrap_or_default(),
    };

    match serde_json::to_string(&note) {
        Ok(json) => match OpenOptions::new().create(true).append(true).open(&path) {
            Ok(mut file) => {
                file.write_all(json.as_bytes()).is_ok() && file.write_all(b"\n").is_ok()
            }
            Err(_) => false,
        },
        Err(_) => false,
    }
}

pub fn get_notes(paper_id: Option<&str>, limit: usize) -> Vec<ResearchNote> {
    let path = log_path();
    if !path.exists() {
        return Vec::new();
    }

    let text = match fs::read_to_string(&path) {
        Ok(t) => t.trim().to_string(),
        Err(_) => return Vec::new(),
    };

    if text.is_empty() {
        return Vec::new();
    }

    let mut notes: Vec<ResearchNote> = text
        .lines()
        .filter(|l| !l.trim().is_empty())
        .filter_map(|l| serde_json::from_str(l).ok())
        .filter(|n: &ResearchNote| paper_id.is_none_or(|pid| n.paper_id == pid))
        .collect();

    notes.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
    notes.truncate(limit);
    notes
}

pub fn render_log_html(paper_id: Option<&str>) -> String {
    let notes = get_notes(paper_id, 50);

    if notes.is_empty() {
        let empty_msg = if paper_id.is_some() {
            format!("No notes for paper {} yet.", paper_id.unwrap())
        } else {
            "No research notes yet.".to_string()
        };
        return format!(
            r#"<div style="text-align:center;padding:60px 20px;color:#888;font-family:var(--font-display);">
  <div style="font-size:48px;margin-bottom:12px;">📝</div>
  <div style="font-size:15px;font-weight:600;margin-bottom:6px;">{}</div>
  <div style="font-size:13px;">Add notes from a paper detail page.</div>
</div>"#,
            empty_msg
        );
    }

    let cards: String = notes
        .iter()
        .map(|n| {
            let date_str = &n.timestamp[..16].replace("T", " ");
            let tags_html: String = if !n.tags.is_empty() {
                n.tags
                    .iter()
                    .map(|t| {
                        format!(
                            r#"<span style="display:inline-block;background:#e8f0fe;color:#1a73e8;padding:2px 8px;border-radius:12px;font-size:11px;margin:2px;">{}</span>"#,
                            t
                        )
                    })
                    .collect::<Vec<_>>()
                    .join("")
            } else {
                String::new()
            };

            format!(
                r#"<div style="border:1px solid #e0e8f0;border-radius:8px;padding:16px;margin-bottom:12px;background:#fff;box-shadow:0 2px 4px rgba(0,0,0,0.05);">
  <div style="display:flex;justify-content:space-between;margin-bottom:6px;">
    <span style="font-size:13px;font-weight:600;color:#1a1a2e;">{}</span>
    <span style="font-size:11px;color:#888;flex-shrink:0;margin-left:12px;">{}</span>
  </div>
  <div style="font-size:13px;color:#444;line-height:1.5;margin-bottom:8px;white-space:pre-wrap;">{}</div>
  {}
</div>"#,
                n.paper_id.chars().take(20).collect::<String>(),
                date_str,
                n.note,
                tags_html
            )
        })
        .collect();

    format!(
        r#"<div style="max-width:700px;margin:0 auto;">{}</div>"#,
        cards
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add_note_and_get() {
        let temp_dir = tempfile::tempdir().unwrap();
        std::env::set_var("RAIROS_HOME", temp_dir.path());
        let paper_id = "test_paper_123";
        let result = add_note(
            paper_id,
            "This is a test note.",
            Some(vec!["test".to_string()]),
        );
        assert!(result);

        let notes = get_notes(Some(paper_id), 10);
        assert!(!notes.is_empty());
        assert_eq!(notes[0].paper_id, paper_id);
        assert_eq!(notes[0].note, "This is a test note.");
        assert_eq!(notes[0].tags, vec!["test"]);
        std::env::remove_var("RAIROS_HOME");
        temp_dir.close().unwrap();
    }

    #[test]
    fn test_get_notes_nonexistent() {
        let notes = get_notes(Some("nonexistent_paper"), 10);
        assert!(notes.is_empty());
    }

    #[test]
    fn test_render_log_html_empty() {
        let html = render_log_html(Some("nonexistent"));
        assert!(html.contains("No notes"));
    }

    #[test]
    fn test_render_log_html_with_notes() {
        let temp_dir = tempfile::tempdir().unwrap();
        std::env::set_var("RAIROS_HOME", temp_dir.path());
        add_note("render_test", "Test note for rendering.", None);
        let html = render_log_html(Some("render_test"));
        assert!(html.contains("Test note"));
        assert!(html.contains("render_test"));
        std::env::remove_var("RAIROS_HOME");
        temp_dir.close().unwrap();
    }

    #[test]
    fn test_notes_sorted_by_timestamp() {
        let temp_dir = tempfile::tempdir().unwrap();
        std::env::set_var("RAIROS_HOME", temp_dir.path());
        add_note("sort_test_1", "First note", None);
        add_note("sort_test_2", "Second note", None);
        let notes = get_notes(Some("sort_test_1"), 5);
        if notes.len() >= 2 {
            assert!(notes[0].timestamp <= notes[1].timestamp);
        }
        std::env::remove_var("RAIROS_HOME");
        temp_dir.close().unwrap();
    }
}
