//! rairos-skill-discovery — Scan directories for SKILL.md files and parse frontmatter.
//!
//! Ported from `research_loop/skill_discovery.py`.

use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

/// Cached mtime of skill dirs for hot-reload detection.
fn get_mtime_cache() -> &'static std::sync::Mutex<HashMap<PathBuf, f64>> {
    static CACHE: std::sync::LazyLock<std::sync::Mutex<HashMap<PathBuf, f64>>> =
        std::sync::LazyLock::new(|| std::sync::Mutex::new(HashMap::new()));
    &CACHE
}

const SKILL_FILENAME: &str = "SKILL.md";
const SKILL_MARKER: &str = "---";

/// A discovered skill with metadata.
#[derive(Debug, Clone)]
pub struct Skill {
    pub name: String,
    pub description: String,
    pub path: PathBuf,
    pub dir: PathBuf,
}

impl Skill {
    /// Convert to a dictionary representation.
    pub fn to_dict(&self) -> HashMap<String, String> {
        let mut m = HashMap::new();
        m.insert("name".to_string(), self.name.clone());
        m.insert("description".to_string(), self.description.clone());
        m.insert("path".to_string(), self.path.to_string_lossy().to_string());
        m.insert("dir".to_string(), self.dir.to_string_lossy().to_string());
        m
    }
}

/// Parse YAML frontmatter from SKILL.md content.
fn parse_frontmatter(content: &str) -> HashMap<String, String> {
    if !content.starts_with(SKILL_MARKER) {
        return HashMap::new();
    }
    let after_first = match content.get(3..) {
        Some(s) => s,
        None => return HashMap::new(),
    };
    let end_pos = match after_first.find("---") {
        Some(pos) => pos,
        None => return HashMap::new(),
    };
    let yaml_text = &after_first[..end_pos];
    let mut result = HashMap::new();
    for line in yaml_text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some((key, value)) = line.split_once(':') {
            let key = key.trim();
            let value = value.trim().trim_matches('"').trim_matches('\'');
            if key == "name" || key == "description" {
                result.insert(key.to_string(), value.to_string());
            }
        }
    }
    result
}

/// Scan base directory for skill directories and parse their SKILL.md.
fn discover_in_dir(base: &std::path::Path) -> Vec<Skill> {
    if !base.exists() {
        return Vec::new();
    }
    let mut skills = Vec::new();
    if let Ok(entries) = fs::read_dir(base) {
        for item in entries.filter_map(Result::ok) {
            let item_path = item.path();
            if !item_path.is_dir() {
                continue;
            }
            let skill_md = item_path.join(SKILL_FILENAME);
            if !skill_md.exists() {
                continue;
            }
            if let Ok(text) = fs::read_to_string(&skill_md) {
                let fm = parse_frontmatter(&text);
                let name = fm.get("name").map(String::as_str).unwrap_or_else(|| {
                    item_path.file_name().and_then(|n| n.to_str()).unwrap_or("")
                });
                let desc = fm.get("description").map(String::as_str).unwrap_or("");
                skills.push(Skill {
                    name: name.to_string(),
                    description: desc.to_string(),
                    path: skill_md,
                    dir: item_path,
                });
            }
        }
    }
    skills
}

fn default_home_dir() -> PathBuf {
    std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("~"))
}

/// Discover all skills from project and user skill directories.
pub fn discover_skills(
    project_skills_dir: Option<PathBuf>,
    user_skills_dir: Option<PathBuf>,
) -> Vec<Skill> {
    let mut discovered: HashMap<String, Skill> = HashMap::new();

    // Project skills
    let proj_dir = project_skills_dir.unwrap_or_else(|| {
        std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|p| p.to_path_buf()))
            .unwrap_or_default()
            .join(".claude")
            .join("skills")
    });
    for skill in discover_in_dir(&proj_dir) {
        discovered.insert(skill.name.clone(), skill);
    }

    // User skills
    let user_dir = user_skills_dir
        .map(|p| {
            let s = p.to_string_lossy();
            if s.starts_with('~') {
                if let Ok(home) = std::env::var("HOME") {
                    PathBuf::from(s.replace('~', &home))
                } else {
                    p
                }
            } else {
                p
            }
        })
        .unwrap_or_else(|| default_home_dir().join(".claude").join("skills"));

    for skill in discover_in_dir(&user_dir) {
        discovered.entry(skill.name.clone()).or_insert(skill);
    }

    let mut skills: Vec<_> = discovered.into_values().collect();
    skills.sort_by(|a, b| a.name.cmp(&b.name));
    skills
}

