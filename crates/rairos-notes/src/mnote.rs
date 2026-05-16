//! M-Note (comparison note) management.

use crate::render::render_mnote;
use glob::glob;
use regex::Regex;
use std::path::{Path, PathBuf};

fn short(stem: &str, n: usize) -> String {
    let re = Regex::new(r"^P\s*-\s*\d{4}\s*-\s*").unwrap();
    let s = re.replace(stem, "").trim().to_string();
    if s.len() <= n {
        return s;
    }
    let truncated = s[..n - 5]
        .trim_end_matches('-')
        .trim_end_matches('_')
        .trim_end_matches(' ')
        .to_string();
    let suffix = format!("{:05}", truncated.hash64() % 100000);
    format!("{}~{}", truncated, suffix)
}

trait Hash64 {
    fn hash64(&self) -> u64;
}

impl Hash64 for str {
    fn hash64(&self) -> u64 {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        let mut hasher = DefaultHasher::new();
        self.hash(&mut hasher);
        hasher.finish()
    }
}

pub fn mnote_filename(tag: &str, a: &Path, b: &Path, c: &Path) -> String {
    let a_short = short(a.file_stem().unwrap().to_string_lossy().as_ref(), 19);
    let b_short = short(b.file_stem().unwrap().to_string_lossy().as_ref(), 19);
    let c_short = short(c.file_stem().unwrap().to_string_lossy().as_ref(), 19);
    format!("M - {tag} - {a_short} vs {b_short} vs {c_short}.md")
}

fn parse_current_abc(md: &str) -> (Option<String>, Option<String>, Option<String>) {
    fn find(label: &str, md: &str) -> Option<String> {
        let pattern = format!(r"(?m)^\-\s*{}:\s*(.+)\s*$", regex::escape(label));
        let re = Regex::new(&pattern).unwrap();
        re.captures(md)
            .map(|c| c.get(1).unwrap().as_str().trim().to_string())
    }
    (find("A", md), find("B", md), find("C", md))
}

fn today_iso() -> String {
    chrono::Utc::now().format("%Y-%m-%d").to_string()
}

fn append_view_evolution_log(
    md: &str,
    old_abc: (Option<String>, Option<String>, Option<String>),
    new_abc: (Option<String>, Option<String>, Option<String>),
) -> String {
    let today = today_iso();
    let block = format!(
        r#"

* {today}

  * 旧观点：A/B/C = {}/{}/{}
  * 新证据：新增/更新同主题论文，A/B/C 刷新为 {}/{}/{}
  * 更新结论：

"#,
        old_abc.0.as_deref().unwrap_or("?"),
        old_abc.1.as_deref().unwrap_or("?"),
        old_abc.2.as_deref().unwrap_or("?"),
        new_abc.0.as_deref().unwrap_or("?"),
        new_abc.1.as_deref().unwrap_or("?"),
        new_abc.2.as_deref().unwrap_or("?"),
    );

    let re = Regex::new(r"^##\s+View Evolution Log\s*$").unwrap();
    if let Some(m) = re.find(md) {
        let insert_pos = m.end();
        format!(
            "{}{}{}{}",
            &md[..insert_pos],
            "\n",
            block,
            &md[insert_pos..]
        )
    } else {
        format!("{}\n\n## View Evolution Log\n{}", md.trim_end(), block)
    }
}

