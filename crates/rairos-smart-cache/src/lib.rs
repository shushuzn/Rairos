//! rairos-smart-cache — Smart Cache Manager with compression, prioritization, and cost optimization.
//!
//! Ported from `core/smart_cache.py`.

use flate2::read::ZlibDecoder;
use flate2::write::ZlibEncoder;
use flate2::Compression;
use lazy_static::lazy_static;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::PathBuf;
use std::sync::RwLock;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheEntry {
    pub key: String,
    pub created_at: f64,
    pub accessed_at: f64,
    pub access_count: u32,
    pub size_bytes: usize,
    pub priority: i32,
    pub compressed: bool,
    pub ttl: Option<u64>,
}

#[derive(Debug)]
pub struct SmartCache {
    cache_dir: PathBuf,
    max_size_bytes: usize,
    compression_threshold_bytes: usize,
    default_ttl: u64,
    compression_level: u32,
    index: RwLock<HashMap<String, CacheEntry>>,
    stats: RwLock<CacheStats>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CacheStats {
    pub hits: u64,
    pub misses: u64,
    pub evictions: u64,
    pub compressions: u64,
    pub decompressions: u64,
    pub bytes_saved: u64,
    pub total_writes: u64,
}

impl SmartCache {
    pub fn new(
        cache_dir: PathBuf,
        max_size_mb: f64,
        compression_threshold_kb: f64,
        default_ttl: u64,
        compression_level: u32,
    ) -> Self {
        let cache_path = cache_dir.join("smart_cache");
        fs::create_dir_all(&cache_path).ok();

        Self {
            cache_dir: cache_path,
            max_size_bytes: (max_size_mb * 1024.0 * 1024.0) as usize,
            compression_threshold_bytes: (compression_threshold_kb * 1024.0) as usize,
            default_ttl,
            compression_level,
            index: RwLock::new(HashMap::new()),
            stats: RwLock::new(CacheStats::default()),
        }
    }

    fn now_secs() -> f64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs_f64()
    }

    fn get_cache_path(&self, key: &str) -> PathBuf {
        let subdir = &key[..2.min(key.len())];
        let path = self.cache_dir.join(subdir);
        fs::create_dir_all(&path).ok();
        path.join(format!("{}.cache", key))
    }

    fn compress(&self, data: &[u8]) -> Vec<u8> {
        let mut encoder = ZlibEncoder::new(Vec::new(), Compression::new(self.compression_level));
        encoder.write_all(data).unwrap();
        encoder.finish().unwrap()
    }

    fn decompress(&self, data: &[u8]) -> Vec<u8> {
        let mut decoder = ZlibDecoder::new(data);
        let mut result = Vec::new();
        decoder.read_to_end(&mut result).unwrap();
        result
    }

    fn get_total_size(&self) -> usize {
        let index = self.index.read().unwrap();
        index.values().map(|e| e.size_bytes).sum()
    }

    fn evict_if_needed(&self) {
        while self.get_total_size() > self.max_size_bytes {
            let did_evict = {
                let mut index = self.index.write().unwrap();
                if index.is_empty() {
                    break;
                }

                let mut min_score = f64::INFINITY;
                let mut evict_key: Option<String> = None;

                for (key, entry) in index.iter() {
                    let score = entry.accessed_at - (entry.priority as f64 * 1000.0);
                    if score < min_score {
                        min_score = score;
                        evict_key = Some(key.clone());
                    }
                }

                if let Some(ref key) = evict_key {
                    let path = self.get_cache_path(key);
                    let _ = fs::remove_file(&path);
                    if let Some(parent) = path.parent() {
                        if parent != self.cache_dir
                            && parent.read_dir().map_or(true, |mut d| d.next().is_none())
                        {
                            let _ = fs::remove_dir(parent);
                        }
                    }
                    index.remove(key);
                    true
                } else {
                    false
                }
            };

            if did_evict {
                let mut stats = self.stats.write().unwrap();
                stats.evictions += 1;
            } else {
                break;
            }
        }
    }

