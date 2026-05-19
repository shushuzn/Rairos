//! Research Session Snapstate — pause/resume for deep research agent workflows.
//!
//! Mirrors research_loop/snapstate.py for Rust-side persistence.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

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

// ─── Query Deduplication ──────────────────────────────────────────────────────

/// Search query with metadata for deduplication
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchQueryRecord {
    pub query: String,
    pub gap_type: String,
    pub variant_suffix: Option<String>,
    pub timestamp: f64,
    pub result_count: usize,
    pub paper_ids: Vec<String>,
}

impl SearchQueryRecord {
    pub fn new(query: &str, gap_type: &str) -> Self {
        Self {
            query: query.to_string(),
            gap_type: gap_type.to_string(),
            variant_suffix: None,
            timestamp: now_secs(),
            result_count: 0,
            paper_ids: Vec::new(),
        }
    }

    /// Generate a variant suffix based on gap type
    pub fn with_variant(mut self, variant: &str) -> Self {
        self.variant_suffix = Some(variant.to_string());
        self.query = format!("{} [gap:{}]", self.query, variant);
        self
    }
}

/// Query deduplication manager
pub struct QueryDeduplicator {
    history: Vec<SearchQueryRecord>,
    similarity_threshold: f64,
}

impl Default for QueryDeduplicator {
    fn default() -> Self {
        Self::new(0.75)
    }
}

impl QueryDeduplicator {
    pub fn new(similarity_threshold: f64) -> Self {
        Self {
            history: Vec::new(),
            similarity_threshold,
        }
    }

    /// Calculate Jaccard similarity between two queries
    pub fn query_similarity(&self, q1: &str, q2: &str) -> f64 {
        let words1: std::collections::HashSet<String> = q1
            .split_whitespace()
            .map(|w| w.to_lowercase())
            .collect();
        let words2: std::collections::HashSet<String> = q2
            .split_whitespace()
            .map(|w| w.to_lowercase())
            .collect();

        if words1.is_empty() && words2.is_empty() {
            return 1.0;
        }

        let intersection = words1.intersection(&words2).count() as f64;
        let union = words1.union(&words2).count() as f64;

        if union == 0.0 {
            0.0
        } else {
            intersection / union
        }
    }

    /// Check if query is too similar to any in history
    pub fn is_duplicate(&self, query: &str) -> Option<&SearchQueryRecord> {
        for record in &self.history {
            let sim = self.query_similarity(query, &record.query);
            if sim >= self.similarity_threshold {
                return Some(record);
            }
        }
        None
    }

    /// Add query to history
    pub fn record(&mut self, query: SearchQueryRecord) {
        // Keep history bounded
        if self.history.len() >= 1000 {
            self.history.remove(0);
        }
        self.history.push(query);
    }

    /// Generate variant query if duplicate found
    pub fn generate_variant(&self, query: &str, gap_type: &str) -> String {
        let suffix = format!("[variant:{}:{}]", gap_type, now_secs() as i64);
        format!("{} {}", query, suffix)
    }

    /// Load history from file
    pub fn load_from_file(path: &std::path::Path) -> Self {
        let history = if let Ok(data) = std::fs::read_to_string(path) {
            serde_json::from_str(&data).unwrap_or_default()
        } else {
            Vec::new()
        };
        Self {
            history,
            similarity_threshold: 0.75,
        }
    }

    /// Save history to file
    pub fn save_to_file(&self, path: &std::path::Path) -> Result<(), String> {
        let json = serde_json::to_string_pretty(&self.history)
            .map_err(|e| e.to_string())?;
        std::fs::write(path, json).map_err(|e| e.to_string())
    }
}

// ─── Route Plan Checkpoint ────────────────────────────────────────────────────

/// Checkpoint for research plan execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanCheckpoint {
    pub checkpoint_id: String,
    pub plan_id: String,
    pub created_at: f64,
    pub current_step_id: String,
    pub completed_steps: Vec<String>,
    pub gaps: Vec<SnapGap>,
    pub search_history: Vec<String>,
    pub processed_papers: Vec<String>,
    pub notes: String,
}

impl PlanCheckpoint {
    pub fn new(plan_id: &str, current_step_id: &str) -> Self {
        Self {
            checkpoint_id: uuid::Uuid::new_v4()
                .to_string()
                .chars()
                .filter(|c| *c != '-')
                .take(8)
                .collect(),
            plan_id: plan_id.to_string(),
            created_at: now_secs(),
            current_step_id: current_step_id.to_string(),
            completed_steps: Vec::new(),
            gaps: Vec::new(),
            search_history: Vec::new(),
            processed_papers: Vec::new(),
            notes: String::new(),
        }
    }

