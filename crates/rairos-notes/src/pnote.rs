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
                    if name_str.chars().next().map(|c| c.is_ascii_digit()).unwrap_or(false) {
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
pub fn pnotes_by_tag(
    root: &Path,
) -> HashMap<String, Vec<(String, std::path::PathBuf)>> {
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
                mapping.entry(t).or_default().push((date.clone(), p.clone()));
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
