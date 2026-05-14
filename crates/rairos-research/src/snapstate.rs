//! Research Session Snapstate — pause/resume for deep research agent workflows.
//!
//! Mirrors research_loop/snapstate.py for Rust-side persistence.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

// ─── PaperSnapshot ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapPaper {
    #[serde(default)]
    pub arxiv_id: String,
    #[serde(default)]
    pub title: String,
    #[serde(rename = "abstract", default)]
    pub abstract_text: String,
    #[serde(default)]
    pub url: String,
    #[serde(default)]
    pub extracted_text: String,
    #[serde(default)]
    pub summary: String,
    #[serde(default)]
    pub gaps_found: Vec<String>,
    #[serde(default)]
    pub notes: String,
    #[serde(default)]
    pub keywords: Vec<String>,
}

// ─── GapSnapshot ───────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapGap {
    #[serde(default)]
    pub gap_type: String,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub matched_papers: Vec<String>,
    #[serde(default)]
    pub archetype_match: f64,
    #[serde(default)]
    pub accepted: bool,
}

// ─── ResearchSession ───────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SnapSession {
    #[serde(default)]
    pub session_id: String,
    #[serde(default)]
    pub query: String,
    #[serde(default)]
    pub created_at: f64,
    #[serde(default)]
    pub updated_at: f64,
    #[serde(default)]
    pub iteration: i32,
    #[serde(default = "default_max_iterations")]
    pub max_iterations: i32,
    #[serde(default)]
    pub papers: Vec<SnapPaper>,
    #[serde(default)]
    pub gaps: Vec<SnapGap>,
    #[serde(default)]
    pub search_history: Vec<String>,
    #[serde(default)]
    pub hypotheses: Vec<String>,
    #[serde(default)]
    pub findings: Vec<String>,
    #[serde(default)]
    pub reflections: Vec<String>,
    #[serde(default)]
    pub archetype: HashMap<String, f64>,
    #[serde(default = "default_status")]
    pub status: String,
    #[serde(default)]
    pub error: String,
}

fn default_max_iterations() -> i32 { 3 }
fn default_status() -> String { "running".to_string() }

// ─── Snapstate Manager ─────────────────────────────────────────────────────────

pub struct Snapstate {
    base_dir: PathBuf,
}

impl Default for Snapstate {
    fn default() -> Self {
        let dir = dirs_next()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(".ai_research_os")
            .join("sessions");
        std::fs::create_dir_all(&dir).ok();
        Self { base_dir: dir }
    }
}

impl Snapstate {
    pub fn new(base_dir: Option<PathBuf>) -> Self {
        let dir = base_dir.unwrap_or_else(|| {
            dirs_next()
                .unwrap_or_else(|| PathBuf::from("."))
                .join(".ai_research_os")
                .join("sessions")
        });
        std::fs::create_dir_all(&dir).ok();
        Self { base_dir: dir }
    }

    pub fn session_path(&self, session_id: &str) -> PathBuf {
        self.base_dir.join(format!("{}.json", session_id))
    }

    pub fn new_session(&self, query: &str, max_iterations: i32) -> SnapSession {
        let now = now_secs();
        let id: String = uuid::Uuid::new_v4()
            .to_string()
            .chars()
            .filter(|c| *c != '-')
            .take(8)
            .collect();
        SnapSession {
            session_id: id,
            query: query.to_string(),
            created_at: now,
            updated_at: now,
            max_iterations,
            status: "running".to_string(),
            ..Default::default()
        }
    }

    pub fn save(&self, session: &SnapSession) -> Result<PathBuf, String> {
        let path = self.session_path(&session.session_id);
        let tmp_path = path.with_extension("json.tmp");

        let file = std::fs::File::create(&tmp_path)
            .map_err(|e| format!("Failed to create tmp file: {}", e))?;
        let writer = std::io::BufWriter::new(file);
        serde_json::to_writer_pretty(writer, session)
            .map_err(|e| format!("Failed to serialize session: {}", e))?;

        std::fs::rename(&tmp_path, &path)
            .map_err(|e| format!("Failed to rename tmp file: {}", e))?;

        Ok(path)
    }

    pub fn load(&self, session_id: &str) -> Option<SnapSession> {
        let path = self.session_path(session_id);
        if !path.exists() {
            return None;
        }
        let data = std::fs::read_to_string(&path).ok()?;
        serde_json::from_str(&data).ok()
    }

    pub fn create_checkpoint(&self, session: &SnapSession) -> Result<String, String> {
        let checkpoint_id: String = uuid::Uuid::new_v4()
            .to_string()
            .chars()
            .filter(|c| *c != '-')
            .take(8)
            .collect();
        let check_dir = self.base_dir.join(format!("{}_checkpoints", session.session_id));
        std::fs::create_dir_all(&check_dir)
            .map_err(|e| format!("Failed to create checkpoint dir: {}", e))?;
        let check_path = check_dir.join(format!("{}.json", checkpoint_id));

        let file = std::fs::File::create(&check_path)
            .map_err(|e| format!("Failed to create checkpoint: {}", e))?;
        let writer = std::io::BufWriter::new(file);
        serde_json::to_writer_pretty(writer, session)
            .map_err(|e| format!("Failed to serialize checkpoint: {}", e))?;

        Ok(checkpoint_id)
    }
}

fn dirs_next() -> Option<PathBuf> {
    std::env::var("HOME")
        .ok()
        .map(PathBuf::from)
        .or_else(|| std::env::var("USERPROFILE").ok().map(PathBuf::from))
}

fn now_secs() -> f64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs_f64()
}
