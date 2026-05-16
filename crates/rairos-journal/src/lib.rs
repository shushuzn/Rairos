//! rairos-journal — Research journal for tracking activities and thoughts.
//!
//! Ported from `llm/journal.py`.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JournalEntry {
    pub id: String,
    pub content: String,
    #[serde(default)]
    pub created_at: String,
    #[serde(default)]
    pub updated_at: String,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub question_id: String,
    #[serde(default)]
    pub experiment_id: String,
    #[serde(default)]
    pub paper_id: String,
    #[serde(default)]
    pub mood: String,
    #[serde(default)]
    pub highlights: Vec<String>,
}

impl JournalEntry {
    pub fn new(content: &str) -> Self {
        let now = Utc::now().to_rfc3339();
        Self {
            id: uuid::Uuid::new_v4().to_string()[..8].to_string(),
            content: content.to_string(),
            created_at: now.clone(),
            updated_at: now,
            tags: Vec::new(),
            question_id: String::new(),
            experiment_id: String::new(),
            paper_id: String::new(),
            mood: String::new(),
            highlights: Vec::new(),
        }
    }

    pub fn with_tags(mut self, tags: Vec<String>) -> Self {
        self.tags = tags;
        self
    }

    pub fn with_mood(mut self, mood: &str) -> Self {
        self.mood = mood.to_string();
        self
    }

    pub fn with_question_id(mut self, question_id: &str) -> Self {
        self.question_id = question_id.to_string();
        self
    }

    pub fn with_experiment_id(mut self, experiment_id: &str) -> Self {
        self.experiment_id = experiment_id.to_string();
        self
    }

    pub fn with_paper_id(mut self, paper_id: &str) -> Self {
        self.paper_id = paper_id.to_string();
        self
    }
}

pub struct Journal {
    data_file: PathBuf,
}

impl Journal {
    pub fn new(data_dir: Option<PathBuf>) -> Self {
        let dir = data_dir.unwrap_or_else(|| {
            dirs::home_dir()
                .unwrap_or_else(|| PathBuf::from("."))
                .join(".ai_research_os")
                .join("journal")
        });
        let _ = fs::create_dir_all(&dir);
        Self {
            data_file: dir.join("journal.json"),
        }
    }

    fn load_entries(&self) -> Vec<JournalEntry> {
        if !self.data_file.exists() {
            return Vec::new();
        }
        match fs::read_to_string(&self.data_file) {
            Ok(text) => {
                if text.trim().is_empty() {
                    return Vec::new();
                }
                serde_json::from_str(&text).unwrap_or_else(|_| Vec::new())
            }
            Err(_) => Vec::new(),
        }
    }

    fn save_entries(&self, entries: &[JournalEntry]) -> bool {
        if let Some(parent) = self.data_file.parent() {
            if fs::create_dir_all(parent).is_err() {
                return false;
            }
        }
        match serde_json::to_string_pretty(entries) {
            Ok(json) => {
                let tmp = self.data_file.with_extension("tmp");
                if fs::write(&tmp, json).is_ok() && fs::rename(&tmp, &self.data_file).is_ok() {
                    return true;
                }
                false
            }
            Err(_) => false,
        }
    }

    pub fn add(&self, content: &str) -> Option<JournalEntry> {
        let entry = JournalEntry::new(content);
        let mut entries = self.load_entries();
        entries.push(entry.clone());
        if self.save_entries(&entries) {
            Some(entry)
        } else {
            None
        }
    }

    pub fn get(&self, entry_id: &str) -> Option<JournalEntry> {
        self.load_entries().into_iter().find(|e| e.id == entry_id)
    }

    pub fn update(
        &self,
        entry_id: &str,
        content: Option<&str>,
        tags: Option<Vec<String>>,
    ) -> Option<JournalEntry> {
        let mut entries = self.load_entries();
        let mut found = false;
        let mut updated_entry: Option<JournalEntry> = None;

        for entry in entries.iter_mut() {
            if entry.id == entry_id {
                if let Some(c) = content {
                    entry.content = c.to_string();
                }
                if let Some(t) = tags {
                    entry.tags = t;
                }
                entry.updated_at = Utc::now().to_rfc3339();
                updated_entry = Some(entry.clone());
                found = true;
                break;
            }
        }

        if found && self.save_entries(&entries) {
            return updated_entry;
        }
        None
    }