pub fn ensure_or_update_mnote(mnote_dir: &Path, tag: &str, top3: &[PathBuf]) -> Option<PathBuf> {
    std::fs::create_dir_all(mnote_dir).ok()?;
    if top3.len() < 3 {
        return None;
    }

    let pattern = mnote_dir.join(format!("M - {tag} - *.md"));
    let existing: Vec<_> = glob(&pattern.to_string_lossy())
        .ok()?
        .filter_map(|p| p.ok())
        .filter(|p| p.is_file())
        .collect();
    let mut existing = existing;
    existing.sort();

    let a = &top3[0];
    let b = &top3[1];
    let c = &top3[2];
    let new_a = a.file_stem().unwrap().to_string_lossy();
    let new_b = b.file_stem().unwrap().to_string_lossy();
    let new_c = c.file_stem().unwrap().to_string_lossy();

    if existing.is_empty() {
        let fname = mnote_filename(tag, a, b, c);
        let path = mnote_dir.join(&fname);
        let title = format!("{tag}: {new_a} vs {new_b} vs {new_c}");
        let _ = std::fs::write(&path, render_mnote(&title, &new_a, &new_b, &new_c));
        return Some(path);
    }

    let path = &existing[0];
    let md = std::fs::read_to_string(path).unwrap_or_default();
    let (cur_a, cur_b, cur_c) = parse_current_abc(&md);

    if cur_a.is_none() || cur_b.is_none() || cur_c.is_none() {
        let md2 = format!(
            "{}\n\n---\n\n## 当前 A/B/C（自动补齐）\n\n- A: {}\n- B: {}\n- C: {}\n",
            md.trim_end(),
            new_a,
            new_b,
            new_c
        );
        let _ = std::fs::write(path, md2);
        return Some(path.clone());
    }

    if (cur_a.as_deref(), cur_b.as_deref(), cur_c.as_deref())
        != (
            Some(new_a.as_ref()),
            Some(new_b.as_ref()),
            Some(new_c.as_ref()),
        )
    {
        let re_a = Regex::new(r"^\-\s*A:\s*.*$").unwrap();
        let re_b = Regex::new(r"^\-\s*B:\s*.*$").unwrap();
        let re_c = Regex::new(r"^\-\s*C:\s*.*$").unwrap();
        let mut md2 = re_a.replace(&md, format!("- A: {new_a}")).to_string();
        md2 = re_b.replace(&md2, format!("- B: {new_b}")).to_string();
        md2 = re_c.replace(&md2, format!("- C: {new_c}")).to_string();
        md2 = append_view_evolution_log(
            &md2,
            (cur_a, cur_b, cur_c),
            (
                Some(new_a.to_string()),
                Some(new_b.to_string()),
                Some(new_c.to_string()),
            ),
        );
        let _ = std::fs::write(path, md2);
    }

    Some(path.clone())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_short_removes_p_prefix() {
        let result = short("P - 2017 - Attention Is All You Need", 19);
        assert!(!result.contains("P"));
        assert!(!result.contains("2017"));
    }

    #[test]
    fn test_short_truncates_long_names() {
        let result = short(
            "P - 2017 - Very Long Paper Title That Should Be Truncated",
            19,
        );
        // Should be <= 20 chars with ~XXXXX suffix (n-5 truncated + 5-char hash)
        assert!(result.len() <= 20);
        assert!(result.contains('~'));
    }

    #[test]
    fn test_short_preserves_short_names() {
        let result = short("P - 2020 - GPT-3", 19);
        assert_eq!(result, "GPT-3");
    }

    #[test]
    fn test_short_handles_no_p_prefix() {
        let result = short("Some Paper Title", 19);
        assert_eq!(result, "Some Paper Title");
    }

    #[test]
    fn test_mnote_filename_format() {
        let a = Path::new("P - 2017 - Attention Is All You Need.md");
        let b = Path::new("P - 2020 - GPT-3.md");
        let c = Path::new("P - 2023 - Claude.md");
        let result = mnote_filename("LLM", a, b, c);
        assert!(result.starts_with("M - LLM - "));
        assert!(result.ends_with(".md"));
        assert!(result.contains(" vs "));
    }

    #[test]
    fn test_parse_current_abc() {
        let md = r#"
## 当前 A/B/C（自动补齐）

- A: P - 2017 - Attention
- B: P - 2020 - GPT-3
- C: P - 2023 - Claude
"#;
        let (a, b, c) = parse_current_abc(md);
        assert!(a.unwrap().contains("Attention"));
        assert!(b.unwrap().contains("GPT-3"));
        assert!(c.unwrap().contains("Claude"));
    }

    #[test]
    fn test_parse_current_abc_missing_field() {
        let md = r#"
## 当前 A/B/C（自动补齐）

- A: Paper A
- C: Paper C
"#;
        let (a, b, c) = parse_current_abc(md);
        assert!(a.is_some());
        assert!(b.is_none());
        assert!(c.is_some());
    }

    #[test]
    fn test_append_view_evolution_log_creates_section() {
        let md = "# M - Test\n\nSome content";
        let result = append_view_evolution_log(
            md,
            (None, None, None),
            (
                Some("A1".to_string()),
                Some("B1".to_string()),
                Some("C1".to_string()),
            ),
        );
        assert!(result.contains("## View Evolution Log"));
        assert!(result.contains("A1"));
        assert!(result.contains("B1"));
        assert!(result.contains("C1"));
    }

    #[test]
    fn test_append_view_evolution_log_appends_to_existing() {
        let md = "# M - Test\n\n## View Evolution Log\n\n* 2024-01-01\n\n  * Old entry";
        let result = append_view_evolution_log(
            md,
            (
                Some("OldA".to_string()),
                Some("OldB".to_string()),
                Some("OldC".to_string()),
            ),
            (
                Some("NewA".to_string()),
                Some("NewB".to_string()),
                Some("NewC".to_string()),
            ),
        );
        assert!(result.contains("OldA"));
        assert!(result.contains("NewA"));
        // Should have two dated entries
        let count = result.matches("* 20").count();
        assert!(count >= 2);
    }

    #[test]
    fn test_ensure_or_update_mnote_creates_new() {
        let tmp_dir = std::env::temp_dir().join("mnote_test_create");
        std::fs::create_dir_all(&tmp_dir).ok();

        let a = tmp_dir.join("P - 2017 - Attention.md");
        let b = tmp_dir.join("P - 2020 - GPT-3.md");
        let c = tmp_dir.join("P - 2023 - Claude.md");
        std::fs::write(&a, "").ok();
        std::fs::write(&b, "").ok();
        std::fs::write(&c, "").ok();

        let result = ensure_or_update_mnote(&tmp_dir, "LLM", &[a.clone(), b.clone(), c.clone()]);
        assert!(result.is_some());
        let path = result.unwrap();
        assert!(path.exists());
        let content = std::fs::read_to_string(&path).unwrap();
        assert!(content.contains("# M - LLM:"));
        assert!(content.contains("## 比较维度"));

        std::fs::remove_dir_all(&tmp_dir).ok();
    }

    #[test]
    fn test_ensure_or_update_mnote_less_than_3_returns_none() {
        let tmp_dir = std::env::temp_dir().join("mnote_test_less");
        std::fs::create_dir_all(&tmp_dir).ok();
        let a = tmp_dir.join("A.md");
        std::fs::write(&a, "").ok();
        let result = ensure_or_update_mnote(&tmp_dir, "LLM", &[a.clone()]);
        assert!(result.is_none());
        std::fs::remove_dir_all(&tmp_dir).ok();
    }

    #[test]
    fn test_ensure_or_update_mnote_updates_existing_abc() {
        let tmp_dir = std::env::temp_dir().join("mnote_test_update");
        std::fs::create_dir_all(&tmp_dir).ok();

        let a = tmp_dir.join("P - 2017 - OldPaper.md");
        let b = tmp_dir.join("P - 2020 - GPT3.md");
        let c = tmp_dir.join("P - 2023 - Claude.md");
        let d = tmp_dir.join("P - 2024 - NewPaper.md");
        std::fs::write(&a, "").ok();
        std::fs::write(&b, "").ok();
        std::fs::write(&c, "").ok();
        std::fs::write(&d, "").ok();

        // Create initial mnote
        let initial =
            ensure_or_update_mnote(&tmp_dir, "LLM", &[a.clone(), b.clone(), c.clone()]).unwrap();

        // Update with new papers
        let result = ensure_or_update_mnote(&tmp_dir, "LLM", &[d.clone(), b.clone(), c.clone()]);
        assert!(result.is_some());
        let content = std::fs::read_to_string(&initial).unwrap();
        assert!(content.contains("P - 2024 - NewPaper") || content.contains("View Evolution Log"));

        std::fs::remove_dir_all(&tmp_dir).ok();
    }
}