/// Re-scan skill directories, returning fresh list of skills if dirs changed.
pub fn reload_skills(
    project_skills_dir: Option<PathBuf>,
    user_skills_dir: Option<PathBuf>,
) -> Vec<Skill> {
    let proj_dir_raw = project_skills_dir.clone();
    let user_dir_raw = user_skills_dir.clone();
    let proj_dir = proj_dir_raw.unwrap_or_else(|| PathBuf::from(".claude/skills"));
    let user_dir = user_dir_raw
        .map(|p| {
            let s = p.to_string_lossy();
            if s.starts_with('~') {
                if let Ok(home) = std::env::var("HOME") {
                    PathBuf::from(s.replace('~', &home))
                } else {
                    p
                }
            } else {
                p
            }
        })
        .unwrap_or_else(|| default_home_dir().join(".claude").join("skills"));

    let mut dirs: Vec<PathBuf> = Vec::new();
    if proj_dir.exists() {
        dirs.push(proj_dir);
    }
    if user_dir.exists() {
        dirs.push(user_dir);
    }

    let mut changed = false;
    for d in &dirs {
        if let Ok(m) = d.metadata() {
            if let Ok(modified) = m.modified() {
                let mtime = modified
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_secs_f64())
                    .unwrap_or(0.0);
                let cache = get_mtime_cache().lock().unwrap();
                if cache.get(d).copied() != Some(mtime) {
                    changed = true;
                }
            }
        }
    }

    if changed {
        discover_skills(project_skills_dir.clone(), user_skills_dir.clone())
    } else {
        Vec::new()
    }
}

/// Find a skill by exact name.
pub fn get_skill_by_name(name: &str, skills: Option<&[Skill]>) -> Option<Skill> {
    match skills {
        Some(slice) => slice.iter().find(|s| s.name == name).cloned(),
        None => {
            let all = discover_skills(None, None);
            all.into_iter().find(|s| s.name == name)
        }
    }
}

/// Match skills by query string (name + description keyword match).
pub fn match_skills(query: &str, skills: Option<&[Skill]>) -> Vec<Skill> {
    let skill_list: Vec<Skill> = match skills {
        Some(slice) => slice.to_vec(),
        None => discover_skills(None, None),
    };
    let q = query.to_lowercase();
    let mut scored: Vec<(i32, usize, Skill)> = Vec::new();
    for s in &skill_list {
        let name_lower = s.name.to_lowercase();
        let desc_lower = s.description.to_lowercase();
        let score = if name_lower.contains(&q) {
            if desc_lower.contains(&q) {
                2
            } else {
                3
            }
        } else if desc_lower.contains(&q) {
            1
        } else {
            continue;
        };
        let name_pos = name_lower.find(&q).unwrap_or(usize::MAX);
        scored.push((score, name_pos, s.clone()));
    }
    scored.sort_by_key(|x| (-x.0, x.1));
    scored.into_iter().map(|(_, _, s)| s).collect()
}

/// Quick helper — return just skill names as strings.
pub fn list_skill_names() -> Vec<String> {
    discover_skills(None, None)
        .into_iter()
        .map(|s| s.name)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::TempDir;

    fn make_skill(parent: &std::path::Path, name: &str, description: &str) -> PathBuf {
        let skill_dir = parent.join(name);
        std::fs::create_dir(&skill_dir).unwrap();
        let skill_md = skill_dir.join("SKILL.md");
        let mut f = std::fs::File::create(&skill_md).unwrap();
        writeln!(f, "---").unwrap();
        writeln!(f, "name: {}", name).unwrap();
        writeln!(f, "description: {}", description).unwrap();
        writeln!(f, "---").unwrap();
        writeln!(f).unwrap();
        writeln!(f, "# Skill content").unwrap();
        skill_md
    }

    #[test]
    fn test_parse_frontmatter() {
        let content = "---\nname: test-skill\ndescription: A test skill\n---\n# Body";
        let fm = parse_frontmatter(content);
        assert_eq!(fm.get("name"), Some(&"test-skill".to_string()));
        assert_eq!(fm.get("description"), Some(&"A test skill".to_string()));
    }

    #[test]
    fn test_parse_frontmatter_missing() {
        assert!(parse_frontmatter("no frontmatter").is_empty());
        assert!(parse_frontmatter("---no closing").is_empty());
    }

    #[test]
    fn test_discover_in_dir() {
        let dir = TempDir::new().unwrap();
        make_skill(dir.path(), "my-skill", "Does things");
        let skills = discover_in_dir(dir.path());
        assert_eq!(skills.len(), 1);
        assert_eq!(skills[0].name, "my-skill");
        assert_eq!(skills[0].description, "Does things");
    }

    #[test]
    fn test_discover_in_dir_empty() {
        let dir = TempDir::new().unwrap();
        assert!(discover_in_dir(dir.path()).is_empty());
    }

    #[test]
    fn test_get_skill_by_name() {
        let dir = TempDir::new().unwrap();
        make_skill(dir.path(), "find-me", "Description");
        let skills = discover_in_dir(dir.path());
        assert!(get_skill_by_name("find-me", Some(&skills)).is_some());
        assert!(get_skill_by_name("not-found", Some(&skills)).is_none());
    }

    #[test]
    fn test_match_skills() {
        let dir = TempDir::new().unwrap();
        make_skill(dir.path(), "rust-coding", "Write Rust fast");
        make_skill(dir.path(), "python-coding", "Write Python code");
        let skills = discover_in_dir(dir.path());
        let matched = match_skills("rust", Some(&skills));
        assert_eq!(matched.len(), 1);
        assert_eq!(matched[0].name, "rust-coding");
    }

    #[test]
    fn test_list_skill_names() {
        let dir = TempDir::new().unwrap();
        make_skill(dir.path(), "alpha", "First");
        make_skill(dir.path(), "beta", "Second");
        let all = discover_in_dir(dir.path());
        let names: Vec<String> = all.iter().map(|s| s.name.clone()).collect();
        assert!(names.contains(&"alpha".to_string()));
        assert!(names.contains(&"beta".to_string()));
    }
}
