//! Skill Discovery — scan .claude/skills/ for SKILL.md files.
//!
//! Mirrors research_loop/skill_discovery.py

use std::collections::HashMap;
use std::path::{Path, PathBuf};

// ─── Skill struct ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct Skill {
    pub name: String,
    pub description: String,
    pub path: PathBuf,
    pub dir: PathBuf,
}

// ─── Frontmatter parser (no YAML dependency) ───────────────────────────────────

fn parse_frontmatter(text: &str) -> HashMap<String, String> {
    let mut result = HashMap::new();
    let lines: Vec<&str> = text.lines().collect();

    // Must start with ---
    if lines.is_empty() || lines[0].trim() != "---" {
        return result;
    }

    // Find closing ---
    let mut end = 1;
    while end < lines.len() && lines[end].trim() != "---" {
        end += 1;
    }
    if end >= lines.len() {
        return result;
    }

    // Parse key: value lines between the --- markers
    let interesting_keys = ["name", "description"];
    for line in &lines[1..end] {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        if let Some(pos) = trimmed.find(':') {
            let key = trimmed[..pos].trim().to_lowercase();
            if interesting_keys.contains(&key.as_str()) {
                let value = trimmed[pos + 1..].trim();
                // Strip surrounding quotes
                let value = value
                    .strip_prefix('"')
                    .and_then(|v| v.strip_suffix('"'))
                    .or_else(|| value.strip_prefix('\'').and_then(|v| v.strip_suffix('\'')))
                    .unwrap_or(value)
                    .to_string();
                result.insert(key, value);
            }
        }
    }

    result
}

// ─── Discover skills from a directory ──────────────────────────────────────────

fn discover_in_dir(base: &Path, skills: &mut HashMap<String, Skill>) {
    if !base.is_dir() {
        return;
    }

    let Ok(entries) = std::fs::read_dir(base) else {
        return;
    };

    for entry in entries.flatten() {
        if !entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
            continue;
        }
        let skill_dir = entry.path();
        let skill_md = skill_dir.join("SKILL.md");
        if !skill_md.exists() {
            continue;
        }

        let text = match std::fs::read_to_string(&skill_md) {
            Ok(t) => t,
            Err(_) => continue,
        };

        let fm = parse_frontmatter(&text);
        let dir_name = skill_dir
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();
        let name = fm.get("name").cloned().unwrap_or(dir_name);
        let description = fm.get("description").cloned().unwrap_or_default();

        // Only insert if not already present (project skills shadow user skills)
        skills.entry(name.clone()).or_insert(Skill {
            name,
            description,
            path: skill_md,
            dir: skill_dir,
        });
    }
}

// ─── Find project root (walk up from CWD or use env) ───────────────────────────

fn find_project_root() -> Option<PathBuf> {
    if let Ok(cwd) = std::env::current_dir() {
        let mut dir = Some(cwd.as_path());
        while let Some(d) = dir {
            if d.join(".claude").join("skills").is_dir() {
                return Some(d.to_path_buf());
            }
            dir = d.parent();
        }
    }
    // Fallback: check CARGO_MANIFEST_DIR
    if let Ok(manifest) = std::env::var("CARGO_MANIFEST_DIR") {
        let p = PathBuf::from(manifest);
        if p.join(".claude").join("skills").is_dir() {
            return Some(p);
        }
    }
    None
}

// ─── Public API ────────────────────────────────────────────────────────────────

pub fn discover_skills() -> Vec<Skill> {
    let mut skills: HashMap<String, Skill> = HashMap::new();

    // Project skills first
    if let Some(project_root) = find_project_root() {
        let project_skills = project_root.join(".claude").join("skills");
        discover_in_dir(&project_skills, &mut skills);
    }

    // User skills (only fill slots not taken by project)
    if let Some(home) = dirs_next() {
        let user_skills = home.join(".claude").join("skills");
        discover_in_dir(&user_skills, &mut skills);
    }

    let mut result: Vec<Skill> = skills.into_values().collect();
    result.sort_by(|a, b| a.name.cmp(&b.name));
    result
}