    pub fn delete(&self, entry_id: &str) -> bool {
        let mut entries = self.load_entries();
        let original_len = entries.len();
        entries.retain(|e| e.id != entry_id);
        if entries.len() < original_len {
            self.save_entries(&entries)
        } else {
            false
        }
    }

    pub fn list_entries(
        &self,
        limit: usize,
        tag: Option<&str>,
        question_id: Option<&str>,
        experiment_id: Option<&str>,
        today: bool,
        days: i64,
    ) -> Vec<JournalEntry> {
        let mut entries = self.load_entries();

        if today {
            let today_str = Utc::now().format("%Y-%m-%d").to_string();
            entries.retain(|e| e.created_at.starts_with(&today_str));
        } else if days > 0 {
            let cutoff = Utc::now() - chrono::Duration::days(days);
            let cutoff_str = cutoff.to_rfc3339();
            entries.retain(|e| e.created_at >= cutoff_str);
        }

        if let Some(t) = tag {
            let t_lower = t.to_lowercase();
            entries.retain(|e| e.tags.iter().any(|tag| tag.to_lowercase() == t_lower));
        }
        if let Some(qid) = question_id {
            entries.retain(|e| e.question_id == qid);
        }
        if let Some(eid) = experiment_id {
            entries.retain(|e| e.experiment_id == eid);
        }

        entries.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        entries.truncate(limit);
        entries
    }

    pub fn search(&self, query: &str, limit: usize) -> Vec<JournalEntry> {
        let q_lower = query.to_lowercase();
        let mut entries: Vec<_> = self
            .load_entries()
            .into_iter()
            .filter(|e| e.content.to_lowercase().contains(&q_lower))
            .collect();
        entries.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        entries.truncate(limit);
        entries
    }

    pub fn stats(&self) -> HashMap<String, serde_json::Value> {
        let entries = self.load_entries();
        if entries.is_empty() {
            return HashMap::from([
                ("total".into(), serde_json::json!(0)),
                ("this_week".into(), serde_json::json!(0)),
                ("this_month".into(), serde_json::json!(0)),
            ]);
        }

        let now = Utc::now();
        let week_ago = now - chrono::Duration::days(7);
        let month_ago = now - chrono::Duration::days(30);

        let mut tags_count: HashMap<String, usize> = HashMap::new();
        let mut mood_count: HashMap<String, usize> = HashMap::new();

        for e in &entries {
            for t in &e.tags {
                *tags_count.entry(t.clone()).or_insert(0) += 1;
            }
            if !e.mood.is_empty() {
                *mood_count.entry(e.mood.clone()).or_insert(0) += 1;
            }
        }

        let this_week = entries
            .iter()
            .filter(|e| {
                if let Ok(dt) = DateTime::parse_from_rfc3339(&e.created_at) {
                    dt.with_timezone(&Utc) >= week_ago
                } else {
                    false
                }
            })
            .count();

        let this_month = entries
            .iter()
            .filter(|e| {
                if let Ok(dt) = DateTime::parse_from_rfc3339(&e.created_at) {
                    dt.with_timezone(&Utc) >= month_ago
                } else {
                    false
                }
            })
            .count();

        let mut top_tags: Vec<_> = tags_count.into_iter().collect();
        top_tags.sort_by_key(|b| std::cmp::Reverse(b.1));
        top_tags.truncate(10);

        let top_tags_json: Vec<Vec<serde_json::Value>> = top_tags
            .into_iter()
            .map(|(k, v)| vec![serde_json::Value::String(k), serde_json::json!(v)])
            .collect();

        HashMap::from([
            ("total".into(), serde_json::json!(entries.len())),
            ("this_week".into(), serde_json::json!(this_week)),
            ("this_month".into(), serde_json::json!(this_month)),
            ("top_tags".into(), serde_json::json!(top_tags_json)),
            ("mood_distribution".into(), serde_json::json!(mood_count)),
        ])
    }

