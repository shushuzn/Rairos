//! rairos-watch-papers — Watch papers.json for changes and trigger KG rebuilds.
//!
//! Ported from `core/watch_papers.py`.
//!
//! Uses a simple file-hash polling loop — no external dependencies, cross-platform.

use md5::{Digest, Md5};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

/// File change watcher with callback support.
pub struct Watcher {
    path: PathBuf,
    interval: Duration,
    on_change: Option<Box<dyn Fn(String) + Send + Sync>>,
    running: Arc<AtomicBool>,
}

impl Watcher {
    /// Create a new watcher for the given path.
    pub fn new(path: impl Into<PathBuf>, interval_secs: f64) -> Self {
        Self {
            path: path.into(),
            interval: Duration::from_secs_f64(interval_secs),
            on_change: None,
            running: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Set the callback to invoke when changes are detected.
    pub fn on_change<F>(&mut self, callback: F)
    where
        F: Fn(String) + Send + Sync + 'static,
    {
        self.on_change = Some(Box::new(callback));
    }

    /// Compute MD5 hash of file contents.
    fn hash(&self) -> Option<String> {
        if !self.path.exists() {
            return None;
        }
        let data = fs::read(&self.path).ok()?;
        let digest = Md5::digest(&data);
        Some(format!("{:x}", digest))
    }

    /// Detect whether the file has changed since last check.
    fn detect_change(&mut self) -> bool {
        let current = match self.hash() {
            Some(h) => h,
            None => return false,
        };

        // Get last hash from a shared state (simplified - in production would use Arc<Mutex>)
        // For now, just detect if file exists/doesn't exist
        false
    }

    /// Start the watch loop. Blocks until stop() is called.
    pub fn start(&mut self) {
        self.running.store(true, Ordering::SeqCst);
        let initial_hash = self.hash();
        let mut last_hash = initial_hash;

        println!(
            "[watch] Monitoring {} (poll every {:?})",
            self.path.display(),
            self.interval
        );

        while self.running.load(Ordering::SeqCst) {
            thread::sleep(self.interval);

            if !self.running.load(Ordering::SeqCst) {
                break;
            }

            let current = self.hash();
            if current.is_some() && current != last_hash {
                last_hash = current;
                println!("[watch] Change detected in {}", self.path.display());
                if let Some(ref callback) = self.on_change {
                    callback(self.path.to_string_lossy().to_string());
                }
            }
        }
    }

    /// Stop the watch loop.
    pub fn stop(&self) {
        self.running.store(false, Ordering::SeqCst);
    }
}

impl Drop for Watcher {
    fn drop(&mut self) {
        self.stop();
    }
}

/// Watch result containing change detection info.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WatchResult {
    pub path: String,
    pub changed: bool,
    pub hash: Option<String>,
}

/// Check if a file has changed since last known hash.
pub fn check_file_change(path: &Path, last_hash: Option<&str>) -> (bool, Option<String>) {
    if !path.exists() {
        return (false, None);
    }

    let data = match fs::read(path) {
        Ok(d) => d,
        Err(_) => return (false, None),
    };

    let digest = Md5::digest(&data);
    let current_hash = format!("{:x}", digest);

    let changed = match last_hash {
        Some(h) => h != current_hash,
        None => true,
    };

    (changed, Some(current_hash))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::tempdir;

    #[test]
    fn test_hash_changes_on_file_modification() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("test.json");

        // Write initial content
        std::fs::write(&file_path, r#"{"key": "value1"}"#).unwrap();

        let (_, hash1) = check_file_change(&file_path, None);
        assert!(hash1.is_some());

        // Modify file
        std::fs::write(&file_path, r#"{"key": "value2"}"#).unwrap();

        let (_, hash2) = check_file_change(&file_path, hash1.as_deref());
        assert!(hash2 != hash1);
    }

    #[test]
    fn test_hash_same_when_unchanged() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("test.json");

        std::fs::write(&file_path, r#"{"key": "value"}"#).unwrap();

        let (_, hash1) = check_file_change(&file_path, None);
        let (_, hash2) = check_file_change(&file_path, hash1.as_deref());

        assert_eq!(hash1, hash2);
    }

    #[test]
    fn test_nonexistent_file() {
        let path = Path::new("/nonexistent/path/file.json");
        let (changed, hash) = check_file_change(path, None);
        assert!(!changed);
        assert!(hash.is_none());
    }
}
