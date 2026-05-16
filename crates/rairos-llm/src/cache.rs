//! Disk-based LLM response cache with TTL.
//!
//! Stores cached responses as JSON files in ~/.ai_research_os/cache/llm/,
//! keyed by SHA-256 hash of the serialized request.
//! Cache entries expire after a configurable TTL (default 1 hour).

use rairos_core::constants::{AIROS_DIR_NAME, CACHE_DIR, LLM_CACHE_DIR};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

const DEFAULT_TTL_SECS: u64 = 3600; // 1 hour

/// Cache entry stored on disk
#[derive(Debug, Clone, Serialize, Deserialize)]
struct CacheEntry {
    /// Unix timestamp in milliseconds when this entry was created
    created_at_ms: u64,
    /// TTL in milliseconds (0 = never expires)
    ttl_ms: u64,
    /// The cached response content
    content: String,
    /// Optional metadata (e.g. model used, token count)
    metadata: HashMap<String, String>,
}

impl CacheEntry {
    fn is_expired(&self) -> bool {
        if self.ttl_ms == 0 {
            return false;
        }
        let now_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;
        now_ms > self.created_at_ms + self.ttl_ms
    }
}

/// LLM response cache
pub struct LlmCache {
    cache_dir: PathBuf,
    default_ttl: Duration,
}

impl Default for LlmCache {
    fn default() -> Self {
        let base = dirs_next()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(AIROS_DIR_NAME)
            .join(CACHE_DIR);
        Self {
            cache_dir: base.join(LLM_CACHE_DIR),
            default_ttl: Duration::from_secs(DEFAULT_TTL_SECS),
        }
    }
}

impl LlmCache {
    pub fn new(cache_dir: Option<PathBuf>, ttl: Option<Duration>) -> Self {
        let dir = cache_dir.unwrap_or_else(|| {
            dirs_next()
                .unwrap_or_else(|| PathBuf::from("."))
                .join(AIROS_DIR_NAME)
                .join(CACHE_DIR)
                .join(LLM_CACHE_DIR)
        });
        Self {
            cache_dir: dir,
            default_ttl: ttl.unwrap_or_else(|| Duration::from_secs(DEFAULT_TTL_SECS)),
        }
    }

    /// Generate a cache key from the request content
    pub fn make_key(model: &str, messages: &[u8], temperature: f32) -> String {
        use std::hash::{Hash, Hasher};
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        model.hash(&mut hasher);
        messages.hash(&mut hasher);
        temperature.to_bits().hash(&mut hasher);
        format!("{:x}", hasher.finish())
    }

    /// Get a cached response. Returns None if not found or expired.
    pub fn get(&self, key: &str) -> Option<String> {
        let path = self.cache_dir.join(format!("{}.json", key));
        if !path.exists() {
            return None;
        }
        let data = std::fs::read_to_string(&path).ok()?;
        let entry: CacheEntry = serde_json::from_str(&data).ok()?;
        if entry.is_expired() {
            let _ = std::fs::remove_file(&path);
            return None;
        }
        Some(entry.content)
    }

    /// Store a response in the cache
    pub fn set(&self, key: &str, content: &str, ttl: Option<Duration>, metadata: HashMap<String, String>) -> Result<(), String> {
        std::fs::create_dir_all(&self.cache_dir)
            .map_err(|e| format!("Failed to create cache dir: {}", e))?;

        let now_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;

        let ttl_ms = ttl.unwrap_or(self.default_ttl).as_millis() as u64;
        let entry = CacheEntry {
            created_at_ms: now_ms,
            ttl_ms,
            content: content.to_string(),
            metadata,
        };

        let path = self.cache_dir.join(format!("{}.json", key));
        let json = serde_json::to_string(&entry)
            .map_err(|e| format!("Failed to serialize cache entry: {}", e))?;
        std::fs::write(&path, json)
            .map_err(|e| format!("Failed to write cache: {}", e))?;
        Ok(())
    }

    /// Invalidate a specific cache entry
    pub fn invalidate(&self, key: &str) -> bool {
        let path = self.cache_dir.join(format!("{}.json", key));
        if path.exists() {
            std::fs::remove_file(&path).is_ok()
        } else {
            false
        }
    }

    /// Clear all expired entries
    pub fn clear_expired(&self) -> usize {
        let ok_entries = std::fs::read_dir(&self.cache_dir).ok();
        let mut cleared = 0;
        if let Some(entries) = ok_entries {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().map(|e| e == "json").unwrap_or(false) {
                    if let Ok(data) = std::fs::read_to_string(&path) {
                        if let Ok(cache_entry) = serde_json::from_str::<CacheEntry>(&data) {
                            if cache_entry.is_expired() {
                                let _ = std::fs::remove_file(&path);
                                cleared += 1;
                            }
                        }
                    }
                }
            }
        }
        cleared
    }

    /// Get cache statistics
    pub fn stats(&self) -> CacheStats {
        let total = std::fs::read_dir(&self.cache_dir)
            .map(|entries| entries.flatten().count())
            .unwrap_or(0);
        let expired = self.clear_expired();
        CacheStats {
            total_entries: total,
            expired_removed: expired,
            cache_dir: self.cache_dir.to_string_lossy().to_string(),
        }
    }

    /// Get the cache directory path
    pub fn path(&self) -> &PathBuf {
        &self.cache_dir
    }
}

/// Cache statistics
#[derive(Debug)]
pub struct CacheStats {
    pub total_entries: usize,
    pub expired_removed: usize,
    pub cache_dir: String,
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

    #[test]
    fn test_cache_set_get() {
        let dir = std::env::temp_dir().join(format!("llm_cache_test_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let cache = LlmCache::new(Some(dir.clone()), Some(Duration::from_secs(3600)));
        let key = "test_key_123";
        cache.set(key, "cached response", None, HashMap::new()).unwrap();
        let result = cache.get(key);
        assert_eq!(result, Some("cached response".to_string()));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_cache_expiry() {
        let dir = std::env::temp_dir().join(format!("llm_cache_exp_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let cache = LlmCache::new(Some(dir.clone()), Some(Duration::from_secs(3600)));
        let key = "exp_key";
        cache.set(key, "data", Some(Duration::from_millis(1)), HashMap::new()).unwrap();
        std::thread::sleep(std::time::Duration::from_millis(10));
        assert!(cache.get(key).is_none(), "expired entry should return None");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_make_key_deterministic() {
        let key1 = LlmCache::make_key("gpt4", b"hello", 0.7);
        let key2 = LlmCache::make_key("gpt4", b"hello", 0.7);
        assert_eq!(key1, key2, "same input should produce same key");
    }

    #[test]
    fn test_make_key_different_model() {
        let key1 = LlmCache::make_key("gpt4", b"hello", 0.7);
        let key2 = LlmCache::make_key("gpt3", b"hello", 0.7);
        assert_ne!(key1, key2, "different model should produce different key");
    }

    #[test]
    fn test_invalidate() {
        let dir = std::env::temp_dir().join(format!("llm_cache_inv_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let cache = LlmCache::new(Some(dir.clone()), None);
        cache.set("k", "v", None, HashMap::new()).unwrap();
        assert!(cache.get("k").is_some());
        assert!(cache.invalidate("k"));
        assert!(cache.get("k").is_none());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
