//! rairos-tracker-base — Generic JSON file persistence utilities.
//!
//! Ported from `llm/tracker_base.py`.

use serde::{de::DeserializeOwned, Serialize};
use std::fs;
use std::path::Path;

pub fn load_jsonl<T: DeserializeOwned>(path: &Path) -> Vec<T> {
    if !path.exists() {
        return Vec::new();
    }
    match fs::read_to_string(path) {
        Ok(text) => {
            if text.trim().is_empty() {
                return Vec::new();
            }
            serde_json::from_str(&text).unwrap_or_else(|_| Vec::new())
        }
        Err(_) => Vec::new(),
    }
}

pub fn save_jsonl<T: Serialize>(path: &Path, items: &[T]) -> bool {
    if let Some(parent) = path.parent() {
        if fs::create_dir_all(parent).is_err() {
            return false;
        }
    }
    match serde_json::to_string_pretty(items) {
        Ok(json) => {
            let tmp = path.with_extension("tmp");
            if fs::write(&tmp, json).is_ok() && fs::rename(&tmp, path).is_ok() {
                return true;
            }
            false
        }
        Err(_) => false,
    }
}

pub fn ensure_dir(path: &Path) -> bool {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).is_ok()
    } else {
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
    struct DummyItem {
        id: String,
        name: String,
        value: f64,
    }

    impl DummyItem {
        fn new(id: &str, name: &str, value: f64) -> Self {
            Self {
                id: id.to_string(),
                name: name.to_string(),
                value,
            }
        }
    }

    #[test]
    fn test_save_and_load() {
        let path = std::env::temp_dir().join("test_tracker_items.json");
        let items = vec![
            DummyItem::new("1", "alpha", 1.0),
            DummyItem::new("2", "beta", 2.0),
        ];

        assert!(save_jsonl(&path, &items));

        let loaded: Vec<DummyItem> = load_jsonl(&path);
        assert_eq!(loaded.len(), 2);
        assert_eq!(loaded[0].id, "1");
        assert_eq!(loaded[1].id, "2");
    }

    #[test]
    fn test_load_nonexistent() {
        let path = std::env::temp_dir().join("nonexistent_path_xyz789.json");
        if path.exists() {
            let _ = std::fs::remove_file(&path);
        }
        let loaded: Vec<DummyItem> = load_jsonl(&path);
        assert!(loaded.is_empty());
    }

    #[test]
    fn test_save_empty() {
        let path = std::env::temp_dir().join("empty_items.json");
        let items: Vec<DummyItem> = vec![];
        assert!(save_jsonl(&path, &items));
        let loaded: Vec<DummyItem> = load_jsonl(&path);
        assert!(loaded.is_empty());
    }
}
