//! rairos-cache — HTTP response cache for arXiv and Crossref API calls.
//!
//! Ported from `core/cache.py`.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::RwLock;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone)]
pub struct CacheConfig {
    pub cache_dir: PathBuf,
    pub ttl_seconds: u64,
    pub max_cache_files: usize,
    pub memory_cache_max_size: usize,
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            cache_dir: dirs::cache_dir()
                .unwrap_or_else(|| PathBuf::from("."))
                .join("ai_research_os"),
            ttl_seconds: 3600,
            max_cache_files: 10000,
            memory_cache_max_size: 1000,
        }
    }
}

impl CacheConfig {
    pub fn cache_dir(&self) -> PathBuf {
        let mut path = self.cache_dir.clone();
        path.push("rairos_cache");
        path
    }

    pub fn source_dir(&self, source: &str) -> PathBuf {
        self.cache_dir().join(source)
    }
}

type MemoryCacheKey = (String, String);
type MemoryCacheValue = (f64, serde_json::Value);

#[derive(Default)]
struct GlobalCache {
    config: CacheConfig,
    memory: HashMap<MemoryCacheKey, MemoryCacheValue>,
}

fn now_secs() -> f64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs_f64()
}

lazy_static::lazy_static! {
    static ref GLOBAL_CACHE: RwLock<GlobalCache> = RwLock::new(GlobalCache::default());
}

pub fn get_cache_config() -> CacheConfig {
    GLOBAL_CACHE.read().unwrap().config.clone()
}

pub fn configure_cache(
    cache_dir: PathBuf,
    ttl_seconds: u64,
    max_cache_files: usize,
    memory_cache_max_size: usize,
) {
    let mut cache = GLOBAL_CACHE.write().unwrap();
    cache.config = CacheConfig {
        cache_dir,
        ttl_seconds,
        max_cache_files,
        memory_cache_max_size,
    };
}

fn cache_dir(source: &str) -> PathBuf {
    let config = GLOBAL_CACHE.read().unwrap();
    let dir = config.config.source_dir(source);
    drop(config);
    fs::create_dir_all(&dir).ok();
    dir
}

fn cache_path(source: &str, key: &str) -> PathBuf {
    let safe = key.replace(['/', '\\'], "_");
    cache_dir(source).join(format!("{}.json", safe))
}

