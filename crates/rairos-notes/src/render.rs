//! Renderers for C/M/P notes.

use crate::frontmatter::Frontmatter;
use regex::Regex;
use std::sync::LazyLock;

static RE_TITLE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?m)^#\s+(.+)$").expect("valid regex")
});
static RE_SOURCE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\*\*Source:\*\*\s+(\w+):\s+(\S+)").expect("valid regex")
});

pub struct PnoteMetadata {
    pub title: String,
    pub authors: Vec<String>,
    pub year: String,
    pub date: String,
    pub source: String,
    pub uid: String,
    pub r#abstract: String,
    pub tags: Vec<String>,
    pub path: String,
}

impl PnoteMetadata {
    pub fn from_markdown(path: &std::path::Path, md: &str) -> Self {
        let fm = Frontmatter::parse(md);
        let tags = crate::frontmatter::parse_tags_from_frontmatter(&fm);
        let date = crate::frontmatter::parse_date_from_frontmatter(&fm).unwrap_or_default();
        let year = if date.len() >= 4 {
            date[..4].to_string()
        } else {
            String::new()
        };

        let title = RE_TITLE
            .captures(md)
            .and_then(|c| c.get(1))
            .map(|m| m.as_str().trim().to_string())
            .unwrap_or_else(|| path.file_stem().unwrap().to_string_lossy().to_string());

        let (source, uid) = RE_SOURCE
            .captures(md)
            .map(|c| {
                (
                    c.get(1).unwrap().as_str().to_lowercase(),
                    c.get(2).unwrap().as_str().to_string(),
                )
            })
            .unwrap_or_else(|| ("arxiv".to_string(), String::new()));

        PnoteMetadata {
            title,
            authors: Vec::new(),
            year,
            date,
            source,
            uid,
            r#abstract: String::new(),
            tags,
            path: path.to_string_lossy().to_string(),
        }
    }
}

pub fn render_cnote(concept: &str) -> String {
    format!(
        r#"# C - {concept}

## 核心定义

-

## 产生背景

-

## 技术本质

-

## 关联笔记

"#
    )
}

pub fn render_mnote(title: &str, a: &str, b: &str, c: &str) -> String {
    format!(
        r#"# M - {title}

## 比较维度

| 维度 | {a} | {b} | {c} |
|------|-----|-----|-----|
| 方法 | - | - | - |
| 效果 | - | - | - |
| 局限性 | - | - | - |

## 关键差异

-

## 共同结论

-

## View Evolution Log

"#
    )
}

pub fn render_pnote(title: &str, source: &str, uid: &str, date: &str, tags: &[String]) -> String {
    let tags_str = if tags.is_empty() {
        String::new()
    } else {
        format!("\ntags: [{}]", tags.join(", "))
    };

    format!(
        r#"------------------
title: {title}
source: {source}: {uid}
date: {date}{tags_str}
------------------

# {title}

## 摘要

-

## 关键发现

-

## 方法

-

## 局限性

-

## 相关工作

"#
    )
}
