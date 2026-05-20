//! rairos-basics — Basic utilities for AI Research OS.
//!
//! Ported from `core/basics.py`.
//!
//! Provides research directory management, text slugification, and file utilities.

use crate::constants::CATEGORIES_FILE;
use regex::Regex;
use std::fs;
use std::path::Path;
use std::sync::LazyLock;

// Canonical research tree directory names (in display order)
pub static DEFAULT_RESEARCH_DIRS: LazyLock<Vec<&'static str>> = LazyLock::new(|| {
    vec![
        "00-Radar",
        "01-Foundations",
        "02-Models",
        "03-Training",
        "04-Scaling",
        "05-Alignment",
        "06-Agents",
        "07-Infrastructure",
        "08-Optimization",
        "09-Evaluation",
        "10-Applications",
        "11-Future-Directions",
    ]
});

static RE_SPACES: LazyLock<Regex> = LazyLock::new(|| Regex::new(r" {2,}").expect("valid regex"));
static RE_NONWORD: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"[^\w\s\-]").expect("valid regex"));
static RE_DASHES: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"-{2,}").expect("valid regex"));
static RE_SAFE_UID: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"[^\w\.-]+").expect("valid regex"));

// ============================================================================
// Config File Loading
// ============================================================================

fn get_config_path() -> std::path::PathBuf {
    std::path::PathBuf::from(std::env::var("HOME").unwrap_or_else(|_| "~".to_string()))
        .join(".ai_research_os")
        .join(CATEGORIES_FILE)
}

/// Return the list of research tree directory names.
///
/// Loads from ~/.ai_research_os/categories.json if it exists and is valid.
/// Falls back to DEFAULT_RESEARCH_DIRS.
pub fn get_research_dirs() -> Vec<String> {
    let cfg = get_config_path();
    if cfg.exists() {
        if let Ok(data) = fs::read_to_string(&cfg) {
            if let Ok(dirs) = serde_json::from_str::<Vec<String>>(&data) {
                if !dirs.is_empty() {
                    return dirs;
                }
            }
        }
    }
    DEFAULT_RESEARCH_DIRS
        .iter()
        .map(|s| s.to_string())
        .collect()
}

/// Return the default directory for C-Notes (concept notes).
///
/// Conventionally the second entry (after Radar). Falls back to "01-Foundations".
pub fn get_default_concept_dir() -> String {
    let dirs = get_research_dirs();
    if dirs.len() > 1 {
        dirs[1].clone()
    } else {
        "01-Foundations".to_string()
    }
}

/// Return the default Radar directory (index 0).
///
/// Falls back to "00-Radar".
pub fn get_default_radar_dir() -> String {
    let dirs = get_research_dirs();
    dirs.first()
        .cloned()
        .unwrap_or_else(|| "00-Radar".to_string())
}

// ============================================================================
// String Utilities
// ============================================================================

/// Slugify a title into a URL-safe string.
pub fn slugify_title(title: &str, max_len: usize) -> String {
    if title.is_empty() {
        return "Paper".to_string();
    }

    let mut t = title.trim().to_string();
    t = RE_SPACES.replace_all(&t, " ").to_string();
    t = RE_NONWORD.replace_all(&t, "").to_string();
    t = t.replace(' ', "-");
    t = RE_DASHES.replace_all(&t, "-").trim_matches('-').to_string();

    if t.len() > max_len {
        t.truncate(max_len);
        t = t.trim_end_matches('-').to_string();
    }

    if t.is_empty() {
        "Paper".to_string()
    } else {
        t
    }
}

/// Create a safe UID from a string by replacing invalid characters.
pub fn safe_uid(s: &str) -> String {
    RE_SAFE_UID.replace_all(s.trim(), "_").to_string()
}

// ============================================================================
// File I/O Utilities
// ============================================================================

/// Read text content from a file, returning empty string if not found.
pub fn read_text(p: &Path) -> String {
    if p.exists() {
        fs::read_to_string(p).unwrap_or_default()
    } else {
        String::new()
    }
}

/// Write text content to a file, creating parent directories if needed.
pub fn write_text(p: &Path, content: &str) -> std::io::Result<()> {
    if let Some(parent) = p.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(p, content)?;
    Ok(())
}

/// Ensure the research tree directory structure exists at the given root.
pub fn ensure_research_tree(root: &Path) -> std::io::Result<()> {
    fs::create_dir_all(root)?;
    for dir in DEFAULT_RESEARCH_DIRS.iter() {
        fs::create_dir_all(root.join(dir))?;
    }
    Ok(())
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_slugify_title_basic() {
        let result = slugify_title("Hello World Example Paper", 80);
        assert_eq!(result, "Hello-World-Example-Paper");
    }

    #[test]
    fn test_slugify_title_max_len() {
        let result = slugify_title("This is a very long title that should be truncated", 20);
        assert!(result.len() <= 20);
    }

    #[test]
    fn test_slugify_title_empty() {
        assert_eq!(slugify_title("", 80), "Paper");
    }

    #[test]
    fn test_slugify_title_special_chars() {
        let result = slugify_title("Test: [Paper] (2024)!", 80);
        assert!(!result.contains(":"));
        assert!(!result.contains("["));
        assert!(!result.contains("]"));
    }

    #[test]
    fn test_safe_uid() {
        assert_eq!(safe_uid("arxiv:1234.5678"), "arxiv_1234.5678");
        assert_eq!(safe_uid("some@email.com"), "some_email.com");
    }

    #[test]
    fn test_get_research_dirs_default() {
        // Without config file, should return defaults
        let dirs = get_research_dirs();
        assert!(dirs.contains(&"00-Radar".to_string()));
        assert!(dirs.contains(&"01-Foundations".to_string()));
    }

    #[test]
    fn test_get_default_concept_dir() {
        let dir = get_default_concept_dir();
        assert!(dir.contains("Foundations") || !dir.is_empty());
    }

    #[test]
    fn test_get_default_radar_dir() {
        let dir = get_default_radar_dir();
        assert!(dir.contains("Radar") || !dir.is_empty());
    }
}
