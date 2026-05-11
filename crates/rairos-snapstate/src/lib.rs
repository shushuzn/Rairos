//! Research Session Snapstate — pause/resume for deep research agent workflows.
//!
//! Save, load, list, checkpoint, rollback, fork, and compare research sessions.
//!
//! Python original: `research_loop/snapstate.py` (313 lines)

use serde::{Deserialize, Serialize};
use serde_json;
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

// ─── Dataclasses ────────────────────────────────────────────────────────────────

/// A paper captured during research.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaperSnapshot {
    pub arxiv_id: String,
    pub title: String,
    #[serde(rename = "abstract")]
    pub abstract_text: String,
    pub url: String,
    pub extracted_text: String,
    pub summary: String,
    pub gaps_found: Vec<String>,
    pub notes: String,
    pub keywords: Vec<String>,
}

impl Default for PaperSnapshot {
    fn default() -> Self {
        Self {
            arxiv_id: String::new(),
            title: String::new(),
            abstract_text: String::new(),
            url: String::new(),
            extracted_text: String::new(),
            summary: String::new(),
            gaps_found: Vec::new(),
            notes: String::new(),
            keywords: Vec::new(),
        }
    }
}

/// A research gap captured during agent iteration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GapSnapshot {
    pub gap_type: String,
    pub title: String,
    pub description: String,
    /// arxiv_ids of matched papers
    pub matched_papers: Vec<String>,
    pub archetype_match: f64,
    pub accepted: bool,
}

/// Complete state of a deep research agent run.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResearchSession {
    pub session_id: String,
    pub query: String,
    pub created_at: f64,
    pub updated_at: f64,
    pub iteration: i32,
    pub max_iterations: i32,

    // Search state
    pub papers: Vec<PaperSnapshot>,
    pub gaps: Vec<GapSnapshot>,
    pub search_history: Vec<String>,

    // Agent memory
    pub hypotheses: Vec<String>,
    pub findings: Vec<String>,
    pub reflections: Vec<String>,

    // Archetype context
    pub archetype: HashMap<String, f64>,

    // Status
    pub status: String,
    pub error: String,
}

impl ResearchSession {
    pub fn duration(&self) -> f64 {
        now_timestamp() - self.created_at
    }
}

// ─── Snapstate ────────────────────────────────────────────────────────────────

fn session_file_path(base_dir: &PathBuf, session_id: &str) -> PathBuf {
    base_dir.join(format!("{session_id}.json"))
}

fn checkpoint_dir(base_dir: &PathBuf, session_id: &str) -> PathBuf {
    base_dir.join(format!("{session_id}_checkpoints"))
}

fn now_timestamp() -> f64 {
    chrono::Utc::now().timestamp() as f64
        + chrono::Utc::now().timestamp_subsec_nanos() as f64 / 1_000_000_000.0
}

fn uuid_short() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    format!("{:x}", nanos)[..8].to_string()
}

/// Snapstate manager — save, load, list research sessions.
#[derive(Debug, Clone)]
pub struct Snapstate {
    pub base_dir: PathBuf,
}

impl Default for Snapstate {
    fn default() -> Self {
        Self::new(None)
    }
}

impl Snapstate {
    pub fn new(base_dir: Option<PathBuf>) -> Self {
        let base_dir = base_dir.unwrap_or_else(|| {
            dirs::home_dir()
                .unwrap_or_else(|| PathBuf::from("."))
                .join(".ai_research_os")
                .join("sessions")
        });
        let _ = fs::create_dir_all(&base_dir);
        Self { base_dir }
    }

    /// Save session to disk. Returns path.
    pub fn save(&self, session: &ResearchSession) -> PathBuf {
        let mut session = session.clone();
        session.updated_at = now_timestamp();
        let path = session_file_path(&self.base_dir, &session.session_id);
        let json = serde_json::to_string_pretty(&session).unwrap_or_default();
        let tmp = {
            let mut t = path.clone();
            t.set_extension("tmp");
            t
        };
        let _ = fs::write(&tmp, &json);
        let _ = fs::rename(&tmp, &path);
        path
    }

