//! P-Note collection and tag-based queries.

use crate::frontmatter::{parse_date_from_frontmatter, parse_tags_from_frontmatter, Frontmatter};
use crate::render::PnoteMetadata;
use std::collections::HashMap;
use std::path::Path;

/// Collect all P-notes from the research tree.
pub fn collect_pnotes(root: &Path) -> Vec<std::path::PathBuf> {
    let mut all_dirs = std::collections::HashSet::new();

    // Scan all research tree subdirectories (00-Radar through 11-Future-Directions)
    if let Ok(entries) = std::fs::read_dir(root) {
        for entry in entries.flatten() {
            if entry.path().is_dir() {
                if let Some(name) = entry.path().file_name() {
                    let name_str = name.to_string_lossy();
                    if name_str
                        .chars()
                        .next()
                        .map(|c| c.is_ascii_digit())
                        .unwrap_or(false)
                    {
                        all_dirs.insert(name_str.to_string());
                    }
                }
            }
        }
    }

    // Legacy compat
    all_dirs.insert("02-Papers".to_string());
    all_dirs.insert("Papers".to_string());
    all_dirs.insert("papers".to_string());

    let mut pnotes = Vec::new();
    if let Ok(entries) = walkdir(root) {
        for path in entries {
            if path.is_file() && path.extension().map(|e| e == "md").unwrap_or(false) {
                if let Some(parent) = path.parent() {
                    if let Some(parent_name) = parent.file_name() {
                        if all_dirs.contains(parent_name.to_string_lossy().as_ref()) {
                            pnotes.push(path);
                        }
                    }
                }
            }
        }
    }
    pnotes.sort();
    pnotes
}

fn walkdir(root: &Path) -> std::io::Result<Vec<std::path::PathBuf>> {
    let mut results = Vec::new();
    walkdir_recursive(root, &mut results);
    Ok(results)
}

fn walkdir_recursive(dir: &Path, results: &mut Vec<std::path::PathBuf>) {
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                walkdir_recursive(&path, results);
            } else {
                results.push(path);
            }
        }
    }
}

/// Group P-notes by tag, sorted by date descending.
pub fn pnotes_by_tag(root: &Path) -> HashMap<String, Vec<(String, std::path::PathBuf)>> {
    let mut mapping: HashMap<String, Vec<(String, std::path::PathBuf)>> = HashMap::new();

    for p in collect_pnotes(root) {
        if let Ok(md) = std::fs::read_to_string(&p) {
            let fm = Frontmatter::parse(&md);
            let tags = parse_tags_from_frontmatter(&fm);
            if tags.is_empty() {
                continue;
            }

            let date = parse_date_from_frontmatter(&fm).unwrap_or_else(|| {
                p.metadata()
                    .ok()
                    .and_then(|m| {
                        m.modified().ok().and_then(|t| {
                            t.duration_since(std::time::UNIX_EPOCH)
                                .ok()
                                .map(|d| d.as_secs() as i64)
                        })
                    })
                    .and_then(|secs| chrono::DateTime::from_timestamp(secs, 0))
                    .map(|dt| dt.format("%Y-%m-%d").to_string())
                    .unwrap_or_default()
            });

            for t in tags {
                mapping
                    .entry(t)
                    .or_default()
                    .push((date.clone(), p.clone()));
            }
        }
    }

    for entries in mapping.values_mut() {
        entries.sort_by(|a, b| b.0.cmp(&a.0));
    }
    mapping
}

pub fn wikilink_for_pnote(pnote_path: &Path) -> String {
    format!("[[{}]]", pnote_path.file_stem().unwrap().to_string_lossy())
}

