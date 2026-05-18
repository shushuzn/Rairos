//! C-Note creation and link management.

use crate::pnote::wikilink_for_pnote;
use crate::render::render_cnote;
use regex::Regex;
use std::path::Path;
use std::sync::LazyLock;

static RE_LEADING_HASHES: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^#+\s+").unwrap()
});
static RE_BLANK_LINES: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(\s*\n)*").unwrap()
});
static RE_SECTION_END: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\n##\s+").unwrap()
});
static RE_WIKILINK_LINE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^-\s*\[\[[^\]]+\]\](?:[^\n]*)?\n?").unwrap()
});

pub fn ensure_cnote(concept_dir: &Path, concept: &str) -> std::path::PathBuf {
    let path = concept_dir.join(format!("C - {concept}.md"));
    if !path.exists() {
        let content = render_cnote(concept);
        let _ = std::fs::write(&path, content);
    }
    path
}

fn read_text(path: &Path) -> String {
    std::fs::read_to_string(path).unwrap_or_default()
}

fn write_text(path: &Path, content: &str) -> std::io::Result<()> {
    std::fs::write(path, content)
}

pub fn upsert_link_under_heading(md: &str, heading: &str, link_line: &str) -> String {
    let clean_heading = RE_LEADING_HASHES.replace(heading, "").trim().to_string();

    let pattern = format!(r"(^##\s+{}(?:\s*|\s+.*)$)", regex::escape(&clean_heading));
    let re_pattern = Regex::new(&pattern).unwrap();

    let m = re_pattern.find(md);
    if m.is_none() {
        return format!(
            "{}\n\n## {}\n\n{}\n",
            md.trim_end(),
            clean_heading,
            link_line
        );
    }

    let m = m.unwrap();
    let match_line = m.as_str().split('\n').next().unwrap();
    let start = m.start() + match_line.len();
    let after = &md[start..];

    let m2 = RE_BLANK_LINES.find(after);
    let insert_pos = start + m2.map(|m| m.end()).unwrap_or(0);

    let rest = &after[m2.map(|m| m.end()).unwrap_or(0)..];
    let m3 = RE_SECTION_END.find(rest);
    let section_end = m3.map(|m| insert_pos + m.start()).unwrap_or(md.len());

    let section_content = &md[insert_pos..section_end].trim_start_matches('\n');

    let cleaned = RE_WIKILINK_LINE.replace_all(section_content, "");
    let cleaned = cleaned.trim();

    let new_section = if cleaned.is_empty() {
        link_line.trim_end().to_string()
    } else {
        format!("{}\n{}", link_line.trim_end(), cleaned)
    };

    format!("{}{}{}", &md[..insert_pos], new_section, &md[section_end..])
}

pub fn update_cnote_links(cnote_path: &Path, pnote_path: &Path) -> std::io::Result<()> {
    let md = read_text(cnote_path);
    let link_line = wikilink_for_pnote(pnote_path);
    let md2 = upsert_link_under_heading(&md, "关联笔记", &link_line);
    write_text(cnote_path, &md2)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_upsert_link_under_heading_existing_section() {
        // Test that a heading with existing content is found and the link is inserted
        let md = r#"# C - Transformer

## 核心定义

Some content here

## 关联笔记

## 技术本质

Other content
"#;
        let result = upsert_link_under_heading(md, "关联笔记", "- [[P - 2020 - GPT-3]]");
        // The section should be found and the link prepended
        assert!(result.contains("[[P - 2020 - GPT-3]]"));
        // Since section is empty (no wikilink present), the link is the section content
        assert!(result.contains("## 关联笔记"));
    }

    #[test]
    fn test_upsert_link_under_heading_no_section_creates_it() {
        let md = r#"# C - Transformer

## 核心定义

Content
"#;
        let result = upsert_link_under_heading(md, "关联笔记", "- [[P - 2020 - GPT-3]]");
        assert!(result.contains("## 关联笔记"));
        assert!(result.contains("[[P - 2020 - GPT-3]]"));
    }

    #[test]
    fn test_upsert_link_under_heading_empty_section() {
        let md = r#"# C - Transformer

## 关联笔记

"#;
        let result = upsert_link_under_heading(md, "关联笔记", "- [[P - 2020 - GPT-3]]");
        // Should prepend the link to empty section
        assert!(result.contains("[[P - 2020 - GPT-3]]"));
    }

    #[test]
    fn test_upsert_link_under_heading_strips_leading_hashes() {
        let md = r#"## 关联笔记

"#;
        let result = upsert_link_under_heading(md, "## 关联笔记", "- [[Test]]");
        assert!(result.contains("## 关联笔记"));
    }

    #[test]
    fn test_upsert_link_under_heading_preserves_other_sections() {
        let md = r#"# C - Transformer

## 核心定义

Definition content

## 关联笔记

## 技术本质

Technical content
"#;
        let result = upsert_link_under_heading(md, "关联笔记", "- [[P - Test]]");
        assert!(result.contains("## 核心定义"));
        assert!(result.contains("Definition content"));
        assert!(result.contains("## 技术本质"));
        assert!(result.contains("Technical content"));
    }

    #[test]
    fn test_upsert_link_removes_only_wikilink_lines() {
        let md = r#"## 关联笔记

- [[P - Old]]
Some manual note here
- Regular bullet
"#;
        let result = upsert_link_under_heading(md, "关联笔记", "- [[P - New]]");
        // Only the wikilink line should be removed, manual content preserved
        assert!(result.contains("Some manual note here"));
        assert!(result.contains("Regular bullet"));
        assert!(result.contains("[[P - New]]"));
    }

    #[test]
    fn test_ensure_cnote_creates_file() {
        let tmp_dir = std::env::temp_dir();
        let path = ensure_cnote(&tmp_dir, "TestConcept");
        assert!(path.exists());
        let content = std::fs::read_to_string(&path).unwrap();
        assert!(content.contains("# C - TestConcept"));
        assert!(content.contains("## 核心定义"));
        assert!(content.contains("## 产生背景"));
        assert!(content.contains("## 技术本质"));
        assert!(content.contains("## 关联笔记"));
        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn test_ensure_cnote_does_not_overwrite() {
        let tmp_dir = std::env::temp_dir();
        let path = tmp_dir.join("C - Existing.md");
        std::fs::write(&path, "Custom content").unwrap();
        let result = ensure_cnote(&tmp_dir, "Existing");
        assert_eq!(std::fs::read_to_string(&result).unwrap(), "Custom content");
        std::fs::remove_file(&path).ok();
    }
}