    /// Load session by ID. Returns None if not found.
    pub fn load(&self, session_id: &str) -> Option<ResearchSession> {
        let path = session_file_path(&self.base_dir, session_id);
        if !path.exists() {
            return None;
        }
        let text = fs::read_to_string(&path).ok()?;
        serde_json::from_str(&text).ok()
    }

    /// Load the most recently updated session.
    pub fn load_latest(&self) -> Option<ResearchSession> {
        let entries: Vec<_> = fs::read_dir(&self.base_dir).ok()?.collect();
        let mut sessions: Vec<_> = entries
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| {
                e.path().extension().map_or(false, |ext| ext == "json")
                    && e.path().file_stem().map_or(false, |s| {
                        !s.to_string_lossy().ends_with(".tmp")
                    })
            })
            .collect();
        sessions.sort_by(|a, b| {
            let ta = a.metadata().ok().and_then(|m| m.modified().ok());
            let tb = b.metadata().ok().and_then(|m| m.modified().ok());
            match (ta, tb) {
                (Some(a), Some(b)) => b.cmp(&a),
                (Some(_), None) => std::cmp::Ordering::Less,
                (None, Some(_)) => std::cmp::Ordering::Greater,
                (None, None) => std::cmp::Ordering::Equal,
            }
        });
        sessions.first().and_then(|e| {
            e.path().file_stem()
                .map(|s| self.load(&s.to_string_lossy()))
                .unwrap_or(None)
        })
    }

    /// List all saved sessions (summary info only).
    pub fn list_sessions(&self) -> Vec<serde_json::Value> {
        let entries: Vec<_> = match fs::read_dir(&self.base_dir) {
            Ok(e) => e.filter_map(|r| r.ok()).collect(),
            Err(_) => return vec![],
        };

        let mut sessions: Vec<_> = entries
            .into_iter()
            .filter(|e| {
                e.path().extension().map_or(false, |ext| ext == "json")
                    && e.path().file_stem().map_or(false, |s| {
                        !s.to_string_lossy().ends_with(".tmp")
                    })
            })
            .collect();

        sessions.sort_by(|a, b| {
            let ta = a.metadata().ok().and_then(|m| m.modified().ok());
            let tb = b.metadata().ok().and_then(|m| m.modified().ok());
            match (ta, tb) {
                (Some(a), Some(b)) => b.cmp(&a),
                _ => std::cmp::Ordering::Equal,
            }
        });

        sessions
            .iter()
            .map(|entry| {
                let path = entry.path();
                let stem = path.file_stem().unwrap_or_default().to_string_lossy().to_string();
                match fs::read_to_string(&path) {
                    Ok(text) => match serde_json::from_str::<serde_json::Value>(&text) {
                        Ok(data) => serde_json::json!({
                            "session_id": data.get("session_id").and_then(|v| v.as_str()).unwrap_or(&stem),
                            "query": data.get("query").and_then(|v| v.as_str()).unwrap_or(""),
                            "status": data.get("status").and_then(|v| v.as_str()).unwrap_or("?"),
                            "iteration": data.get("iteration").and_then(|v| v.as_i64()).unwrap_or(0),
                            "papers": data.get("papers").and_then(|v| v.as_array()).map(|a| a.len()).unwrap_or(0),
                            "gaps": data.get("gaps").and_then(|v| v.as_array()).map(|a| a.len()).unwrap_or(0),
                        }),
                        Err(_) => serde_json::json!({ "session_id": stem, "status": "corrupt" }),
                    },
                    Err(_) => serde_json::json!({ "session_id": stem, "status": "corrupt" }),
                }
            })
            .collect()
    }

    /// Delete a session. Returns true if deleted.
    pub fn delete(&self, session_id: &str) -> bool {
        let path = session_file_path(&self.base_dir, session_id);
        if path.exists() {
            return fs::remove_file(&path).is_ok();
        }
        false
    }

    /// Create a new research session.
    pub fn new_session(
        &self,
        query: &str,
        max_iterations: i32,
        archetype: HashMap<String, f64>,
    ) -> ResearchSession {
        let now = now_timestamp();
        ResearchSession {
            session_id: uuid_short(),
            query: query.to_string(),
            created_at: now,
            updated_at: now,
            iteration: 0,
            max_iterations,
            papers: Vec::new(),
            gaps: Vec::new(),
            search_history: Vec::new(),
            hypotheses: Vec::new(),
            findings: Vec::new(),
            reflections: Vec::new(),
            archetype,
            status: "running".to_string(),
            error: String::new(),
        }
    }

    // ─── Checkpoint & Rollback ──────────────────────────────────────────────

    /// Save a named checkpoint. Returns checkpoint_id.
    pub fn create_checkpoint(&self, session: &ResearchSession) -> String {
        let checkpoint_id = uuid_short();
        let ck_dir = checkpoint_dir(&self.base_dir, &session.session_id);
        let _ = fs::create_dir_all(&ck_dir);
        let ck_path = ck_dir.join(format!("{checkpoint_id}.json"));
        let json = serde_json::to_string_pretty(session).unwrap_or_default();
        let _ = fs::write(&ck_path, json);
        checkpoint_id
    }

    /// Restore session to a previous checkpoint.
    pub fn rollback_to(&self, session_id: &str, checkpoint_id: &str) -> Option<ResearchSession> {
        let ck_dir = checkpoint_dir(&self.base_dir, session_id);
        let ck_path = ck_dir.join(format!("{checkpoint_id}.json"));
        if !ck_path.exists() {
            return None;
        }
        let text = fs::read_to_string(&ck_path).ok()?;
        let restored: ResearchSession = serde_json::from_str(&text).ok()?;
        self.save(&restored);
        Some(restored)
    }

    /// List all checkpoints for a session.
    pub fn list_checkpoints(&self, session_id: &str) -> Vec<serde_json::Value> {
        let ck_dir = checkpoint_dir(&self.base_dir, session_id);
        let entries: Vec<_> = match fs::read_dir(&ck_dir) {
            Ok(e) => e.filter_map(|r| r.ok()).collect(),
            Err(_) => return vec![],
        };

        let mut checkpoints: Vec<_> = entries
            .into_iter()
            .filter(|e| {
                e.path().extension().map_or(false, |ext| ext == "json")
            })
            .collect();

        checkpoints.sort_by(|a, b| {
            let ta = a.metadata().ok().and_then(|m| m.modified().ok());
            let tb = b.metadata().ok().and_then(|m| m.modified().ok());
            match (ta, tb) {
                (Some(a), Some(b)) => b.cmp(&a),
                _ => std::cmp::Ordering::Equal,
            }
        });

        checkpoints
            .iter()
            .map(|entry| {
                let path = entry.path();
                let stem = path.file_stem().unwrap_or_default().to_string_lossy().to_string();
                let created_at = entry
                    .metadata()
                    .ok()
                    .and_then(|m| m.modified().ok())
                    .map(|t| {
                        t.duration_since(std::time::UNIX_EPOCH)
                            .unwrap_or_default()
                            .as_secs_f64()
                    })
                    .unwrap_or(0.0);
                match fs::read_to_string(&path) {
                    Ok(text) => match serde_json::from_str::<serde_json::Value>(&text) {
                        Ok(data) => serde_json::json!({
                            "checkpoint_id": stem,
                            "created_at": created_at,
                            "iteration": data.get("iteration").and_then(|v| v.as_i64()).unwrap_or(0),
                            "papers": data.get("papers").and_then(|v| v.as_array()).map(|a| a.len()).unwrap_or(0),
                            "gaps": data.get("gaps").and_then(|v| v.as_array()).map(|a| a.len()).unwrap_or(0),
                        }),
                        Err(_) => serde_json::json!({ "checkpoint_id": stem, "corrupt": true }),
                    },
                    Err(_) => serde_json::json!({ "checkpoint_id": stem, "corrupt": true }),
                }
            })
            .collect()
    }

    /// Delete a specific checkpoint.
    pub fn delete_checkpoint(&self, session_id: &str, checkpoint_id: &str) -> bool {
        let ck_path = checkpoint_dir(&self.base_dir, session_id).join(format!("{checkpoint_id}.json"));
        if ck_path.exists() {
            return fs::remove_file(&ck_path).is_ok();
        }
        false
    }

    /// Fork a session at a checkpoint into a new session with a new session_id.
    pub fn fork_session(
        &self,
        session_id: &str,
        checkpoint_id: &str,
        new_query: Option<&str>,
    ) -> Option<ResearchSession> {
        let ck_dir = checkpoint_dir(&self.base_dir, session_id);
        let ck_path = ck_dir.join(format!("{checkpoint_id}.json"));
        if !ck_path.exists() {
            return None;
        }
        let text = fs::read_to_string(&ck_path).ok()?;
        let mut forked: ResearchSession = serde_json::from_str(&text).ok()?;
        let now = now_timestamp();
        forked.session_id = uuid_short();
        forked.status = "running".to_string();
        forked.created_at = now;
        forked.updated_at = now;
        if let Some(q) = new_query {
            forked.query = q.to_string();
        }
        self.save(&forked);
        Some(forked)
    }

    /// Return a diff summary between two sessions.
    pub fn compare_sessions(&self, session_id_a: &str, session_id_b: &str) -> serde_json::Value {
        let sess_a = self.load(session_id_a);
        let sess_b = self.load(session_id_b);

        if sess_a.is_none() || sess_b.is_none() {
            return serde_json::json!({ "error": "One or both sessions not found" });
        }

        let sess_a = sess_a.unwrap();
        let sess_b = sess_b.unwrap();

        let papers_a: std::collections::HashSet<_> = sess_a
            .papers
            .iter()
            .filter_map(|p| {
                if p.arxiv_id.is_empty() {
                    None
                } else {
                    Some(p.arxiv_id.clone())
                }
            })
            .collect();
        let papers_b: std::collections::HashSet<_> = sess_b
            .papers
            .iter()
            .filter_map(|p| {
                if p.arxiv_id.is_empty() {
                    None
                } else {
                    Some(p.arxiv_id.clone())
                }
            })
            .collect();
        let gaps_a: std::collections::HashSet<_> =
            sess_a.gaps.iter().map(|g| g.title.clone()).collect();
        let gaps_b: std::collections::HashSet<_> =
            sess_b.gaps.iter().map(|g| g.title.clone()).collect();

        serde_json::json!({
            "session_id_a": session_id_a,
            "session_id_b": session_id_b,
            "status_a": sess_a.status,
            "status_b": sess_b.status,
            "iteration_a": sess_a.iteration,
            "iteration_b": sess_b.iteration,
            "papers_a_count": papers_a.len(),
            "papers_b_count": papers_b.len(),
            "papers_diff": papers_a.len() as i64 - papers_b.len() as i64,
            "gaps_a_count": gaps_a.len(),
            "gaps_b_count": gaps_b.len(),
            "gaps_diff": gaps_a.len() as i64 - gaps_b.len() as i64,
            "shared_papers": papers_a.intersection(&papers_b).cloned().collect::<Vec<_>>(),
            "shared_papers_count": papers_a.intersection(&papers_b).count(),
            "unique_to_a": papers_a.difference(&papers_b).cloned().collect::<Vec<_>>(),
            "unique_to_b": papers_b.difference(&papers_a).cloned().collect::<Vec<_>>(),
            "iteration_diff": sess_a.iteration as i64 - sess_b.iteration as i64,
        })
    }
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::env::temp_dir;

    #[test]
    fn test_new_session() {
        let snap = Snapstate::new(Some(temp_dir().join("rairos_snapstate_test")));
        let session = snap.new_session("test query", 3, HashMap::new());
        assert_eq!(session.query, "test query");
        assert_eq!(session.max_iterations, 3);
        assert_eq!(session.status, "running");
        assert!(!session.session_id.is_empty());
    }

    #[test]
    fn test_save_and_load() {
        let tmp = temp_dir().join("rairos_snapstate_test2");
        let snap = Snapstate::new(Some(tmp.clone()));
        let session = snap.new_session("test", 2, HashMap::new());
        let path = snap.save(&session);
        assert!(path.exists());

        let loaded = snap.load(&session.session_id);
        assert!(loaded.is_some());
        let loaded = loaded.unwrap();
        assert_eq!(loaded.query, "test");
        assert_eq!(loaded.max_iterations, 2);

        let _ = fs::remove_file(&path);
        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn test_session_duration() {
        let snap = Snapstate::new(Some(temp_dir().join("rairos_snapstate_test3")));
        let mut session = snap.new_session("test", 1, HashMap::new());
        session.created_at = now_timestamp() - 10.0;
        let dur = session.duration();
        assert!(dur >= 9.0 && dur <= 11.0);
    }
}