pub fn match_skills(query: &str, skills: &[Skill]) -> Vec<Skill> {
    let q = query.to_lowercase();
    let mut scored: Vec<(i32, usize, &Skill)> = Vec::new();

    for skill in skills {
        let name_lower = skill.name.to_lowercase();
        let desc_lower = skill.description.to_lowercase();

        let q_in_name = name_lower.contains(&q);
        let q_in_desc = desc_lower.contains(&q);

        let score = if q_in_name && q_in_desc {
            3
        } else if q_in_name {
            2
        } else if q_in_desc {
            1
        } else {
            continue;
        };

        let position = name_lower.find(&q).unwrap_or(usize::MAX);
        scored.push((score, position, skill));
    }

    // Sort by score desc, then by position asc
    scored.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| a.1.cmp(&b.1)));

    scored.into_iter().map(|(_, _, s)| s.clone()).collect()
}

fn dirs_next() -> Option<PathBuf> {
    std::env::var("HOME")
        .ok()
        .map(PathBuf::from)
        .or_else(|| std::env::var("USERPROFILE").ok().map(PathBuf::from))
}

// ─── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_frontmatter() {
        let text = "---\nname: test-skill\ndescription: A test\n---\nContent here";
        let fm = parse_frontmatter(text);
        assert_eq!(fm.get("name").map(|s| s.as_str()), Some("test-skill"));
        assert_eq!(fm.get("description").map(|s| s.as_str()), Some("A test"));
    }

    #[test]
    fn test_parse_frontmatter_no_closing() {
        let text = "---\nname: test";
        let fm = parse_frontmatter(text);
        assert!(fm.is_empty());
    }

    #[test]
    fn test_parse_frontmatter_quoted_value() {
        let text = "---\nname: \"my skill\"\ndescription: 'another'\n---";
        let fm = parse_frontmatter(text);
        assert_eq!(fm.get("name").map(|s| s.as_str()), Some("my skill"));
        assert_eq!(fm.get("description").map(|s| s.as_str()), Some("another"));
    }

    #[test]
    fn test_parse_frontmatter_ignores_unknown_keys() {
        let text = "---\nname: test\nunknown: will be ignored\n---";
        let fm = parse_frontmatter(text);
        assert_eq!(fm.get("name").map(|s| s.as_str()), Some("test"));
        assert_eq!(fm.len(), 1);
    }

    #[test]
    fn test_discover_empty_dir() {
        let tmp = std::env::temp_dir().join("rairos_skills_test");
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(&tmp).unwrap();
        let mut skills = HashMap::new();
        discover_in_dir(&tmp, &mut skills);
        assert!(skills.is_empty());
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn test_match_skills() {
        let skills = vec![
            Skill {
                name: "rust-coding".to_string(),
                description: "Rust programming".to_string(),
                path: PathBuf::from("/x/SKILL.md"),
                dir: PathBuf::from("/x"),
            },
            Skill {
                name: "python-script".to_string(),
                description: "Python automation".to_string(),
                path: PathBuf::from("/y/SKILL.md"),
                dir: PathBuf::from("/y"),
            },
        ];
        let matched = match_skills("rust", &skills);
        assert_eq!(matched.len(), 1);
        assert_eq!(matched[0].name, "rust-coding");
    }

    #[test]
    fn test_match_skills_no_match() {
        let skills = vec![
            Skill {
                name: "rust-coding".to_string(),
                description: "Rust programming".to_string(),
                path: PathBuf::from("/x/SKILL.md"),
                dir: PathBuf::from("/x"),
            },
        ];
        let matched = match_skills("java", &skills);
        assert!(matched.is_empty());
    }

    #[test]
    fn test_match_skills_scoring() {
        let skills = vec![
            Skill {
                name: "python".to_string(),
                description: "some unrelated".to_string(),
                path: PathBuf::from("/a"),
                dir: PathBuf::from("/a"),
            },
            Skill {
                name: "other".to_string(),
                description: "python related".to_string(),
                path: PathBuf::from("/b"),
                dir: PathBuf::from("/b"),
            },
        ];
        let matched = match_skills("python", &skills);
        // name match (score=2) should come before desc-only (score=1)
        assert_eq!(matched.len(), 2);
        assert_eq!(matched[0].name, "python");
        assert_eq!(matched[1].name, "other");
    }
}
