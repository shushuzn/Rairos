use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

const MAX_SNAPSHOTS_PER_SESSION: usize = 10;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotEntry {
    pub name: String,
    pub hash: String,
    pub size: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotManifest {
    pub session_id: String,
    pub step: u32,
    pub captured_at: f64,
    pub files: Vec<SnapshotEntry>,
    pub metadata: HashMap<String, String>,
}

#[derive(Debug, Clone)]
pub struct WorkspaceSnapshot {
    base_dir: PathBuf,
}

impl Default for WorkspaceSnapshot {
    fn default() -> Self {
        let base = dirs_next().unwrap_or_else(|| PathBuf::from("."));
        Self {
            base_dir: base.join(".ai_research_os").join("workspace_snapshots"),
        }
    }
}

impl WorkspaceSnapshot {
    pub fn new(base_dir: Option<PathBuf>) -> Self {
        let dir = base_dir.unwrap_or_else(|| {
            dirs_next()
                .unwrap_or_else(|| PathBuf::from("."))
                .join(".ai_research_os")
                .join("workspace_snapshots")
        });
        fs::create_dir_all(&dir).ok();
        Self { base_dir: dir }
    }

    fn step_dir(&self, session_id: &str, step: u32) -> PathBuf {
        self.base_dir.join(session_id).join(format!("step_{step:03}"))
    }

    fn file_hash(path: &Path) -> String {
        let Ok(data) = fs::read(path) else {
            return String::new();
        };
        let hash = Sha256::digest(&data);
        hex::encode(hash)[..16].to_string()
    }

    pub fn capture(
        &self,
        session_id: &str,
        step: u32,
        paths: &[PathBuf],
        metadata: HashMap<String, String>,
    ) -> PathBuf {
        let step_dir = self.step_dir(session_id, step);
        fs::create_dir_all(&step_dir).ok();

        let mut entries = Vec::new();
        for file_path in paths {
            if !file_path.exists() {
                continue;
            }
            let name = file_path
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string();
            let target = step_dir.join(&name);
            fs::copy(file_path, &target).ok();
            let hash = Self::file_hash(file_path);
            let size = fs::metadata(file_path).map(|m| m.len()).unwrap_or(0);
            entries.push(SnapshotEntry { name, hash, size });
        }

        let manifest = SnapshotManifest {
            session_id: session_id.to_string(),
            step,
            captured_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs_f64(),
            files: entries,
            metadata,
        };
        let json = serde_json::to_string_pretty(&manifest).unwrap_or_default();
        fs::write(step_dir.join("_snapshot_manifest.json"), json).ok();
        self.prune_old(session_id);
        step_dir
    }

    pub fn rollback(&self, session_id: &str, step: u32, target_dir: &Path) -> Vec<PathBuf> {
        let step_dir = self.step_dir(session_id, step);
        if !step_dir.exists() {
            return Vec::new();
        }
        let manifest_path = step_dir.join("_snapshot_manifest.json");
        let Ok(json) = fs::read_to_string(&manifest_path) else {
            return Vec::new();
        };
        let Ok(manifest) = serde_json::from_str::<SnapshotManifest>(&json) else {
            return Vec::new();
        };
        let mut restored = Vec::new();
        for entry in &manifest.files {
            let src = step_dir.join(&entry.name);
            let dst = target_dir.join(&entry.name);
            if let Some(parent) = dst.parent() {
                fs::create_dir_all(parent).ok();
            }
            if fs::copy(&src, &dst).is_ok() {
                restored.push(dst);
            }
        }
        restored
    }

    pub fn list_snapshots(&self, session_id: &str) -> Vec<serde_json::Value> {
        let session_dir = self.base_dir.join(session_id);
        if !session_dir.exists() {
            return Vec::new();
        }
        let mut snapshots = Vec::new();
        let mut entries: Vec<_> = fs::read_dir(&session_dir)
            .into_iter()
            .flatten()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().map(|t| t.is_dir()).unwrap_or(false))
            .collect();
        entries.sort_by_key(|e| e.file_name());
        for entry in &entries {
            let manifest_path = entry.path().join("_snapshot_manifest.json");
            let manifest = fs::read_to_string(&manifest_path)
                .ok()
                .and_then(|s| serde_json::from_str::<SnapshotManifest>(&s).ok());
            let files = manifest.as_ref().map(|m| m.files.len()).unwrap_or(0);
            let captured_at = manifest
                .as_ref()
                .map(|m| m.captured_at)
                .or_else(|| {
                    fs::metadata(&manifest_path)
                        .ok()
                        .and_then(|m| m.modified().ok())
                        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                        .map(|d| d.as_secs_f64())
                })
                .unwrap_or(0.0);
            snapshots.push(serde_json::json!({
                "step": entry.file_name().to_string_lossy(),
                "path": entry.path().to_string_lossy(),
                "files": files,
                "captured_at": captured_at,
                "metadata": manifest.map(|m| m.metadata).unwrap_or_default(),
            }));
        }
        snapshots
    }

    pub fn latest_step(&self, session_id: &str) -> Option<u32> {
        let snapshots = self.list_snapshots(session_id);
        snapshots
            .iter()
            .filter_map(|s| {
                s["step"].as_str().and_then(|step_str| {
                    step_str
                        .strip_prefix("step_")
                        .and_then(|n| n.parse::<u32>().ok())
                })
            })
            .max()
    }

    fn prune_old(&self, session_id: &str) {
        let mut snapshots = self.list_snapshots(session_id);
        if snapshots.len() <= MAX_SNAPSHOTS_PER_SESSION {
            return;
        }
        snapshots.sort_by(|a, b| {
            a["captured_at"]
                .as_f64()
                .unwrap_or(0.0)
                .partial_cmp(&b["captured_at"].as_f64().unwrap_or(0.0))
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        let to_delete = snapshots.len() - MAX_SNAPSHOTS_PER_SESSION;
        for snap in snapshots.iter().take(to_delete) {
            if let Some(path) = snap["path"].as_str() {
                fs::remove_dir_all(path).ok();
            }
        }
    }
}

fn dirs_next() -> Option<PathBuf> {
    std::env::var("HOME")
        .ok()
        .map(PathBuf::from)
        .or_else(|| std::env::var("USERPROFILE").ok().map(PathBuf::from))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn test_capture_and_list() {
        let dir = tempfile::tempdir().unwrap();
        let snap = WorkspaceSnapshot::new(Some(dir.path().join("snap")));
        let test_file = dir.path().join("test.txt");
        fs::write(&test_file, b"hello").unwrap();

        snap.capture(
            "session-1",
            1,
            &[test_file],
            HashMap::new(),
        );
        let list = snap.list_snapshots("session-1");
        assert_eq!(list.len(), 1);
    }

    #[test]
    fn test_rollback() {
        let dir = tempfile::tempdir().unwrap();
        let snap = WorkspaceSnapshot::new(Some(dir.path().join("snap")));
        let test_file = dir.path().join("src").join("test.txt");
        fs::create_dir_all(test_file.parent().unwrap()).unwrap();
        fs::write(&test_file, b"original").unwrap();

        // Intentional: single-element slice from owned value
        #[allow(clippy::cloned_ref_to_slice_refs)]
        snap.capture("sess-1", 1, &[test_file.clone()], HashMap::new());

        fs::write(&test_file, b"modified").unwrap();
        let target = dir.path().join("restore");
        snap.rollback("sess-1", 1, &target);
        let restored = target.join("test.txt");
        assert!(restored.exists());
    }

    #[test]
    fn test_empty_session() {
        let dir = tempfile::tempdir().unwrap();
        let snap = WorkspaceSnapshot::new(Some(dir.path().join("snap")));
        assert!(snap.list_snapshots("nonexistent").is_empty());
        assert!(snap.latest_step("nonexistent").is_none());
    }

    #[test]
    fn test_prune() {
        let dir = tempfile::tempdir().unwrap();
        let snap = WorkspaceSnapshot::new(Some(dir.path().join("snap")));
        let f = dir.path().join("f.txt");
        fs::write(&f, b"data").unwrap();
        for i in 0..MAX_SNAPSHOTS_PER_SESSION + 2 {
            // Intentional: single-element slice from owned value
        #[allow(clippy::cloned_ref_to_slice_refs)]
        snap.capture("prune-test", i as u32, &[f.clone()], HashMap::new());
        }
        let list = snap.list_snapshots("prune-test");
        assert!(list.len() <= MAX_SNAPSHOTS_PER_SESSION);
    }
}
