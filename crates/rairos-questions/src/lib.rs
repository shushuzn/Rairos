//! rairos-questions — Research question tracker.
//!
//! Ported from `llm/question_tracker.py` + `cli/cmd/question.py`.
//! JSON file format is backward-compatible with the Python version:
//! `~/.ai_research_os/questions/questions.json`.

use anyhow::{Context, Result};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::io::Write;
use std::path::PathBuf;

// ── Enums ──────────────────────────────────────────────────────────────────

/// Python-compatible question status values.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum QuestionStatus {
    Open,
    InProgress,
    Resolved,
    Wontfix,
}

impl QuestionStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            QuestionStatus::Open => "open",
            QuestionStatus::InProgress => "in_progress",
            QuestionStatus::Resolved => "resolved",
            QuestionStatus::Wontfix => "wontfix",
        }
    }
}

/// Python-compatible question source values.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum QuestionSource {
    Manual,
    GapDetection,
    Hypothesis,
    LiteratureReview,
}

impl QuestionSource {
    pub fn as_str(&self) -> &'static str {
        match self {
            QuestionSource::Manual => "manual",
            QuestionSource::GapDetection => "gap_detection",
            QuestionSource::Hypothesis => "hypothesis",
            QuestionSource::LiteratureReview => "literature_review",
        }
    }
}

// ── Data model ─────────────────────────────────────────────────────────────

/// Mirrors Python's `ResearchQuestion` dataclass fields exactly.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResearchQuestion {
    pub id: String,
    pub question: String,
    pub source: QuestionSource,
    pub status: QuestionStatus,
    #[serde(default)]
    pub related_papers: Vec<String>,
    pub created_at: String,
    pub updated_at: String,
    #[serde(default)]
    pub notes: String,
    #[serde(default = "default_priority")]
    pub priority: u8,
    #[serde(default)]
    pub topic: String,
}

fn default_priority() -> u8 {
    5
}

// ── Tracker ────────────────────────────────────────────────────────────────

/// Persistent research question tracker.
///
/// In-memory: `HashMap` for O(1) lookup.
/// On disk: JSON array (Vec) for Python file-format compatibility.
pub struct QuestionTracker {
    file_path: PathBuf,
    pub(crate) questions: HashMap<String, ResearchQuestion>,
}

impl QuestionTracker {
    /// Create or load from `~/.ai_research_os/questions/questions.json`.
    pub fn new() -> Result<Self> {
        let data_dir = Self::default_data_dir();
        fs::create_dir_all(&data_dir)?;
        Self::open(data_dir.join("questions.json"))
    }

    /// Open from a specific file path (useful for testing).
    pub fn open(file_path: PathBuf) -> Result<Self> {
        let questions = if file_path.exists() {
            let content = fs::read_to_string(&file_path)?;
            if content.trim().is_empty() {
                HashMap::new()
            } else {
                let list: Vec<ResearchQuestion> = serde_json::from_str(&content).context(
                    format!("Failed to parse questions.json at {}", file_path.display()),
                )?;
                list.into_iter().map(|q| (q.id.clone(), q)).collect()
            }
        } else {
            HashMap::new()
        };

        Ok(Self { file_path, questions })
    }

    /// Path matched to Python's `~/.ai_research_os/questions/`.
    fn default_data_dir() -> PathBuf {
        dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(".ai_research_os")
            .join("questions")
    }

    // ── CRUD ────────────────────────────────────────────────────────────

    /// Add a new question. Signature matches Python's `QuestionTracker.add()`.
    pub fn add(
        &mut self,
        question: String,
        source: QuestionSource,
        topic: String,
        priority: u8,
        notes: String,
    ) -> ResearchQuestion {
        let now = Utc::now().format("%Y-%m-%dT%H:%M:%S").to_string();
        let id = &uuid::Uuid::new_v4().to_string()[..8];
        let entry = ResearchQuestion {
            id: id.to_string(),
            question,
            source,
            status: QuestionStatus::Open,
            related_papers: Vec::new(),
            created_at: now.clone(),
            updated_at: now,
            notes,
            priority: priority.clamp(1, 10),
            topic,
        };
        self.questions.insert(entry.id.clone(), entry.clone());
        entry
    }

    /// Get a question by ID.
    pub fn get(&self, id: &str) -> Option<&ResearchQuestion> {
        self.questions.get(id)
    }

