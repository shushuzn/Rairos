//! C-Note creation and link management.

use crate::render::render_cnote;
use crate::pnote::wikilink_for_pnote;
use regex::Regex;
use std::path::Path;

const RE_LEADING_HASHES: &str = r"^#+\s+";
const RE_BLANK_LINES: &str = r"(\s*\n)*";
const RE_SECTION_END: &str = r"\n##\s+";
const RE_WIKILINK_LINE: &str = r"^-\s*\[\[[^\]]+\]\](?:[^\n]*)?\n?";
const RE_PLACEHOLDER_DASHES: &str = r"^[-–—\s]+$";
const RE_PUNCTUATION: &str = r"[.。?！]";
const RE_H2_LINE: &str = r"^(##\s+.+)$";

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
    let re_leading = Regex::new(RE_LEADING_HASHES).unwrap();
    let clean_heading = re_leading.replace(heading, "").trim().to_string();

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

    let re_blank = Regex::new(RE_BLANK_LINES).unwrap();
    let m2 = re_blank.find(after);
    let insert_pos = start + m2.map(|m| m.end()).unwrap_or(0);

    let re_section_end = Regex::new(RE_SECTION_END).unwrap();
    let rest = &after[m2.map(|m| m.end()).unwrap_or(0)..];
    let m3 = re_section_end.find(rest);
    let section_end = m3.map(|m| insert_pos + m.start()).unwrap_or(md.len());

    let section_content = &md[insert_pos..section_end].trim_start_matches('\n');

    let re_wikilink = Regex::new(RE_WIKILINK_LINE).unwrap();
    let cleaned = re_wikilink.replace_all(section_content, "");
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

fn section_is_empty(md: &str, section: &str) -> bool {
    let pattern = format!(r"(?:^|\n)(##\s+{}\s*\n)(.*?)(?=\n##\s+|\Z)", regex::escape(section));
    let re = Regex::new(&pattern).unwrap();

    let m = re.captures(md);
    if m.is_none() {
        return true;
    }

    let content = m.unwrap().get(2).map(|g| g.as_str().trim()).unwrap_or("");
    if content.is_empty() || Regex::new(RE_PLACEHOLDER_DASHES).unwrap().is_match(content) {
        return true;
    }
    if content.len() < 20 && !Regex::new(RE_PUNCTUATION).unwrap().is_match(content) {
        return true;
    }
    false
}

fn fill_cnote_section(md: &str, section: &str, new_content: &str) -> String {
    let pattern = format!(
        r"((?:^|\n)(##\s+{}\s*\n))(.*?)(?=\n##\s+|\Z)",
        regex::escape(section)
    );
    let re = Regex::new(&pattern).unwrap();

    let m = re.captures(md);
    if m.is_none() {
        return format!("{}\n\n## {}\n\n{}", md.trim_end(), section, new_content.trim());
    }

    let caps = m.unwrap();
    let heading = caps.get(1).unwrap().as_str();
    format!(
        "{}{}{}",
        &md[..caps.get(0).unwrap().start()],
        heading,
        new_content.trim()
    )
}

fn parse_cnote_sections(draft: &str) -> std::collections::HashMap<String, String> {
    let mut sections = std::collections::HashMap::new();
    let mut current_section: Option<String> = None;
    let mut current_content = Vec::new();

    let re_h2 = Regex::new(RE_H2_LINE).unwrap();

    for line in draft.lines() {
        if let Some(caps) = re_h2.captures(line) {
            if let Some(cs) = current_section.take() {
                sections.insert(cs, current_content.join("\n").trim().to_string());
            }
            current_section = Some(caps.get(1).unwrap().as_str().replace("##", "").trim().to_string());
            current_content.clear();
        } else if current_section.is_some() {
            current_content.push(line);
        }
    }

    if let Some(cs) = current_section {
        sections.insert(cs, current_content.join("\n").trim().to_string());
    }
    sections
}
