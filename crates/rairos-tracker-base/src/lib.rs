//! rairos-tracker-base — Generic JSON file persistence utilities.
//!
//! Ported from `llm/tracker_base.py` (78 LOC, pure stdlib).
//!
//! Provides a reusable [`JsonFileStore`] trait for dataclass-based trackers
//! that need safe JSON file load/save with atomic writes.

use std::fs::{self, File};
use std::io::{self, BufRead, BufReader, Write};
use std::path::Path;

// ─────────────────────────────────────────────────────────────────────────────
// Core trait
// ─────────────────────────────────────────────────────────────────────────────

/// Sentinel trait for self-referential trait bounds.
pub trait JsonFileStoreSealed {}

/// JSON file persistence mixin for tracker classes.
///
/// Subclasses implement:
/// - [`data_file`](JsonFileStore::data_file) — path to the JSON file
/// - [`from_dict`](JsonFileStore::from_dict) — reconstruct instance from a dict
/// - [`to_dict`](JsonFileStore::to_dict) — serialize instance to a dict
pub trait JsonFileStore: JsonFileStoreSealed {
    /// Path to the JSON data file (one JSON object per line, i.e. JSONL).
    fn data_file(&self) -> &Path;

    /// Reconstruct an instance from a plain dict.
    fn from_dict(d: &dict::Dict) -> Self
    where
        Self: Sized;

    /// Serialize self to a plain dict.
    fn to_dict(&self) -> dict::Dict;

    /// Load all items from the JSONL file.
    /// Returns an empty `Vec` if the file is absent, empty, or unreadable.
    fn load(&self) -> Vec<Self>
    where
        Self: Sized,
    {
        load_jsonl(self.data_file(), Self::from_dict)
    }