    /// Get a mutable reference for in-place editing.
    pub fn get_mut(&mut self, id: &str) -> Option<&mut ResearchQuestion> {
        self.questions.get_mut(id)
    }

    /// List questions with optional filters.
    /// Sorted by priority descending (matching Python behaviour).
    pub fn list(
        &self,
        topic: Option<&str>,
        status: Option<&QuestionStatus>,
        source: Option<&QuestionSource>,
    ) -> Vec<&ResearchQuestion> {
        let mut results: Vec<&ResearchQuestion> = self
            .questions
            .values()
            .filter(|q| {
                let topic_match = topic.is_none_or(|t| {
                    t.is_empty() || q.topic.to_lowercase().contains(&t.to_lowercase())
                });
                let status_match = status.is_none_or(|s| q.status == *s);
                let source_match = source.is_none_or(|s| q.source == *s);
                topic_match && status_match && source_match
            })
            .collect();
        results.sort_by(|a, b| b.priority.cmp(&a.priority));
        results
    }

    /// Update a question's fields (partial update — only set Some fields).
    pub fn update(
        &mut self,
        id: &str,
        status: Option<QuestionStatus>,
        notes: Option<String>,
        priority: Option<u8>,
    ) -> Result<()> {
        let entry = self.questions.get_mut(id).context("Question not found")?;
        if let Some(s) = status {
            entry.status = s;
        }
        if let Some(n) = notes {
            entry.notes = n;
        }
        if let Some(p) = priority {
            entry.priority = p.clamp(1, 10);
        }
        entry.updated_at = Utc::now().format("%Y-%m-%dT%H:%M:%S").to_string();
        Ok(())
    }

    /// Link a paper to a question.
    pub fn link_paper(&mut self, id: &str, paper_id: &str) -> Result<()> {
        let entry = self.questions.get_mut(id).context("Question not found")?;
        let pid = paper_id.to_string();
        if !entry.related_papers.contains(&pid) {
            entry.related_papers.push(pid);
            entry.updated_at = Utc::now().format("%Y-%m-%dT%H:%M:%S").to_string();
        }
        Ok(())
    }

    /// Unlink a paper from a question.
    pub fn unlink_paper(&mut self, id: &str, paper_id: &str) -> Result<()> {
        let entry = self.questions.get_mut(id).context("Question not found")?;
        entry.related_papers.retain(|p| p != paper_id);
        entry.updated_at = Utc::now().format("%Y-%m-%dT%H:%M:%S").to_string();
        Ok(())
    }

    /// Delete a question by ID.
    pub fn delete(&mut self, id: &str) -> Result<()> {
        self.questions.remove(id).context("Question not found")?;
        Ok(())
    }

    // ── Sync (gap detection bridge) ─────────────────────────────────────

    /// Sync from gap detection results. Add questions for gaps that don't
    /// already have a similar question (substring matching like Python).
    pub fn sync_from_gaps(
        &mut self,
        gaps: &[String],
        topic: &str,
        priority: u8,
    ) -> Vec<ResearchQuestion> {
        let mut new_questions = Vec::new();
        let priority = priority.clamp(1, 10);

        for gap in gaps {
            let exists = self.questions.values().any(|q| {
                q.question.to_lowercase().contains(&gap.to_lowercase())
                    || gap.to_lowercase().contains(&q.question.to_lowercase())
            });
            if !exists {
                let q = self.add(
                    format!("如何解决: {}?", gap),
                    QuestionSource::GapDetection,
                    topic.to_string(),
                    priority,
                    String::new(),
                );
                new_questions.push(q);
            }
        }

        new_questions
    }

    // ── Stats ───────────────────────────────────────────────────────────

    /// Get question statistics (matches Python's `get_stats()`).
    pub fn stats(&self) -> QuestionStats {
        let mut stats = QuestionStats::default();
        for q in self.questions.values() {
            match q.status {
                QuestionStatus::Open => stats.open += 1,
                QuestionStatus::InProgress => stats.in_progress += 1,
                QuestionStatus::Resolved => stats.resolved += 1,
                QuestionStatus::Wontfix => stats.wontfix += 1,
            }
            match q.source {
                QuestionSource::Manual => stats.manual += 1,
                QuestionSource::GapDetection => stats.gap_detection += 1,
                QuestionSource::Hypothesis => stats.hypothesis += 1,
                QuestionSource::LiteratureReview => stats.literature_review += 1,
            }
        }
        stats
    }

    // ── Persistence ─────────────────────────────────────────────────────