pub fn read_pnote_metadata(pnote_path: &Path) -> Option<PnoteMetadata> {
    let md = std::fs::read_to_string(pnote_path).ok()?;
    Some(PnoteMetadata::from_markdown(pnote_path, &md))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wikilink_for_pnote() {
        let path = Path::new("/research/01-Foundations/P - 2017 - Attention Is All You Need.md");
        let result = wikilink_for_pnote(path);
        assert_eq!(result, "[[P - 2017 - Attention Is All You Need]]");
    }

    #[test]
    fn test_wikilink_strips_extension() {
        let path = Path::new("/some/path/Note.md");
        let result = wikilink_for_pnote(path);
        assert_eq!(result, "[[Note]]");
    }

    #[test]
    fn test_collect_pnotes_finds_named_dirs() {
        let tmp_dir = std::env::temp_dir().join("pnotes_test_collect");
        std::fs::create_dir_all(tmp_dir.join("01-Foundations")).ok();
        std::fs::create_dir_all(tmp_dir.join("02-Papers")).ok();
        std::fs::create_dir_all(tmp_dir.join("legacy")).ok(); // should be ignored

        // Create some p-notes
        std::fs::write(
            tmp_dir.join("01-Foundations/P - 2017 - Attention.md"),
            "# Attention",
        )
        .ok();
        std::fs::write(tmp_dir.join("02-Papers/P - 2020 - GPT-3.md"), "# GPT-3").ok();
        std::fs::write(tmp_dir.join("legacy/Paper.md"), "# Legacy").ok(); // not in numbered dir

        let pnotes = collect_pnotes(&tmp_dir);
        assert_eq!(pnotes.len(), 2);

        std::fs::remove_dir_all(&tmp_dir).ok();
    }

    #[test]
    fn test_collect_pnotes_respects_parent_directory() {
        let tmp_dir = std::env::temp_dir().join("pnotes_test_parent");
        std::fs::create_dir_all(tmp_dir.join("01-Foundations")).ok();
        std::fs::create_dir_all(tmp_dir.join("02-Papers")).ok();

        // Create p-note in 01-Foundations
        std::fs::write(
            tmp_dir.join("01-Foundations/P - 2017 - Attention.md"),
            "# Attention",
        )
        .ok();
        // Create non-markdown file in 02-Papers (should be ignored)
        std::fs::write(tmp_dir.join("02-Papers/Paper.txt"), "not markdown").ok();

        let pnotes = collect_pnotes(&tmp_dir);
        assert_eq!(pnotes.len(), 1);

        std::fs::remove_dir_all(&tmp_dir).ok();
    }

    #[test]
    fn test_pnotes_by_tag_extracts_tags() {
        let tmp_dir = std::env::temp_dir().join("pnotes_test_tags");
        std::fs::create_dir_all(tmp_dir.join("01-Foundations")).ok();

        let md = r#"------------------
title: Attention Is All You Need
date: 2024-01-15
tags:
  - LLM
  - Transformer
------------------
# Attention
"#;
        std::fs::write(tmp_dir.join("01-Foundations/P - 2017 - Attention.md"), md).ok();

        let by_tag = pnotes_by_tag(&tmp_dir);
        assert!(by_tag.contains_key("LLM"));
        assert!(by_tag.contains_key("Transformer"));

        std::fs::remove_dir_all(&tmp_dir).ok();
    }

    #[test]
    fn test_pnotes_by_tag_sorts_by_date_descending() {
        let tmp_dir = std::env::temp_dir().join("pnotes_test_sort");
        std::fs::create_dir_all(tmp_dir.join("01-Foundations")).ok();

        let md_old = r#"------------------
title: Old Paper
date: 2023-01-01
tags:
  - LLM
------------------
# Old
"#;
        let md_new = r#"------------------
title: New Paper
date: 2024-01-01
tags:
  - LLM
------------------
# New
"#;
        std::fs::write(tmp_dir.join("01-Foundations/P - 2023 - Old.md"), md_old).ok();
        std::fs::write(tmp_dir.join("01-Foundations/P - 2024 - New.md"), md_new).ok();

        let by_tag = pnotes_by_tag(&tmp_dir);
        let entries = by_tag.get("LLM").unwrap();
        // Should be sorted newest first
        assert!(entries[0].0 >= entries[1].0);

        std::fs::remove_dir_all(&tmp_dir).ok();
    }

    #[test]
    fn test_pnotes_by_tag_uses_file_mtime_as_fallback() {
        let tmp_dir = std::env::temp_dir().join("pnotes_test_mtime");
        std::fs::create_dir_all(tmp_dir.join("01-Foundations")).ok();

        // Note without date in frontmatter
        let md = r#"------------------
title: No Date Paper
tags:
  - LLM
------------------
# No Date
"#;
        let path = tmp_dir.join("01-Foundations/P - 2020 - NoDate.md");
        std::fs::write(&path, md).ok();

        let by_tag = pnotes_by_tag(&tmp_dir);
        // Should still appear, using file mtime
        assert!(by_tag.contains_key("LLM"));

        std::fs::remove_dir_all(&tmp_dir).ok();
    }

    #[test]
    fn test_read_pnote_metadata_parses_title() {
        let tmp_dir = std::env::temp_dir().join("pnotes_test_meta");
        std::fs::create_dir_all(&tmp_dir).ok();

        let md = r#"------------------
title: Test Paper
date: 2024-01-15
tags:
  - LLM
------------------
# Custom Heading

Some content
"#;
        let path = tmp_dir.join("P - 2024 - Test.md");
        std::fs::write(&path, md).ok();

        let meta = read_pnote_metadata(&path);
        assert!(meta.is_some());
        let meta = meta.unwrap();
        assert_eq!(meta.title, "Custom Heading"); // from # heading, not filename
        assert_eq!(meta.date, "2024-01-15");
        assert_eq!(meta.year, "2024");
        assert!(meta.tags.contains(&"LLM".to_string()));

        std::fs::remove_dir_all(&tmp_dir).ok();
    }

    #[test]
    fn test_read_pnote_metadata_uses_filename_when_no_heading() {
        let tmp_dir = std::env::temp_dir().join("pnotes_test_filename");
        std::fs::create_dir_all(&tmp_dir).ok();

        let md = r#"------------------
title: Test
------------------
"#;
        let path = tmp_dir.join("P - 2024 - Test Paper.md");
        std::fs::write(&path, md).ok();

        let meta = read_pnote_metadata(&path);
        assert!(meta.is_some());
        // Should fall back to filename stem
        assert!(meta.unwrap().title.contains("Test Paper"));

        std::fs::remove_dir_all(&tmp_dir).ok();
    }

    #[test]
    fn test_read_pnote_metadata_parses_source() {
        let tmp_dir = std::env::temp_dir().join("pnotes_test_source");
        std::fs::create_dir_all(&tmp_dir).ok();

        let md = r#"------------------
title: Test
------------------
**Source:** ARXIV: 1706.03762
"#;
        let path = tmp_dir.join("P - 2017 - Test.md");
        std::fs::write(&path, md).ok();

        let meta = read_pnote_metadata(&path).unwrap();
        assert_eq!(meta.source, "arxiv");
        assert_eq!(meta.uid, "1706.03762");

        std::fs::remove_dir_all(&tmp_dir).ok();
    }

    #[test]
    fn test_read_pnote_metadata_handles_missing_frontmatter() {
        let tmp_dir = std::env::temp_dir().join("pnotes_test_no_fm");
        std::fs::create_dir_all(&tmp_dir).ok();

        let md = "# Just a heading\n\nSome content";
        let path = tmp_dir.join("P - 2024 - NoFM.md");
        std::fs::write(&path, md).ok();

        let meta = read_pnote_metadata(&path);
        assert!(meta.is_some()); // Should not panic, returns defaults

        std::fs::remove_dir_all(&tmp_dir).ok();
    }
}