fn evict_memory_cache_if_needed() {
    let items_to_evict: Vec<MemoryCacheKey> = {
        let cache = GLOBAL_CACHE.read().unwrap();
        let max_size = cache.config.memory_cache_max_size;
        let memory_len = cache.memory.len();

        if memory_len <= max_size {
            return;
        }

        let mut items: Vec<_> = cache.memory.iter().collect();
        items.sort_by(|a, b| {
            a.1 .0
                .partial_cmp(&b.1 .0)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        let evict_count = (memory_len / 5).max(1);
        items
            .into_iter()
            .take(evict_count)
            .map(|(k, _)| k.clone())
            .collect()
    };

    if !items_to_evict.is_empty() {
        let mut cache = GLOBAL_CACHE.write().unwrap();
        for key in items_to_evict {
            cache.memory.remove(&key);
        }
    }
}

pub fn get_cached(source: &str, key: &str) -> Option<serde_json::Value> {
    let cache_key = (source.to_string(), key.to_string());
    let ttl = GLOBAL_CACHE.read().unwrap().config.ttl_seconds;

    {
        let mut cache = GLOBAL_CACHE.write().unwrap();
        if let Some((timestamp, data)) = cache.memory.get(&cache_key) {
            if now_secs() - timestamp < ttl as f64 {
                return Some(data.clone());
            } else {
                cache.memory.remove(&cache_key);
            }
        }
    }

    let path = cache_path(source, key);
    if !path.exists() {
        return None;
    }

    let metadata = match path.metadata() {
        Ok(m) => m,
        Err(_) => return None,
    };

    if let Ok(modified) = metadata.modified() {
        let age = SystemTime::now()
            .duration_since(modified)
            .map(|d| d.as_secs())
            .unwrap_or(0);

        if age > ttl {
            return None;
        }
    }

    match fs::read_to_string(&path) {
        Ok(contents) => match serde_json::from_str::<serde_json::Value>(&contents) {
            Ok(data) => {
                let mut cache = GLOBAL_CACHE.write().unwrap();
                cache.memory.insert(cache_key, (now_secs(), data.clone()));
                evict_memory_cache_if_needed();
                Some(data)
            }
            Err(_) => None,
        },
        Err(_) => None,
    }
}

pub fn set_cached(source: &str, key: &str, data: &serde_json::Value) {
    let cache_key = (source.to_string(), key.to_string());

    let should_evict = {
        let mut cache = GLOBAL_CACHE.write().unwrap();
        cache.memory.insert(cache_key, (now_secs(), data.clone()));
        cache.memory.len() > cache.config.memory_cache_max_size
    };

    if should_evict {
        evict_memory_cache_if_needed();
    }

    let path = cache_path(source, key);
    if let Ok(json) = serde_json::to_string_pretty(data) {
        let _ = fs::write(path, json);
    }
}

pub fn clear_cache(source: Option<&str>) {
    let mut cache = GLOBAL_CACHE.write().unwrap();

    if let Some(src) = source {
        let keys: Vec<_> = cache
            .memory
            .keys()
            .filter(|(s, _)| s == src)
            .cloned()
            .collect();
        for key in keys {
            cache.memory.remove(&key);
        }

        let dir = cache.config.source_dir(src);
        if let Ok(entries) = fs::read_dir(dir) {
            for entry in entries.filter_map(|e| e.ok()) {
                let _ = fs::remove_file(entry.path());
            }
        }
    } else {
        cache.memory.clear();

        let root = cache.config.cache_dir();
        if let Ok(entries) = fs::read_dir(&root) {
            for entry in entries.filter_map(|e| e.ok()) {
                if entry.path().is_dir() {
                    if let Ok(subentries) = fs::read_dir(entry.path()) {
                        for subentry in subentries.filter_map(|e| e.ok()) {
                            let _ = fs::remove_file(subentry.path());
                        }
                    }
                }
            }
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheStats {
    pub memory_cache_size: usize,
    pub memory_cache_max_size: usize,
    pub disk_cache_dir: String,
    pub ttl_seconds: u64,
    pub max_cache_files: usize,
    pub disk_cache_sizes: HashMap<String, usize>,
}

pub fn get_cache_stats() -> CacheStats {
    let cache = GLOBAL_CACHE.read().unwrap();
    let memory_size = cache.memory.len();
    let config = cache.config.clone();
    let mut disk_sizes = HashMap::new();

    let root = config.cache_dir();
    if let Ok(entries) = fs::read_dir(&root) {
        for entry in entries.filter_map(|e| e.ok()) {
            if entry.path().is_dir() {
                if let Some(name) = entry.path().file_name() {
                    let count = fs::read_dir(entry.path())
                        .map(|d| d.filter_map(|e| e.ok()).count())
                        .unwrap_or(0);
                    disk_sizes.insert(name.to_string_lossy().to_string(), count);
                }
            }
        }
    }

    CacheStats {
        memory_cache_size: memory_size,
        memory_cache_max_size: config.memory_cache_max_size,
        disk_cache_dir: root.to_string_lossy().to_string(),
        ttl_seconds: config.ttl_seconds,
        max_cache_files: config.max_cache_files,
        disk_cache_sizes: disk_sizes,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_cache_config() {
        let config = get_cache_config();
        assert!(config.ttl_seconds > 0);
    }

    #[test]
    fn test_cache_config_default() {
        let config = CacheConfig::default();
        assert!(config.ttl_seconds > 0);
        assert!(config.max_cache_files > 0);
    }

    #[test]
    fn test_config_source_dir() {
        let config = CacheConfig::default();
        let dir = config.source_dir("test");
        assert!(dir.to_string_lossy().ends_with("test"));
    }

    #[test]
    fn test_set_and_get_memory_cache() {
        configure_cache(std::env::temp_dir().join("test_mem_cache"), 3600, 100, 50);

        let data = serde_json::json!({"key": "value"});
        set_cached("mem_test", "key1", &data);

        let result = get_cached("mem_test", "key1");
        assert!(result.is_some());
    }

    #[test]
    fn test_get_cached_nonexistent() {
        configure_cache(std::env::temp_dir().join("test_nonexistent"), 3600, 100, 50);

        let result = get_cached("nonexistent_source", "nonexistent_key");
        assert!(result.is_none());
    }

    #[test]
    fn test_clear_memory_cache_source() {
        configure_cache(std::env::temp_dir().join("test_clear_mem"), 3600, 100, 50);

        let data = serde_json::json!({"test": true});
        set_cached("source_x", "key1", &data);
        set_cached("source_y", "key2", &data);

        clear_cache(Some("source_x"));

        assert!(get_cached("source_x", "key1").is_none());
        assert!(get_cached("source_y", "key2").is_some());
    }

    #[test]
    fn test_get_cache_stats() {
        configure_cache(std::env::temp_dir().join("test_stats"), 3600, 100, 50);

        let stats = get_cache_stats();
        assert!(stats.ttl_seconds > 0);
        assert!(stats.max_cache_files > 0);
    }
}
