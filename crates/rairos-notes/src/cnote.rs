//! C-Note creation and link management.

use crate::render::render_cnote;
use crate::pnote::wikilink_for_pnote;
use regex::Regex;
use std::path::Path;

const RE_LEADING_HASHES: &str = r"^#+\s+";
const RE_BLANK_LINES: &str = r"(\s*\n)*";
const RE_SECTION_END: &str = r"\n##\s+";
const RE_WIKILINK_LINE: &str = r"^-\s*\[\[[^\]]+\]\](?:[^\n]*)?\n?";

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