    /// Save all items to the JSONL file atomically (write-then-rename).
    fn save(&self, items: &[Self])
    where
        Self: Sized,
    {
        save_jsonl(self.data_file(), items, Self::to_dict);
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Dict type (plain String -> String map, matching Python dict)
// ─────────────────────────────────────────────────────────────────────────────

pub mod dict {
    use std::collections::HashMap;
    pub type Dict = HashMap<String, String>;
}

// ─────────────────────────────────────────────────────────────────────────────
// JSON parsing / serialization (pure stdlib, no external deps)
// ─────────────────────────────────────────────────────────────────────────────

/// Parse a single JSON object `{"key": "value", ...}` from a byte slice.
/// Returns `None` if no `{` is found.
pub fn parse_dict(bytes: &[u8]) -> Option<dict::Dict> {
    // Skip leading whitespace
    let bytes = trim_start(bytes);
    if bytes.is_empty() || bytes[0] != b'{' {
        return None;
    }
    let mut dict = dict::Dict::new();
    let mut bytes = &bytes[1..];
    // empty object — only `}` qualifies; whitespace is handled below
    if matches!(bytes.first(), Some(&b'}') | None) {
        return Some(dict);
    }
    loop {
        // skip whitespace
        while matches!(bytes.first(), Some(&b' ' | b'\t' | b'\n' | b'\r')) {
            bytes = &bytes[1..];
        }
        // expect string key
        let (key, rest) = parse_string(bytes)?;
        bytes = rest;
        // skip whitespace and colon
        while matches!(bytes.first(), Some(&b' ' | b'\t' | b'\n' | b'\r' | b':')) {
            bytes = &bytes[1..];
        }
        // parse value
        let (value, rest) = parse_value(bytes)?;
        dict.insert(key, value);
        bytes = rest;
        // skip whitespace, look for comma or closing brace
        while matches!(bytes.first(), Some(&b' ' | b'\t' | b'\n' | b'\r')) {
            bytes = &bytes[1..];
        }
        match bytes.first() {
            Some(&b',') => {
                bytes = &bytes[1..];
            }
            Some(&b'}') => return Some(dict),
            _ => return None,
        }
    }
}

/// Parse a JSON string value from the front of `bytes`.
fn parse_value(bytes: &[u8]) -> Option<(String, &[u8])> {
    let bytes = trim_start(bytes);
    if bytes.is_empty() {
        return None;
    }
    match bytes[0] {
        b'"' => parse_string(bytes),
        b'n' if bytes.starts_with(b"null") => Some((String::new(), &bytes[4..])),
        b't' if bytes.starts_with(b"true") => Some(("true".to_string(), &bytes[4..])),
        b'f' if bytes.starts_with(b"false") => Some(("false".to_string(), &bytes[5..])),
        _ => parse_number(bytes),
    }
}

/// Parse a JSON string `"..."` from the front of `bytes`.
fn parse_string(bytes: &[u8]) -> Option<(String, &[u8])> {
    if bytes.is_empty() || bytes[0] != b'"' {
        return None;
    }
    let mut s = String::new();
    let mut bytes = &bytes[1..];
    loop {
        if bytes.is_empty() {
            return None;
        }
        match bytes[0] {
            b'"' => return Some((s, &bytes[1..])),
            b'\\' if bytes.len() >= 2 => {
                match bytes[1] {
                    b'"' => { s.push('"'); bytes = &bytes[2..]; }
                    b'\\' => { s.push('\\'); bytes = &bytes[2..]; }
                    b'/' => { s.push('/'); bytes = &bytes[2..]; }
                    b'n' => { s.push('\n'); bytes = &bytes[2..]; }
                    b'r' => { s.push('\r'); bytes = &bytes[2..]; }
                    b't' => { s.push('\t'); bytes = &bytes[2..]; }
                    b'u' if bytes.len() >= 6 => {
                        if let Ok(c) = u16::from_str_radix(std::str::from_utf8(&bytes[2..6]).unwrap_or(""), 16) {
                            s.push(char::from_u32(c as u32).unwrap_or('\u{FFFD}'));
                            bytes = &bytes[6..];
                        } else {
                            return None;
                        }
                    }
                    _ => return None,
                }
            }
            c => {
                if c < 0x80 {
                    // ASCII — single byte, safe to cast
                    s.push(c as char);
                    bytes = &bytes[1..];
                } else {
                    // Multi-byte UTF-8 — decode properly
                    let ch = decode_utf8(bytes);
                    if ch == '\u{FFFD}' {
                        return None;
                    }
                    s.push(ch);
                    bytes = &bytes[char_len(ch)..];
                }
            }
        }
    }
}

fn decode_utf8(bytes: &[u8]) -> char {
    if bytes.is_empty() {
        return '\u{FFFD}';
    }
    let b0 = bytes[0];
    if b0 < 0x80 {
        return b0 as char;
    }
    let (ch, _len) = if (b0 & 0xE0) == 0xC0 && bytes.len() >= 2 {
        let ch = ((b0 as u32 & 0x1F) << 6) | (bytes[1] as u32 & 0x3F);
        (ch, 2)
    } else if (b0 & 0xF0) == 0xE0 && bytes.len() >= 3 {
        let ch = ((b0 as u32 & 0x0F) << 12)
            | ((bytes[1] as u32 & 0x3F) << 6)
            | (bytes[2] as u32 & 0x3F);
        (ch, 3)
    } else if (b0 & 0xF8) == 0xF0 && bytes.len() >= 4 {
        let ch = ((b0 as u32 & 0x07) << 18)
            | ((bytes[1] as u32 & 0x3F) << 12)
            | ((bytes[2] as u32 & 0x3F) << 6)
            | (bytes[3] as u32 & 0x3F);
        (ch, 4)
    } else {
        return '\u{FFFD}';
    };
    char::from_u32(ch).unwrap_or('\u{FFFD}')
}

fn char_len(ch: char) -> usize {
    let n = ch as u32;
    if n < 0x80 { 1 } else if n < 0x800 { 2 } else if n < 0x10000 { 3 } else { 4 }
}

/// Parse a JSON number from the front of `bytes`.  Returns the unparsed string.
fn parse_number(bytes: &[u8]) -> Option<(String, &[u8])> {
    let bytes = trim_start(bytes);
    if bytes.is_empty() || !matches!(bytes[0], b'-' | b'0'..=b'9') {
        return None;
    }
    let mut i = 1;
    while i < bytes.len() && matches!(bytes[i], b'0'..=b'9' | b'.' | b'e' | b'E' | b'+' | b'-') {
        i += 1;
    }
    let num = String::from_utf8_lossy(&bytes[..i]).to_string();
    Some((num, &bytes[i..]))
}

fn trim_start(bytes: &[u8]) -> &[u8] {
    let mut i = 0;
    while i < bytes.len() && matches!(bytes[i], b' ' | b'\t' | b'\n' | b'\r') {
        i += 1;
    }
    &bytes[i..]
}

/// Serialize a dict as a JSON object bytes.
pub fn dict_to_json(d: &dict::Dict) -> Vec<u8> {
    let mut out = Vec::new();
    out.push(b'{');
    let mut it = d.iter();
    if let Some((k, v)) = it.next() {
        string_to_json(&mut out, k);
        out.push(b':');
        out.push(b' ');
        string_to_json(&mut out, v);
        for (k, v) in it {
            out.push(b',');
            out.push(b' ');
            string_to_json(&mut out, k);
            out.push(b':');
            out.push(b' ');
            string_to_json(&mut out, v);
        }
    }
    out.push(b'}');
    out
}

fn string_to_json(out: &mut Vec<u8>, s: &str) {
    out.push(b'"');
    for c in s.chars() {
        match c {
            '"' => out.extend_from_slice(b"\\\""),
            '\\' => out.extend_from_slice(b"\\\\"),
            '\n' => out.extend_from_slice(b"\\n"),
            '\r' => out.extend_from_slice(b"\\r"),
            '\t' => out.extend_from_slice(b"\\t"),
            c if c.is_control() => {
                out.push(b'\\');
                out.push(b'u');
                out.push(b'0');
                out.push(b'0');
                let hex = format!("{:02x}", c as u32);
                out.extend_from_slice(hex.as_bytes());
            }
            c => out.extend_from_slice(c.to_string().as_bytes()),
        }
    }
    out.push(b'"');
}

// ─────────────────────────────────────────────────────────────────────────────
// JSONL helpers
// ─────────────────────────────────────────────────────────────────────────────

type FromDictFn<T> = fn(&dict::Dict) -> T;
type ToDictFn<T> = fn(&T) -> dict::Dict;

/// Load items from a JSONL file using the provided `from_dict` constructor.
pub fn load_jsonl<T>(path: &Path, from_dict: FromDictFn<T>) -> Vec<T> {
    if !path.exists() {
        return Vec::new();
    }
    let file = match File::open(path) {
        Ok(f) => f,
        Err(_) => return Vec::new(),
    };
    let reader = BufReader::new(file);
    let mut items = Vec::new();
    for line in reader.split(b'\n') {
        let line = match line {
            Ok(l) => l,
            Err(_) => continue,
        };
        if line.is_empty() {
            continue;
        }
        if let Some(d) = parse_dict(&line) {
            items.push(from_dict(&d));
        }
    }
    items
}

/// Save items to a JSONL file atomically (write-then-rename).
pub fn save_jsonl<T>(path: &Path, items: &[T], to_dict: ToDictFn<T>) {
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    let tmp = path.with_extension("tmp");
    let file = match File::create(&tmp) {
        Ok(f) => f,
        Err(_) => return,
    };
    let mut writer = io::BufWriter::new(file);
    for item in items {
        let d = to_dict(item);
        let json = dict_to_json(&d);
        if writer.write_all(&json).is_err() {
            return;
        }
        if writer.write_all(b"\n").is_err() {
            return;
        }
    }
    let _ = writer.flush();
    let _ = fs::rename(&tmp, path);
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // Minimal tracker-like item for testing
    #[derive(Debug, Clone, PartialEq)]
    struct DummyItem {
        id: String,
        name: String,
        value: f64,
    }

    impl DummyItem {
        fn new(id: &str, name: &str, value: f64) -> Self {
            Self { id: id.to_string(), name: name.to_string(), value }
        }
    }

    impl JsonFileStoreSealed for DummyItem {}

    impl JsonFileStore for DummyItem {
        fn data_file(&self) -> &Path {
            unreachable!("data_file not used in tests")
        }
        fn from_dict(d: &dict::Dict) -> Self {
            Self {
                id: d.get("id").cloned().unwrap_or_default(),
                name: d.get("name").cloned().unwrap_or_default(),
                value: d.get("value").and_then(|v| v.parse().ok()).unwrap_or(0.0),
            }
        }
        fn to_dict(&self) -> dict::Dict {
            let mut m = dict::Dict::new();
            m.insert("id".to_string(), self.id.clone());
            m.insert("name".to_string(), self.name.clone());
            m.insert("value".to_string(), self.value.to_string());
            m
        }
    }

    // ── Self-referential store: correct pattern for JsonFileStore trait ─────────

    #[allow(dead_code)]
    struct DummyItemStore {
        path: std::path::PathBuf,
        items: Vec<DummyItem>,
    }

    impl DummyItemStore {
        #[allow(dead_code)]
        fn new(path: std::path::PathBuf, items: Vec<DummyItem>) -> Self {
            Self { path, items }
        }
    }

    impl JsonFileStoreSealed for DummyItemStore {}

    impl JsonFileStore for DummyItemStore {
        fn data_file(&self) -> &Path {
            &self.path
        }
        fn from_dict(_d: &dict::Dict) -> Self {
            // This store doesn't use dict conversion — it holds items directly
            Self::new(std::env::temp_dir().join("test_tracker_items.jsonl"), vec![])
        }
        fn to_dict(&self) -> dict::Dict {
            dict::Dict::new()
        }
        fn save(&self, items: &[Self]) {
            // Override save so we can pass DummyItem, not DummyItemStore
            let as_dummy: Vec<DummyItem> = items.iter().flat_map(|s| s.items.clone()).collect();
            save_jsonl(&self.path, &as_dummy, DummyItem::to_dict_item);
        }
        fn load(&self) -> Vec<Self> {
            // Override load to return DummyItemStore wrapping DummyItems
            let items = load_jsonl(&self.path, DummyItem::from_dict_item);
            vec![Self::new(self.path.clone(), items)]
        }
    }

    // ── Test helpers using the free functions ──────────────────────────────────

    impl DummyItem {
        fn to_dict_item(this: &DummyItem) -> dict::Dict {
            let mut m = dict::Dict::new();
            m.insert("id".to_string(), this.id.clone());
            m.insert("name".to_string(), this.name.clone());
            m.insert("value".to_string(), this.value.to_string());
            m
        }
        fn from_dict_item(d: &dict::Dict) -> DummyItem {
            DummyItem {
                id: d.get("id").cloned().unwrap_or_default(),
                name: d.get("name").cloned().unwrap_or_default(),
                value: d.get("value").and_then(|v| v.parse().ok()).unwrap_or(0.0),
            }
        }
    }

    // ── JSON parsing / dict tests ────────────────────────────────────────────────

    #[test]
    fn test_parse_dict_basic() {
        let d = parse_dict(b"{\"id\": \"1\", \"name\": \"alpha\"}").unwrap();
        assert_eq!(d.get("id").unwrap(), "1");
        assert_eq!(d.get("name").unwrap(), "alpha");
    }

    #[test]
    fn test_parse_dict_empty() {
        let d = parse_dict(b"{}").unwrap();
        assert!(d.is_empty());
    }

    #[test]
    fn test_parse_dict_whitespace() {
        let d = parse_dict(b"  {  \"a\" : \"b\"  }  ").unwrap();
        assert_eq!(d.get("a").unwrap(), "b");
    }

    #[test]
    fn test_parse_dict_unicode() {
        let d = parse_dict(r#"{"key": "héllo"}"#.as_bytes()).unwrap();
        assert_eq!(d.get("key").unwrap(), "héllo");
    }

    #[test]
    fn test_parse_dict_escape() {
        let d = parse_dict(r#"{"k": "a\"b\\c"}"#.as_bytes()).unwrap();
        assert_eq!(d.get("k").unwrap(), "a\"b\\c");
    }

    #[test]
    fn test_dict_to_json_roundtrip() {
        let mut d = dict::Dict::new();
        d.insert("id".to_string(), "1".to_string());
        d.insert("name".to_string(), "alpha".to_string());
        let json = dict_to_json(&d);
        let parsed = parse_dict(&json).unwrap();
        assert_eq!(parsed.get("id").unwrap(), "1");
        assert_eq!(parsed.get("name").unwrap(), "alpha");
    }

    // ── Free function tests ───────────────────────────────────────────────────────

    #[test]
    fn test_save_and_load() {
        let path = std::env::temp_dir().join("test_tracker_items.jsonl");
        let items = vec![
            DummyItem::new("1", "alpha", 1.0),
            DummyItem::new("2", "beta", 2.0),
        ];

        save_jsonl(&path, &items, DummyItem::to_dict_item);

        let loaded: Vec<DummyItem> = load_jsonl(&path, DummyItem::from_dict_item);
        assert_eq!(loaded.len(), 2);
        assert_eq!(loaded[0].id, "1");
        assert_eq!(loaded[0].name, "alpha");
        assert_eq!(loaded[1].id, "2");
        assert_eq!(loaded[1].name, "beta");
    }

    #[test]
    fn test_load_nonexistent() {
        let path = std::env::temp_dir().join("nonexistent_path_xyz789.jsonl");
        if path.exists() {
            let _ = std::fs::remove_file(&path);
        }
        let loaded: Vec<DummyItem> = load_jsonl(&path, DummyItem::from_dict_item);
        assert!(loaded.is_empty());
    }

    #[test]
    fn test_save_empty() {
        let path = std::env::temp_dir().join("empty_items.jsonl");
        let items: Vec<DummyItem> = vec![];
        save_jsonl(&path, &items, DummyItem::to_dict_item);
        let loaded: Vec<DummyItem> = load_jsonl(&path, DummyItem::from_dict_item);
        assert!(loaded.is_empty());
    }

    #[test]
    fn test_atomic_save() {
        let path = std::env::temp_dir().join("atomic_test.jsonl");
        let items = vec![DummyItem::new("x", "y", 1.0)];
        save_jsonl(&path, &items, DummyItem::to_dict_item);
        // Target file must exist, temp file must not
        assert!(path.exists());
        assert!(!path.with_extension("tmp").exists());
    }
}