    pub fn set(&self, key: &str, data: &serde_json::Value, ttl: Option<u64>, priority: i32) {
        let serialized = serde_json::to_vec(data).unwrap();
        let compressed = serialized.len() >= self.compression_threshold_bytes;

        let data_to_store = if compressed {
            let compressed_data = self.compress(&serialized);
            let mut stats = self.stats.write().unwrap();
            stats.compressions += 1;
            stats.bytes_saved += serialized.len().saturating_sub(compressed_data.len()) as u64;
            compressed_data
        } else {
            serialized
        };

        let now = Self::now_secs();
        let entry = CacheEntry {
            key: key.to_string(),
            created_at: now,
            accessed_at: now,
            access_count: 1,
            size_bytes: data_to_store.len(),
            priority,
            compressed,
            ttl,
        };

        let path = self.get_cache_path(key);
        if let Ok(mut file) = File::create(&path) {
            let _ = file.write_all(&data_to_store);
        }

        {
            let mut index = self.index.write().unwrap();
            index.insert(key.to_string(), entry);
        }

        self.evict_if_needed();

        {
            let mut stats = self.stats.write().unwrap();
            stats.total_writes += 1;
        }
    }

    pub fn get(&self, key: &str) -> Option<serde_json::Value> {
        let entry = {
            let index = self.index.read().unwrap();
            index.get(key).cloned()
        };

        let entry = match entry {
            Some(e) => e,
            None => {
                let mut stats = self.stats.write().unwrap();
                stats.misses += 1;
                return None;
            }
        };

        let ttl = entry.ttl.unwrap_or(self.default_ttl);
        if (Self::now_secs() - entry.created_at) > ttl as f64 {
            self.remove(key);
            let mut stats = self.stats.write().unwrap();
            stats.misses += 1;
            return None;
        }

        let path = self.get_cache_path(key);
        if !path.exists() {
            self.remove(key);
            let mut stats = self.stats.write().unwrap();
            stats.misses += 1;
            return None;
        }

        let mut file = match File::open(&path) {
            Ok(f) => f,
            Err(_) => {
                self.remove(key);
                let mut stats = self.stats.write().unwrap();
                stats.misses += 1;
                return None;
            }
        };
        let mut data = Vec::new();
        if file.read_to_end(&mut data).is_err() {
            let mut stats = self.stats.write().unwrap();
            stats.misses += 1;
            return None;
        }

        let decompressed = if entry.compressed {
            let decompressed_data = self.decompress(&data);
            let mut stats = self.stats.write().unwrap();
            stats.decompressions += 1;
            decompressed_data
        } else {
            data
        };

        let result = match serde_json::from_slice(&decompressed) {
            Ok(r) => r,
            Err(_) => {
                let mut stats = self.stats.write().unwrap();
                stats.misses += 1;
                return None;
            }
        };

        {
            let mut index = self.index.write().unwrap();
            if let Some(e) = index.get_mut(key) {
                e.accessed_at = Self::now_secs();
                e.access_count += 1;
            }
        }

        {
            let mut stats = self.stats.write().unwrap();
            stats.hits += 1;
        }

        Some(result)
    }

    pub fn remove(&self, key: &str) {
        let path = self.get_cache_path(key);
        let _ = fs::remove_file(&path);

        let mut index = self.index.write().unwrap();
        index.remove(key);
    }

    pub fn clear(&self) {
        let keys: Vec<String> = {
            let index = self.index.read().unwrap();
            index.keys().cloned().collect()
        };

        for key in keys {
            self.remove(&key);
        }

        let mut index = self.index.write().unwrap();
        index.clear();
    }

    pub fn get_stats(&self) -> SmartCacheStats {
        let total_size = self.get_total_size();
        let total_entries = self.index.read().unwrap().len();

        let (hits, misses) = {
            let stats = self.stats.read().unwrap();
            (stats.hits, stats.misses)
        };

        let hit_rate = if hits + misses > 0 {
            (hits as f64 / (hits + misses) as f64) * 100.0
        } else {
            0.0
        };

        let more_stats = self.stats.read().unwrap().clone();

        SmartCacheStats {
            total_entries,
            total_size_mb: total_size as f64 / (1024.0 * 1024.0),
            max_size_mb: self.max_size_bytes as f64 / (1024.0 * 1024.0),
            usage_percent: if self.max_size_bytes > 0 {
                (total_size as f64 / self.max_size_bytes as f64) * 100.0
            } else {
                0.0
            },
            hits,
            misses,
            hit_rate_percent: hit_rate,
            evictions: more_stats.evictions,
            compressions: more_stats.compressions,
            decompressions: more_stats.decompressions,
            bytes_saved: more_stats.bytes_saved,
            total_writes: more_stats.total_writes,
        }
    }