    /// Atomic write: write to `.tmp` then rename over target.
    /// Output is a JSON array (Vec) for Python file-format compat.
    pub fn save(&self) -> Result<()> {
        let mut list: Vec<&ResearchQuestion> = self.questions.values().collect();
        list.sort_by(|a, b| a.id.cmp(&b.id));
        let json = serde_json::to_string_pretty(&list)?;

        let tmp_path = self.file_path.with_extension("tmp");
        let mut tmp = fs::File::create(&tmp_path)?;
        tmp.write_all(json.as_bytes())?;
        tmp.sync_all()?;
        fs::rename(&tmp_path, &self.file_path)?;

        Ok(())
    }
}

// ── Stats ──────────────────────────────────────────────────────────────────

#[derive(Debug, Default, Serialize)]
pub struct QuestionStats {
    // By status
    pub open: usize,
    pub in_progress: usize,
    pub resolved: usize,
    pub wontfix: usize,
    // By source
    pub manual: usize,
    pub gap_detection: usize,
    pub hypothesis: usize,
    pub literature_review: usize,
}

// ── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn setup_tracker() -> (QuestionTracker, tempfile::TempDir) {
        let dir = tempfile::tempdir().unwrap();
        let file_path = dir.path().join("questions.json");
        let tracker = QuestionTracker::open(file_path).unwrap();
        (tracker, dir)
    }

    #[test]
    fn test_add_and_get() {
        let (mut tracker, _dir) = setup_tracker();
        let q = tracker.add(
            "What is attention?".into(),
            QuestionSource::Manual,
            "LLM".into(),
            8,
            "key question".into(),
        );
        assert_eq!(q.id.len(), 8);
        assert_eq!(q.question, "What is attention?");
        assert_eq!(q.source, QuestionSource::Manual);
        assert_eq!(q.priority, 8);

        let fetched = tracker.get(&q.id).unwrap();
        assert_eq!(fetched.question, "What is attention?");
    }

    #[test]
    fn test_list_empty() {
        let (tracker, _dir) = setup_tracker();
        let results = tracker.list(None, None, None);
        assert!(results.is_empty());
    }

    #[test]
    fn test_list_with_filters() {
        let (mut tracker, _dir) = setup_tracker();
        tracker.add("Q1".into(), QuestionSource::Manual, "AI".into(), 5, "".into());
        tracker.add("Q2".into(), QuestionSource::GapDetection, "ML".into(), 8, "".into());
        tracker.add("Q3".into(), QuestionSource::Manual, "AI".into(), 3, "".into());

        let all = tracker.list(None, None, None);
        assert_eq!(all.len(), 3);

        let ai = tracker.list(Some("AI"), None, None);
        assert_eq!(ai.len(), 2);

        let manual = tracker.list(None, None, Some(&QuestionSource::Manual));
        assert_eq!(manual.len(), 2);
    }

    #[test]
    fn test_update() {
        let (mut tracker, _dir) = setup_tracker();
        let q = tracker.add("Test".into(), QuestionSource::Manual, "".into(), 5, "".into());

        tracker
            .update(&q.id, Some(QuestionStatus::Resolved), Some("done".into()), Some(10))
            .unwrap();

        let updated = tracker.get(&q.id).unwrap();
        assert_eq!(updated.status, QuestionStatus::Resolved);
        assert_eq!(updated.notes, "done");
        assert_eq!(updated.priority, 10);
    }

    #[test]
    fn test_update_nonexistent() {
        let (mut tracker, _dir) = setup_tracker();
        let result = tracker.update("bad-id", None, None, None);
        assert!(result.is_err());
    }

    #[test]
    fn test_delete() {
        let (mut tracker, _dir) = setup_tracker();
        let q = tracker.add("Delete me".into(), QuestionSource::Manual, "".into(), 5, "".into());
        assert!(tracker.get(&q.id).is_some());

        tracker.delete(&q.id).unwrap();
        assert!(tracker.get(&q.id).is_none());
    }

    #[test]
    fn test_delete_nonexistent() {
        let (mut tracker, _dir) = setup_tracker();
        let result = tracker.delete("bad-id");
        assert!(result.is_err());
    }

    #[test]
    fn test_link_unlink_paper() {
        let (mut tracker, _dir) = setup_tracker();
        let q = tracker.add("Link test".into(), QuestionSource::Manual, "".into(), 5, "".into());

        tracker.link_paper(&q.id, "paper-123").unwrap();
        assert_eq!(tracker.get(&q.id).unwrap().related_papers.len(), 1);

        // Duplicate link should be idempotent
        tracker.link_paper(&q.id, "paper-123").unwrap();
        assert_eq!(tracker.get(&q.id).unwrap().related_papers.len(), 1);

        tracker.unlink_paper(&q.id, "paper-123").unwrap();
        assert_eq!(tracker.get(&q.id).unwrap().related_papers.len(), 0);
    }

    #[test]
    fn test_save_and_reload_python_compat() {
        let dir = tempfile::tempdir().unwrap();
        let file_path = dir.path().join("questions.json");

        // Write Python-format JSON array (same format Python's JsonFileStore writes)
        let python_json = serde_json::json!([
            {
                "id": "abc12345",
                "question": "Python compatibility test",
                "source": "manual",
                "status": "open",
                "related_papers": ["arxiv:2301.00001"],
                "created_at": "2024-01-01T12:00:00",
                "updated_at": "2024-01-01T12:00:00",
                "notes": "from Python",
                "priority": 7,
                "topic": "AI"
            }
        ]);
        fs::write(&file_path, serde_json::to_string_pretty(&python_json).unwrap()).unwrap();

        // Rust must be able to read the Python-format file
        let tracker = QuestionTracker::open(file_path).unwrap();
        assert_eq!(tracker.questions.len(), 1);
        let q = tracker.questions.get("abc12345").unwrap();
        assert_eq!(q.question, "Python compatibility test");
        assert_eq!(q.source, QuestionSource::Manual);
        assert_eq!(q.status, QuestionStatus::Open);
        assert_eq!(q.priority, 7);
        assert_eq!(q.related_papers, vec!["arxiv:2301.00001"]);
    }

    #[test]
    fn test_roundtrip_save_preserves_data() {
        let dir = tempfile::tempdir().unwrap();
        let file_path = dir.path().join("questions.json");

        let mut tracker = QuestionTracker::open(file_path.clone()).unwrap();
        tracker.add("Q1".into(), QuestionSource::Manual, "AI".into(), 5, "".into());
        tracker.add("Q2".into(), QuestionSource::GapDetection, "ML".into(), 8, "".into());
        tracker.save().unwrap();

        // Reload and verify
        let loaded = QuestionTracker::open(file_path).unwrap();
        assert_eq!(loaded.questions.len(), 2);
        let list = loaded.list(None, None, None);
        // Priority descending: 8 first, then 5
        assert_eq!(list[0].priority, 8);
        assert_eq!(list[1].priority, 5);
    }

    #[test]
    fn test_stats() {
        let (mut tracker, _dir) = setup_tracker();
        tracker.add("Q1".into(), QuestionSource::Manual, "".into(), 5, "".into());
        tracker.add("Q2".into(), QuestionSource::GapDetection, "".into(), 8, "".into());

        let stats = tracker.stats();
        assert_eq!(stats.open, 2);
        assert_eq!(stats.manual, 1);
        assert_eq!(stats.gap_detection, 1);
    }

    #[test]
    fn test_sync_from_gaps() {
        let (mut tracker, _dir) = setup_tracker();
        tracker.add("Existing".into(), QuestionSource::Manual, "".into(), 5, "".into());

        let gaps = vec!["长文档场景下的检索效率".to_string()];
        let new = tracker.sync_from_gaps(&gaps, "RAG", 7);
        assert_eq!(new.len(), 1);
        assert!(new[0].question.contains("如何解决"));

        // Dedup — same gap should not create duplicate
        let again = tracker.sync_from_gaps(&gaps, "RAG", 7);
        assert!(again.is_empty());
    }

    #[test]
    fn test_empty_file_loads_cleanly() {
        let dir = tempfile::tempdir().unwrap();
        let file_path = dir.path().join("questions.json");
        // Empty file
        fs::write(&file_path, "").unwrap();

        let tracker = QuestionTracker::open(file_path).unwrap();
        assert!(tracker.questions.is_empty());
    }

    #[test]
    fn test_priority_clamping() {
        let (mut tracker, _dir) = setup_tracker();
        let q = tracker.add(
            "Test".into(),
            QuestionSource::Manual,
            "".into(),
            99,
            "".into(),
        );
        assert_eq!(q.priority, 10);

        let q2 = tracker.add("Low".into(), QuestionSource::Manual, "".into(), 0, "".into());
        assert_eq!(q2.priority, 1);
    }
}
