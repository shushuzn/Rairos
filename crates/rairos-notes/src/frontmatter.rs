//! Frontmatter parsing for markdown notes.
//!
//! Supports YAML-like frontmatter with key: value pairs and list items.

use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::LazyLock;

static RE_KEY: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"^\s*([A-Za-z0-9_\-]+)\s*:\s*(.*)\s*$"#).expect("valid regex")
});

static RE_LIST_ITEM: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"^\s+-\s+(.*)\s*$"#).expect("valid regex")
});

static RE_DATE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^\d{4}-\d{2}-\d{2}$").expect("valid regex")
});

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Frontmatter {
    pub data: HashMap<String, FrontmatterValue>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum FrontmatterValue {
    String(String),
    List(Vec<String>),
}

impl Frontmatter {
    pub fn parse(md: &str) -> Self {
        let mut data = HashMap::new();
        let lines: Vec<&str> = md.lines().collect();
        let mut i = 0;

        // Skip opening ------------------ delimiter if present
        if lines
            .first()
            .map(|l| l.trim() == "------------------")
            .unwrap_or(false)
        {
            i = 1;
        }

        while i < lines.len() {
            let line = lines[i];
            if line.trim() == "------------------" {
                break;
            }

            if let Some(caps) = RE_KEY.captures(line) {
                let key = caps.get(1).unwrap().as_str().trim().to_string();
                let val = caps.get(2).unwrap().as_str().trim().to_string();

                if val.is_empty() && i + 1 < lines.len() {
                    if RE_LIST_ITEM.is_match(lines[i + 1]) {
                        let mut items = Vec::new();
                        let mut j = i + 1;
                        while j < lines.len() {
                            if let Some(item_caps) = RE_LIST_ITEM.captures(lines[j]) {
                                items.push(item_caps.get(1).unwrap().as_str().trim().to_string());
                                j += 1;
                            } else {
                                break;
                            }
                        }
                        data.insert(key, FrontmatterValue::List(items));
                        i = j - 1;
                    } else {
                        data.insert(key, FrontmatterValue::String(val));
                    }
                } else {
                    data.insert(key, FrontmatterValue::String(val));
                }
            }
            i += 1;
        }

        Frontmatter { data }
    }

    pub fn get(&self, key: &str) -> Option<&FrontmatterValue> {
        self.data.get(key)
    }

    pub fn get_str(&self, key: &str) -> Option<String> {
        match self.data.get(key)? {
            FrontmatterValue::String(s) => Some(s.clone()),
            FrontmatterValue::List(_) => None,
        }
    }

    pub fn get_list(&self, key: &str) -> Option<Vec<String>> {
        match self.data.get(key)? {
            FrontmatterValue::List(items) => Some(items.clone()),
            FrontmatterValue::String(s) => Some(vec![s.clone()]),
        }
    }
}

pub fn parse_tags_from_frontmatter(fm: &Frontmatter) -> Vec<String> {
    match fm.get("tags") {
        Some(FrontmatterValue::List(items)) => items
            .iter()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect(),
        Some(FrontmatterValue::String(s)) => {
            let s = s.trim();
            if s.is_empty() {
                return Vec::new();
            }
            if s.contains(',') && !s.starts_with('[') {
                s.split(',')
                    .map(|t| t.trim().to_string())
                    .filter(|t| !t.is_empty())
                    .collect()
            } else if s.starts_with('[') && s.ends_with(']') {
                let inner = &s[1..s.len() - 1];
                inner
                    .split(',')
                    .map(|t| t.trim().to_string())
                    .filter(|t| !t.is_empty())
                    .collect()
            } else {
                Vec::new()
            }
        }
        None => Vec::new(),
    }
}

pub fn parse_date_from_frontmatter(fm: &Frontmatter) -> Option<String> {
    let d = fm.get_str("date")?.trim().to_string();
    if d.is_empty() {
        return None;
    }
    if RE_DATE.is_match(&d) {
        Some(d)
    } else {
        tracing::warn!("Unrecognized date format in frontmatter: {:?}", d);
        Some(d)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_frontmatter() {
        let md = r#"title: Test Paper
date: 2024-01-15
tags:
  - LLM
  - Agent
------------------
# Test
"#;
        let fm = Frontmatter::parse(md);
        assert_eq!(fm.get_str("title").expect("valid regex"), "Test Paper");
        assert_eq!(fm.get_str("date").expect("valid regex"), "2024-01-15");
        let tags = parse_tags_from_frontmatter(&fm);
        assert_eq!(tags, vec!["LLM", "Agent"]);
    }

    #[test]
    fn test_parse_tags_comma_separated() {
        let md = r#"title: Test
tags: LLM, Agent, RAG
------------------
"#;
        let fm = Frontmatter::parse(md);
        let tags = parse_tags_from_frontmatter(&fm);
        assert_eq!(tags, vec!["LLM", "Agent", "RAG"]);
    }

    #[test]
    fn test_parse_tags_brackets() {
        let md = r#"title: Test
tags: [LLM, Agent]
------------------
"#;
        let fm = Frontmatter::parse(md);
        let tags = parse_tags_from_frontmatter(&fm);
        assert_eq!(tags, vec!["LLM", "Agent"]);
    }

    #[test]
    fn test_parse_tags_empty() {
        let md = r#"title: Test
tags:
------------------
"#;
        let fm = Frontmatter::parse(md);
        let tags = parse_tags_from_frontmatter(&fm);
        assert!(tags.is_empty());
    }

    #[test]
    fn test_parse_date() {
        let md = r#"title: Test
date: 2024-01-15
------------------
"#;
        let fm = Frontmatter::parse(md);
        let date = parse_date_from_frontmatter(&fm);
        assert_eq!(date, Some("2024-01-15".to_string()));
    }

    #[test]
    fn test_parse_date_missing() {
        let md = r#"title: Test
------------------
"#;
        let fm = Frontmatter::parse(md);
        let date = parse_date_from_frontmatter(&fm);
        assert_eq!(date, None);
    }

    #[test]
    fn test_frontmatter_get_list() {
        let md = r#"title: Test
authors:
  - Alice
  - Bob
------------------
"#;
        let fm = Frontmatter::parse(md);
        let authors = fm.get_list("authors");
        assert_eq!(authors, Some(vec!["Alice".to_string(), "Bob".to_string()]));
    }

    #[test]
    fn test_frontmatter_get_string_as_list() {
        let md = r#"title: Test
author: Alice
------------------
"#;
        let fm = Frontmatter::parse(md);
        let authors = fm.get_list("author");
        assert_eq!(authors, Some(vec!["Alice".to_string()]));
    }
}