    pub fn cleanup_expired(&self) -> usize {
        let now = Self::now_secs();
        let mut removed = 0;

        let keys_to_remove: Vec<String> = {
            let index = self.index.read().unwrap();
            index
                .iter()
                .filter(|(_, entry)| {
                    let ttl = entry.ttl.unwrap_or(self.default_ttl);
                    (now - entry.created_at) > ttl as f64
                })
                .map(|(k, _)| k.clone())
                .collect()
        };

        for key in keys_to_remove {
            self.remove(&key);
            removed += 1;
        }

        removed
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SmartCacheStats {
    pub total_entries: usize,
    pub total_size_mb: f64,
    pub max_size_mb: f64,
    pub usage_percent: f64,
    pub hits: u64,
    pub misses: u64,
    pub hit_rate_percent: f64,
    pub evictions: u64,
    pub compressions: u64,
    pub decompressions: u64,
    pub bytes_saved: u64,
    pub total_writes: u64,
}

lazy_static! {
    static ref GLOBAL_SMART_CACHE: RwLock<Option<SmartCache>> = RwLock::new(None);
}

pub fn get_smart_cache() -> SmartCache {
    let cache = GLOBAL_SMART_CACHE.read().unwrap();
    if let Some(ref c) = *cache {
        return SmartCache::new(
            c.cache_dir.clone(),
            c.max_size_bytes as f64 / (1024.0 * 1024.0),
            c.compression_threshold_bytes as f64 / 1024.0,
            c.default_ttl,
            c.compression_level,
        );
    }
    get_smart_cache_with_defaults()
}

pub fn get_smart_cache_with_defaults() -> SmartCache {
    let default_dir = dirs::cache_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("ai_research_os")
        .join("smart_cache");
    SmartCache::new(default_dir, 500.0, 10.0, 86400, 6)
}

pub fn configure_smart_cache(cache_dir: PathBuf, max_size_mb: f64) {
    let cache = SmartCache::new(cache_dir, max_size_mb, 10.0, 86400, 6);
    let mut global = GLOBAL_SMART_CACHE.write().unwrap();
    *global = Some(cache);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_cache() -> SmartCache {
        SmartCache::new(
            std::env::temp_dir().join("test_smart_cache"),
            10.0,
            1.0,
            3600,
            6,
        )
    }

    #[test]
    fn test_smart_cache_new() {
        let cache = test_cache();
        assert!(cache.get_stats().total_entries == 0);
    }

    #[test]
    fn test_smart_cache_set_and_get() {
        let cache = test_cache();
        let data = serde_json::json!({"key": "value", "number": 42});
        cache.set("test_key", &data, None, 0);

        let result = cache.get("test_key");
        assert!(result.is_some());
        assert_eq!(result.unwrap(), data);
    }

    #[test]
    fn test_smart_cache_get_nonexistent() {
        let cache = test_cache();
        let result = cache.get("nonexistent");
        assert!(result.is_none());
    }

    #[test]
    fn test_smart_cache_remove() {
        let cache = test_cache();
        let data = serde_json::json!({"test": true});
        cache.set("remove_key", &data, None, 0);
        cache.remove("remove_key");

        let result = cache.get("remove_key");
        assert!(result.is_none());
    }

    #[test]
    fn test_smart_cache_clear() {
        let cache = test_cache();
        let data = serde_json::json!({"test": true});
        cache.set("key1", &data, None, 0);
        cache.set("key2", &data, None, 0);

        cache.clear();
        assert!(cache.get("key1").is_none());
        assert!(cache.get("key2").is_none());
    }

    #[test]
    fn test_smart_cache_stats() {
        let cache = test_cache();
        let data = serde_json::json!({"test": true});
        cache.set("key1", &data, None, 0);
        cache.get("key1");
        cache.get("nonexistent");

        let stats = cache.get_stats();
        assert_eq!(stats.total_entries, 1);
        assert!(stats.hits >= 1);
        assert!(stats.misses >= 1);
    }

    #[test]
    fn test_smart_cache_priority() {
        let cache = test_cache();
        let data = serde_json::json!({"test": true});
        cache.set("low_prio", &data, None, 0);
        cache.set("high_prio", &data, None, 5);

        let stats1 = cache.get_stats();
        assert_eq!(stats1.total_entries, 2);
    }
}