    pub fn render_list(&self, entries: &[JournalEntry], verbose: bool) -> String {
        if entries.is_empty() {
            return "No journal entries.".to_string();
        }

        let mood_icons: HashMap<&str, &str> = HashMap::from([
            ("productive", "⚡"),
            ("stuck", "😓"),
            ("excited", "🎉"),
            ("neutral", "📝"),
        ]);

        let mut lines = Vec::new();
        for e in entries {
            let icon = mood_icons.get(e.mood.as_str()).unwrap_or(&"📝");
            let date = &e.created_at[..10];
            lines.push(format!(
                "{} [{}] {}",
                icon,
                date,
                &e.content[..e.content.len().min(80)]
            ));
            if verbose {
                if !e.tags.is_empty() {
                    lines.push(format!("   Tags: {}", e.tags.join(", ")));
                }
                if !e.question_id.is_empty() {
                    lines.push(format!("   Question: {}", e.question_id));
                }
                if !e.experiment_id.is_empty() {
                    lines.push(format!("   Experiment: {}", e.experiment_id));
                }
            }
        }
        lines.join("\n")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_journal() -> (Journal, tempfile::TempDir) {
        let temp_dir = tempfile::tempdir().unwrap();
        let journal = Journal::new(Some(temp_dir.path().to_path_buf()));
        (journal, temp_dir)
    }

    #[test]
    fn test_add_entry() {
        let (journal, _td) = temp_journal();
        let entry = journal.add("Test content");
        assert!(entry.is_some());
        let entry = entry.unwrap();
        assert_eq!(entry.content, "Test content");
        assert!(!entry.id.is_empty());
    }

    #[test]
    fn test_get_entry() {
        let (journal, _td) = temp_journal();
        let entry = journal.add("Test content").unwrap();
        let retrieved = journal.get(&entry.id);
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().content, "Test content");
    }

    #[test]
    fn test_update_entry() {
        let (journal, _td) = temp_journal();
        let entry = journal.add("Original content").unwrap();
        let updated = journal.update(&entry.id, Some("Updated content"), None);
        assert!(updated.is_some());
        assert_eq!(updated.unwrap().content, "Updated content");
    }

    #[test]
    fn test_delete_entry() {
        let (journal, _td) = temp_journal();
        let entry = journal.add("To be deleted").unwrap();
        assert!(journal.delete(&entry.id));
        assert!(journal.get(&entry.id).is_none());
    }

    #[test]
    fn test_list_entries_with_tag() {
        let (journal, _td) = temp_journal();
        let entry = JournalEntry::new("Content with tag").with_tags(vec!["research".to_string()]);
        let mut entries = journal.load_entries();
        entries.push(entry);
        journal.save_entries(&entries);

        let filtered = journal.list_entries(50, Some("research"), None, None, false, 0);
        assert!(!filtered.is_empty());
    }

    #[test]
    fn test_search_entries() {
        let (journal, _td) = temp_journal();
        journal.add("This contains the word apple");
        journal.add("This contains the word banana");
        let results = journal.search("apple", 10);
        assert_eq!(results.len(), 1);
        assert!(results[0].content.contains("apple"));
    }

    #[test]
    fn test_stats() {
        let (journal, _td) = temp_journal();
        journal.add("Entry 1");
        let stats = journal.stats();
        assert_eq!(stats.get("total").unwrap().as_i64().unwrap(), 1);
    }

    #[test]
    fn test_render_list() {
        let (journal, _td) = temp_journal();
        let entry = journal.add("Test entry").unwrap();
        let rendered = journal.render_list(&[entry], false);
        assert!(rendered.contains("Test entry"));
    }

    #[test]
    fn test_entry_with_all_fields() {
        let entry = JournalEntry::new("Full entry")
            .with_tags(vec!["tag1".to_string(), "tag2".to_string()])
            .with_mood("productive")
            .with_question_id("q123")
            .with_experiment_id("e456")
            .with_paper_id("p789");

        assert_eq!(entry.content, "Full entry");
        assert_eq!(entry.tags, vec!["tag1", "tag2"]);
        assert_eq!(entry.mood, "productive");
        assert_eq!(entry.question_id, "q123");
        assert_eq!(entry.experiment_id, "e456");
        assert_eq!(entry.paper_id, "p789");
    }
}