    /// Save checkpoint to file
    pub fn save(&self, base_dir: &Path) -> Result<PathBuf, String> {
        let dir = base_dir.join("checkpoints");
        std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
        let filename = format!("{}_{}.json", self.plan_id, self.checkpoint_id);
        let path = dir.join(&filename);
        let json = serde_json::to_string_pretty(self).map_err(|e| e.to_string())?;
        std::fs::write(&path, json).map_err(|e| e.to_string())?;
        Ok(path)
    }

    /// Load latest checkpoint for a plan
    pub fn load_latest(base_dir: &Path, plan_id: &str) -> Option<Self> {
        let dir = base_dir.join("checkpoints");
        if !dir.exists() {
            return None;
        }

        let mut latest: Option<(f64, PathBuf)> = None;
        if let Ok(entries) = std::fs::read_dir(&dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if let Some(filename) = path.file_name().and_then(|f| f.to_str()) {
                    if filename.starts_with(plan_id) && filename.ends_with(".json") {
                        if let Ok(metadata) = std::fs::metadata(&path) {
                            if let Ok(modified) = metadata.modified() {
                                let timestamp = modified
                                    .duration_since(std::time::UNIX_EPOCH)
                                    .unwrap_or_default()
                                    .as_secs_f64();
                                if latest.as_ref().map_or(true, |(t, _)| timestamp > *t) {
                                    latest = Some((timestamp, path));
                                }
                            }
                        }
                    }
                }
            }
        }

        latest.and_then(|(_, path)| {
            std::fs::read_to_string(&path)
                .ok()
                .and_then(|data| serde_json::from_str(&data).ok())
        })
    }
}

// ─── Auto-checkpoint Manager ─────────────────────────────────────────────────

/// Configuration for automatic checkpointing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckpointConfig {
    pub enabled: bool,
    pub every_n_steps: i32,
    pub interval_seconds: i32,
    pub include_gaps: bool,
    pub include_search_history: bool,
    pub include_plan_state: bool,
    pub include_papers: bool,
}

impl Default for CheckpointConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            every_n_steps: 1,
            interval_seconds: 60,
            include_gaps: true,
            include_search_history: true,
            include_plan_state: true,
            include_papers: true,
        }
    }
}

/// Automatic checkpoint manager
pub struct AutoCheckpoint {
    config: CheckpointConfig,
    last_checkpoint_time: f64,
    steps_since_checkpoint: i32,
    base_dir: PathBuf,
}

impl AutoCheckpoint {
    pub fn new(config: CheckpointConfig, base_dir: PathBuf) -> Self {
        Self {
            config,
            last_checkpoint_time: now_secs(),
            steps_since_checkpoint: 0,
            base_dir,
        }
    }

    /// Check if checkpoint should be created
    pub fn should_checkpoint(&mut self) -> bool {
        if !self.config.enabled {
            return false;
        }

        let now = now_secs();
        let time_elapsed = now - self.last_checkpoint_time;

        // Check time-based checkpoint
        if time_elapsed >= self.config.interval_seconds as f64 {
            return true;
        }

        // Check step-based checkpoint
        self.steps_since_checkpoint += 1;
        if self.steps_since_checkpoint >= self.config.every_n_steps {
            self.steps_since_checkpoint = 0;
            return true;
        }

        false
    }

    /// Mark that checkpoint was created
    pub fn checkpoint_created(&mut self) {
        self.last_checkpoint_time = now_secs();
        self.steps_since_checkpoint = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_query_similarity() {
        let dedup = QueryDeduplicator::new(0.75);
        let sim = dedup.query_similarity(
            "transformer efficiency optimization",
            "transformer optimization efficiency",
        );
        assert!(sim > 0.5, "Queries should be similar: {}", sim);
    }

    #[test]
    fn test_is_duplicate() {
        let mut dedup = QueryDeduplicator::new(0.75);
        dedup.record(SearchQueryRecord::new(
            "transformer efficiency optimization",
            "method_limitation",
        ));

        assert!(dedup.is_duplicate("transformer efficiency").is_some());
        assert!(dedup.is_duplicate("completely different topic").is_none());
    }

    #[test]
    fn test_generate_variant() {
        let dedup = QueryDeduplicator::new(0.75);
        let variant = dedup.generate_variant("transformer efficiency", "method_limitation");
        assert!(variant.contains("[variant:"));
        assert!(variant.contains("transformer efficiency"));
    }

    #[test]
    fn test_checkpoint_creation() {
        let config = CheckpointConfig::default();
        let base_dir = PathBuf::from("/tmp/test_checkpoints");
        let mut manager = AutoCheckpoint::new(config, base_dir.clone());

        // Should not checkpoint immediately
        assert!(!manager.should_checkpoint());

        manager.steps_since_checkpoint = 1;
        // Should checkpoint after step threshold
        assert!(manager.should_checkpoint());
    }
}
